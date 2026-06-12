/// High-level context API wrapping the storage layer.
///
/// Provides cross-agent context access via IPC message handling and
/// automatic storage of agent outputs with TTL.
///
/// Stories: 4.2, 4.3
use std::path::Path;
use std::sync::Arc;

use crate::storage::context_store::{ContextEntry, ContextStore, ContextStoreError};

/// TTL for agent last_output entries: 1 hour.
pub const AGENT_OUTPUT_TTL_SECS: u64 = 3600;

/// IPC-level context request types (mirrors message bus protocol).
#[derive(Debug, Clone)]
pub enum ContextRequest {
    /// Read a key; returns the value if present.
    Read { key: String },
    /// Write a key with optional TTL.
    Write {
        key: String,
        value: String,
        ttl_seconds: Option<u64>,
    },
    /// List all keys for this session.
    List,
    /// Dump all entries as JSON.
    Dump,
}

/// Response to a context request.
#[derive(Debug, Clone)]
pub enum ContextResponse {
    Value(Option<String>),
    List(Vec<(String, String)>),
    DumpJson(String),
    Ok,
    Error(String),
}

/// High-level context store wrapping `storage::ContextStore`.
/// Designed to be shared across async tasks via `Arc`.
#[derive(Clone)]
pub struct SharedContextStore {
    inner: Arc<ContextStore>,
}

impl SharedContextStore {
    /// Open the shared context store for a session.
    pub fn open(session_id: &str, db_path: &Path) -> Result<Self, ContextStoreError> {
        let store = ContextStore::open(session_id, db_path)?;
        Ok(Self {
            inner: Arc::new(store),
        })
    }

    /// Wrap an existing `Arc<ContextStore>` (daemon wiring).
    pub fn from_arc(inner: Arc<ContextStore>) -> Self {
        Self { inner }
    }

    // ─── Agent output helpers (Story 4.2) ────────────────────────────────────

    /// Store an agent's last output at `agent:{id}:last_output` with TTL of 1 hour.
    pub fn store_agent_output(&self, agent_id: &str, output: &str) -> Result<(), ContextStoreError> {
        let key = format!("agent:{agent_id}:last_output");
        self.inner.set(&key, output, Some(AGENT_OUTPUT_TTL_SECS))
    }

    /// Read another agent's last output.
    pub fn read_agent_output(&self, agent_id: &str) -> Result<Option<String>, ContextStoreError> {
        let key = format!("agent:{agent_id}:last_output");
        self.inner.get(&key)
    }

    // ─── Generic KV access ───────────────────────────────────────────────────

    pub fn set(&self, key: &str, value: &str, ttl_seconds: Option<u64>) -> Result<(), ContextStoreError> {
        self.inner.set(key, value, ttl_seconds)
    }

    pub fn set_with_author(
        &self,
        key: &str,
        value: &str,
        ttl_seconds: Option<u64>,
        author: &str,
    ) -> Result<(), ContextStoreError> {
        self.inner.set_with_author(key, value, ttl_seconds, author)
    }

    pub fn get(&self, key: &str) -> Result<Option<String>, ContextStoreError> {
        self.inner.get(key)
    }

    pub fn list(&self) -> Result<Vec<(String, String)>, ContextStoreError> {
        self.inner.list()
    }

    pub fn dump(&self) -> Result<Vec<ContextEntry>, ContextStoreError> {
        self.inner.dump()
    }

    pub fn dump_json(&self) -> Result<String, ContextStoreError> {
        self.inner.dump_json()
    }

    pub fn delete(&self, key: &str) -> Result<(), ContextStoreError> {
        self.inner.delete(key)
    }

    // ─── Session lifecycle (Story 4.3) ───────────────────────────────────────

    /// Remove all context entries for this session (called on `bmux kill -s`).
    pub fn cleanup_session(&self) -> Result<usize, ContextStoreError> {
        self.inner.cleanup_session()
    }

    /// Sweep expired entries across all sessions in this database.
    pub fn cleanup_expired_all(&self) -> Result<usize, ContextStoreError> {
        self.inner.cleanup_expired_all()
    }

    // ─── IPC message dispatch (Story 4.2) ────────────────────────────────────

