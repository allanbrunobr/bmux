//! Structured agent spawn for PTY panes — env_clear, minimal allowlist, optional sandbox wrap.
//!
//! Note: `portable-pty` closes all fds ≥ 3 before exec, so `create_secret_fd` cannot be
//! passed through this path; agents receive `BMUX_SECRETS_PATH` instead.

use std::path::Path;

use portable_pty::CommandBuilder;
use tracing::warn;

use super::envelope::SecurityEnvelope;

/// Build a [`CommandBuilder`] for direct PTY spawn (no shell line injection).
pub fn build_agent_pty_command(
    envelope: &SecurityEnvelope,
    ipc_socket: &Path,
    agent_name: &str,
    agent_type: &str,
    binary: &str,
    args: &[String],
    model: &str,
) -> CommandBuilder {
    let inner = if agent_type == "shell" {
        build_shell_command(binary, args)
    } else {
        build_isolated_command(envelope, ipc_socket, agent_name, agent_type, binary, args, model)
    };

    wrap_sandbox(envelope, agent_name, agent_type, inner)
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
        let cmd = build_agent_pty_command(
            &envelope,
            Path::new("/tmp/ipc.sock"),
            "arch",
            "claude-code",
            "claude",
            &[],
            "claude-sonnet-4-20250514",
        );
        let argv = cmd.get_argv();
        assert_eq!(argv.first().map(|s| s.to_str()), Some(Some("claude")));
        assert!(cmd.get_env("BMUX_SESSION").is_some());
        assert!(cmd.get_env("BMUX_PROJECT_DIR").is_some());
    }
}