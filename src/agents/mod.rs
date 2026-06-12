// Agents module — agent system for BMUX.
//
// Provides:
// - AgentAdapter trait (common interface for all agents)
// - ShellAdapter, ClaudeCodeAdapter, OpenCodeAdapter, PiAdapter, GeminiAdapter, CustomAdapter
// - AgentRegistry (manages spawned agents and their runtime state)
// - CLI command handlers for the `bmux agent` subcommand

pub mod adapter;
pub mod claude_code;
pub mod commands;
pub mod custom;
pub mod gemini;
pub mod generic_cli;
pub mod opencode;
pub mod pi;
pub mod registry;
pub mod runtime;
pub mod shell;

// Re-export key types for ergonomic use
pub use adapter::{AgentAdapter, AgentConfig, AgentOutput, AgentStatus, Task};
pub use generic_cli::GenericCliAdapter;
pub use registry::{AgentInfo, AgentRegistry, AggregateStats};