    /// Handle a context request arriving via IPC and return a response.
    pub fn handle_request(&self, req: ContextRequest) -> ContextResponse {
        match req {
            ContextRequest::Read { key } => {
                match self.inner.get(&key) {
                    Ok(val) => ContextResponse::Value(val),
                    Err(e) => ContextResponse::Error(e.to_string()),
                }
            }
            ContextRequest::Write { key, value, ttl_seconds } => {
                match self.inner.set(&key, &value, ttl_seconds) {
                    Ok(()) => ContextResponse::Ok,
                    Err(e) => ContextResponse::Error(e.to_string()),
                }
            }
            ContextRequest::List => {
                match self.inner.list() {
                    Ok(entries) => ContextResponse::List(entries),
                    Err(e) => ContextResponse::Error(e.to_string()),
                }
            }
            ContextRequest::Dump => {
                match self.inner.dump_json() {
                    Ok(json) => ContextResponse::DumpJson(json),
                    Err(e) => ContextResponse::Error(e.to_string()),
                }
            }
        }
    }
}

/// Background TTL cleanup task (Story 4.3).
/// Sweeps expired entries every 60 seconds.
pub async fn run_cleanup_task(store: SharedContextStore) {
    use tokio::time::{interval, Duration};
    let mut ticker = interval(Duration::from_secs(60));
    loop {
        ticker.tick().await;
        if let Err(e) = store.cleanup_expired_all() {
            tracing::warn!("TTL cleanup error: {e}");
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    fn make_shared_store() -> (SharedContextStore, TempDir) {
        let dir = TempDir::new().unwrap();
        let store = SharedContextStore::open("test-session", dir.path()).unwrap();
        (store, dir)
    }

    // ── Story 4.2: agent output storage ──────────────────────────────────────

    #[test]
    fn test_store_and_read_agent_output() {
        let (store, _dir) = make_shared_store();
        store.store_agent_output("arch", "design doc content").unwrap();
        let out = store.read_agent_output("arch").unwrap();
        assert_eq!(out, Some("design doc content".to_string()));
    }

    #[test]
    fn test_read_missing_agent_output_returns_none() {
        let (store, _dir) = make_shared_store();
        let out = store.read_agent_output("nonexistent").unwrap();
        assert_eq!(out, None);
    }

    #[test]
    fn test_agent_output_key_format() {
        let (store, _dir) = make_shared_store();
        store.store_agent_output("tests", "test output").unwrap();
        // Key accessible via generic get with correct namespace
        let val = store.get("agent:tests:last_output").unwrap();
        assert_eq!(val, Some("test output".to_string()));
    }

    // ── IPC dispatch ─────────────────────────────────────────────────────────

    #[test]
    fn test_handle_request_write_and_read() {
        let (store, _dir) = make_shared_store();

        let resp = store.handle_request(ContextRequest::Write {
            key: "ipc-key".to_string(),
            value: "ipc-value".to_string(),
            ttl_seconds: None,
        });
        assert!(matches!(resp, ContextResponse::Ok));

        let resp = store.handle_request(ContextRequest::Read {
            key: "ipc-key".to_string(),
        });
        assert!(matches!(resp, ContextResponse::Value(Some(v)) if v == "ipc-value"));
    }

    #[test]
    fn test_handle_request_read_missing() {
        let (store, _dir) = make_shared_store();
        let resp = store.handle_request(ContextRequest::Read {
            key: "missing".to_string(),
        });
        assert!(matches!(resp, ContextResponse::Value(None)));
    }

    #[test]
    fn test_handle_request_list() {
        let (store, _dir) = make_shared_store();
        store.set("a", "1", None).unwrap();
        store.set("b", "2", None).unwrap();

        let resp = store.handle_request(ContextRequest::List);
        if let ContextResponse::List(entries) = resp {
            assert_eq!(entries.len(), 2);
        } else {
            panic!("expected List response");
        }
    }

    #[test]
    fn test_handle_request_dump() {
        let (store, _dir) = make_shared_store();
        store.set("x", "y", None).unwrap();

        let resp = store.handle_request(ContextRequest::Dump);
        if let ContextResponse::DumpJson(json) = resp {
            let parsed: Vec<serde_json::Value> = serde_json::from_str(&json).unwrap();
            assert_eq!(parsed.len(), 1);
        } else {
            panic!("expected DumpJson response");
        }
    }
}
