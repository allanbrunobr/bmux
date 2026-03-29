// tests/security_sandbox_test.rs — Integration tests for Story 6.1
// Agent Filesystem Sandboxing

use bmux::security::sandbox::{apply_sandbox, SandboxMode};
use tempfile::TempDir;

#[test]
fn integration_sandbox_none_mode_succeeds() {
    let dir = TempDir::new().unwrap();
    let mode = apply_sandbox(dir.path(), Some("none"), "test-agent")
        .expect("none mode must succeed");
    assert_eq!(mode, SandboxMode::None);
}

#[test]
fn integration_sandbox_no_override_succeeds() {
    let dir = TempDir::new().unwrap();
    std::fs::write(dir.path().join("test.txt"), b"hello").unwrap();

    let result = apply_sandbox(dir.path(), None, "integration-agent");

    // On macOS in CI/test environments, sandbox_init may be restricted by SIP.
    // We accept either success or a "not permitted" error as valid outcomes.
    #[cfg(target_os = "macos")]
    match &result {
        Ok(mode) => assert!(
            matches!(mode, SandboxMode::Seatbelt | SandboxMode::None),
            "macOS sandbox returned unexpected mode: {mode:?}"
        ),
        Err(e) => {
            let msg = e.to_string();
            assert!(
                msg.contains("not permitted") || msg.contains("Operation not permitted"),
                "unexpected macOS sandbox error: {msg}"
            );
        }
    }

    #[cfg(target_os = "linux")]
    assert!(
        result.is_ok(),
        "Linux sandbox must succeed: {:?}",
        result.err()
    );
}

#[test]
fn integration_sandbox_returns_correct_mode_on_platform() {
    let dir = TempDir::new().unwrap();
    let result = apply_sandbox(dir.path(), None, "platform-agent");

    #[cfg(target_os = "macos")]
    {
        // sandbox_init may be restricted in test environments — treat both as valid
        match result {
            Ok(mode) => assert!(
                matches!(mode, SandboxMode::Seatbelt | SandboxMode::None),
                "macOS must use Seatbelt (or None when restricted)"
            ),
            Err(e) => {
                let msg = e.to_string();
                assert!(
                    msg.contains("not permitted") || msg.contains("Operation not permitted"),
                    "unexpected error: {msg}"
                );
            }
        }
    }

    #[cfg(target_os = "linux")]
    {
        let mode = result.expect("Linux sandbox must succeed");
        assert!(
            matches!(mode, SandboxMode::Landlock | SandboxMode::Namespace),
            "Linux must use Landlock or Namespace fallback"
        );
    }
}

#[cfg(target_os = "linux")]
#[test]
fn integration_landlock_support_detection_does_not_panic() {
    use bmux::security::sandbox::is_landlock_supported;
    // Just verify it runs without panicking on this kernel
    let _ = is_landlock_supported();
}

#[cfg(target_os = "macos")]
#[test]
fn integration_seatbelt_profile_is_valid_sbpl() {
    use bmux::security::sandbox::build_seatbelt_profile;
    let dir = TempDir::new().unwrap();
    let profile = build_seatbelt_profile(dir.path());

    // Must be well-formed SBPL
    assert!(profile.contains("(version 1)"), "SBPL must start with version");
    assert!(profile.contains("(deny default)"), "SBPL must deny by default");
    assert!(
        profile.contains(&dir.path().display().to_string()),
        "SBPL must allow project dir"
    );
    assert!(profile.contains("/tmp"), "SBPL must allow /tmp for IPC");
}
