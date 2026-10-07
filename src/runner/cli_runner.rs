//! Headless CLI Runner for Talos v3.
//!
//! Executes coding agents locally in headless mode (e.g. claude -p, codex exec, agy --headless).

use std::io::{BufRead, BufReader};
use std::process::{Command, Stdio};
use std::sync::mpsc::Sender;
use std::time::Instant;

use super::contract::{
    BackendRunner, CancelToken, ExecutionTarget, RunRequest, RunSummary, RunnerError, StreamEvent,
};

pub struct CliHeadlessRunner;

impl CliHeadlessRunner {
    pub fn new() -> Self {
        Self
    }
}

impl Default for CliHeadlessRunner {
    fn default() -> Self {
        Self::new()
    }
}

impl BackendRunner for CliHeadlessRunner {
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

        let (agent_bin, extra_args) = match &req.target {
            ExecutionTarget::CliAgent {
                agent,
                model_override,
            } => {
                let mut args = Vec::new();
                if let Some(ref m) = model_override {
                    args.push("--model".to_string());
                    args.push(m.clone());
                }
                (agent.clone(), args)
            }
            _ => ("claude".to_string(), Vec::new()),
        };

        // Enrich prompt with system instructions and memory if present
        let mut full_prompt = String::new();
        if !req.system_prompt.is_empty() {
            full_prompt.push_str(&format!("System: {}\n\n", req.system_prompt));
        }
        if !req.retrieved_context.is_empty() {
            full_prompt.push_str("Retrieved Memory Context:\n");
            for chunk in &req.retrieved_context {
                full_prompt.push_str(&format!("- [{}] {}\n", chunk.source, chunk.content));
            }
            full_prompt.push('\n');
        }
        full_prompt.push_str(&req.prompt);

        // Build command depending on known agent conventions
        let mut cmd = Command::new(&agent_bin);
        cmd.current_dir(&req.cwd);
        cmd.stdout(Stdio::piped());
        cmd.stderr(Stdio::piped());

        match agent_bin.as_str() {
            "claude" => {
                cmd.arg("-p");
                cmd.arg(&full_prompt);
                for arg in extra_args {
                    cmd.arg(arg);
                }
            }
            "codex" => {
                cmd.arg("exec");
                cmd.arg(&full_prompt);
                for arg in extra_args {
                    cmd.arg(arg);
                }
            }
            "agy" => {
                cmd.arg("--headless");
                cmd.arg(&full_prompt);
                for arg in extra_args {
                    cmd.arg(arg);
                }
            }
            _ => {
                cmd.args(&extra_args);
                cmd.arg(&full_prompt);
            }
        }

        let mut child = cmd
            .spawn()
            .map_err(|e| RunnerError::CliExecution(format!("Failed to spawn {agent_bin}: {e}")))?;

        let stdout = child.stdout.take().ok_or_else(|| {
            RunnerError::CliExecution("Failed to capture stdout of child process".into())
        })?;

        let reader = BufReader::new(stdout);
        let mut full_response = String::new();

        for line_res in reader.lines() {
            if cancel.is_canceled() {
                let _ = child.kill();
                return Err(RunnerError::Canceled);
            }

            match line_res {
                Ok(line) => {
                    full_response.push_str(&line);
                    full_response.push('\n');
                    let _ = tx.send(StreamEvent::Delta(line));
                }
                Err(e) => {
                    let _ = tx.send(StreamEvent::Error(e.to_string()));
                    break;
                }
            }
        }

        let status = child
            .wait()
            .map_err(|e| RunnerError::CliExecution(format!("Failed to wait on child: {e}")))?;

        if !status.success() {
            return Err(RunnerError::CliExecution(format!(
                "Process exited with non-zero status: {status}"
            )));
        }

        let summary = RunSummary {
            target: req.target.clone(),
            full_response,
            usage: None,
            latency_ms: start.elapsed().as_millis() as u64,
            provider: format!("cli-{agent_bin}"),
        };

        let _ = tx.send(StreamEvent::Done(summary.clone()));
        Ok(summary)
    }
}
