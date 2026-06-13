//! Structured agent spawn for PTY panes — env_clear, minimal allowlist, optional sandbox wrap.
//!
//! Secret and IPC keys are injected via preserved pipe fds (`BMUX_SECRET_FD`, `BMUX_IPC_HMAC_FD`)
//! using `tui::pty_spawn` (selective fd close instead of `portable-pty`'s `close_random_fds`).

#[cfg(unix)]
use std::os::unix::io::RawFd;
use std::path::Path;

use portable_pty::CommandBuilder;
use tracing::warn;

use super::envelope::SecurityEnvelope;

/// Command plus pipe fds to preserve into the child at slots 3..N.
#[derive(Debug)]
pub struct AgentPtySpawn {
    pub command: CommandBuilder,
    #[cfg(unix)]
    pub inject_fds: Vec<RawFd>,
    #[cfg(not(unix))]
    pub inject_fds: Vec<i32>,
}

/// Build a structured PTY spawn bundle for direct spawn (no shell line injection).
#[allow(clippy::too_many_arguments)]
pub fn build_agent_pty_command(
    envelope: &SecurityEnvelope,
    ipc_socket: &Path,
    agent_name: &str,
    agent_type: &str,
    binary: &str,
    args: &[String],
    model: &str,
    ipc_hmac_key: Option<&[u8]>,
) -> AgentPtySpawn {
    let mut inject_fds = Vec::new();
    let inner = if agent_type == "shell" {
        build_shell_command(binary, args)
    } else {
        build_isolated_command(envelope, ipc_socket, agent_name, agent_type, binary, args, model)
    };

    let mut cmd = wrap_sandbox(envelope, agent_name, agent_type, inner);

    #[cfg(unix)]
    if agent_type != "shell" {
        attach_secret_fd(envelope, &mut cmd, &mut inject_fds);
        attach_ipc_hmac_fd(ipc_hmac_key, &mut cmd, &mut inject_fds);
    }

    AgentPtySpawn {
        command: cmd,
        inject_fds,
    }
}

#[cfg(unix)]
fn attach_secret_fd(
    envelope: &SecurityEnvelope,
    cmd: &mut CommandBuilder,
    inject_fds: &mut Vec<RawFd>,
) {
    const PRIMARY_KEYS: &[&str] = &["anthropics", "openai", "google"];
    for key in PRIMARY_KEYS {
        if let Ok(fd) = envelope.secrets.create_secret_fd(key) {
            let slot = 3 + inject_fds.len() as RawFd;
            inject_fds.push(fd);
            cmd.env("BMUX_SECRET_FD", slot.to_string());
            cmd.env_remove("BMUX_SECRETS_PATH");
            return;
        }
    }
}

#[cfg(unix)]
fn attach_ipc_hmac_fd(
    key: Option<&[u8]>,
    cmd: &mut CommandBuilder,
    inject_fds: &mut Vec<RawFd>,
) {
    let Some(key_bytes) = key else { return };
    if key_bytes.len() < 32 {
        return;
    }
    let (read_fd, write_fd) = match create_pipe() {
        Ok(pair) => pair,
        Err(_) => return,
    };
    let written = unsafe {
        libc::write(
            write_fd,
            key_bytes.as_ptr() as *const libc::c_void,
            key_bytes.len(),
        )
    };
    unsafe {
        libc::close(write_fd);
    }
    if written < 0 {
        unsafe {
            libc::close(read_fd);
        }
        return;
    }
    let slot = 3 + inject_fds.len() as RawFd;
    inject_fds.push(read_fd);
    cmd.env("BMUX_IPC_HMAC_FD", slot.to_string());
}

#[cfg(unix)]
fn create_pipe() -> std::io::Result<(RawFd, RawFd)> {
    let mut fds = [0i32; 2];
    let rc = unsafe { libc::pipe(fds.as_mut_ptr()) };
    if rc != 0 {
        return Err(std::io::Error::last_os_error());
    }
    Ok((fds[0], fds[1]))
}

fn build_shell_command(binary: &str, args: &[String]) -> CommandBuilder {
    let mut cmd = CommandBuilder::new(binary);
    for arg in args {
        cmd.arg(arg);
    }
    cmd
}

