//! The Jev decision engine: typed System-1 decisions woven into the agent
//! lifecycle. Six cases, one contract: every decision has a model-driven path
//! and a deterministic fallback; the `provider` field says who answered.

pub mod atomicity;
pub mod audit;
mod client;
pub mod guardrail;
pub mod intent;
pub mod sizing;
pub mod triage;

pub use client::Jev;

/// The input a sizing decision needs.
pub struct SizingInput {
    pub title: String,
    pub description: String,
    pub context: Option<String>,
}

/// The input an intent-routing decision needs.
pub struct IntentInput {
    pub user_message: String,
    pub history: Option<String>,
    pub current_branch: Option<String>,
}

/// The input a guardrail decision needs.
pub struct GuardrailInput {
    pub command: String,
}

/// The input a spec-audit decision needs.
pub struct AuditInput {
    pub doc_type: String, // "PRD" or "RFC"
    pub content: String,
    pub project_name: Option<String>,
}

/// The input an atomicity decision needs.
pub struct AtomicityInput {
    pub title: String,
    pub description: String,
    pub module_scope: Option<String>,
}

/// The input a test-triage decision needs.
pub struct TriageInput {
    pub command: String,
    pub exit_code: i32,
    pub test_output: String,
    pub git_diff: Option<String>,
}