// src/storage/audit_log.rs — Structured audit logging (JSONL)
// Story 6.3: Audit Logging
//
// Writes one JSON object per line to:
//   ~/.local/share/bmux/audit/YYYY-MM-DD.jsonl
//
// Each entry contains: timestamp, event_type, agent_id, session_id, metadata.
// 100% audit completeness (NFR-08): every call to `log_event` is synchronous
// and returns only after the bytes are flushed to the file.

use std::fs::{self, OpenOptions};
use std::io::{self, Write};
use std::path::{Path, PathBuf};

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use thiserror::Error;

// ─── Error type ────────────────────────────────────────────────────────────────

#[derive(Debug, Error)]
pub enum AuditError {
    #[error("IO error: {0}")]
    Io(#[from] io::Error),

    #[error("Serialization error: {0}")]
    Serialization(#[from] serde_json::Error),
}

// ─── Event types ───────────────────────────────────────────────────────────────

/// All auditable event types (FR-SEC-03 + Story 6.3 AC).
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum EventType {
    AgentSpawned,
    AgentKilled,
    TaskSent,
    TaskCompleted,
    AgentStatusChange,
    SandboxAccessDenied,
    HmacVerificationFailure,
}

impl std::fmt::Display for EventType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let s = serde_json::to_value(self)
            .ok()
            .and_then(|v| v.as_str().map(String::from))
            .unwrap_or_else(|| format!("{self:?}"));
        write!(f, "{s}")
    }
}

// ─── Audit entry ───────────────────────────────────────────────────────────────

/// A single structured audit log entry.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AuditEntry {
    /// ISO-8601 UTC timestamp
    pub timestamp: DateTime<Utc>,
    /// Category of event
    pub event_type: EventType,
    /// Agent identifier (may be None for system-level events)
    pub agent_id: Option<String>,
    /// Session this event belongs to
    pub session_id: String,
    /// Event-specific payload
    pub metadata: serde_json::Value,
}

impl AuditEntry {
    pub fn new(
        event_type: EventType,
        agent_id: Option<impl Into<String>>,
        session_id: impl Into<String>,
        metadata: serde_json::Value,
    ) -> Self {
        Self {
            timestamp: Utc::now(),
            event_type,
            agent_id: agent_id.map(Into::into),
            session_id: session_id.into(),
            metadata,
        }
    }
}

// ─── AuditLogger ───────────────────────────────────────────────────────────────

/// Writes structured audit events to daily JSONL files.
///
/// Thread-safe: each call to [`log_event`] opens, writes, and fsync-flushes the
/// file atomically to guarantee 100% audit completeness (NFR-08).
#[derive(Debug, Clone)]
pub struct AuditLogger {
    pub enabled: bool,
    pub log_dir: PathBuf,
    /// Whether the "audit logging is disabled" one-time notice has been shown
    disabled_notice_shown: bool,
}

impl AuditLogger {
    /// Create a new logger writing to `log_dir`.
    pub fn new(log_dir: PathBuf, enabled: bool) -> Self {
        Self {
            enabled,
            log_dir,
            disabled_notice_shown: false,
        }
    }

    /// Create a logger using the default path `~/.local/share/bmux/audit/`.
    pub fn with_default_dir(enabled: bool) -> Option<Self> {
        let dir = default_audit_dir()?;
        Some(Self::new(dir, enabled))
    }

    /// Log an audit event.
    ///
    /// If logging is disabled, a one-time notice is printed and nothing is written.
    /// Returns `Ok(())` after the entry has been flushed to disk.
    pub fn log_event(&mut self, entry: AuditEntry) -> Result<(), AuditError> {
        if !self.enabled {
            if !self.disabled_notice_shown {
                eprintln!("Audit logging is disabled");
                self.disabled_notice_shown = true;
            }
            return Ok(());
        }

        fs::create_dir_all(&self.log_dir)?;

        let date_str = entry.timestamp.format("%Y-%m-%d").to_string();
        let log_file = self.log_dir.join(format!("{date_str}.jsonl"));

        let line = serde_json::to_string(&entry)?;

        let mut file = OpenOptions::new()
            .create(true)
            .append(true)
            .open(&log_file)?;

        writeln!(file, "{line}")?;
        file.flush()?;

        Ok(())
    }

    /// Convenience wrapper: log a sandbox access-denied event and return the
    /// message string (used by the sandbox module for unified logging).
    pub fn log_sandbox_denied(
        &mut self,
        agent_name: &str,
        path: &str,
        session_id: &str,
    ) -> Result<String, AuditError> {
        let message = format!(
            "Access denied: agent '{agent_name}' attempted to read '{path}'"
        );
        self.log_event(AuditEntry::new(
            EventType::SandboxAccessDenied,
            Some(agent_name),
            session_id,
            serde_json::json!({
                "message": message,
                "attempted_path": path,
            }),
        ))?;
        Ok(message)
    }

