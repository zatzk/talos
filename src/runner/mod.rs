//! Universal Runner module for Talos v3.
//!
//! Provides execution runners, stream management, and model/agent selector.

pub mod api_runner;
pub mod cli_runner;
pub mod contract;
pub mod selector;

pub use api_runner::ApiRunner;
pub use cli_runner::CliHeadlessRunner;
pub use contract::{
    BackendRunner, CancelToken, ChatTurn, ExecutionTarget, MemoryChunk, RunRequest, RunSummary,
    RunnerError, StreamEvent, TokenUsage,
};
pub use selector::{SelectorItem, SelectorState};
