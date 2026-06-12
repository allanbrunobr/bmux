/// Sled-backed shared context store, namespaced per session.
///
/// Keys are stored as raw `user_key` bytes. Isolation is per-session via a
/// separate sled database directory (`ctx_{session_id}/`).
/// Value format: serialized `StoredEntry` JSON
///
/// Stories: 4.1, 4.2, 4.3, 4.4
use std::path::Path;
use std::time::{SystemTime, UNIX_EPOCH};

use serde::{Deserialize, Serialize};
use thiserror::Error;

#[derive(Debug, Error)]
pub enum ContextStoreError {
    #[error("sled error: {0}")]
    Sled(#[from] sled::Error),
    #[error("serialization error: {0}")]
    Serde(#[from] serde_json::Error),
    #[error("I/O error: {0}")]
    Io(#[from] std::io::Error),
}

pub type Result<T> = std::result::Result<T, ContextStoreError>;

/// Internal representation stored in sled.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StoredEntry {
    /// The stored value (arbitrary string).
    pub value: String,
    /// Unix timestamp (seconds) when the entry was created.
    pub created_at: u64,
    /// Optional Unix timestamp (seconds) when this entry expires.
    pub expires_at: Option<u64>,
}

/// A context entry as returned to callers (includes the key).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ContextEntry {
    pub key: String,
    pub value: String,
    pub created_at: u64,
    pub expires_at: Option<u64>,
}

/// The main context store handle.
pub struct ContextStore {
    db: sled::Db,
}

impl ContextStore {
    /// Open (or create) the context store for a session.
    ///
    /// Each session gets its own sled database at `{base_path}/ctx_{session_id}/`
    /// to avoid lock contention between concurrent sessions.
    pub fn open(session_id: &str, base_path: &Path) -> Result<Self> {
        // Sanitize session_id for use as a directory name
        let safe_id = session_id.replace(['/', '\\', ':'], "_");
        let db_path = base_path.join(format!("ctx_{safe_id}"));
        std::fs::create_dir_all(&db_path)?;
        let db = sled::open(&db_path)?;
        let _ = session_id;
        Ok(Self { db })
    }

    // ─── internal key helpers ────────────────────────────────────────────────

    fn sled_key(&self, user_key: &str) -> Vec<u8> {
        user_key.as_bytes().to_vec()
    }

    fn user_key_from_bytes(bytes: &[u8]) -> Option<&str> {
        std::str::from_utf8(bytes).ok()
    }

    fn now_secs() -> u64 {
        SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap_or_default()
            .as_secs()
    }

    // ─── public API ──────────────────────────────────────────────────────────

    /// Store a key-value pair with optional TTL (in seconds).
    pub fn set(&self, key: &str, value: &str, ttl_seconds: Option<u64>) -> Result<()> {
        let now = Self::now_secs();
        let entry = StoredEntry {
            value: value.to_string(),
            created_at: now,
            expires_at: ttl_seconds.map(|t| now + t),
        };
        let encoded = serde_json::to_vec(&entry)?;
        self.db.insert(self.sled_key(key), encoded)?;
        Ok(())
    }

    /// Retrieve a value by key. Returns `None` if not found or expired.
    pub fn get(&self, key: &str) -> Result<Option<String>> {
        if let Some(bytes) = self.db.get(self.sled_key(key))? {
            let entry: StoredEntry = serde_json::from_slice(&bytes)?;
            if self.is_expired(&entry) {
                // Lazy delete
                self.db.remove(self.sled_key(key))?;
                return Ok(None);
            }
            Ok(Some(entry.value))
        } else {
            Ok(None)
        }
    }

    /// List all non-expired entries for this session.
    /// Returns `(key, value_preview)` pairs — value preview truncated to 80 chars.
    pub fn list(&self) -> Result<Vec<(String, String)>> {
        let now = Self::now_secs();
        let mut result = Vec::new();

        for item in self.db.iter() {
            let (k, v) = item?;
            let entry: StoredEntry = serde_json::from_slice(&v)?;
            if entry.expires_at.is_some_and(|exp| exp <= now) {
                continue; // skip expired
            }
            if let Some(user_key) = Self::user_key_from_bytes(&k) {
                let preview = if entry.value.len() > 80 {
                    format!("{}...", &entry.value[..77])
                } else {
                    entry.value.clone()
                };
                result.push((user_key.to_string(), preview));
            }
        }
        Ok(result)
    }

    /// Delete a specific key.
    pub fn delete(&self, key: &str) -> Result<()> {
        self.db.remove(self.sled_key(key))?;
        Ok(())
    }