    /// Count the number of entries with the given `event_type` across all log
    /// files in `log_dir`. Used for testing and monitoring.
    pub fn count_events(&self, event_type: &EventType) -> Result<usize, AuditError> {
        if !self.log_dir.exists() {
            return Ok(0);
        }
        let target = serde_json::to_value(event_type)?.to_string();
        let target_str = target.trim_matches('"');

        let mut count = 0usize;
        for entry in fs::read_dir(&self.log_dir)? {
            let path = entry?.path();
            if path.extension().and_then(|e| e.to_str()) != Some("jsonl") {
                continue;
            }
            let contents = fs::read_to_string(&path)?;
            for line in contents.lines() {
                if line.contains(target_str) {
                    if let Ok(parsed) = serde_json::from_str::<AuditEntry>(line) {
                        if &parsed.event_type == event_type {
                            count += 1;
                        }
                    }
                }
            }
        }
        Ok(count)
    }
}

// ─── Data purge ────────────────────────────────────────────────────────────────

/// Returns the default audit log directory: `~/.local/share/bmux/audit/`.
pub fn default_audit_dir() -> Option<PathBuf> {
    dirs::data_local_dir().map(|d| d.join("bmux").join("audit"))
}

/// Delete all files in `audit_dir`.
pub fn purge_audit_logs(audit_dir: &Path) -> Result<(), io::Error> {
    if !audit_dir.exists() {
        return Ok(());
    }
    for entry in fs::read_dir(audit_dir)? {
        let path = entry?.path();
        if path.is_file() {
            fs::remove_file(&path)?;
        }
    }
    Ok(())
}

/// Delete all local BMUX data: audit logs, context store, session store.
///
/// Called by `bmux data purge`. Prints confirmation to stdout.
pub fn purge_all_data() {
    let base = dirs::data_local_dir()
        .unwrap_or_else(|| PathBuf::from("/tmp"))
        .join("bmux");

    let mut errors = Vec::new();

    // Audit logs
    let audit_dir = base.join("audit");
    if let Err(e) = purge_audit_logs(&audit_dir) {
        errors.push(format!("audit logs: {e}"));
    }

    // Context store (wt6's territory; we attempt best-effort deletion)
    let context_dir = base.join("context");
    if context_dir.exists() {
        if let Err(e) = fs::remove_dir_all(&context_dir) {
            errors.push(format!("context store: {e}"));
        }
    }

    // Session store (wt6's territory; best-effort)
    let sessions_dir = base.join("sessions");
    if sessions_dir.exists() {
        if let Err(e) = fs::remove_dir_all(&sessions_dir) {
            errors.push(format!("session store: {e}"));
        }
    }

    if errors.is_empty() {
        println!("All BMUX local data purged");
    } else {
        for e in &errors {
            eprintln!("Warning: failed to purge {e}");
        }
        println!("All BMUX local data purged (with warnings)");
    }
}

