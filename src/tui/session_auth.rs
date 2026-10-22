//! HMAC authentication for the TUI daemon Unix socket (ADR-006).

use anyhow::{anyhow, Result};
use serde::{Deserialize, Serialize};

use crate::security::hmac;

use super::protocol::{ClientMessage, ServerMessage};

/// Per-session key for signing daemon protocol frames.
#[derive(Debug, Clone)]
pub struct SessionAuth {
    key_bytes: Vec<u8>,
}

impl SessionAuth {
    pub fn generate() -> Result<Self> {
        let key_bytes = hmac::generate_key().map_err(|e| anyhow!("{e}"))?;
        Ok(Self { key_bytes })
    }

    pub fn from_hex(hex_str: &str) -> Result<Self> {
        let key_bytes = hex::decode(hex_str).map_err(|e| anyhow!("invalid auth key hex: {e}"))?;
        if key_bytes.len() < 32 {
            return Err(anyhow!("auth key must be at least 32 bytes"));
        }
        Ok(Self { key_bytes })
    }

    pub fn key_hex(&self) -> String {
        hex::encode(&self.key_bytes)
    }

    pub fn key_bytes(&self) -> &[u8] {
        &self.key_bytes
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SignedClientFrame {
    #[serde(flatten)]
    pub msg: ClientMessage,
    pub hmac: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SignedServerFrame {
    #[serde(flatten)]
    pub msg: ServerMessage,
    pub hmac: String,
}

fn client_signable(msg: &ClientMessage) -> Result<Vec<u8>> {
    Ok(serde_json::to_vec(msg)?)
}

fn server_signable(msg: &ServerMessage) -> Result<Vec<u8>> {
    Ok(serde_json::to_vec(msg)?)
}

pub fn sign_client(auth: &SessionAuth, msg: ClientMessage) -> Result<SignedClientFrame> {
    let bytes = client_signable(&msg)?;
    let tag = hmac::sign_message(auth.key_bytes(), &bytes)?;
    Ok(SignedClientFrame {
        msg,
        hmac: hex::encode(tag),
    })
}

pub fn verify_client(auth: &SessionAuth, frame: &SignedClientFrame) -> Result<ClientMessage> {
    let bytes = client_signable(&frame.msg)?;
    let tag = hex::decode(&frame.hmac).map_err(|e| anyhow!("invalid client hmac hex: {e}"))?;
    hmac::verify_message(auth.key_bytes(), &bytes, &tag)
        .map_err(|_| anyhow!("client HMAC verification failed"))?;
    Ok(frame.msg.clone())
}

pub fn sign_server(auth: &SessionAuth, msg: ServerMessage) -> Result<SignedServerFrame> {
    let bytes = server_signable(&msg)?;
    let tag = hmac::sign_message(auth.key_bytes(), &bytes)?;
    Ok(SignedServerFrame {
        msg,
        hmac: hex::encode(tag),
    })
}

pub fn verify_server(auth: &SessionAuth, frame: &SignedServerFrame) -> Result<ServerMessage> {
    let bytes = server_signable(&frame.msg)?;
    let tag = hex::decode(&frame.hmac).map_err(|e| anyhow!("invalid server hmac hex: {e}"))?;
    hmac::verify_message(auth.key_bytes(), &bytes, &tag)
        .map_err(|_| anyhow!("server HMAC verification failed"))?;
    Ok(frame.msg.clone())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::tui::keybindings::Action;

    #[test]
    fn client_roundtrip() {
        let auth = SessionAuth::generate().unwrap();
        let msg = ClientMessage::Action {
            action: Action::ToggleZoom,
        };
        let signed = sign_client(&auth, msg.clone()).unwrap();
        let verified = verify_client(&auth, &signed).unwrap();
        assert_eq!(verified, msg);
    }

    #[test]
    fn tampered_client_frame_rejected() {
        let auth = SessionAuth::generate().unwrap();
        let mut signed = sign_client(
            &auth,
            ClientMessage::ContextGet {
                key: "x".to_string(),
            },
        )
        .unwrap();
        signed.hmac = "00".repeat(32);
        assert!(verify_client(&auth, &signed).is_err());
    }

    /// A state frame must still verify after a trip through JSON. Without
    /// serde_json's `float_roundtrip`, this cost parses back one ULP off,
    /// the HMAC check fails and the client silently drops every frame.
    #[test]
    fn server_state_with_float_cost_survives_json() {
        use crate::tui::protocol::{AgentBadge, SessionSnapshot};
        let auth = SessionAuth::generate().unwrap();
        let snap = SessionSnapshot {
            name: "s".to_string(),
            active_window: 0,
            windows: Vec::new(),
            status_message: None,
            agents: vec![AgentBadge {
                name: "dev".to_string(),
                agent_type: "claude-code".to_string(),
                model: "claude-sonnet-4-5".to_string(),
                status: "idle".to_string(),
                tokens: 1234,
                cost_usd: 4.2749999999999996e-5,
                cost_tracked: true,
                pane_id: Some(1),
            }],
            status_bar: true,
        };
        let signed = sign_server(&auth, ServerMessage::State(snap)).unwrap();
        let wire = serde_json::to_string(&signed).unwrap();
        let received: SignedServerFrame = serde_json::from_str(&wire).unwrap();
        assert!(verify_server(&auth, &received).is_ok());
    }
}
