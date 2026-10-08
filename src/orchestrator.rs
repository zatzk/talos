//! The Talos lead — seeded on first run and pinned to the top of the fleet.
//!
//! Talos's orchestration is the control-plane pattern: a long-lived **lead**
//! session (Mission Control) over a control-plane checkout, dispatching worker
//! sessions and coordinating them through the mailbox. The lead is the
//! orchestrator; `Ctrl+N` only ever makes a single worker-shaped session, which
//! is why the lead has to be present and legible or the whole model is
//! invisible.
//!
//! So on every start this ensures the lead exists: if the control plane is
//! missing it seeds one from the shipped template, if the session is missing it
//! spawns it, and either way it pins the row above everything else. Best-effort,
//! like the extension self-heal it sits beside — a failure is a notice, never a
//! reason the interface does not start.

use std::path::{Path, PathBuf};

use crate::backend::BackendRegistry;
use crate::storage::Database;

/// The lead's session name. Carries the glyph because the session list is where
/// it has to be found, and a plain name reads as just another worker.
pub const LEAD_NAME: &str = "📡 Talos Mission Control";

/// Where a control plane lives when nothing says otherwise.
fn default_control_plane() -> Option<PathBuf> {
    if let Ok(dir) = std::env::var("TALOS_CONTROL_PLANE") {
        if dir.is_empty() || dir == "none" || dir == "off" || dir == "0" {
            return None;
        }
        return Some(PathBuf::from(dir));
    }
    let submodule = Path::new(env!("CARGO_MANIFEST_DIR")).join("code-documentation");
    if submodule.is_dir() {
        return Some(submodule);
    }
    crate::paths::home_dir().map(|h| h.join("Code/code-documentation"))
}

/// The shipped control-plane template (the files a fresh control plane gets).
/// Source build first, then a copy staged under the data dir for a prebuilt
/// install — the same resolution the harness sync uses.
fn template_dir() -> Option<PathBuf> {
    if let Ok(dir) = std::env::var("TALOS_CONTROL_PLANE_TEMPLATE") {
        let p = PathBuf::from(dir);
        if p.is_dir() {
            return Some(p);
        }
    }
    let source = Path::new(env!("CARGO_MANIFEST_DIR")).join("control-plane");
    if source.is_dir() {
        return Some(source);
    }
    if let Some(data) = crate::paths::data_directory() {
        let staged = data.join("control-plane-template");
        if staged.is_dir() {
            return Some(staged);
        }
    }
    None
}

/// The lead agent: `TALOS_LEAD_AGENT`, else the registry default (agent `None`
/// in the spawn request).
fn lead_agent() -> Option<String> {
    std::env::var("TALOS_LEAD_AGENT").ok().filter(|s| !s.is_empty())
}

/// Copy every file of `template` into `dest` that is not already there, then
/// ensure the generated artifacts stay out of git. Non-destructive: a control
/// plane you have grown is never overwritten. Returns how many files landed.
fn seed_control_plane(template: &Path, dest: &Path) -> Result<usize, String> {
    std::fs::create_dir_all(dest).map_err(|e| format!("create {}: {e}", dest.display()))?;
    if !dest.join(".git").exists() {
        // A control plane is versioned; `git init` is best-effort (git may be
        // absent, and a non-repo still works as a control plane).
        let _ = std::process::Command::new("git")
            .args(["init", "-q"])
            .current_dir(dest)
            .status();
    }

    let mut copied = 0usize;
    let mut stack = vec![template.to_path_buf()];
    while let Some(dir) = stack.pop() {
        let Ok(entries) = std::fs::read_dir(&dir) else {
            continue;
        };
        for entry in entries.flatten() {
            let path = entry.path();
            if path.is_dir() {
                stack.push(path);
                continue;
            }
            let Ok(rel) = path.strip_prefix(template) else {
                continue;
            };
            let target = dest.join(rel);
            if target.exists() {
                continue;
            }
            if let Some(parent) = target.parent() {
                let _ = std::fs::create_dir_all(parent);
            }
            if std::fs::copy(&path, &target).is_ok() {
                copied += 1;
            }
        }
    }

    let gitignore = dest.join(".gitignore");
    let existing = std::fs::read_to_string(&gitignore).unwrap_or_default();
    let mut append = String::new();
    for entry in [
        "TALOS.rendered.md",
        "registry/repos.generated.yaml",
        "orchestration/session-profiles.local.yaml",
    ] {
        if !existing.lines().any(|l| l.trim() == entry) {
            append.push_str(entry);
            append.push('\n');
        }
    }
    if !append.is_empty() {
        use std::io::Write;
        if let Ok(mut f) = std::fs::OpenOptions::new()
            .create(true)
            .append(true)
            .open(&gitignore)
        {
            if !existing.is_empty() && !existing.ends_with('\n') {
                let _ = f.write_all(b"\n");
            }
            let _ = f.write_all(append.as_bytes());
        }
    }
    Ok(copied)
}

