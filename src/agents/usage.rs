//! Token usage and cost tracking for agents that run as interactive TUIs.
//!
//! Interactive agents do not report usage on stdout, so BMUX reads it from the
//! transcript the agent writes to disk. Claude Code is supported today: BMUX
//! starts it with `--session-id <uuid>` and tails
//! `~/.claude/projects/<project>/<uuid>.jsonl` (and `$CLAUDE_CONFIG_DIR/projects`).
//! Every assistant message in that file carries a `usage` block; BMUX sums the
//! final usage of each message and prices it with [`price_per_mtok`].
//!
//! Costs are estimates at public list prices. They do not include discounts,
//! batch pricing or subscription plans.

use std::collections::HashMap;
use std::fs::File;
use std::io::{Read, Seek, SeekFrom};
use std::path::{Path, PathBuf};

/// List prices in USD per million tokens.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct ModelPrice {
    pub input: f64,
    pub output: f64,
}

impl ModelPrice {
    /// Prompt-cache writes are billed at 1.25× the input price.
    pub fn cache_write(&self) -> f64 {
        self.input * 1.25
    }

    /// Prompt-cache reads are billed at 0.1× the input price.
    pub fn cache_read(&self) -> f64 {
        self.input * 0.1
    }
}

/// Look up the list price for a model id such as `claude-sonnet-4-20250514`.
///
/// Returns `None` for unknown model families; callers fall back to the
/// agent's configured `cost_per_1k_tokens`.
pub fn price_per_mtok(model: &str) -> Option<ModelPrice> {
    let m = model.to_ascii_lowercase();
    let price = |input, output| Some(ModelPrice { input, output });

    if m.contains("opus") {
        // Opus 3, 4 and 4.1 were priced higher than later Opus releases.
        let legacy = m.contains("3-opus")
            || m.contains("opus-3")
            || m.contains("opus-4-0")
            || m.contains("opus-4-1")
            || m.contains("opus-4-2025");
        return if legacy {
            price(15.0, 75.0)
        } else {
            price(5.0, 25.0)
        };
    }
    if m.contains("sonnet") {
        return price(3.0, 15.0);
    }
    if m.contains("haiku") {
        if m.contains("3-haiku") || m.contains("haiku-3-2") {
            return price(0.25, 1.25);
        }
        if m.contains("3-5-haiku") || m.contains("haiku-3-5") {
            return price(0.8, 4.0);
        }
        return price(1.0, 5.0);
    }
    None
}

/// Token counts of one assistant message, as written in the transcript.
#[derive(Debug, Clone, Copy, Default, PartialEq)]
struct MessageUsage {
    input: u64,
    output: u64,
    cache_write: u64,
    cache_read: u64,
    cost_usd: f64,
}

/// Aggregated usage for one agent.
#[derive(Debug, Clone, Default, PartialEq)]
pub struct UsageTotals {
    pub input_tokens: u64,
    pub output_tokens: u64,
    pub cache_write_tokens: u64,
    pub cache_read_tokens: u64,
    pub cost_usd: f64,
    /// Most recent model seen in the transcript.
    pub model: Option<String>,
}

impl UsageTotals {
    /// All tokens processed, including prompt-cache reads and writes.
    pub fn total_tokens(&self) -> u64 {
        self.input_tokens + self.output_tokens + self.cache_write_tokens + self.cache_read_tokens
    }
}

/// Directories that can hold Claude Code transcripts: `$CLAUDE_CONFIG_DIR/projects`
/// when set, then `~/.claude/projects`. Both are searched because agents start
/// with a minimal environment that may not include `CLAUDE_CONFIG_DIR`.
pub fn claude_projects_dirs() -> Vec<PathBuf> {
    let mut dirs_out = Vec::new();
    if let Some(dir) = std::env::var_os("CLAUDE_CONFIG_DIR") {
        dirs_out.push(PathBuf::from(dir).join("projects"));
    }
    if let Some(home) = dirs::home_dir() {
        let default = home.join(".claude").join("projects");
        if !dirs_out.contains(&default) {
            dirs_out.push(default);
        }
    }
    dirs_out
}

/// Find `<projects>/<any project>/<session_id>.jsonl`.
pub fn find_transcript(projects_dir: &Path, session_id: &str) -> Option<PathBuf> {
    let file_name = format!("{session_id}.jsonl");
    let entries = std::fs::read_dir(projects_dir).ok()?;
    entries
        .flatten()
        .map(|e| e.path().join(&file_name))
        .find(|p| p.is_file())
}