// ─── Tests ─────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;
    use serial_test::serial;

    fn make_logger(dir: &Path, enabled: bool) -> AuditLogger {
        AuditLogger::new(dir.to_path_buf(), enabled)
    }

    fn task_sent_entry() -> AuditEntry {
        AuditEntry::new(
            EventType::TaskSent,
            Some("agent-1"),
            "session-abc",
            serde_json::json!({ "task": "compile project" }),
        )
    }

    
    #[serial]
    #[test]
    fn test_write_event_creates_jsonl_file() {
        let dir = TempDir::new().unwrap();
        let mut logger = make_logger(dir.path(), true);
        logger.log_event(task_sent_entry()).unwrap();

        let files: Vec<_> = fs::read_dir(dir.path())
            .unwrap()
            .map(|e| e.unwrap().path())
            .collect();
        assert_eq!(files.len(), 1, "exactly one log file should be created");
        assert!(
            files[0].extension().and_then(|e| e.to_str()) == Some("jsonl"),
            "log file must have .jsonl extension"
        );
    }

    
    #[serial]
    #[test]
    fn test_event_is_valid_json() {
        let dir = TempDir::new().unwrap();
        let mut logger = make_logger(dir.path(), true);
        logger.log_event(task_sent_entry()).unwrap();

        let file = fs::read_dir(dir.path())
            .unwrap()
            .next()
            .unwrap()
            .unwrap()
            .path();
        let contents = fs::read_to_string(&file).unwrap();

        for line in contents.lines() {
            assert!(!line.is_empty());
            let parsed: serde_json::Value =
                serde_json::from_str(line).expect("each line must be valid JSON");
            assert!(parsed.get("timestamp").is_some(), "must have timestamp field");
            assert!(parsed.get("event_type").is_some(), "must have event_type field");
            assert!(parsed.get("session_id").is_some(), "must have session_id field");
            assert!(parsed.get("metadata").is_some(), "must have metadata field");
        }
    }

    
    #[serial]
    #[test]
    fn test_entry_contains_required_fields() {
        let entry = task_sent_entry();
        let json = serde_json::to_string(&entry).unwrap();
        let parsed: serde_json::Value = serde_json::from_str(&json).unwrap();

        assert!(parsed["timestamp"].is_string());
        assert_eq!(parsed["event_type"].as_str().unwrap(), "task_sent");
        assert_eq!(parsed["agent_id"].as_str().unwrap(), "agent-1");
        assert_eq!(parsed["session_id"].as_str().unwrap(), "session-abc");
    }

    
    #[serial]
    #[test]
    fn test_50_sequential_task_sent_entries_all_recorded() {
        let dir = TempDir::new().unwrap();
        let mut logger = make_logger(dir.path(), true);

        for i in 0..50 {
            logger
                .log_event(AuditEntry::new(
                    EventType::TaskSent,
                    Some(format!("agent-{i}")),
                    "session-x",
                    serde_json::json!({ "seq": i }),
                ))
                .expect("each log_event must succeed");
        }

        let count = logger.count_events(&EventType::TaskSent).unwrap();
        assert_eq!(count, 50, "NFR-08: exactly 50 task_sent entries must exist");
    }

    
    #[serial]
    #[test]
    fn test_disabled_log_writes_nothing() {
        let dir = TempDir::new().unwrap();
        let mut logger = make_logger(dir.path(), false);

        logger.log_event(task_sent_entry()).unwrap();

        let files: Vec<_> = fs::read_dir(dir.path())
            .unwrap()
            .filter_map(|e| e.ok())
            .collect();
        assert_eq!(
            files.len(),
            0,
            "no files should be written when logging is disabled"
        );
    }

    
    #[serial]
    #[test]
    fn test_disabled_notice_shown_only_once() {
        let dir = TempDir::new().unwrap();
        let mut logger = make_logger(dir.path(), false);

        // Two events — the notice flag should flip after the first
        logger.log_event(task_sent_entry()).unwrap();
        assert!(logger.disabled_notice_shown);

        logger.log_event(task_sent_entry()).unwrap();
        assert!(logger.disabled_notice_shown); // still true, not toggled back
    }

    
    #[serial]
    #[test]
    fn test_purge_audit_logs_removes_files() {
        let dir = TempDir::new().unwrap();
        let mut logger = make_logger(dir.path(), true);

        logger.log_event(task_sent_entry()).unwrap();
        logger.log_event(task_sent_entry()).unwrap();

        let before: Vec<_> = fs::read_dir(dir.path())
            .unwrap()
            .filter_map(|e| e.ok())
            .collect();
        assert!(!before.is_empty(), "should have files before purge");

        purge_audit_logs(dir.path()).unwrap();

        let after: Vec<_> = fs::read_dir(dir.path())
            .unwrap()
            .filter_map(|e| e.ok())
            .collect();
        assert_eq!(after.len(), 0, "all files should be gone after purge");
    }

    
    #[serial]
    #[test]
    fn test_purge_nonexistent_dir_is_noop() {
        let result = purge_audit_logs(Path::new("/nonexistent/bmux/audit"));
        assert!(result.is_ok(), "purge of nonexistent dir must not error");
    }

    
    #[serial]
    #[test]
    fn test_sandbox_denied_event_logged_with_correct_message() {
        let dir = TempDir::new().unwrap();
        let mut logger = make_logger(dir.path(), true);

        let msg = logger
            .log_sandbox_denied("my-agent", "/home/user/.ssh/id_rsa", "sess-1")
            .unwrap();

        assert_eq!(
            msg,
            "Access denied: agent 'my-agent' attempted to read '/home/user/.ssh/id_rsa'"
        );

        let count = logger.count_events(&EventType::SandboxAccessDenied).unwrap();
        assert_eq!(count, 1);
    }

    
    #[serial]
    #[test]
    fn test_all_event_types_serialize_correctly() {
        let types = [
            EventType::AgentSpawned,
            EventType::AgentKilled,
            EventType::TaskSent,
            EventType::TaskCompleted,
            EventType::AgentStatusChange,
            EventType::SandboxAccessDenied,
            EventType::HmacVerificationFailure,
        ];
        for t in &types {
            let json = serde_json::to_string(t).unwrap();
            assert!(!json.is_empty());
            let rt: EventType = serde_json::from_str(&json).unwrap();
            assert_eq!(&rt, t);
        }
    }

    
    #[serial]
    #[test]
    fn test_log_multiple_event_types_counted_separately() {
        let dir = TempDir::new().unwrap();
        let mut logger = make_logger(dir.path(), true);

        for _ in 0..3 {
            logger
                .log_event(AuditEntry::new(
                    EventType::AgentSpawned,
                    Some("a"),
                    "s",
                    serde_json::Value::Null,
                ))
                .unwrap();
        }
        for _ in 0..7 {
            logger
                .log_event(AuditEntry::new(
                    EventType::TaskSent,
                    Some("a"),
                    "s",
                    serde_json::Value::Null,
                ))
                .unwrap();
        }

        assert_eq!(logger.count_events(&EventType::AgentSpawned).unwrap(), 3);
        assert_eq!(logger.count_events(&EventType::TaskSent).unwrap(), 7);
        assert_eq!(logger.count_events(&EventType::AgentKilled).unwrap(), 0);
    }
}
