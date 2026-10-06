//! `talos-cli harness [sync]` — keep the CLI agent directories in step with
//! spec-harness-kit.
//!
//! Startup does this automatically when the harness changed (see
//! [`crate::harness`]); this is the manual surface — run it after editing the
//! harness in place, or to see what a sync would do (`--check`).

use clap::{Args, Subcommand};

use crate::harness;

use super::output::CommandOutput;

/// `harness` subcommands.
#[derive(Subcommand, Debug)]
pub enum Action {
    /// Sync the harness into the CLI agent directories. Stamp-gated: a no-op
    /// unless the harness changed, or `--force` is passed.
    Sync(SyncArgs),
    /// Report where the harness is and whether a sync is pending. Changes nothing.
    Status,
}

#[derive(Args, Debug)]
pub struct SyncArgs {
    /// Reinstall even when the harness is unchanged.
    #[arg(long)]
    pub force: bool,
}

/// Run a harness action.
pub fn run(action: Action) -> Result<CommandOutput, String> {
    match action {
        Action::Sync(args) => {
            let msg = harness::sync(args.force)?;
            Ok(CommandOutput::new(
                serde_json::json!({ "synced": true, "detail": msg }),
                msg,
            ))
        }
        Action::Status => {
            let status = harness::status();
            let human = status
                .get("detail")
                .and_then(|v| v.as_str())
                .unwrap_or("")
                .to_string();
            Ok(CommandOutput::new(status, human))
        }
    }
}