/// Incrementally reads a Claude Code transcript and keeps running totals.
///
/// Only complete lines are consumed, so a line that is still being written is
/// picked up on the next poll.
#[derive(Debug)]
pub struct TranscriptTracker {
    session_id: String,
    projects_dirs: Vec<PathBuf>,
    fallback_cost_per_1k: f64,
    path: Option<PathBuf>,
    offset: u64,
    messages: HashMap<String, MessageUsage>,
    model: Option<String>,
    last_totals: Option<UsageTotals>,
    working: bool,
    completed_turns: u64,
    last_text: Option<String>,
}

impl TranscriptTracker {
    pub fn new(session_id: &str, projects_dir: PathBuf, fallback_cost_per_1k: f64) -> Self {
        Self {
            session_id: session_id.to_string(),
            projects_dirs: vec![projects_dir],
            fallback_cost_per_1k,
            path: None,
            offset: 0,
            messages: HashMap::new(),
            model: None,
            last_totals: None,
            working: false,
            completed_turns: 0,
            last_text: None,
        }
    }

    /// Tracker for a Claude Code session in the default transcript location.
    pub fn for_claude_session(session_id: &str, fallback_cost_per_1k: f64) -> Option<Self> {
        let dirs = claude_projects_dirs();
        let first = dirs.first()?.clone();
        let mut tracker = Self::new(session_id, first, fallback_cost_per_1k);
        tracker.projects_dirs = dirs;
        Some(tracker)
    }

    pub fn session_id(&self) -> &str {
        &self.session_id
    }

    /// `true` while the agent is in the middle of a turn (prompt received,
    /// final answer not written yet).
    pub fn is_working(&self) -> bool {
        self.working
    }

    /// Number of turns the agent has finished since it started.
    pub fn completed_turns(&self) -> u64 {
        self.completed_turns
    }

    /// Text of the most recent assistant message, if any.
    pub fn last_assistant_text(&self) -> Option<&str> {
        self.last_text.as_deref()
    }

    /// Current running totals.
    pub fn current_totals(&self) -> UsageTotals {
        self.totals()
    }

    /// Read new transcript lines. Returns the totals when they changed.
    pub fn poll(&mut self) -> Option<UsageTotals> {
        if self.path.is_none() {
            self.path = self
                .projects_dirs
                .iter()
                .find_map(|dir| find_transcript(dir, &self.session_id));
        }
        let path = self.path.clone()?;
        let mut file = File::open(&path).ok()?;
        let len = file.metadata().ok()?.len();
        if len < self.offset {
            // Transcript was truncated or replaced: start over.
            self.offset = 0;
            self.messages.clear();
            self.working = false;
        }
        if len == self.offset {
            return None;
        }
        file.seek(SeekFrom::Start(self.offset)).ok()?;
        let mut buf = Vec::with_capacity((len - self.offset) as usize);
        file.take(len - self.offset).read_to_end(&mut buf).ok()?;

        let last_newline = buf.iter().rposition(|&b| b == b'\n')?;
        for line in buf[..last_newline].split(|&b| b == b'\n') {
            self.ingest_line(line);
        }
        self.offset += (last_newline + 1) as u64;

        let totals = self.totals();
        if self.last_totals.as_ref() == Some(&totals) {
            return None;
        }
        self.last_totals = Some(totals.clone());
        Some(totals)
    }

    fn ingest_line(&mut self, line: &[u8]) {
        let Ok(entry) = serde_json::from_slice::<serde_json::Value>(line) else {
            return;
        };
        let Some(message) = entry.get("message") else {
            return;
        };
        let entry_type = entry.get("type").and_then(|v| v.as_str()).unwrap_or("");
        let is_meta = entry
            .get("isMeta")
            .and_then(|v| v.as_bool())
            .unwrap_or(false);
        if entry_type == "user" && !is_meta {
            // A prompt or a tool result: the agent is (still) working.
            self.working = true;
            return;
        }
        if entry_type == "assistant" {
            self.track_turn(message);
        }
        let Some(usage) = message.get("usage") else {
            return;
        };
        let model = message
            .get("model")
            .and_then(|m| m.as_str())
            .filter(|m| !m.starts_with('<'))
            .map(str::to_string);

        let count = |key: &str| usage.get(key).and_then(|v| v.as_u64()).unwrap_or(0);
        let mut msg = MessageUsage {
            input: count("input_tokens"),
            output: count("output_tokens"),
            cache_write: count("cache_creation_input_tokens"),
            cache_read: count("cache_read_input_tokens"),
            cost_usd: 0.0,
        };
        msg.cost_usd = match entry.get("costUSD").and_then(|v| v.as_f64()) {
            Some(cost) => cost,
            None => self.price(model.as_deref(), &msg),
        };

        // Claude Code writes one line per content block; all lines of the same
        // message share its id, and the last one carries the final usage.
        let key = message
            .get("id")
            .and_then(|v| v.as_str())
            .map(str::to_string)
            .or_else(|| {
                entry
                    .get("uuid")
                    .and_then(|v| v.as_str())
                    .map(str::to_string)
            })
            .unwrap_or_else(|| format!("line-{}", self.messages.len()));
        self.messages.insert(key, msg);
        if model.is_some() {
            self.model = model;
        }
    }

