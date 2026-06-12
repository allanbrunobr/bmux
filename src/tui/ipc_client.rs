/// Lightweight IPC client for CLI subcommands.
///
/// Connects to a session daemon's Unix socket, sends a single query,
/// and returns the response. Used by `bmux agent list`, `bmux task send`, etc.
use std::path::{Path, PathBuf};

use anyhow::{Context, Result};
use tokio::io::{AsyncBufReadExt, AsyncWriteExt, BufReader};
use tokio::net::UnixStream;
use tokio::time::{timeout, Duration};

use super::protocol::{ClientMessage, ServerMessage};
use super::session::{list_sessions, socket_path};

/// Resolve which session to connect to.
///
/// Priority:
/// 1. Explicit `--session` / `-s` flag
/// 2. `BMUX_SESSION` environment variable
/// 3. Auto-detect if exactly one session is running
pub fn resolve_session(explicit: Option<&str>) -> Result<PathBuf> {
    // 1. Explicit flag
    if let Some(name) = explicit {
        let sock = socket_path(name);
        if sock.exists() {
            return Ok(sock);
        }
        anyhow::bail!(
            "No session named '{}' is running. Use `bmux ls` to see active sessions.",
            name
        );
    }

    // 2. Environment variable
    if let Ok(name) = std::env::var("BMUX_SESSION") {
        let sock = socket_path(&name);
        if sock.exists() {
            return Ok(sock);
        }
        anyhow::bail!(
            "BMUX_SESSION='{}' but no such session is running.",
            name
        );
    }

    // 3. Auto-detect single session
    let sessions = list_sessions();
    match sessions.len() {
        0 => anyhow::bail!(
            "No active bmux sessions. Start one with `bmux new <name>`."
        ),
        1 => Ok(sessions[0].socket_path.clone()),
        n => anyhow::bail!(
            "{n} sessions running. Specify which one with `-s <name>` or set BMUX_SESSION.\n\
             Active sessions: {}",
            sessions.iter().map(|s| s.name.as_str()).collect::<Vec<_>>().join(", ")
        ),
    }
}

/// Send a query to a session daemon and return the response string.
///
/// Opens a new connection, sends one `ClientMessage`, reads one `ServerMessage`,
/// then closes. This is intentionally short-lived — CLI commands are fire-and-forget.
const QUERY_TIMEOUT: Duration = Duration::from_secs(5);

pub async fn query(sock_path: &Path, msg: ClientMessage) -> Result<String> {
    timeout(QUERY_TIMEOUT, query_inner(sock_path, msg))
        .await
        .map_err(|_| anyhow::anyhow!("Session query timed out after {}s", QUERY_TIMEOUT.as_secs()))?
}

async fn query_inner(sock_path: &Path, msg: ClientMessage) -> Result<String> {
    let stream = UnixStream::connect(sock_path)
        .await
        .with_context(|| format!("Cannot connect to session at {}", sock_path.display()))?;

    let (reader, mut writer) = stream.into_split();
    let mut reader = BufReader::new(reader);

    let json = serde_json::to_string(&msg)?;
    writer.write_all(json.as_bytes()).await?;
    writer.write_all(b"\n").await?;
    writer.flush().await?;

    let mut line = String::new();
    reader.read_line(&mut line).await?;

    if line.is_empty() {
        anyhow::bail!("Session closed connection without responding.");
    }

    let response: ServerMessage = serde_json::from_str(line.trim())?;

    match response {
        ServerMessage::QueryResult { data } => Ok(data),
        ServerMessage::Error { msg } => anyhow::bail!("{}", msg),
        other => anyhow::bail!("Unexpected response: {:?}", other),
    }
}
