//! Universal Runner Contract for Talos v3.
//!
//! Provides the core traits, request/response models, execution targets,
//! stream events, and cancellation tokens for API and CLI agent execution.

use std::path::PathBuf;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::mpsc::Sender;
use std::sync::Arc;

use serde::{Deserialize, Serialize};

/// Target for execution: automatic routing, direct API, or local headless CLI agent.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum ExecutionTarget {
    /// Let Jev decide persona, tier, and backend.
    Auto,
    /// Direct API call (e.g. OpenRouter, 9Router, local LLM endpoint).
    ApiDirect {
        provider: String,
        model: String,
    },
    /// Headless invocation of a CLI agent (claude, codex, agy, pi, omp, etc.).
    CliAgent {
        agent: String,
        model_override: Option<String>,
    },
}

impl Default for ExecutionTarget {
    fn default() -> Self {
        Self::Auto
    }
}

/// A single turn in conversation history.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ChatTurn {
    pub role: String,
    pub content: String,
}

/// A chunk of memory retrieved before execution.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MemoryChunk {
    pub source: String,
    pub score: f32,
    pub content: String,
}

/// Token usage report.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct TokenUsage {
    pub prompt_tokens: usize,
    pub completion_tokens: usize,
    pub total_tokens: usize,
}

/// Final summary of an execution run.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RunSummary {
    pub target: ExecutionTarget,
    pub full_response: String,
    pub usage: Option<TokenUsage>,
    pub latency_ms: u64,
    pub provider: String,
}

/// Request sent to a BackendRunner.
#[derive(Debug, Clone)]
pub struct RunRequest {
    pub workspace_id: String,
    pub thread_id: String,
    pub prompt: String,
    pub system_prompt: String,
    pub history: Vec<ChatTurn>,
    pub retrieved_context: Vec<MemoryChunk>,
    pub target: ExecutionTarget,
    pub cwd: PathBuf,
}

/// Events streamed back during execution.
#[derive(Debug, Clone)]
pub enum StreamEvent {
    Delta(String),
    Usage(TokenUsage),
    Artifact(PathBuf),
    Done(RunSummary),
    Error(String),
}

/// Cancellation token allowing graceful abort of ongoing runs (e.g., on Esc).
#[derive(Debug, Clone, Default)]
pub struct CancelToken {
    canceled: Arc<AtomicBool>,
}

impl CancelToken {
    pub fn new() -> Self {
        Self {
            canceled: Arc::new(AtomicBool::new(false)),
        }
    }

    pub fn cancel(&self) {
        self.canceled.store(true, Ordering::SeqCst);
    }

    pub fn is_canceled(&self) -> bool {
        self.canceled.load(Ordering::SeqCst)
    }
}

#[derive(Debug)]
pub enum RunnerError {
    Canceled,
    Transport(String),
    CliExecution(String),
    InvalidRequest(String),
    Timeout(u64),
}

impl std::fmt::Display for RunnerError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Canceled => write!(f, "Execution canceled by user"),
            Self::Transport(msg) => write!(f, "Transport or HTTP error: {msg}"),
            Self::CliExecution(msg) => write!(f, "CLI execution failed: {msg}"),
            Self::InvalidRequest(msg) => write!(f, "Invalid request: {msg}"),
            Self::Timeout(ms) => write!(f, "Timeout exceeded: {ms}ms"),
        }
    }
}

impl std::error::Error for RunnerError {}

/// Trait implemented by API and CLI runners.
pub trait BackendRunner: Send + Sync {
    fn run(
        &self,
        req: RunRequest,
        tx: Sender<StreamEvent>,
        cancel: CancelToken,
    ) -> Result<RunSummary, RunnerError>;
}