/// Ensure the lead session exists and is pinned, seeding the control plane if
/// needed. Returns user-facing notices (empty when the lead was already there).
pub fn ensure_lead_on_startup(db: &Database, backends: &BackendRegistry) -> Vec<String> {
    let mut notices = Vec::new();

    let Some(control_plane) = default_control_plane() else {
        return notices;
    };

    // Seed the control plane if it is absent and we have a template.
    if !control_plane.exists() {
        match template_dir() {
            Some(template) => match seed_control_plane(&template, &control_plane) {
                Ok(n) => notices.push(format!(
                    "seeded a control plane at {} ({n} file(s))",
                    control_plane.display()
                )),
                Err(e) => {
                    notices.push(format!("could not seed the control plane: {e}"));
                    return notices;
                }
            },
            None => {
                // No template and no checkout: nothing to open the lead on.
                return notices;
            }
        }
    }

    // Already have the lead? Just keep it pinned and stop.
    match db.get_session_by_name(LEAD_NAME) {
        Ok(Some(_)) => {
            pin_lead(db);
            return notices;
        }
        Ok(None) => {}
        Err(e) => {
            notices.push(format!("could not look up the lead session: {e}"));
            return notices;
        }
    }

    let req = crate::session_ops::SpawnRequest {
        name: LEAD_NAME.to_string(),
        repo_path: control_plane.clone(),
        agent: lead_agent(),
        ..Default::default()
    };
    match crate::session_ops::spawn_session_headless(db, backends, req) {
        Ok(res) => {
            let _ = db.set_display_order(&[(res.session_id.clone(), LEAD_PIN)]);
            notices.push(format!(
                "started the lead session '{LEAD_NAME}' over {}",
                control_plane.display()
            ));
        }
        Err(e) => notices.push(format!("could not start the lead session: {e}")),
    }
    notices
}

/// The lead's manual position: below every default (null or `0`) so it sorts
/// first — `ORDER BY display_order IS NULL, display_order` puts a set, negative
/// order ahead of the nulls.
const LEAD_PIN: i64 = -1;

/// Pin an existing lead back to the top. Targeted (only the lead's row), so a
/// user's ordering of every other session is untouched.
fn pin_lead(db: &Database) {
    if let Ok(Some(session)) = db.get_session_by_name(LEAD_NAME) {
        let _ = db.set_display_order(&[(session.id.clone(), LEAD_PIN)]);
    }
}

/// A JSON status of the lead, for `talos-cli`.
pub fn status(db: &Database) -> serde_json::Value {
    let control_plane = default_control_plane();
    let present = db
        .get_session_by_name(LEAD_NAME)
        .ok()
        .flatten()
        .is_some();
    let cp_path = control_plane.as_ref().map(|p| p.display().to_string());
    let cp_exists = control_plane.as_ref().map(|p| p.exists());
    serde_json::json!({
        "lead_session": LEAD_NAME,
        "present": present,
        "control_plane": cp_path,
        "control_plane_exists": cp_exists,
        "template_found": template_dir().is_some(),
    })
}

/// Result of approving a spec and committing changes.
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct SpecApprovalResult {
    pub rfc_id: String,
    pub prd_id: Option<String>,
    pub committed_files: Vec<String>,
    pub created_tasks_count: usize,
    pub commit_hash: Option<String>,
}