    /// Dump all non-expired entries as a JSON string (Story 4.4).
    pub fn dump_json(&self) -> Result<String> {
        let entries = self.dump()?;
        Ok(serde_json::to_string_pretty(&entries)?)
    }

    /// Dump all non-expired entries for this session (Story 4.4).
    pub fn dump(&self) -> Result<Vec<ContextEntry>> {
        let now = Self::now_secs();
        let mut result = Vec::new();

        for item in self.db.iter() {
            let (k, v) = item?;
            let entry: StoredEntry = serde_json::from_slice(&v)?;
            if entry.expires_at.is_some_and(|exp| exp <= now) {
                continue;
            }
            if let Some(user_key) = Self::user_key_from_bytes(&k) {
                result.push(ContextEntry {
                    key: user_key.to_string(),
                    value: entry.value,
                    created_at: entry.created_at,
                    expires_at: entry.expires_at,
                });
            }
        }
        Ok(result)
    }

    /// Remove all context entries for this session (called on session kill — Story 4.3).
    pub fn cleanup_session(&self) -> Result<usize> {
        let mut keys_to_remove: Vec<Vec<u8>> = Vec::new();

        for item in self.db.iter() {
            let (k, _) = item?;
            keys_to_remove.push(k.to_vec());
        }

        let count = keys_to_remove.len();
        for k in keys_to_remove {
            self.db.remove(k)?;
        }
        Ok(count)
    }

    /// Sweep and remove expired entries across ALL sessions in this database.
    /// Called by background cleanup task every 60s (Story 4.3).
    pub fn cleanup_expired_all(&self) -> Result<usize> {
        let now = Self::now_secs();
        let mut keys_to_remove: Vec<Vec<u8>> = Vec::new();

        for item in self.db.iter() {
            let (k, v) = item?;
            if let Ok(entry) = serde_json::from_slice::<StoredEntry>(&v) {
                if entry.expires_at.is_some_and(|exp| exp <= now) {
                    keys_to_remove.push(k.to_vec());
                }
            }
        }

        let count = keys_to_remove.len();
        for k in keys_to_remove {
            self.db.remove(k)?;
        }
        Ok(count)
    }

    // ─── helper ──────────────────────────────────────────────────────────────

