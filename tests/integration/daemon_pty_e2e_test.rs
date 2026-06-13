use std::time::Duration;

use bmux::tui::ipc_client::{auth_for_socket, encode_signed_client, parse_signed_server, query};
use bmux::tui::protocol::{ClientMessage, ServerMessage};
use bmux::tui::server::DaemonServer;
use bmux::tui::session::{meta_path, socket_path};
use tokio::io::{AsyncBufReadExt, AsyncWriteExt, BufReader};
use tokio::net::UnixStream;

#[tokio::test]
async fn integration_daemon_pty_spawn_echo_output() {
    let name = format!("pty-e2e-{}", uuid::Uuid::new_v4());
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
    assert!(sock.exists());

    // Spawn a shell agent pane
    let spawn_result = query(
        &sock,
        ClientMessage::AgentSpawn {
            agent_type: "shell".to_string(),
            name: "e2e-shell".to_string(),
            model: None,
        },
    )
    .await
    .unwrap();
    assert!(
        spawn_result.contains("spawned successfully"),
        "spawn failed: {spawn_result}"
    );

    // Connect as TUI client, send echo command, poll snapshot for output
    let auth = auth_for_socket(&sock).unwrap();
    let stream = UnixStream::connect(&sock).await.unwrap();
    let (reader, mut writer) = stream.into_split();

    writer
        .write_all(
            encode_signed_client(&auth, ClientMessage::Init { rows: 24, cols: 80 })
                .unwrap()
                .as_bytes(),
        )
        .await
        .unwrap();

    let echo_cmd = b"echo BMUX_PTY_E2E_OK\n";
    writer
        .write_all(
            encode_signed_client(&auth, ClientMessage::Input { data: echo_cmd.to_vec() })
                .unwrap()
                .as_bytes(),
        )
        .await
        .unwrap();

    let mut lines = BufReader::new(reader).lines();
    let mut saw_marker = false;
    let deadline = tokio::time::Instant::now() + Duration::from_secs(5);

    while tokio::time::Instant::now() < deadline {
        tokio::select! {
            line = lines.next_line() => {
                if let Ok(Some(line)) = line {
                    if let Ok(ServerMessage::State(snap)) = parse_signed_server(&auth, &line) {
                        for win in &snap.windows {
                            for pane in &win.pane_cells {
                                let text: String = pane
                                    .cells
                                    .iter()
                                    .flatten()
                                    .map(|c| if c.ch == '\0' { ' ' } else { c.ch })
                                    .collect();
                                if text.contains("BMUX_PTY_E2E_OK") {
                                    saw_marker = true;
                                    break;
                                }
                            }
                        }
                    }
                }
                if saw_marker {
                    break;
                }
            }
            _ = tokio::time::sleep(Duration::from_millis(50)) => {
                writer
                    .write_all(
                        encode_signed_client(&auth, ClientMessage::GetState)
                            .unwrap()
                            .as_bytes(),
                    )
                    .await
                    .unwrap();
            }
        }
    }

    assert!(saw_marker, "PTY output should contain echo marker");

    handle.abort();
    let _ = std::fs::remove_file(&sock);
    let _ = std::fs::remove_file(&meta);
}