    fn track_turn(&mut self, message: &serde_json::Value) {
        let text: Vec<&str> = message
            .get("content")
            .and_then(|c| c.as_array())
            .map(|blocks| {
                blocks
                    .iter()
                    .filter(|b| b.get("type").and_then(|t| t.as_str()) == Some("text"))
                    .filter_map(|b| b.get("text").and_then(|t| t.as_str()))
                    .collect()
            })
            .unwrap_or_default();
        if !text.is_empty() {
            self.last_text = Some(text.join("\n"));
        }
        match message.get("stop_reason").and_then(|v| v.as_str()) {
            Some("end_turn") | Some("stop_sequence") | Some("max_tokens") => {
                if self.working {
                    self.completed_turns += 1;
                }
                self.working = false;
            }
            Some(_) => self.working = true,
            None => {}
        }
    }

    fn price(&self, model: Option<&str>, msg: &MessageUsage) -> f64 {
        const M: f64 = 1_000_000.0;
        match model.and_then(price_per_mtok) {
            Some(p) => {
                msg.input as f64 * p.input / M
                    + msg.output as f64 * p.output / M
                    + msg.cache_write as f64 * p.cache_write() / M
                    + msg.cache_read as f64 * p.cache_read() / M
            }
            None => {
                let tokens = msg.input + msg.output + msg.cache_write + msg.cache_read;
                tokens as f64 / 1000.0 * self.fallback_cost_per_1k
            }
        }
    }

