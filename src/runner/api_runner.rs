//! API Runner for Talos v3.
//!
//! Direct HTTP client streaming responses from OpenRouter, 9Router, or OpenAI-compatible endpoints.

use std::sync::mpsc::Sender;
use std::time::Instant;

use serde_json::json;

use super::contract::{
    BackendRunner, CancelToken, ExecutionTarget, RunRequest, RunSummary, RunnerError, StreamEvent,
    TokenUsage,
};

pub struct ApiRunner {
    pub endpoint: String,
    pub api_key: Option<String>,
}

impl ApiRunner {
    pub fn new(endpoint: impl Into<String>, api_key: Option<String>) -> Self {
        Self {
            endpoint: endpoint.into(),
            api_key,
        }
    }

    pub fn from_env() -> Self {
        let endpoint = std::env::var("TALOS_API_ENDPOINT")
            .unwrap_or_else(|_| "https://openrouter.ai/api/v1/chat/completions".into());
        let api_key = std::env::var("TALOS_API_KEY")
            .or_else(|_| std::env::var("OPENROUTER_API_KEY"))
            .ok();
        Self { endpoint, api_key }
    }
}

impl BackendRunner for ApiRunner {
    fn run(
        &self,
        req: RunRequest,
        tx: Sender<StreamEvent>,
        cancel: CancelToken,
    ) -> Result<RunSummary, RunnerError> {
        let start = Instant::now();

        if cancel.is_canceled() {
            return Err(RunnerError::Canceled);
        }

        let model = match &req.target {
            ExecutionTarget::ApiDirect { model, .. } => model.clone(),
            _ => "anthropic/claude-3.7-sonnet".to_string(),
        };

        let mut messages = Vec::new();
        if !req.system_prompt.is_empty() {
            messages.push(json!({
                "role": "system",
                "content": req.system_prompt,
            }));
        }

        // Include retrieved memory context if present
        if !req.retrieved_context.is_empty() {
            let mut context_block = String::from("Retrieved Context:\n");
            for chunk in &req.retrieved_context {
                context_block.push_str(&format!("- [{}] {}\n", chunk.source, chunk.content));
            }
            messages.push(json!({
                "role": "system",
                "content": context_block,
            }));
        }

        for turn in &req.history {
            messages.push(json!({
                "role": turn.role,
                "content": turn.content,
            }));
        }

        messages.push(json!({
            "role": "user",
            "content": req.prompt,
        }));

        let body = json!({
            "model": model,
            "messages": messages,
            "stream": false, // blocking request for simple turn
        });

        let client = reqwest::blocking::Client::builder()
            .timeout(std::time::Duration::from_secs(60))
            .build()
            .map_err(|e| RunnerError::Transport(e.to_string()))?;

        let mut request = client.post(&self.endpoint).json(&body);
        if let Some(ref key) = self.api_key {
            request = request.header("Authorization", format!("Bearer {key}"));
        }

        let response = request
            .send()
            .map_err(|e| RunnerError::Transport(e.to_string()))?;

        if !response.status().is_success() {
            let err_text = response.text().unwrap_or_default();
            return Err(RunnerError::Transport(format!("HTTP error: {err_text}")));
        }

        let res_json: serde_json::Value = response
            .json()
            .map_err(|e| RunnerError::Transport(format!("Failed to parse JSON: {e}")))?;

        let content = res_json["choices"][0]["message"]["content"]
            .as_str()
            .unwrap_or("")
            .to_string();

        let prompt_tokens = res_json["usage"]["prompt_tokens"].as_u64().unwrap_or(0) as usize;
        let completion_tokens =
            res_json["usage"]["completion_tokens"].as_u64().unwrap_or(0) as usize;
        let usage = TokenUsage {
            prompt_tokens,
            completion_tokens,
            total_tokens: prompt_tokens + completion_tokens,
        };

        let _ = tx.send(StreamEvent::Delta(content.clone()));
        let _ = tx.send(StreamEvent::Usage(usage.clone()));

        let summary = RunSummary {
            target: req.target.clone(),
            full_response: content,
            usage: Some(usage),
            latency_ms: start.elapsed().as_millis() as u64,
            provider: "api-direct".into(),
        };

        let _ = tx.send(StreamEvent::Done(summary.clone()));
        Ok(summary)
    }
}
