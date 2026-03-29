/// Session metadata store — persistence for detach/reattach.
///
/// The ContextStore already persists to disk via sled, so context survives
/// detach/reattach automatically. This module manages the session registry
/// (session metadata) and drives context cleanup on kill.
///
/// Story: 4.3
use std::path::Path;
use std::time::{SystemTime, UNIX_EPOCH};

use serde::{Deserialize, Serialize};
use thiserror::Error;

use crate::storage::context_store::{ContextStore, ContextStoreError};

#[derive(Debug, Error)]
pub enum SessionStoreError {
    #[error("sled error: {0}")]
    Sled(#[from] sled::Error),
    #[error("serialization error: {0}")]
    Serde(#[from] serde_json::Error),
    #[error("context store error: {0}")]
    Context(#[from] ContextStoreError),
    #[error("I/O error: {0}")]
    Io(#[from] std::io::Error),
}

pub type Result<T> = std::result::Result<T, SessionStoreError>;

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum SessionState {
    Active,
    Detached,
    Terminated,
}

/// Metadata stored for a session.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SessionMeta {
    pub session_id: String,
    pub created_at: u64,
    pub last_attached_at: u64,
    pub detached_at: Option<u64>,
    pub state: SessionState,
}

/// Manages session metadata in a sled tree.
pub struct SessionStore {
    /// Sled DB for session metadata (opened at `{base_path}/sessions/`).
    db: sled::Db,
    /// Base path used when opening ContextStore instances for session cleanup.
    context_base_path: std::path::PathBuf,
}

impl SessionStore {
    /// Open the session store at `{base_path}/sessions/`.
    ///
    /// Session metadata is stored separately from context data so that sled
    /// lock conflicts do not occur when `kill()` opens a ContextStore.
    pub fn open(base_path: &Path) -> Result<Self> {
        std::fs::create_dir_all(base_path)?;
        let sessions_path = base_path.join("sessions");
        std::fs::create_dir_all(&sessions_path)?;
        let db = sled::open(&sessions_path)?;
        Ok(Self {
            db,
            context_base_path: base_path.to_path_buf(),
        })
    }

    fn now_secs() -> u64 {
        SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap_or_default()
            .as_secs()
    }

    /// Register a new session (or re-register on reattach).
    pub fn attach(&self, session_id: &str) -> Result<()> {
        let now = Self::now_secs();
        let meta = if let Some(existing) = self.get_meta(session_id)? {
            SessionMeta {
                last_attached_at: now,
                detached_at: None,
                state: SessionState::Active,
                ..existing
            }
        } else {
            SessionMeta {
                session_id: session_id.to_string(),
                created_at: now,
                last_attached_at: now,
                detached_at: None,
                state: SessionState::Active,
            }
        };
        self.save_meta(&meta)?;
        Ok(())
    }

    /// Mark session as detached; context entries are preserved.
    pub fn detach(&self, session_id: &str) -> Result<()> {
        if let Some(mut meta) = self.get_meta(session_id)? {
            meta.state = SessionState::Detached;
            meta.detached_at = Some(Self::now_secs());
            self.save_meta(&meta)?;
        }
        Ok(())
    }

    /// Terminate a session and remove all its context entries (Story 4.3).
    pub fn kill(&self, session_id: &str) -> Result<usize> {
        if let Some(mut meta) = self.get_meta(session_id)? {
            meta.state = SessionState::Terminated;
            self.save_meta(&meta)?;
        }
        // Clean up all context entries for this session.
        // ContextStore::open uses `context_base_path/ctx_{session_id}/` — distinct from
        // the session metadata DB at `sessions/`, so no sled lock conflict.
        let context_store = ContextStore::open(session_id, &self.context_base_path)?;
        let removed = context_store.cleanup_session()?;
        Ok(removed)
    }

    /// Returns session metadata if it exists.
    pub fn get_meta(&self, session_id: &str) -> Result<Option<SessionMeta>> {
        if let Some(bytes) = self.db.get(session_id.as_bytes())? {
            let meta: SessionMeta = serde_json::from_slice(&bytes)?;
            Ok(Some(meta))
        } else {
            Ok(None)
        }
    }

    /// List all sessions and their states.
    pub fn list_sessions(&self) -> Result<Vec<SessionMeta>> {
        let mut sessions = Vec::new();
        for item in self.db.iter() {
            let (_k, v) = item?;
            if let Ok(meta) = serde_json::from_slice::<SessionMeta>(&v) {
                sessions.push(meta);
            }
        }
        Ok(sessions)
    }

    fn save_meta(&self, meta: &SessionMeta) -> Result<()> {
        let bytes = serde_json::to_vec(meta)?;
        self.db.insert(meta.session_id.as_bytes(), bytes)?;
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;
    use serial_test::serial;

    fn make_session_store() -> (SessionStore, TempDir) {
        let dir = TempDir::new().unwrap();
        let store = SessionStore::open(dir.path()).unwrap();
        (store, dir)
    }

    // ── Story 4.3: session persistence ──────────────────────────────────────

    
    #[serial]
    #[test]
    fn test_attach_creates_session() {
        let (store, _dir) = make_session_store();
        store.attach("my-session").unwrap();
        let meta = store.get_meta("my-session").unwrap().unwrap();
        assert_eq!(meta.session_id, "my-session");
        assert_eq!(meta.state, SessionState::Active);
    }

    
    #[serial]
    #[test]
    fn test_detach_preserves_session_metadata() {
        let (store, _dir) = make_session_store();
        store.attach("detach-test").unwrap();
        store.detach("detach-test").unwrap();
        let meta = store.get_meta("detach-test").unwrap().unwrap();
        assert_eq!(meta.state, SessionState::Detached);
        assert!(meta.detached_at.is_some());
    }

    
    #[serial]
    #[test]
    fn test_reattach_restores_active_state() {
        let (store, _dir) = make_session_store();
        store.attach("re-test").unwrap();
        store.detach("re-test").unwrap();
        store.attach("re-test").unwrap(); // reattach
        let meta = store.get_meta("re-test").unwrap().unwrap();
        assert_eq!(meta.state, SessionState::Active);
        assert_eq!(meta.detached_at, None);
    }

    
    #[serial]
    #[test]
    fn test_kill_removes_context_entries() {
        let dir = TempDir::new().unwrap();
        let session_store = SessionStore::open(dir.path()).unwrap();
        session_store.attach("kill-test").unwrap();

        // Add context entries, then drop the store handle so sled lock is released
        {
            let ctx = ContextStore::open("kill-test", dir.path()).unwrap();
            ctx.set("k1", "v1", None).unwrap();
            ctx.set("k2", "v2", None).unwrap();
            assert_eq!(ctx.list().unwrap().len(), 2);
        } // ctx dropped here → sled lock released

        // Kill the session (opens its own ContextStore handle)
        let removed = session_store.kill("kill-test").unwrap();
        assert_eq!(removed, 2);

        // Re-open to verify entries gone
        let ctx2 = ContextStore::open("kill-test", dir.path()).unwrap();
        assert!(ctx2.list().unwrap().is_empty());
    }

    
    #[serial]
    #[test]
    fn test_context_persists_through_detach_reattach() {
        let dir = TempDir::new().unwrap();
        let session_store = SessionStore::open(dir.path()).unwrap();
        session_store.attach("persist-test").unwrap();

        // Set context entries
        let ctx = ContextStore::open("persist-test", dir.path()).unwrap();
        ctx.set("persistent-key", "persistent-value", None).unwrap();

        // Detach
        session_store.detach("persist-test").unwrap();

        // Reattach
        session_store.attach("persist-test").unwrap();

        // Context still there
        assert_eq!(
            ctx.get("persistent-key").unwrap(),
            Some("persistent-value".to_string())
        );
    }

    
    #[serial]
    #[test]
    fn test_list_sessions() {
        let (store, _dir) = make_session_store();
        store.attach("s1").unwrap();
        store.attach("s2").unwrap();
        let sessions = store.list_sessions().unwrap();
        assert_eq!(sessions.len(), 2);
    }

    
    #[serial]
    #[test]
    fn test_kill_marks_session_terminated() {
        let dir = TempDir::new().unwrap();
        let store = SessionStore::open(dir.path()).unwrap();
        store.attach("terminate-test").unwrap();
        store.kill("terminate-test").unwrap();
        let meta = store.get_meta("terminate-test").unwrap().unwrap();
        assert_eq!(meta.state, SessionState::Terminated);
    }
}