/// Approve an RFC/PRD spec, break down canonical tasks, ingest them into the board,
/// and automatically git commit in the `code-documentation` repository (Ctrl+A).
pub fn approve_spec_and_commit(
    db: &Database,
    _workspace_id: &str,
    rfc_id: &str,
    prd_id: Option<&str>,
    tasks: &[crate::storage::tasks::CanonicalTask],
) -> anyhow::Result<SpecApprovalResult> {
    let cp = default_control_plane().ok_or_else(|| anyhow::anyhow!("No control-plane directory configured"))?;
    if !cp.is_dir() {
        anyhow::bail!("Control plane directory {} does not exist", cp.display());
    }

    // 1. Ingest tasks into the workspace board
    let mut created_count = 0;
    for task in tasks {
        db.upsert_canonical_task(task)?;
        created_count += 1;
    }

    // 2. Stage files in code-documentation submodule
    let mut add_cmd = crate::git::git_program();
    add_cmd.current_dir(&cp);
    add_cmd.args(&["add", "."]);
    crate::git::run_git(add_cmd, "git add code-documentation")?;

    // 3. Commit with structured message
    let commit_msg = format!(
        "feat(spec): approve and break down {} [skip ci]\n\nAutomated spec approval and task breakdown via Talos v3.",
        rfc_id
    );
    let mut commit_cmd = crate::git::git_program();
    commit_cmd.current_dir(&cp);
    commit_cmd.args(&["commit", "-m", &commit_msg]);
    // git commit may return error if working tree is clean; ignore exit code 1 if no changes to commit
    let _ = crate::git::run_git(commit_cmd, "git commit code-documentation");

    // 4. Capture latest commit hash
    let mut rev_cmd = crate::git::git_program();
    rev_cmd.current_dir(&cp);
    rev_cmd.args(&["rev-parse", "--short", "HEAD"]);
    let commit_hash = rev_cmd.output().ok().and_then(|out| {
        if out.status.success() {
            Some(String::from_utf8_lossy(&out.stdout).trim().to_string())
        } else {
            None
        }
    });

    Ok(SpecApprovalResult {
        rfc_id: rfc_id.to_string(),
        prd_id: prd_id.map(|s| s.to_string()),
        committed_files: vec![format!("rfcs/{rfc_id}.md")],
        created_tasks_count: created_count,
        commit_hash,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn temp(name: &str) -> PathBuf {
        let d = std::env::temp_dir().join(format!("talos-orch-{name}-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&d);
        d
    }

    #[test]
    fn seed_is_non_destructive_and_lays_the_template() {
        let template = temp("tpl");
        std::fs::create_dir_all(template.join("orchestration/playbooks")).unwrap();
        std::fs::write(template.join("TALOS.md"), "standing context\n").unwrap();
        std::fs::write(template.join("orchestration/playbooks/prd.md"), "prd\n").unwrap();

        let dest = temp("dest");
        let n = seed_control_plane(&template, &dest).unwrap();
        assert_eq!(n, 2);
        assert!(dest.join("TALOS.md").is_file());
        assert!(dest.join("orchestration/playbooks/prd.md").is_file());
        assert!(dest.join(".gitignore").is_file());

        // A user's edit survives a re-seed.
        std::fs::write(dest.join("TALOS.md"), "edited by the operator\n").unwrap();
        let n = seed_control_plane(&template, &dest).unwrap();
        assert_eq!(n, 0, "nothing new should be written");
        assert_eq!(
            std::fs::read_to_string(dest.join("TALOS.md")).unwrap(),
            "edited by the operator\n"
        );

        let _ = std::fs::remove_dir_all(&template);
        let _ = std::fs::remove_dir_all(&dest);
    }

    #[test]
    fn default_control_plane_resolves_submodule_or_env() {
        std::env::set_var("TALOS_CONTROL_PLANE", "/tmp/custom-control-plane");
        assert_eq!(
            default_control_plane(),
            Some(PathBuf::from("/tmp/custom-control-plane"))
        );
        std::env::remove_var("TALOS_CONTROL_PLANE");

        let cp = default_control_plane().expect("should resolve a default control plane");
        assert!(cp.ends_with("code-documentation"));
    }
}