    fn is_expired(&self, entry: &StoredEntry) -> bool {
        entry
            .expires_at
            .is_some_and(|exp| exp <= Self::now_secs())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;
    use serial_test::serial;

    fn make_store() -> (ContextStore, TempDir) {
        let dir = TempDir::new().unwrap();
        let store = ContextStore::open("test-session", dir.path()).unwrap();
        (store, dir)
    }

    // ── Story 4.1: basic set/get ──────────────────────────────────────────────

    
    #[serial]
    #[test]
    fn test_set_and_get() {
        let (store, _dir) = make_store();
        store.set("project:tech-stack", "Rust + ratatui + tokio", None).unwrap();
        let val = store.get("project:tech-stack").unwrap();
        assert_eq!(val, Some("Rust + ratatui + tokio".to_string()));
    }

    
    #[serial]
    #[test]
    fn test_get_nonexistent_returns_none() {
        let (store, _dir) = make_store();
        let val = store.get("nonexistent:key").unwrap();
        assert_eq!(val, None);
    }

    
    #[serial]
    #[test]
    fn test_list_shows_all_entries_with_preview() {
        let (store, _dir) = make_store();
        store.set("key1", "value1", None).unwrap();
        store.set("key2", "value2", None).unwrap();
        let list = store.list().unwrap();
        assert_eq!(list.len(), 2);
        let keys: Vec<_> = list.iter().map(|(k, _)| k.as_str()).collect();
        assert!(keys.contains(&"key1"));
        assert!(keys.contains(&"key2"));
    }

    
    #[serial]
    #[test]
    fn test_list_preview_truncated_to_80_chars() {
        let (store, _dir) = make_store();
        let long_val = "x".repeat(100);
        store.set("long-key", &long_val, None).unwrap();
        let list = store.list().unwrap();
        assert_eq!(list.len(), 1);
        // preview should be <= 80 chars
        assert!(list[0].1.len() <= 80);
    }

    
    #[serial]
    #[test]
    fn test_delete_removes_entry() {
        let (store, _dir) = make_store();
        store.set("to-delete", "val", None).unwrap();
        store.delete("to-delete").unwrap();
        assert_eq!(store.get("to-delete").unwrap(), None);
    }

    // ── Story 4.3: TTL expiry ────────────────────────────────────────────────

    
    #[serial]
    #[test]
    fn test_expired_entry_not_returned_on_get() {
        let dir = TempDir::new().unwrap();
        let store = ContextStore::open("ttl-session", dir.path()).unwrap();

        // Manually insert an already-expired entry
        let entry = StoredEntry {
            value: "expired".to_string(),
            created_at: 1000,
            expires_at: Some(1001), // already expired (epoch 1001)
        };
        let sled_key = store.sled_key("exp-key");
        store.db.insert(sled_key, serde_json::to_vec(&entry).unwrap()).unwrap();

        // get should return None
        assert_eq!(store.get("exp-key").unwrap(), None);
    }

    
    #[serial]
    #[test]
    fn test_expired_entries_excluded_from_list() {
        let dir = TempDir::new().unwrap();
        let store = ContextStore::open("ttl-session2", dir.path()).unwrap();

        // Insert expired
        let expired = StoredEntry {
            value: "gone".to_string(),
            created_at: 1000,
            expires_at: Some(1001),
        };
        store.db.insert(store.sled_key("expired-key"), serde_json::to_vec(&expired).unwrap()).unwrap();

        // Insert valid
        store.set("valid-key", "here", None).unwrap();

        let list = store.list().unwrap();
        assert_eq!(list.len(), 1);
        assert_eq!(list[0].0, "valid-key");
    }

    
    #[serial]
    #[test]
    fn test_cleanup_expired_all() {
        let dir = TempDir::new().unwrap();
        let store = ContextStore::open("cleanup-session", dir.path()).unwrap();

        let expired = StoredEntry {
            value: "gone".to_string(),
            created_at: 1000,
            expires_at: Some(1001),
        };
        store.db.insert(store.sled_key("e1"), serde_json::to_vec(&expired).unwrap()).unwrap();
        store.db.insert(store.sled_key("e2"), serde_json::to_vec(&expired).unwrap()).unwrap();
        store.set("live", "alive", None).unwrap();

        let removed = store.cleanup_expired_all().unwrap();
        assert_eq!(removed, 2);
        assert_eq!(store.get("live").unwrap(), Some("alive".to_string()));
    }

    
    #[serial]
    #[test]
    fn test_cleanup_session_removes_all_session_entries() {
        let dir = TempDir::new().unwrap();
        let store = ContextStore::open("sess-cleanup", dir.path()).unwrap();
        store.set("k1", "v1", None).unwrap();
        store.set("k2", "v2", None).unwrap();

        let count = store.cleanup_session().unwrap();
        assert_eq!(count, 2);
        assert!(store.list().unwrap().is_empty());
    }

    // ── Story 4.4: JSON dump ─────────────────────────────────────────────────

    
    #[serial]
    #[test]
    fn test_dump_returns_all_entries_with_metadata() {
        let (store, _dir) = make_store();
        store.set("k1", "v1", None).unwrap();
        store.set("k2", "v2", Some(3600)).unwrap();

        let entries = store.dump().unwrap();
        assert_eq!(entries.len(), 2);

        let k2 = entries.iter().find(|e| e.key == "k2").unwrap();
        assert!(k2.expires_at.is_some());
        assert!(k2.created_at > 0);
    }

    
    #[serial]
    #[test]
    fn test_dump_json_is_valid_json() {
        let (store, _dir) = make_store();
        store.set("project:tech-stack", "Rust + ratatui", None).unwrap();

        let json = store.dump_json().unwrap();
        let parsed: Vec<serde_json::Value> = serde_json::from_str(&json).unwrap();
        assert_eq!(parsed.len(), 1);
    }

    // ── Session isolation ────────────────────────────────────────────────────

    
    #[serial]
    #[test]
    fn test_different_sessions_are_isolated() {
        let dir = TempDir::new().unwrap();
        let s1 = ContextStore::open("session-a", dir.path()).unwrap();
        let s2 = ContextStore::open("session-b", dir.path()).unwrap();

        s1.set("shared-key", "value-a", None).unwrap();
        s2.set("shared-key", "value-b", None).unwrap();

        assert_eq!(s1.get("shared-key").unwrap(), Some("value-a".to_string()));
        assert_eq!(s2.get("shared-key").unwrap(), Some("value-b".to_string()));

        s1.cleanup_session().unwrap();
        // s2 should still have its entry
        assert_eq!(s2.get("shared-key").unwrap(), Some("value-b".to_string()));
    }
}
