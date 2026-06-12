use std::time::Duration;

use bmux::tui::ipc_client::query;
use bmux::tui::protocol::ClientMessage;
use bmux::tui::server::DaemonServer;
use bmux::tui::session::{meta_path, socket_path};

#[tokio::test]
async fn integration_daemon_context_namespacing() {
    let name = format!("ctx-{}", uuid::Uuid::new_v4());
    let sock = socket_path(&name);
    let meta = meta_path(&name);
    let _ = std::fs::remove_file(&sock);
    let _ = std::fs::remove_file(&meta);

    let server = DaemonServer::new(&name, 24, 80).unwrap();
    let handle = tokio::spawn(async move {
        let _ = server.run().await;
    });

    for _ in 0..100 {
        if sock.exists() {
            break;
        }
        tokio::time::sleep(Duration::from_millis(20)).await;
    }
    assert!(sock.exists(), "daemon socket should exist");

    let denied = query(
        &sock,
        ClientMessage::ContextSet {
            key: "agent:arch:notes".to_string(),
            value: "blocked".to_string(),
            author: Some("testador".to_string()),
        },
    )
    .await
    .unwrap();
    assert!(
        denied.contains("Error"),
        "cross-agent write should be denied: {denied}"
    );

    let ok = query(
        &sock,
        ClientMessage::ContextSet {
            key: "agent:arch:notes".to_string(),
            value: "allowed".to_string(),
            author: Some("arch".to_string()),
        },
    )
    .await
    .unwrap();
    assert!(ok.contains("Set"), "owner write should succeed: {ok}");

    let value = query(
        &sock,
        ClientMessage::ContextGet {
            key: "agent:arch:notes".to_string(),
        },
    )
    .await
    .unwrap();
    assert_eq!(value, "allowed");

    handle.abort();
    let _ = std::fs::remove_file(&sock);
    let _ = std::fs::remove_file(&meta);
}