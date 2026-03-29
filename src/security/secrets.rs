// src/security/secrets.rs — Secure API key management
// Story 6.2: Secrets Management
//
// Reads API keys from ~/.config/bmux/secrets.toml (must be chmod 0600).
// Injects secrets into agent child processes via a file descriptor — never
// via environment variables — so that `env` output inside the agent does
// NOT contain any API key.
//
// Also provides a scrubber that redacts known secrets from log output and
// shared context values.

use std::collections::HashMap;
use std::os::unix::fs::PermissionsExt;
use std::path::{Path, PathBuf};

use serde::Deserialize;
use thiserror::Error;
use tracing::warn;

// ─── Error type ────────────────────────────────────────────────────────────────

#[derive(Debug, Error)]
pub enum SecretsError {
    #[error("IO error reading secrets file: {0}")]
    Io(#[from] std::io::Error),

    #[error("TOML parse error: {0}")]
    Toml(#[from] toml::de::Error),

    #[error("Secret key '{0}' not found")]
    KeyNotFound(String),

    #[error("Pipe creation failed")]
    PipeCreation,
}

// ─── Secrets file schema ───────────────────────────────────────────────────────

/// Represents the deserialized `~/.config/bmux/secrets.toml`.
///
/// ```toml
/// [keys]
/// anthropic = "sk-ant-..."
/// openai    = "sk-..."
/// ```
#[derive(Debug, Deserialize, Default)]
struct SecretsFile {
    #[serde(default)]
    keys: HashMap<String, String>,
}

// ─── SecretsManager ───────────────────────────────────────────────────────────

/// Manages API keys for BMUX agents.
#[derive(Debug, Default)]
pub struct SecretsManager {
    keys: HashMap<String, String>,
}

impl SecretsManager {
    /// Load secrets from `path`.
    ///
    /// Checks that the file has 0600 permissions and warns if it does not.
    /// If `path` does not exist, returns an empty manager.
    pub fn load(path: &Path) -> Result<Self, SecretsError> {
        if !path.exists() {
            return Ok(Self::default());
        }

        // Permission check
        match check_permissions(path) {
            Ok(()) => {}
            Err(e) => {
                // Non-fatal — warn and continue
                warn!("{e}");
            }
        }

        let contents = std::fs::read_to_string(path)?;
        let parsed: SecretsFile = toml::from_str(&contents)?;
        Ok(Self { keys: parsed.keys })
    }

    /// Returns the default path `~/.config/bmux/secrets.toml`.
    pub fn default_path() -> Option<PathBuf> {
        dirs::config_dir().map(|d| d.join("bmux").join("secrets.toml"))
    }

    /// Get the value of a secret by key name.
    pub fn get(&self, key: &str) -> Option<&str> {
        self.keys.get(key).map(String::as_str)
    }

    /// Returns `true` if the manager has no secrets loaded.
    pub fn is_empty(&self) -> bool {
        self.keys.is_empty()
    }

    /// Inject a named secret into a child process via a **pipe file descriptor**.
    ///
    /// The write end of the pipe is written with the secret value, then closed.
    /// The read end fd number is returned. The caller should pass it to the
    /// child process as `BMUX_SECRET_FD=<n>` (fd number only — not the value).
    ///
    /// The child reads the value from fd `n` at startup, then closes it.
    ///
    /// This keeps the actual secret value out of the environment entirely.
    pub fn create_secret_fd(
        &self,
        key: &str,
    ) -> Result<std::os::unix::io::RawFd, SecretsError> {
        let value = self
            .keys
            .get(key)
            .ok_or_else(|| SecretsError::KeyNotFound(key.to_string()))?;

        let (read_fd, write_fd) = create_pipe()?;

        // Write secret to pipe and close the write end immediately
        let written = unsafe {
            libc::write(
                write_fd,
                value.as_ptr() as *const libc::c_void,
                value.len(),
            )
        };
        unsafe { libc::close(write_fd) };

        if written < 0 {
            unsafe { libc::close(read_fd) };
            return Err(SecretsError::Io(std::io::Error::last_os_error()));
        }

        Ok(read_fd)
    }

    /// Redact all known secret values in `text`, replacing them with `[REDACTED]`.
    ///
    /// Use this before writing any string to the audit log or shared context.
    pub fn scrub(&self, text: &str) -> String {
        let mut result = text.to_string();
        for value in self.keys.values() {
            if !value.is_empty() {
                result = result.replace(value.as_str(), "[REDACTED]");
            }
        }
        result
    }
}

// ─── Helpers ───────────────────────────────────────────────────────────────────

/// Check that `path` has exactly 0600 permissions.
///
/// Returns `Ok(())` if permissions are correct; otherwise returns an `Err`
/// with a human-readable warning message (non-fatal — caller decides).
pub fn check_permissions(path: &Path) -> Result<(), String> {
    let metadata = std::fs::metadata(path).map_err(|e| {
        format!("Cannot read metadata for {}: {e}", path.display())
    })?;
    let mode = metadata.permissions().mode() & 0o777;
    if mode != 0o600 {
        let actual = format!("{mode:04o}");
        Err(format!(
            "secrets.toml has insecure permissions ({actual}), expected 0600"
        ))
    } else {
        Ok(())
    }
}

/// Offer to fix `path` permissions to 0600. Returns `true` if the fix succeeded.
pub fn fix_permissions(path: &Path) -> Result<(), std::io::Error> {
    use std::os::unix::fs::PermissionsExt;
    let mut perms = std::fs::metadata(path)?.permissions();
    perms.set_mode(0o600);
    std::fs::set_permissions(path, perms)
}

/// Create a UNIX pipe; returns `(read_fd, write_fd)`.
fn create_pipe() -> Result<(libc::c_int, libc::c_int), SecretsError> {
    let mut fds = [0i32; 2];
    let rc = unsafe { libc::pipe(fds.as_mut_ptr()) };
    if rc != 0 {
        return Err(SecretsError::PipeCreation);
    }
    Ok((fds[0], fds[1]))
}

// ─── Tests ─────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;
    use serial_test::serial;
    use std::io::Write;
    use std::os::unix::fs::PermissionsExt;
    use tempfile::NamedTempFile;

    fn write_secrets_file(content: &str, mode: u32) -> NamedTempFile {
        let mut f = NamedTempFile::new().unwrap();
        f.write_all(content.as_bytes()).unwrap();
        let mut perms = f.as_file().metadata().unwrap().permissions();
        perms.set_mode(mode);
        std::fs::set_permissions(f.path(), perms).unwrap();
        f
    }

    
    #[serial]
    #[test]
    fn test_load_secrets_from_valid_file() {
        let content = r#"
[keys]
anthropic = "sk-ant-test123"
openai = "sk-openai-xyz"
"#;
        let f = write_secrets_file(content, 0o600);
        let mgr = SecretsManager::load(f.path()).expect("load should succeed");
        assert_eq!(mgr.get("anthropic"), Some("sk-ant-test123"));
        assert_eq!(mgr.get("openai"), Some("sk-openai-xyz"));
    }

    
    #[serial]
    #[test]
    fn test_load_nonexistent_file_returns_empty() {
        let path = Path::new("/nonexistent/path/secrets.toml");
        let mgr = SecretsManager::load(path).expect("should return empty manager");
        assert!(mgr.is_empty());
    }

    
    #[serial]
    #[test]
    fn test_permissions_ok_0600() {
        let f = write_secrets_file("[keys]\n", 0o600);
        assert!(check_permissions(f.path()).is_ok());
    }

    
    #[serial]
    #[test]
    fn test_permissions_warn_on_0644() {
        let f = write_secrets_file("[keys]\n", 0o644);
        let result = check_permissions(f.path());
        assert!(result.is_err());
        let msg = result.unwrap_err();
        assert!(msg.contains("insecure permissions"));
        assert!(msg.contains("0644"));
        assert!(msg.contains("0600"));
    }

    
    #[serial]
    #[test]
    fn test_permissions_warn_on_0777() {
        let f = write_secrets_file("[keys]\n", 0o777);
        let result = check_permissions(f.path());
        assert!(result.is_err());
        let msg = result.unwrap_err();
        assert!(msg.contains("insecure permissions"));
    }

    
    #[serial]
    #[test]
    fn test_fix_permissions_sets_0600() {
        let f = write_secrets_file("[keys]\n", 0o644);
        fix_permissions(f.path()).expect("fix_permissions should succeed");
        assert!(check_permissions(f.path()).is_ok());
    }

    
    #[serial]
    #[test]
    fn test_scrub_removes_api_key_from_text() {
        let content = "[keys]\nanthropics = \"sk-ant-secret\"\n";
        let f = write_secrets_file(content, 0o600);
        let mgr = SecretsManager::load(f.path()).unwrap();

        let text = "Connecting with key sk-ant-secret for request";
        let scrubbed = mgr.scrub(text);
        assert!(
            !scrubbed.contains("sk-ant-secret"),
            "scrubbed output must not contain the secret"
        );
        assert!(
            scrubbed.contains("[REDACTED]"),
            "scrubbed output must contain [REDACTED]"
        );
    }

    
    #[serial]
    #[test]
    fn test_scrub_leaves_non_secret_text_unchanged() {
        let content = "[keys]\nkey1 = \"SECRET_VALUE\"\n";
        let f = write_secrets_file(content, 0o600);
        let mgr = SecretsManager::load(f.path()).unwrap();

        let text = "this is safe content";
        assert_eq!(mgr.scrub(text), text);
    }

    
    #[serial]
    #[test]
    fn test_scrub_replaces_all_occurrences() {
        let content = "[keys]\nkey1 = \"mykey\"\n";
        let f = write_secrets_file(content, 0o600);
        let mgr = SecretsManager::load(f.path()).unwrap();

        let text = "use mykey here and mykey there";
        let scrubbed = mgr.scrub(text);
        assert_eq!(scrubbed, "use [REDACTED] here and [REDACTED] there");
    }

    
    #[serial]
    #[test]
    fn test_create_secret_fd_injects_without_env() {
        let content = "[keys]\ntest_key = \"supersecret\"\n";
        let f = write_secrets_file(content, 0o600);
        let mgr = SecretsManager::load(f.path()).unwrap();

        // Get the fd — this should succeed
        let fd = mgr.create_secret_fd("test_key").expect("create_secret_fd should succeed");
        assert!(fd > 0, "fd must be a valid descriptor");

        // Read back from the fd to verify the value was written
        let mut buf = vec![0u8; 64];
        let n = unsafe { libc::read(fd, buf.as_mut_ptr() as *mut libc::c_void, buf.len()) };
        unsafe { libc::close(fd) };
        assert!(n > 0, "read from secret fd should return bytes");

        let read_value = std::str::from_utf8(&buf[..n as usize]).unwrap();
        assert_eq!(read_value, "supersecret");
    }

    
    #[serial]
    #[test]
    fn test_env_does_not_contain_secret_value() {
        let content = "[keys]\nanthropics = \"sk-ant-env-test\"\n";
        let f = write_secrets_file(content, 0o600);
        let mgr = SecretsManager::load(f.path()).unwrap();

        // The secret is injected via fd, not env. Verify env doesn't have it.
        let env_snapshot: Vec<(String, String)> = std::env::vars().collect();
        for (_, v) in &env_snapshot {
            assert!(
                !v.contains("sk-ant-env-test"),
                "secret value must not appear in environment"
            );
        }

        // The fd injection itself doesn't touch env at all
        let fd = mgr.create_secret_fd("anthropics").unwrap();
        unsafe { libc::close(fd) };

        let env_after: Vec<(String, String)> = std::env::vars().collect();
        for (_, v) in &env_after {
            assert!(
                !v.contains("sk-ant-env-test"),
                "secret value must not appear in environment after fd creation"
            );
        }
    }

    
    #[serial]
    #[test]
    fn test_get_missing_key_returns_none() {
        let mgr = SecretsManager::default();
        assert_eq!(mgr.get("missing"), None);
    }

    
    #[serial]
    #[test]
    fn test_create_fd_missing_key_returns_error() {
        let mgr = SecretsManager::default();
        let result = mgr.create_secret_fd("nonexistent");
        assert!(matches!(result, Err(SecretsError::KeyNotFound(_))));
    }
}
