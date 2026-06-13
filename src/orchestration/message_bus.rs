use std::collections::HashMap;
use std::os::unix::fs::PermissionsExt;
use std::path::PathBuf;
use std::sync::Arc;

use anyhow::{anyhow, Result};
use chrono::{DateTime, Utc};
use ring::hmac;
// ring::rand no longer needed — key gen delegates to security::hmac
use serde::{Deserialize, Serialize};
use tokio::io::{AsyncBufReadExt, AsyncWriteExt, BufReader};
use tokio::net::{UnixListener, UnixStream};
use tokio::sync::{broadcast, RwLock};
use tracing::{error, info, warn};

// ---------------------------------------------------------------------------
// Canonical agent status from agents module
use crate::agents::adapter::AgentStatus;

/// Alias for backward compatibility within orchestration layer.
pub type AgentState = AgentStatus;

// ---------------------------------------------------------------------------
// Message protocol
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "snake_case")]
pub enum MessageType {
    Task,
    Result,
    Status,
    ContextUpdate,
    Heartbeat,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TaskPayload {
    pub content: String,
    pub priority: u8,
    pub preferred_model: Option<String>,
    pub max_cost_usd: Option<f64>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ResultPayload {
    pub content: String,
    pub tokens_used: u64,
    pub cost_usd: f64,
    pub duration_ms: u64,
    pub task_id: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StatusPayload {
    pub state: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HeartbeatPayload {
    pub agent_id: String,
    pub uptime_seconds: u64,
}

/// Wire-format IPC message. The `hmac` field contains the hex-encoded
/// HMAC-SHA256 of the canonical JSON of all other fields.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct IpcMessage {
    pub id: String,
    pub from: String,
    pub to: String,
    #[serde(rename = "type")]
    pub msg_type: MessageType,
    pub payload: serde_json::Value,
    pub timestamp: DateTime<Utc>,
    pub hmac: String,
}

// ---------------------------------------------------------------------------
// HMAC helpers
// ---------------------------------------------------------------------------

/// Generate a fresh 32-byte HMAC-SHA256 key.
/// Delegates to the canonical `security::hmac` module (ADR-001 compliance).
pub fn generate_session_key() -> Result<hmac::Key> {
    let key_bytes = crate::security::hmac::generate_key()
        .map_err(|e| anyhow!("HMAC key generation failed: {e}"))?;
    Ok(hmac::Key::new(hmac::HMAC_SHA256, &key_bytes))
}

/// Produce the bytes that are signed: all fields except `hmac` itself.
fn signable_bytes(msg: &IpcMessage) -> Result<Vec<u8>> {
    let signable = serde_json::json!({
        "id":        &msg.id,
        "from":      &msg.from,
        "to":        &msg.to,
        "type":      &msg.msg_type,
        "payload":   &msg.payload,
        "timestamp": &msg.timestamp,
    });
    Ok(serde_json::to_vec(&signable)?)
}

/// Sign a message and embed the hex HMAC into the `hmac` field.
pub fn sign_message(key: &hmac::Key, msg: &mut IpcMessage) -> Result<()> {
    let bytes = signable_bytes(msg)?;
    let tag = hmac::sign(key, &bytes);
    msg.hmac = hex::encode(tag.as_ref());
    Ok(())
}

/// Verify a message's HMAC. Returns `Ok(true)` if valid, `Ok(false)` if tampered.
pub fn verify_message(key: &hmac::Key, msg: &IpcMessage) -> Result<bool> {
    let bytes = signable_bytes(msg)?;
    let tag_bytes =
        hex::decode(&msg.hmac).map_err(|e| anyhow!("Invalid HMAC hex: {e}"))?;
    Ok(hmac::verify(key, &bytes, &tag_bytes).is_ok())
}

// ---------------------------------------------------------------------------
// Message bus
// ---------------------------------------------------------------------------

pub type AgentStatusMap = Arc<RwLock<HashMap<String, AgentState>>>;

/// Broadcast channel capacity.
const CHANNEL_CAPACITY: usize = 1024;

/// A running MessageBus handles a Unix-socket IPC server.
///
/// Call [`MessageBus::start`] to start accepting connections and routing
/// messages. Subscribe with [`MessageBus::subscribe`] to receive validated
/// inbound messages.
pub struct MessageBus {
    session_id: String,
    socket_path: PathBuf,
    hmac_key: Arc<hmac::Key>,
    hmac_key_bytes: Arc<Vec<u8>>,
    agent_status: AgentStatusMap,
    tx: broadcast::Sender<IpcMessage>,
}

impl MessageBus {
    /// Create a new MessageBus for `session_id`.
    ///
    /// Generates a fresh HMAC key and computes the socket path
    /// `/tmp/bmux-{session_id}.sock`. Does NOT bind the socket yet.
    pub fn new(session_id: impl Into<String>) -> Result<Self> {
        let session_id = session_id.into();
        // Use a separate socket for the agent IPC bus (distinct from the TUI daemon socket)
        let socket_path =
            PathBuf::from(format!("/tmp/bmux-{}-ipc.sock", session_id));
        let key_bytes = crate::security::hmac::generate_key()
            .map_err(|e| anyhow!("HMAC key generation failed: {e}"))?;
        let hmac_key = Arc::new(hmac::Key::new(hmac::HMAC_SHA256, &key_bytes));
        let (tx, _) = broadcast::channel(CHANNEL_CAPACITY);
        Ok(Self {
            session_id,
            socket_path,
            hmac_key,
            hmac_key_bytes: Arc::new(key_bytes),
            agent_status: Arc::new(RwLock::new(HashMap::new())),
            tx,
        })
    }

    /// Socket path used by this bus.
    pub fn socket_path(&self) -> &PathBuf {
        &self.socket_path
    }

    /// HMAC key — hand this to spawned agents so they can sign messages.
    pub fn hmac_key(&self) -> Arc<hmac::Key> {
        Arc::clone(&self.hmac_key)
    }

    /// Raw HMAC key bytes for fd injection into agent children (`BMUX_IPC_HMAC_FD`).
    pub fn hmac_key_bytes(&self) -> &[u8] {
        &self.hmac_key_bytes
    }

    /// Subscribe to validated inbound messages.
    pub fn subscribe(&self) -> broadcast::Receiver<IpcMessage> {
        self.tx.subscribe()
    }

    /// Shared agent status map.
    pub fn agent_status(&self) -> AgentStatusMap {
        Arc::clone(&self.agent_status)
    }

    /// Bind the Unix socket and start accepting client connections.
    ///
    /// Removes any stale socket file first, then creates the socket
    /// with permissions 0600.
    pub async fn start(&self) -> Result<()> {
        // Remove stale socket if present
        if self.socket_path.exists() {
            std::fs::remove_file(&self.socket_path)?;
        }

        let listener = UnixListener::bind(&self.socket_path)?;

        // Restrict permissions to owner-only (0600)
        std::fs::set_permissions(
            &self.socket_path,
            std::fs::Permissions::from_mode(0o600),
        )?;

        info!(
            session = %self.session_id,
            path = %self.socket_path.display(),
            "MessageBus listening"
        );

        let hmac_key = Arc::clone(&self.hmac_key);
        let tx = self.tx.clone();
        let agent_status = Arc::clone(&self.agent_status);

        tokio::spawn(async move {
            loop {
                match listener.accept().await {
                    Ok((stream, _addr)) => {
                        let key = Arc::clone(&hmac_key);
                        let tx2 = tx.clone();
                        let status = Arc::clone(&agent_status);
                        tokio::spawn(handle_connection(stream, key, tx2, status));
                    }
                    Err(e) => {
                        error!(err = %e, "MessageBus accept error");
                        break;
                    }
                }
            }
        });

        Ok(())
    }

    /// Send a message to the socket server (useful for the CLI and tests).
    pub async fn send_message(&self, msg: &IpcMessage) -> Result<()> {
        let mut stream = UnixStream::connect(&self.socket_path).await?;
        let json = serde_json::to_string(msg)?;
        stream.write_all(json.as_bytes()).await?;
        stream.write_all(b"\n").await?;
        Ok(())
    }

    /// Stop the bus by removing the socket file. Existing connections finish.
    pub fn stop(&self) {
        let _ = std::fs::remove_file(&self.socket_path);
    }
}

// ---------------------------------------------------------------------------
// Per-connection handler
// ---------------------------------------------------------------------------

async fn handle_connection(
    stream: UnixStream,
    hmac_key: Arc<hmac::Key>,
    tx: broadcast::Sender<IpcMessage>,
    agent_status: AgentStatusMap,
) {
    let reader = BufReader::new(stream);
    let mut lines = reader.lines();

    while let Ok(Some(line)) = lines.next_line().await {
        if line.trim().is_empty() {
            continue;
        }

        let msg: IpcMessage = match serde_json::from_str(&line) {
            Ok(m) => m,
            Err(e) => {
                warn!(err = %e, "Received malformed IPC message — ignored");
                continue;
            }
        };

        // HMAC verification
        match verify_message(&hmac_key, &msg) {
            Ok(true) => {}
            Ok(false) => {
                warn!(
                    msg_id = %msg.id,
                    from   = %msg.from,
                    "Rejected IPC message: invalid HMAC"
                );
                continue;
            }
            Err(e) => {
                warn!(err = %e, "Failed to verify HMAC — message rejected");
                continue;
            }
        }

        // Route heartbeats: update agent status
        if msg.msg_type == MessageType::Heartbeat {
            if let Ok(h) = serde_json::from_value::<HeartbeatPayload>(msg.payload.clone()) {
                let mut map = agent_status.write().await;
                map.entry(h.agent_id.clone())
                    .or_insert(AgentState::Idle);
                info!(agent = %h.agent_id, uptime = h.uptime_seconds, "Heartbeat received");
            }
        }

        // Route status updates: reflect new agent state
        if msg.msg_type == MessageType::Status {
            if let Ok(s) = serde_json::from_value::<StatusPayload>(msg.payload.clone()) {
                let new_state = match s.state.as_str() {
                    "idle" => AgentState::Idle,
                    "working" => AgentState::Working,
                    "waiting_input" => AgentState::WaitingInput,
                    other => AgentState::Error(other.to_string()),
                };
                let mut map = agent_status.write().await;
                map.insert(msg.from.clone(), new_state);
            }
        }

        // Broadcast to all subscribers (task router, TUI, etc.)
        let _ = tx.send(msg);
    }
}

// ---------------------------------------------------------------------------
// Convenience builder for outbound messages
// ---------------------------------------------------------------------------

pub struct MessageBuilder {
    from: String,
    to: String,
    msg_type: MessageType,
    payload: serde_json::Value,
}

impl MessageBuilder {
    pub fn new(
        from: impl Into<String>,
        to: impl Into<String>,
        msg_type: MessageType,
        payload: serde_json::Value,
    ) -> Self {
        Self {
            from: from.into(),
            to: to.into(),
            msg_type,
            payload,
        }
    }

    /// Build and sign the message with the given key.
    pub fn build_signed(self, key: &hmac::Key) -> Result<IpcMessage> {
        let mut msg = IpcMessage {
            id: uuid::Uuid::new_v4().to_string(),
            from: self.from,
            to: self.to,
            msg_type: self.msg_type,
            payload: self.payload,
            timestamp: Utc::now(),
            hmac: String::new(),
        };
        sign_message(key, &mut msg)?;
        Ok(msg)
    }
}

// ---------------------------------------------------------------------------
// hex encoding helper (avoid extra crate — inline it)
// ---------------------------------------------------------------------------

mod hex {
    pub fn encode(bytes: &[u8]) -> String {
        bytes.iter().map(|b| format!("{:02x}", b)).collect()
    }

    pub fn decode(s: &str) -> Result<Vec<u8>, String> {
        if s.len() % 2 != 0 {
            return Err("odd length hex string".into());
        }
        (0..s.len())
            .step_by(2)
            .map(|i| {
                u8::from_str_radix(&s[i..i + 2], 16)
                    .map_err(|e| e.to_string())
            })
            .collect()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tokio::time::{sleep, Duration};

    fn make_bus() -> MessageBus {
        let id = uuid::Uuid::new_v4().to_string();
        MessageBus::new(&id).expect("bus creation failed")
    }

    // ------------------------------------------------------------------
    // Story 3.1 AC: socket created at correct path with 0600 permissions
    // ------------------------------------------------------------------

    #[tokio::test]
    async fn test_socket_created_at_correct_path() {
        let bus = make_bus();
        bus.start().await.expect("start failed");
        sleep(Duration::from_millis(20)).await;

        let path = bus.socket_path();
        assert!(path.exists(), "socket file should exist");
        assert!(
            path.to_str().unwrap().contains("/tmp/bmux-"),
            "socket path should be in /tmp/bmux-<id>.sock"
        );

        bus.stop();
        assert!(!path.exists(), "socket removed after stop");
    }

    #[tokio::test]
    async fn test_socket_permissions_0600() {
        let bus = make_bus();
        bus.start().await.expect("start failed");
        sleep(Duration::from_millis(20)).await;

        let meta = std::fs::metadata(bus.socket_path()).expect("metadata");
        let mode = meta.permissions().mode() & 0o777;
        assert_eq!(mode, 0o600, "socket must be 0600");

        bus.stop();
    }

    // ------------------------------------------------------------------
    // Story 3.1 AC: HMAC key generated at session init
    // ------------------------------------------------------------------

    #[test]
    fn test_session_hmac_key_generated() {
        // Two buses must have different keys (extremely unlikely to collide)
        let id = uuid::Uuid::new_v4().to_string();
        let b1 = MessageBus::new(&id).unwrap();
        let b2 = MessageBus::new(&id).unwrap();

        // Sign the same message with both keys — signatures must differ
        let mut msg1 = IpcMessage {
            id: "x".into(),
            from: "a".into(),
            to: "b".into(),
            msg_type: MessageType::Heartbeat,
            payload: serde_json::json!({}),
            timestamp: Utc::now(),
            hmac: String::new(),
        };
        let mut msg2 = msg1.clone();

        sign_message(&b1.hmac_key, &mut msg1).unwrap();
        sign_message(&b2.hmac_key, &mut msg2).unwrap();
        assert_ne!(msg1.hmac, msg2.hmac, "separate sessions must have distinct keys");
    }

    // ------------------------------------------------------------------
    // Story 3.1 AC: HMAC-SHA256 verified on receipt; invalid rejected
    // ------------------------------------------------------------------

    #[test]
    fn test_valid_hmac_accepts() {
        let key = generate_session_key().unwrap();
        let mut msg = IpcMessage {
            id: uuid::Uuid::new_v4().to_string(),
            from: "agent-a".into(),
            to: "bus".into(),
            msg_type: MessageType::Task,
            payload: serde_json::json!({"content": "hello"}),
            timestamp: Utc::now(),
            hmac: String::new(),
        };
        sign_message(&key, &mut msg).unwrap();
        assert!(verify_message(&key, &msg).unwrap(), "valid HMAC should pass");
    }

    #[test]
    fn test_tampered_payload_rejected() {
        let key = generate_session_key().unwrap();
        let mut msg = IpcMessage {
            id: uuid::Uuid::new_v4().to_string(),
            from: "agent-a".into(),
            to: "bus".into(),
            msg_type: MessageType::Task,
            payload: serde_json::json!({"content": "original"}),
            timestamp: Utc::now(),
            hmac: String::new(),
        };
        sign_message(&key, &mut msg).unwrap();

        // Tamper after signing
        msg.payload = serde_json::json!({"content": "tampered"});
        assert!(!verify_message(&key, &msg).unwrap(), "tampered message must fail HMAC");
    }

    #[test]
    fn test_wrong_key_rejected() {
        let key1 = generate_session_key().unwrap();
        let key2 = generate_session_key().unwrap();
        let mut msg = IpcMessage {
            id: uuid::Uuid::new_v4().to_string(),
            from: "a".into(),
            to: "b".into(),
            msg_type: MessageType::Status,
            payload: serde_json::json!({"state": "idle"}),
            timestamp: Utc::now(),
            hmac: String::new(),
        };
        sign_message(&key1, &mut msg).unwrap();
        assert!(!verify_message(&key2, &msg).unwrap(), "wrong key must fail verification");
    }

    #[test]
    fn test_invalid_hmac_hex_returns_error() {
        let key = generate_session_key().unwrap();
        let msg = IpcMessage {
            id: "x".into(),
            from: "a".into(),
            to: "b".into(),
            msg_type: MessageType::Heartbeat,
            payload: serde_json::json!({}),
            timestamp: Utc::now(),
            hmac: "not-valid-hex!".into(),
        };
        assert!(verify_message(&key, &msg).is_err(), "invalid hex should return Err");
    }

    // ------------------------------------------------------------------
    // Story 3.1 AC: heartbeat routed → agent status updated
    // ------------------------------------------------------------------

    #[tokio::test]
    async fn test_heartbeat_updates_agent_status() {
        let bus = make_bus();
        bus.start().await.expect("start failed");
        sleep(Duration::from_millis(20)).await;

        let heartbeat_payload = serde_json::to_value(HeartbeatPayload {
            agent_id: "agent-arch".into(),
            uptime_seconds: 42,
        })
        .unwrap();

        let msg = MessageBuilder::new("agent-arch", "bus", MessageType::Heartbeat, heartbeat_payload)
            .build_signed(&bus.hmac_key)
            .unwrap();

        bus.send_message(&msg).await.expect("send failed");
        sleep(Duration::from_millis(50)).await;

        let map = bus.agent_status.read().await;
        assert!(
            map.contains_key("agent-arch"),
            "agent-arch should be registered after heartbeat"
        );

        bus.stop();
    }

    // ------------------------------------------------------------------
    // Story 3.1 AC: invalid-HMAC messages rejected and logged (not routed)
    // ------------------------------------------------------------------

    #[tokio::test]
    async fn test_invalid_hmac_message_not_broadcast() {
        let bus = make_bus();
        bus.start().await.expect("start failed");
        sleep(Duration::from_millis(20)).await;

        let mut rx = bus.subscribe();

        // Send a message with a bogus HMAC
        let bad_msg = IpcMessage {
            id: uuid::Uuid::new_v4().to_string(),
            from: "attacker".into(),
            to: "bus".into(),
            msg_type: MessageType::Task,
            payload: serde_json::json!({"content": "do evil"}),
            timestamp: Utc::now(),
            hmac: "0".repeat(64), // wrong HMAC
        };
        bus.send_message(&bad_msg).await.expect("send failed");
        sleep(Duration::from_millis(50)).await;

        // No valid message should be received
        assert!(rx.try_recv().is_err(), "bad-HMAC message must not be broadcast");

        bus.stop();
    }

    // ------------------------------------------------------------------
    // Story 3.1 AC: status message updates agent state map
    // ------------------------------------------------------------------

    #[tokio::test]
    async fn test_status_message_updates_state() {
        let bus = make_bus();
        bus.start().await.expect("start failed");
        sleep(Duration::from_millis(20)).await;

        let status_payload = serde_json::to_value(StatusPayload {
            state: "working".into(),
        })
        .unwrap();
        let msg = MessageBuilder::new("agent-coder", "bus", MessageType::Status, status_payload)
            .build_signed(&bus.hmac_key)
            .unwrap();

        bus.send_message(&msg).await.expect("send failed");
        sleep(Duration::from_millis(50)).await;

        let map = bus.agent_status.read().await;
        assert_eq!(
            map.get("agent-coder"),
            Some(&AgentState::Working),
            "agent status should be Working"
        );

        bus.stop();
    }

    // ------------------------------------------------------------------
    // Story 3.1 AC: valid messages are broadcast to subscribers
    // ------------------------------------------------------------------

    #[tokio::test]
    async fn test_valid_message_is_broadcast() {
        let bus = make_bus();
        bus.start().await.expect("start failed");
        sleep(Duration::from_millis(20)).await;

        let mut rx = bus.subscribe();

        let task_payload = serde_json::to_value(TaskPayload {
            content: "write tests".into(),
            priority: 1,
            preferred_model: None,
            max_cost_usd: None,
        })
        .unwrap();
        let msg = MessageBuilder::new("orchestrator", "agent-coder", MessageType::Task, task_payload)
            .build_signed(&bus.hmac_key)
            .unwrap();
        let sent_id = msg.id.clone();

        bus.send_message(&msg).await.expect("send failed");
        sleep(Duration::from_millis(50)).await;

        let received = rx.try_recv().expect("should have received a message");
        assert_eq!(received.id, sent_id);
        assert_eq!(received.msg_type, MessageType::Task);

        bus.stop();
    }
}
