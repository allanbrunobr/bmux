#[cfg(unix)]
use std::os::unix::fs::PermissionsExt;
#[cfg(unix)]
use std::time::Duration;

#[cfg(unix)]
use bmux::security::secrets::SecretsManager;
#[cfg(unix)]
use bmux::tui::pty_spawn;
#[cfg(unix)]
use portable_pty::CommandBuilder;

#[cfg(unix)]
#[test]
fn integration_secret_fd_survives_pty_spawn() {
    let dir = tempfile::tempdir().unwrap();
    let secrets_path = dir.path().join("secrets.toml");
    std::fs::write(&secrets_path, "[keys]\nanthropics = \"sk-test-secret\"\n").unwrap();
    std::fs::set_permissions(&secrets_path, std::fs::Permissions::from_mode(0o600)).unwrap();

    let mgr = SecretsManager::load(&secrets_path).unwrap();
    let fd = mgr.create_secret_fd("anthropics").expect("create_secret_fd");

    let mut cmd = CommandBuilder::new("/bin/sh");
    cmd.arg("-c");
    cmd.arg("read -r s <&3; echo SECRET:$s");

    let result = pty_spawn::spawn_with_preserved_fds(8, 40, cmd, vec![fd]).unwrap();
    for parent_fd in result.parent_fds_to_close {
        unsafe {
            libc::close(parent_fd);
        }
    }

    let mut reader = result.master.try_clone_reader().unwrap();
    let mut buf = Vec::new();
    std::thread::sleep(Duration::from_millis(300));
    let _ = std::io::Read::read_to_end(&mut reader, &mut buf);
    let output = String::from_utf8_lossy(&buf);
    assert!(
        output.contains("SECRET:sk-test-secret"),
        "expected secret via preserved fd 3, got: {output}"
    );
}