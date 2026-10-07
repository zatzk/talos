//! Model & Agent Selector for Talos v3.
//!
//! Manages active execution selection (Auto, API provider/model, CLI agent/model)
//! triggered via F4 / Ctrl+O in the TUI or configured per session/thread.

use serde::{Deserialize, Serialize};

use super::contract::ExecutionTarget;

/// An option in the Model/Agent selector.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SelectorItem {
    pub label: String,
    pub description: String,
    pub target: ExecutionTarget,
}

/// State of the model/agent selector.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SelectorState {
    pub current: ExecutionTarget,
    pub available_items: Vec<SelectorItem>,
    pub selected_index: usize,
    pub is_open: bool,
}

impl Default for SelectorState {
    fn default() -> Self {
        Self::new()
    }
}

impl SelectorState {
    pub fn new() -> Self {
        let available_items = vec![
            SelectorItem {
                label: "⚡ Auto (Jev Decision Engine)".into(),
                description: "Automatic persona, tier and backend selection".into(),
                target: ExecutionTarget::Auto,
            },
            SelectorItem {
                label: "Claude 3.7 Sonnet (API)".into(),
                description: "Direct OpenRouter SSE streaming".into(),
                target: ExecutionTarget::ApiDirect {
                    provider: "openrouter".into(),
                    model: "anthropic/claude-3.7-sonnet".into(),
                },
            },
            SelectorItem {
                label: "Claude 3.5 Haiku (API - Fast)".into(),
                description: "Low latency Direct API streaming".into(),
                target: ExecutionTarget::ApiDirect {
                    provider: "openrouter".into(),
                    model: "anthropic/claude-3.5-haiku".into(),
                },
            },
            SelectorItem {
                label: "Claude Code CLI".into(),
                description: "claude -p headless execution".into(),
                target: ExecutionTarget::CliAgent {
                    agent: "claude".into(),
                    model_override: None,
                },
            },
            SelectorItem {
                label: "Codex CLI".into(),
                description: "codex exec headless execution".into(),
                target: ExecutionTarget::CliAgent {
                    agent: "codex".into(),
                    model_override: None,
                },
            },
            SelectorItem {
                label: "Antigravity CLI (agy)".into(),
                description: "agy --headless execution".into(),
                target: ExecutionTarget::CliAgent {
                    agent: "agy".into(),
                    model_override: None,
                },
            },
        ];

        Self {
            current: ExecutionTarget::Auto,
            available_items,
            selected_index: 0,
            is_open: false,
        }
    }

    pub fn toggle_open(&mut self) {
        self.is_open = !self.is_open;
    }

    pub fn next(&mut self) {
        if !self.available_items.is_empty() {
            self.selected_index = (self.selected_index + 1) % self.available_items.len();
        }
    }

    pub fn previous(&mut self) {
        if !self.available_items.is_empty() {
            if self.selected_index == 0 {
                self.selected_index = self.available_items.len() - 1;
            } else {
                self.selected_index -= 1;
            }
        }
    }

    pub fn select_active(&mut self) {
        if let Some(item) = self.available_items.get(self.selected_index) {
            self.current = item.target.clone();
            self.is_open = false;
        }
    }

    pub fn set_target(&mut self, target: ExecutionTarget) {
        self.current = target;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn selector_navigation_and_selection() {
        let mut state = SelectorState::new();
        assert_eq!(state.current, ExecutionTarget::Auto);
        assert!(!state.is_open);

        state.toggle_open();
        assert!(state.is_open);

        state.next();
        assert_eq!(state.selected_index, 1);

        state.select_active();
        assert!(!state.is_open);
        assert!(matches!(state.current, ExecutionTarget::ApiDirect { .. }));
    }
}
