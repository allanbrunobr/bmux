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
    hmac::verify_message(auth.key_bytes(), &bytes, &tag).map_err(|_| anyhow!("client HMAC verification failed"))?;
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
    hmac::verify_message(auth.key_bytes(), &bytes, &tag).map_err(|_| anyhow!("server HMAC verification failed"))?;
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
}