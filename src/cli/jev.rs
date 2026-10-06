//! `talos-cli jev <case>` — the decision engine, exposed to drivers.
//!
//! Six typed System-1 decisions, each with a model path and a deterministic
//! fallback. This is the surface agent hooks and scripts call: a PreToolUse
//! hook runs `talos-cli jev guardrail --command "$TOOL_INPUT"`, a dispatch
//! script sizes a task before spawning, a verify loop triages a failure.
//!
//! Every answer is the same JSON shape and carries `provider` — `jev`,
//! `heuristic-fallback`, or `static-rule` — so a caller never branches on who
//! answered. A timeout or an absent API key is not an error: the fallback
//! answers. Exit 0 means a decision was made, whatever its source.

use clap::{Args, Subcommand};

use crate::jev;

use super::output::CommandOutput;

/// `jev` subcommands: the six decision cases.
#[derive(Subcommand, Debug)]
pub enum Action {
    /// CASO 1 — size a task: size, pipeline, tier, target agent, human review.
    Sizing(SizingArgs),
    /// CASO 2 — route a chat message: intent, target persona, next action.
    Intent(IntentArgs),
    /// CASO 3 — inspect a shell command: SAFE / RISKY / BLOCKED.
    Guardrail(GuardrailArgs),
    /// CASO 4 — audit a PRD or RFC: verdict + completeness.
    #[command(name = "audit-spec")]
    AuditSpec(AuditArgs),
    /// CASO 5 — verify a task is atomic (one commit / PR).
    Atomicity(AtomicityArgs),
    /// CASO 6 — triage a test failure: root cause + healing strategy.
    Triage(TriageArgs),
}

#[derive(Args, Debug)]
pub struct SizingArgs {
    /// The task title.
    #[arg(long)]
    pub title: String,
    /// The task description.
    #[arg(long, default_value = "")]
    pub description: String,
    /// Optional context snippet.
    #[arg(long)]
    pub context: Option<String>,
}

#[derive(Args, Debug)]
pub struct IntentArgs {
    /// The user message to classify.
    #[arg(long = "user-message")]
    pub user_message: String,
    /// Optional conversation history snippet.
    #[arg(long)]
    pub history: Option<String>,
    /// The current branch, when there is one.
    #[arg(long = "current-branch")]
    pub current_branch: Option<String>,
}

#[derive(Args, Debug)]
pub struct GuardrailArgs {
    /// The shell command to inspect.
    #[arg(long)]
    pub command: String,
}

#[derive(Args, Debug)]
pub struct AuditArgs {
    /// Document type: `PRD` or `RFC`.
    #[arg(long = "doc-type")]
    pub doc_type: String,
    /// The document text (or pass it on stdin).
    #[arg(long)]
    pub content: String,
    /// Optional project name.
    #[arg(long = "project-name")]
    pub project_name: Option<String>,
}

#[derive(Args, Debug)]
pub struct AtomicityArgs {
    /// The task title.
    #[arg(long)]
    pub title: String,
    /// The task description.
    #[arg(long, default_value = "")]
    pub description: String,
    /// Optional module scope.
    #[arg(long = "module-scope")]
    pub module_scope: Option<String>,
}

#[derive(Args, Debug)]
pub struct TriageArgs {
    /// The command that failed.
    #[arg(long, default_value = "test")]
    pub command: String,
    /// Its exit code.
    #[arg(long = "exit-code")]
    pub exit_code: i32,
    /// Its output.
    #[arg(long = "test-output")]
    pub test_output: String,
    /// Optional git diff snippet.
    #[arg(long = "git-diff")]
    pub git_diff: Option<String>,
}

/// Run a Jev decision and render it. Errors here are reserved for a caller
/// mistake (an unknown doc type); an unreachable engine is never an error.
pub fn run(action: Action) -> Result<CommandOutput, String> {
    let json = match action {
        Action::Sizing(a) => jev::sizing::run(&jev::SizingInput {
            title: a.title,
            description: a.description,
            context: a.context,
        }),
        Action::Intent(a) => jev::intent::run(&jev::IntentInput {
            user_message: a.user_message,
            history: a.history,
            current_branch: a.current_branch,
        }),
        Action::Guardrail(a) => jev::guardrail::run(&jev::GuardrailInput { command: a.command }),
        Action::AuditSpec(a) => jev::audit::run(&jev::AuditInput {
            doc_type: a.doc_type,
            content: a.content,
            project_name: a.project_name,
        }),
        Action::Atomicity(a) => jev::atomicity::run(&jev::AtomicityInput {
            title: a.title,
            description: a.description,
            module_scope: a.module_scope,
        }),
        Action::Triage(a) => jev::triage::run(&jev::TriageInput {
            command: a.command,
            exit_code: a.exit_code,
            test_output: a.test_output,
            git_diff: a.git_diff,
        }),
    }
    .map_err(|e| e.to_string())?;

    let human = serde_json::to_string_pretty(&json).unwrap_or_else(|_| json.to_string());
    Ok(CommandOutput::new(json, human))
}
