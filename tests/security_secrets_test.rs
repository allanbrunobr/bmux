// tests/security_secrets_test.rs — Integration tests for Story 6.2
// Secrets Management

use bmux::security::secrets::{check_permissions, fix_permissions, SecretsManager};
use std::io::Write;
use std::os::unix::fs::PermissionsExt;
use tempfile::NamedTempFile;

fn write_secrets(content: &str, mode: u32) -> NamedTempFile {
    let mut f = NamedTempFile::new().unwrap();
    f.write_all(content.as_bytes()).unwrap();
    let mut perms = f.as_file().metadata().unwrap().permissions();
    perms.set_mode(mode);
    std::fs::set_permissions(f.path(), perms).unwrap();
    f
}

#[test]
fn integration_secrets_load_and_retrieve() {
    let content = "[keys]\napi_key = \"sk-real-key-abc123\"\n";
    let f = write_secrets(content, 0o600);
    let mgr = SecretsManager::load(f.path()).expect("must load");
    assert_eq!(mgr.get("api_key"), Some("sk-real-key-abc123"));
}

#[test]
fn integration_secrets_missing_file_returns_empty() {
    use std::path::Path;
    let mgr = SecretsManager::load(Path::new("/no/such/file.toml")).unwrap();
    assert!(mgr.is_empty());
}

#[test]
fn integration_secrets_0600_permission_passes() {
    let f = write_secrets("[keys]\n", 0o600);
    assert!(
        check_permissions(f.path()).is_ok(),
        "0600 permissions should pass"
    );
}

#[test]
fn integration_secrets_bad_permissions_warn_message() {
    let f = write_secrets("[keys]\n", 0o644);
    let err = check_permissions(f.path()).unwrap_err();
    assert!(
        err.contains("insecure permissions"),
        "warning must say 'insecure permissions'"
    );
    assert!(
        err.contains("0644"),
        "warning must include actual permissions"
    );
    assert!(err.contains("0600"), "warning must include expected permissions");
}

#[test]
fn integration_secrets_fix_permissions_sets_0600() {
    let f = write_secrets("[keys]\n", 0o666);
    fix_permissions(f.path()).unwrap();
    assert!(
        check_permissions(f.path()).is_ok(),
        "after fix, permissions must be 0600"
    );
}

#[test]
fn integration_secrets_scrub_api_key_from_audit_candidate() {
    let content = "[keys]\nanthropics = \"sk-ant-SUPERSECRET\"\n";
    let f = write_secrets(content, 0o600);
    let mgr = SecretsManager::load(f.path()).unwrap();

    // Simulate an audit log line that might contain the secret
    let log_line = r#"{"event":"task_sent","payload":"use key sk-ant-SUPERSECRET"}"#;
    let scrubbed = mgr.scrub(log_line);

    assert!(
        !scrubbed.contains("sk-ant-SUPERSECRET"),
        "scrubbed audit line must not contain the API key"
    );
    assert!(
        scrubbed.contains("[REDACTED]"),
        "scrubbed audit line must contain [REDACTED]"
    );
}

#[test]
fn integration_secrets_context_value_scrubbed() {
    let content = "[keys]\nkey = \"SECRET123\"\n";
    let f = write_secrets(content, 0o600);
    let mgr = SecretsManager::load(f.path()).unwrap();

    // Simulate a context store value that leaked the key
    let ctx_value = "result=some_output_with_SECRET123_inside";
    let clean = mgr.scrub(ctx_value);
    assert!(!clean.contains("SECRET123"));
}

#[test]
fn integration_secrets_fd_injection_does_not_set_env() {
    let content = "[keys]\nmykey = \"env-test-value-xyz\"\n";
    let f = write_secrets(content, 0o600);
    let mgr = SecretsManager::load(f.path()).unwrap();

    // Snapshot environment BEFORE
    let env_before: std::collections::HashMap<_, _> = std::env::vars().collect();

    let fd = mgr.create_secret_fd("mykey").expect("create_secret_fd must succeed");
    unsafe { libc::close(fd) };

    // Snapshot environment AFTER
    let env_after: std::collections::HashMap<_, _> = std::env::vars().collect();

    // No new env vars containing the secret value should have appeared
    for (k, v) in &env_after {
        assert!(
            !v.contains("env-test-value-xyz"),
            "env var {k} must not contain the secret value"
        );
    }
    // The env should be unchanged
    assert_eq!(env_before.len(), env_after.len(), "env must not gain new vars");
}