fn build_isolated_command(
    envelope: &SecurityEnvelope,
    ipc_socket: &Path,
    agent_name: &str,
    agent_type: &str,
    binary: &str,
    args: &[String],
    model: &str,
) -> CommandBuilder {
    let mut cmd = CommandBuilder::new(binary);
    cmd.env_clear();
    cmd.cwd(&envelope.project_dir);

    for (key, value) in envelope.bmux_env_pairs(ipc_socket, agent_name, agent_type) {
        cmd.env(key, value);
    }
    cmd.env("TERM", "xterm-256color");

    for arg in args {
        cmd.arg(arg);
    }
    if !model.is_empty() {
        cmd.arg("--model");
        cmd.arg(model);
    }

    cmd
}

fn wrap_sandbox(
    envelope: &SecurityEnvelope,
    agent_name: &str,
    agent_type: &str,
    inner: CommandBuilder,
) -> CommandBuilder {
    if agent_type == "shell" || envelope.security.sandbox == "none" {
        return inner;
    }

    #[cfg(target_os = "macos")]
    {
        use std::io::Write;
        let profile = crate::security::sandbox::build_seatbelt_profile(&envelope.project_dir);
        let path = std::env::temp_dir().join(format!(
            "bmux-sandbox-{}-{}.sb",
            envelope.session_id.replace(['/', '\\', ':'], "_"),
            agent_name.replace(['/', '\\', ':'], "_"),
        ));
        if let Ok(mut f) = std::fs::File::create(&path) {
            if f.write_all(profile.as_bytes()).is_ok() {
                let mut wrap = CommandBuilder::new("sandbox-exec");
                wrap.arg("-f");
                wrap.arg(path.to_string_lossy().to_string());
                for arg in inner.get_argv() {
                    wrap.arg(arg);
                }
                return wrap;
            }
        }
        warn!(agent = %agent_name, "Failed to write Seatbelt profile; spawning without sandbox");
        inner
    }

    #[cfg(target_os = "linux")]
    {
        if which::which("bwrap").is_ok() {
            return wrap_with_bwrap(envelope, inner);
        }
        warn!(
            sandbox = %envelope.security.sandbox,
            "bwrap not found; spawning with env -i only (no Landlock in child)"
        );
        inner
    }

    #[cfg(not(any(target_os = "macos", target_os = "linux")))]
    {
        let _ = (agent_name, envelope);
        inner
    }
}

#[cfg(target_os = "linux")]
fn wrap_with_bwrap(envelope: &SecurityEnvelope, inner: CommandBuilder) -> CommandBuilder {
    let project = envelope.project_dir.display().to_string();
    let mut wrap = CommandBuilder::new("bwrap");
    wrap.arg("--unshare-all");
    wrap.arg("--die-with-parent");
    wrap.arg("--new-session");
    wrap.arg("--bind").arg(&project).arg(&project);
    wrap.arg("--bind").arg("/tmp").arg("/tmp");
    for sys in ["/usr", "/lib", "/lib64", "/bin", "/sbin", "/etc", "/dev", "/proc"] {
        if Path::new(sys).exists() {
            if sys == "/dev" || sys == "/proc" {
                wrap.arg("--dev-bind").arg(sys).arg(sys);
            } else {
                wrap.arg("--ro-bind").arg(sys).arg(sys);
            }
        }
    }
    wrap.arg("--");
    for arg in inner.get_argv() {
        wrap.arg(arg);
    }
    wrap
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::config::settings::SecurityConfig;
    use std::path::PathBuf;

    #[test]
    fn isolated_pty_command_clears_env() {
        let envelope = SecurityEnvelope::new(
            "sess",
            PathBuf::from("/tmp/proj"),
            SecurityConfig {
                sandbox: "none".to_string(),
                allow_dangerous_permissions: false,
            },
            false,
        );
        let spawn = build_agent_pty_command(
            &envelope,
            Path::new("/tmp/ipc.sock"),
            "arch",
            "claude-code",
            "claude",
            &[],
            "claude-sonnet-4-20250514",
            None,
        );
        let argv = spawn.command.get_argv();
        assert_eq!(argv.first().map(|s| s.to_str()), Some(Some("claude")));
        assert!(spawn.command.get_env("BMUX_SESSION").is_some());
        assert!(spawn.command.get_env("BMUX_PROJECT_DIR").is_some());
    }
}