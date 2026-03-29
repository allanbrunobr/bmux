// tests/audit_log_test.rs — Integration tests for Story 6.3
// Audit Logging

use bmux::storage::audit_log::{
    purge_audit_logs, AuditEntry, AuditLogger, EventType,
};
use std::fs;
use tempfile::TempDir;

fn logger(dir: &TempDir, enabled: bool) -> AuditLogger {
    AuditLogger::new(dir.path().to_path_buf(), enabled)
}

fn entry(event: EventType) -> AuditEntry {
    AuditEntry::new(event, Some("agent-1"), "session-test", serde_json::Value::Null)
}

#[test]
fn integration_audit_enabled_creates_jsonl_file() {
    let dir = TempDir::new().unwrap();
    let mut log = logger(&dir, true);
    log.log_event(entry(EventType::AgentSpawned)).unwrap();

    let files: Vec<_> = fs::read_dir(dir.path())
        .unwrap()
        .map(|e| e.unwrap().path())
        .collect();
    assert_eq!(files.len(), 1);
    assert!(files[0].to_str().unwrap().ends_with(".jsonl"));
}

#[test]
fn integration_audit_entries_are_valid_jsonl() {
    let dir = TempDir::new().unwrap();
    let mut log = logger(&dir, true);

    for e in [
        EventType::AgentSpawned,
        EventType::TaskSent,
        EventType::TaskCompleted,
        EventType::AgentKilled,
    ] {
        log.log_event(entry(e)).unwrap();
    }

    let file = fs::read_dir(dir.path())
        .unwrap()
        .next()
        .unwrap()
        .unwrap()
        .path();
    let content = fs::read_to_string(&file).unwrap();
    let lines: Vec<&str> = content.lines().collect();
    assert_eq!(lines.len(), 4, "must have 4 lines");

    for line in &lines {
        let parsed: serde_json::Value = serde_json::from_str(line)
            .unwrap_or_else(|_| panic!("line must be valid JSON: {line}"));
        assert!(parsed["timestamp"].is_string());
        assert!(parsed["event_type"].is_string());
        assert!(parsed["session_id"].is_string());
    }
}

/// NFR-08: 100% audit completeness — 50 tasks all recorded
#[test]
fn integration_audit_50_task_sent_all_recorded() {
    let dir = TempDir::new().unwrap();
    let mut log = logger(&dir, true);

    for i in 0..50u32 {
        log.log_event(AuditEntry::new(
            EventType::TaskSent,
            Some(format!("agent-{i}")),
            "session-nfr08",
            serde_json::json!({ "seq": i }),
        ))
        .unwrap();
    }

    let count = log.count_events(&EventType::TaskSent).unwrap();
    assert_eq!(count, 50, "NFR-08: exactly 50 task_sent entries required");
}

#[test]
fn integration_audit_disabled_writes_no_files() {
    let dir = TempDir::new().unwrap();
    let mut log = logger(&dir, false);
    log.log_event(entry(EventType::AgentSpawned)).unwrap();
    log.log_event(entry(EventType::TaskSent)).unwrap();

    let files: Vec<_> = fs::read_dir(dir.path())
        .unwrap()
        .filter_map(|e| e.ok())
        .collect();
    assert_eq!(files.len(), 0, "disabled logger must write no files");
}

#[test]
fn integration_audit_purge_removes_all_log_files() {
    let dir = TempDir::new().unwrap();
    let mut log = logger(&dir, true);

    log.log_event(entry(EventType::AgentSpawned)).unwrap();
    log.log_event(entry(EventType::TaskSent)).unwrap();

    purge_audit_logs(dir.path()).unwrap();

    let remaining: Vec<_> = fs::read_dir(dir.path())
        .unwrap()
        .filter_map(|e| e.ok())
        .collect();
    assert_eq!(remaining.len(), 0, "purge must remove all log files");
}

#[test]
fn integration_audit_sandbox_denied_message_format() {
    let dir = TempDir::new().unwrap();
    let mut log = logger(&dir, true);

    let msg = log
        .log_sandbox_denied("my-agent", "~/.ssh/id_rsa", "sess-1")
        .unwrap();

    assert_eq!(
        msg,
        "Access denied: agent 'my-agent' attempted to read '~/.ssh/id_rsa'"
    );

    let count = log.count_events(&EventType::SandboxAccessDenied).unwrap();
    assert_eq!(count, 1, "sandbox denied event must be recorded");
}

#[test]
fn integration_audit_all_seven_event_types_recordable() {
    let dir = TempDir::new().unwrap();
    let mut log = logger(&dir, true);

    let events = [
        EventType::AgentSpawned,
        EventType::AgentKilled,
        EventType::TaskSent,
        EventType::TaskCompleted,
        EventType::AgentStatusChange,
        EventType::SandboxAccessDenied,
        EventType::HmacVerificationFailure,
    ];

    for e in events {
        log.log_event(entry(e)).unwrap();
    }

    // Read back and verify all 7 lines
    let file = fs::read_dir(dir.path())
        .unwrap()
        .next()
        .unwrap()
        .unwrap()
        .path();
    let lines: Vec<_> = fs::read_to_string(&file)
        .unwrap()
        .lines()
        .map(String::from)
        .collect();
    assert_eq!(lines.len(), 7, "all 7 event types must produce 7 log lines");
}