    fn totals(&self) -> UsageTotals {
        let mut t = UsageTotals {
            model: self.model.clone(),
            ..Default::default()
        };
        for m in self.messages.values() {
            t.input_tokens += m.input;
            t.output_tokens += m.output;
            t.cache_write_tokens += m.cache_write;
            t.cache_read_tokens += m.cache_read;
            t.cost_usd += m.cost_usd;
        }
        t
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Write;

    fn assistant_line(id: &str, model: &str, input: u64, output: u64, cache_read: u64) -> String {
        format!(
            r#"{{"type":"assistant","message":{{"id":"{id}","model":"{model}","role":"assistant","usage":{{"input_tokens":{input},"output_tokens":{output},"cache_creation_input_tokens":0,"cache_read_input_tokens":{cache_read}}}}}}}"#
        ) + "\n"
    }

    fn setup(session: &str) -> (tempfile::TempDir, PathBuf) {
        let dir = tempfile::tempdir().unwrap();
        let project = dir.path().join("-tmp-project");
        std::fs::create_dir_all(&project).unwrap();
        let file = project.join(format!("{session}.jsonl"));
        (dir, file)
    }

    #[test]
    fn prices_known_families() {
        assert_eq!(
            price_per_mtok("claude-sonnet-4-20250514").unwrap().input,
            3.0
        );
        assert_eq!(
            price_per_mtok("claude-opus-4-1-20250805").unwrap().output,
            75.0
        );
        assert_eq!(price_per_mtok("claude-opus-4-5").unwrap().output, 25.0);
        assert_eq!(price_per_mtok("claude-haiku-4-5").unwrap().input, 1.0);
        assert_eq!(
            price_per_mtok("claude-3-5-haiku-20241022").unwrap().input,
            0.8
        );
        assert!(price_per_mtok("gemini-2.5-pro").is_none());
    }

    #[test]
    fn returns_none_until_transcript_exists() {
        let (dir, file) = setup("abc");
        let mut tracker = TranscriptTracker::new("abc", dir.path().to_path_buf(), 0.003);
        assert!(tracker.poll().is_none());
        std::fs::write(&file, assistant_line("m1", "claude-sonnet-4-5", 1000, 0, 0)).unwrap();
        let t = tracker.poll().expect("totals after transcript appears");
        assert_eq!(t.input_tokens, 1000);
        assert!((t.cost_usd - 0.003).abs() < 1e-9);
    }

    #[test]
    fn deduplicates_content_blocks_of_the_same_message() {
        let (dir, file) = setup("s1");
        let mut f = File::create(&file).unwrap();
        // Two content blocks of message m1 (same usage), then message m2.
        f.write_all(assistant_line("m1", "claude-sonnet-4-5", 100, 50, 1000).as_bytes())
            .unwrap();
        f.write_all(assistant_line("m1", "claude-sonnet-4-5", 100, 50, 1000).as_bytes())
            .unwrap();
        f.write_all(assistant_line("m2", "claude-sonnet-4-5", 10, 200, 0).as_bytes())
            .unwrap();
        f.write_all(b"{\"type\":\"user\",\"message\":{\"role\":\"user\",\"content\":\"hi\"}}\n")
            .unwrap();
        drop(f);

        let mut tracker = TranscriptTracker::new("s1", dir.path().to_path_buf(), 0.0);
        let t = tracker.poll().unwrap();
        assert_eq!(t.input_tokens, 110);
        assert_eq!(t.output_tokens, 250);
        assert_eq!(t.cache_read_tokens, 1000);
        assert_eq!(t.total_tokens(), 1360);
        // 110*3 + 250*15 + 1000*0.3 per million
        let expected = (110.0 * 3.0 + 250.0 * 15.0 + 1000.0 * 0.3) / 1_000_000.0;
        assert!((t.cost_usd - expected).abs() < 1e-12);
        assert_eq!(t.model.as_deref(), Some("claude-sonnet-4-5"));
    }

    #[test]
    fn reads_incrementally_and_waits_for_complete_lines() {
        let (dir, file) = setup("s2");
        std::fs::write(&file, assistant_line("m1", "claude-sonnet-4-5", 1000, 0, 0)).unwrap();
        let mut tracker = TranscriptTracker::new("s2", dir.path().to_path_buf(), 0.0);
        assert_eq!(tracker.poll().unwrap().input_tokens, 1000);
        assert!(tracker.poll().is_none(), "no change → None");

        let line = assistant_line("m2", "claude-sonnet-4-5", 500, 0, 0);
        let (head, tail) = line.split_at(20);
        let mut f = std::fs::OpenOptions::new()
            .append(true)
            .open(&file)
            .unwrap();
        f.write_all(head.as_bytes()).unwrap();
        f.flush().unwrap();
        assert!(tracker.poll().is_none(), "partial line is not consumed");
        f.write_all(tail.as_bytes()).unwrap();
        f.flush().unwrap();
        assert_eq!(tracker.poll().unwrap().input_tokens, 1500);
    }

    #[test]
    fn prefers_cost_written_by_the_agent() {
        let (dir, file) = setup("s3");
        std::fs::write(
            &file,
            r#"{"costUSD":0.5,"message":{"id":"m1","model":"claude-sonnet-4-5","usage":{"input_tokens":1,"output_tokens":1}}}"#.to_string() + "\n",
        )
        .unwrap();
        let mut tracker = TranscriptTracker::new("s3", dir.path().to_path_buf(), 0.0);
        assert!((tracker.poll().unwrap().cost_usd - 0.5).abs() < 1e-12);
    }

    #[test]
    fn tracks_turns_and_last_answer() {
        let (dir, file) = setup("s5");
        let mut tracker = TranscriptTracker::new("s5", dir.path().to_path_buf(), 0.0);
        let user = r#"{"type":"user","message":{"role":"user","content":"fix the bug"}}"#;
        let tool = r#"{"type":"assistant","message":{"id":"a1","model":"claude-sonnet-4-5","stop_reason":"tool_use","content":[{"type":"tool_use","name":"Bash"}],"usage":{"input_tokens":5,"output_tokens":5}}}"#;
        let result =
            r#"{"type":"user","message":{"role":"user","content":[{"type":"tool_result"}]}}"#;
        let done = r#"{"type":"assistant","message":{"id":"a2","model":"claude-sonnet-4-5","stop_reason":"end_turn","content":[{"type":"text","text":"Fixed it."}],"usage":{"input_tokens":5,"output_tokens":5}}}"#;

        std::fs::write(&file, format!("{user}\n{tool}\n")).unwrap();
        tracker.poll();
        assert!(tracker.is_working());
        assert_eq!(tracker.completed_turns(), 0);

        let mut f = std::fs::OpenOptions::new()
            .append(true)
            .open(&file)
            .unwrap();
        writeln!(f, "{result}").unwrap();
        writeln!(f, "{done}").unwrap();
        drop(f);
        tracker.poll();
        assert!(!tracker.is_working());
        assert_eq!(tracker.completed_turns(), 1);
        assert_eq!(tracker.last_assistant_text(), Some("Fixed it."));

        // A meta entry does not start a turn.
        let mut f = std::fs::OpenOptions::new()
            .append(true)
            .open(&file)
            .unwrap();
        writeln!(
            f,
            r#"{{"type":"user","isMeta":true,"message":{{"role":"user","content":"x"}}}}"#
        )
        .unwrap();
        drop(f);
        tracker.poll();
        assert!(!tracker.is_working());
    }

    #[test]
    fn unknown_model_uses_fallback_rate() {
        let (dir, file) = setup("s4");
        std::fs::write(&file, assistant_line("m1", "mystery-model", 1000, 1000, 0)).unwrap();
        let mut tracker = TranscriptTracker::new("s4", dir.path().to_path_buf(), 0.002);
        assert!((tracker.poll().unwrap().cost_usd - 0.004).abs() < 1e-12);
    }
}
