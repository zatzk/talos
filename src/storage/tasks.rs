//! Persistence for [`Task`]s — the todo list.
//!
//! A task's agent linkage is stored as `(action_kind, …)` using the same
//! action-specific columns as [`automations`](super::automations); `action_kind`
//! is nullable here, since an unconnected local todo has no action. Soft-delete
//! via `deleted_at` mirrors sessions/worktrees. Mutations are recorded in the
//! audit log under [`EntityType::Task`].

use rusqlite::{params, OptionalExtension};

use crate::session::{AutomationAction, Task, TaskStatus, SOURCE_LOCAL};
use crate::sync::current_time_millis;

use super::audit::{AuditAction, EntityType};
use super::Database;

/// Fields needed to create a task.
pub struct NewTask {
    pub title: String,
    /// Optional markdown description; `None` = blank.
    pub description: Option<String>,
    pub status: TaskStatus,
    /// Agent linkage; `None` = a plain local todo.
    pub action: Option<AutomationAction>,
    pub source: String,
    pub external_id: Option<String>,
    pub external_url: Option<String>,
}

impl NewTask {
    /// A plain local todo (`source = "local"`, no action, no external link).
    pub fn local(title: impl Into<String>) -> Self {
        Self {
            title: title.into(),
            description: None,
            status: TaskStatus::Todo,
            action: None,
            source: SOURCE_LOCAL.to_string(),
            external_id: None,
            external_url: None,
        }
    }
}

impl Database {
    /// Insert a new task, returning its row id.
    pub fn create_task(&self, new: &NewTask) -> rusqlite::Result<i64> {
        let now = current_time_millis() as i64;
        let action_kind = new.action.as_ref().map(|a| a.kind());
        let (target_session, repo_path, worktree_branch, base_branch, agent, extra, command) = new
            .action
            .as_ref()
            .map(super::action_to_columns)
            .unwrap_or_default();
        self.conn.execute(
            "INSERT INTO tasks
                (title, status, action_kind, target_session, repo_path,
                 worktree_branch, base_branch, agent, source, external_id,
                 external_url, created_at, updated_at, deleted_at, description,
                 action_extra_repos, action_command)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12, ?12, NULL, ?13, ?14, ?15)",
            params![
                new.title,
                new.status.as_str(),
                action_kind,
                target_session,
                repo_path,
                worktree_branch,
                base_branch,
                agent,
                new.source,
                new.external_id,
                new.external_url,
                now,
                new.description,
                extra,
                command,
            ],
        )?;
        let id = self.conn.last_insert_rowid();
        self.log_audit(
            EntityType::Task,
            &id.to_string(),
            AuditAction::Created,
            None,
            None,
            Some(&new.title),
        )?;
        Ok(id)
    }

    /// Fetch a single active (non-deleted) task by id.
    pub fn get_task(&self, id: i64) -> rusqlite::Result<Option<Task>> {
        self.conn
            .query_row(
                &format!("SELECT {COLS} FROM tasks WHERE id = ?1 AND deleted_at IS NULL"),
                params![id],
                map_task,
            )
            .optional()
    }

    /// Fetch a single active task by its `(source, external_id)` natural key —
    /// the identity of an item imported from an external tracker. Used by the
    /// task-integration sync extensions to dedup/upsert imported issues. Returns
    /// `None` for a missing or soft-deleted match. (Indexed by
    /// `idx_tasks_external`.)
    pub fn get_task_by_external_id(
        &self,
        source: &str,
        external_id: &str,
    ) -> rusqlite::Result<Option<Task>> {
        self.conn
            .query_row(
                &format!(
                    "SELECT {COLS} FROM tasks
                     WHERE source = ?1 AND external_id = ?2 AND deleted_at IS NULL"
                ),
                params![source, external_id],
                map_task,
            )
            .optional()
    }

    /// List all active tasks, newest first.
    pub fn list_tasks(&self) -> rusqlite::Result<Vec<Task>> {
        // Cached: the snapshot re-reads this on every refresh (ADR-P6).
        let mut stmt = self.conn.prepare_cached(&format!(
            "SELECT {COLS} FROM tasks WHERE deleted_at IS NULL ORDER BY id DESC"
        ))?;
        let rows = stmt.query_map([], map_task)?;
        rows.collect()
    }

    /// Replace a task's definition (everything except id/created_at/deleted_at).
    pub fn update_task(&self, task: &Task) -> rusqlite::Result<()> {
        let action_kind = task.action.as_ref().map(|a| a.kind());
        let (target_session, repo_path, worktree_branch, base_branch, agent, extra, command) = task
            .action
            .as_ref()
            .map(super::action_to_columns)
            .unwrap_or_default();
        let now = current_time_millis() as i64;
        self.conn.execute(
            "UPDATE tasks SET
                title = ?2, status = ?3, action_kind = ?4, target_session = ?5,
                repo_path = ?6, worktree_branch = ?7, base_branch = ?8, agent = ?9,
                source = ?10, external_id = ?11, external_url = ?12, updated_at = ?13,
                description = ?14, action_extra_repos = ?15, action_command = ?16
             WHERE id = ?1",
            params![
                task.id,
                task.title,
                task.status.as_str(),
                action_kind,
                target_session,
                repo_path,
                worktree_branch,
                base_branch,
                agent,
                task.source,
                task.external_id,
                task.external_url,
                now,
                task.description,
                extra,
                command,
            ],
        )?;
        self.log_audit(
            EntityType::Task,
            &task.id.to_string(),
            AuditAction::Updated,
            None,
            None,
            Some(&task.title),
        )?;
        Ok(())
    }

    /// Update only a task's status. Returns whether a row changed.
    pub fn set_task_status(&self, id: i64, status: TaskStatus) -> rusqlite::Result<bool> {
        let now = current_time_millis() as i64;
        let updated = self.conn.execute(
            "UPDATE tasks SET status = ?2, updated_at = ?3 WHERE id = ?1 AND deleted_at IS NULL",
            params![id, status.as_str(), now],
        )?;
        if updated > 0 {
            self.log_audit(
                EntityType::Task,
                &id.to_string(),
                AuditAction::Updated,
                Some("status"),
                None,
                Some(status.as_str()),
            )?;
        }
        Ok(updated > 0)
    }

    /// Soft-delete a task (mark `deleted_at`). Returns whether a row changed.
    pub fn soft_delete_task(&self, id: i64) -> rusqlite::Result<bool> {
        let now = current_time_millis() as i64;
        let updated = self.conn.execute(
            "UPDATE tasks SET deleted_at = ?2 WHERE id = ?1 AND deleted_at IS NULL",
            params![id, now],
        )?;
        if updated > 0 {
            self.log_audit(
                EntityType::Task,
                &id.to_string(),
                AuditAction::Deleted,
                None,
                None,
                None,
            )?;
        }
        Ok(updated > 0)
    }

    /// Upsert a task from canonical specs (RFC/PRD decomposition).
    pub fn upsert_canonical_task(&self, task: &CanonicalTask) -> rusqlite::Result<()> {
        let now = current_time_millis() as i64;
        self.conn.execute(
            "INSERT INTO tasks
                (title, status, source, external_id, created_at, updated_at, description,
                 canonical_id, workspace_id, rfc_id, prd_id, agent, worktree_branch, base_branch,
                 qa_attempts, qa_score)
             VALUES (?1, ?2, 'canonical', ?3, ?4, ?4, ?5, ?3, ?6, ?7, ?8, ?9, ?10, ?11, ?12, ?13)
             ON CONFLICT(workspace_id, canonical_id) WHERE canonical_id IS NOT NULL AND deleted_at IS NULL DO UPDATE SET
                 title = excluded.title,
                 description = excluded.description,
                 rfc_id = excluded.rfc_id,
                 prd_id = excluded.prd_id,
                 updated_at = excluded.updated_at",
            params![
                task.title,
                task.status.as_str(),
                task.canonical_id,
                now,
                task.description,
                task.workspace_id,
                task.rfc_id,
                task.prd_id,
                task.assigned_agent,
                task.worktree_branch,
                task.base_branch,
                task.qa_attempts as i64,
                task.qa_score,
            ],
        )?;
        Ok(())
    }

    /// Get a single canonical task by workspace and canonical ID.
    pub fn get_canonical_task(
        &self,
        workspace_id: &str,
        canonical_id: &str,
    ) -> rusqlite::Result<Option<CanonicalTask>> {
        self.conn
            .query_row(
                "SELECT canonical_id, workspace_id, rfc_id, prd_id, title, description, status,
                        agent, NULL, worktree_branch, base_branch, qa_attempts, qa_score,
                        created_at, updated_at
                 FROM tasks
                 WHERE workspace_id = ?1 AND canonical_id = ?2 AND deleted_at IS NULL",
                params![workspace_id, canonical_id],
                map_canonical_task,
            )
            .optional()
    }

    /// List all canonical tasks in a workspace (for the workspace board).
    pub fn list_canonical_tasks(&self, workspace_id: &str) -> rusqlite::Result<Vec<CanonicalTask>> {
        let mut stmt = self.conn.prepare_cached(
            "SELECT canonical_id, workspace_id, rfc_id, prd_id, title, description, status,
                    agent, NULL, worktree_branch, base_branch, qa_attempts, qa_score,
                    created_at, updated_at
             FROM tasks
             WHERE workspace_id = ?1 AND canonical_id IS NOT NULL AND deleted_at IS NULL
             ORDER BY id ASC",
        )?;
        let rows = stmt.query_map(params![workspace_id], map_canonical_task)?;
        rows.collect()
    }

    /// Update only a canonical task's status on the board.
    pub fn set_canonical_task_status(
        &self,
        workspace_id: &str,
        canonical_id: &str,
        status: TaskStatus,
    ) -> rusqlite::Result<bool> {
        let now = current_time_millis() as i64;
        let updated = self.conn.execute(
            "UPDATE tasks SET status = ?3, updated_at = ?4
             WHERE workspace_id = ?1 AND canonical_id = ?2 AND deleted_at IS NULL",
            params![workspace_id, canonical_id, status.as_str(), now],
        )?;
        Ok(updated > 0)
    }
}

/// A task with canonical identity (`TASK-{rfc}-{seq}`).
#[derive(Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize)]
pub struct CanonicalTask {
    pub canonical_id: String,
    pub workspace_id: String,
    pub rfc_id: String,
    pub prd_id: Option<String>,
    pub title: String,
    pub description: Option<String>,
    pub status: TaskStatus,
    pub assigned_agent: Option<String>,
    pub assigned_model: Option<String>,
    pub worktree_branch: Option<String>,
    pub base_branch: Option<String>,
    pub qa_attempts: u32,
    pub qa_score: Option<f64>,
    pub created_at: u64,
    pub updated_at: u64,
}

fn map_canonical_task(row: &rusqlite::Row) -> rusqlite::Result<CanonicalTask> {
    Ok(CanonicalTask {
        canonical_id: row.get(0)?,
        workspace_id: row.get(1)?,
        rfc_id: row.get(2)?,
        prd_id: row.get(3)?,
        title: row.get(4)?,
        description: row.get(5)?,
        status: TaskStatus::from_db(&row.get::<_, String>(6)?),
        assigned_agent: row.get(7)?,
        assigned_model: row.get(8)?,
        worktree_branch: row.get(9)?,
        base_branch: row.get(10)?,
        qa_attempts: row.get::<_, i64>(11)? as u32,
        qa_score: row.get(12)?,
        created_at: row.get::<_, i64>(13)? as u64,
        updated_at: row.get::<_, i64>(14)? as u64,
    })
}

/// Column list for task SELECTs (keep in sync with [`map_task`]).
const COLS: &str = "id, title, status, action_kind, target_session, repo_path, \
    worktree_branch, base_branch, agent, source, external_id, external_url, \
    created_at, updated_at, deleted_at, description, action_extra_repos, \
    action_command";

fn map_task(row: &rusqlite::Row) -> rusqlite::Result<Task> {
    let action_kind: Option<String> = row.get(3)?;
    let cols: super::ActionColumns = (
        row.get(4)?,
        row.get(5)?,
        row.get(6)?,
        row.get(7)?,
        row.get(8)?,
        row.get(16)?,
        row.get(17)?,
    );

    // An action-less local todo has a NULL `action_kind`; only a present
    // discriminant decodes to an action (Exec round-trips even though the TUI
    // never authors one onto a task).
    let action = action_kind
        .as_deref()
        .map(|kind| super::action_from_columns(kind, cols));

    Ok(Task {
        id: row.get(0)?,
        title: row.get(1)?,
        status: TaskStatus::from_db(&row.get::<_, String>(2)?),
        action,
        source: row.get(9)?,
        external_id: row.get(10)?,
        external_url: row.get(11)?,
        created_at: row.get::<_, i64>(12)? as u64,
        updated_at: row.get::<_, i64>(13)? as u64,
        deleted_at: row.get::<_, Option<i64>>(14)?.map(|v| v as u64),
        description: row.get(15)?,
    })
}

#[cfg(test)]
mod tests {
    use std::path::PathBuf;

    use super::*;
    use crate::session::SessionId;

    #[test]
    fn create_get_list_round_trip_local() {
        let db = Database::open_in_memory().unwrap();
        let id = db.create_task(&NewTask::local("Buy milk")).unwrap();
        assert!(id > 0);

        let task = db.get_task(id).unwrap().unwrap();
        assert_eq!(task.title, "Buy milk");
        assert_eq!(task.status, TaskStatus::Todo);
        assert_eq!(task.action, None);
        assert_eq!(task.source, "local");

        assert_eq!(db.list_tasks().unwrap().len(), 1);
    }

    #[test]
    fn send_action_round_trip() {
        let db = Database::open_in_memory().unwrap();
        let sid = SessionId::default();
        let new = NewTask {
            action: Some(AutomationAction::Send { session_id: sid }),
            ..NewTask::local("Fix login")
        };
        let id = db.create_task(&new).unwrap();
        let got = db.get_task(id).unwrap().unwrap();
        match got.action {
            Some(AutomationAction::Send { session_id }) => assert_eq!(session_id, sid),
            other => panic!("expected send, got {other:?}"),
        }
    }

    #[test]
    fn spawn_action_round_trip() {
        let db = Database::open_in_memory().unwrap();
        let new = NewTask {
            action: Some(AutomationAction::Spawn {
                repo_path: PathBuf::from("/tmp/repo"),
                worktree_branch: Some("feat/task".into()),
                base_branch: Some("main".into()),
                agent: Some("codex".into()),
                extra_repos: Vec::new(),
            }),
            ..NewTask::local("Refactor")
        };
        let id = db.create_task(&new).unwrap();
        let got = db.get_task(id).unwrap().unwrap();
        match got.action {
            Some(AutomationAction::Spawn {
                repo_path,
                worktree_branch,
                base_branch,
                agent,
                extra_repos,
            }) => {
                assert_eq!(repo_path, PathBuf::from("/tmp/repo"));
                assert_eq!(worktree_branch.as_deref(), Some("feat/task"));
                assert_eq!(base_branch.as_deref(), Some("main"));
                assert_eq!(agent.as_deref(), Some("codex"));
                assert!(extra_repos.is_empty());
            }
            other => panic!("expected spawn, got {other:?}"),
        }
    }

    #[test]
    fn spawn_action_multi_repo_round_trip() {
        use crate::session::ExtraRepo;
        let db = Database::open_in_memory().unwrap();
        let new = NewTask {
            action: Some(AutomationAction::Spawn {
                repo_path: PathBuf::from("/tmp/primary"),
                worktree_branch: Some("fleet/multi".into()),
                base_branch: Some("main".into()),
                agent: Some("fleet-worker".into()),
                extra_repos: vec![
                    ExtraRepo {
                        repo_path: PathBuf::from("/tmp/extra-wt"),
                        worktree: true,
                        base_branch: Some("master".into()),
                    },
                    ExtraRepo {
                        repo_path: PathBuf::from("/tmp/extra-dir"),
                        worktree: false,
                        base_branch: None,
                    },
                ],
            }),
            ..NewTask::local("Multi-repo")
        };
        let id = db.create_task(&new).unwrap();
        let got = db.get_task(id).unwrap().unwrap();
        match got.action {
            Some(AutomationAction::Spawn { extra_repos, .. }) => {
                assert_eq!(extra_repos.len(), 2);
                assert_eq!(extra_repos[0].repo_path, PathBuf::from("/tmp/extra-wt"));
                assert!(extra_repos[0].worktree);
                assert_eq!(extra_repos[0].base_branch.as_deref(), Some("master"));
                assert!(!extra_repos[1].worktree);
                assert_eq!(extra_repos[1].base_branch, None);
            }
            other => panic!("expected spawn, got {other:?}"),
        }
    }

    #[test]
    fn set_status_updates_and_lists() {
        let db = Database::open_in_memory().unwrap();
        let id = db.create_task(&NewTask::local("t")).unwrap();
        assert!(db.set_task_status(id, TaskStatus::Done).unwrap());
        assert_eq!(db.get_task(id).unwrap().unwrap().status, TaskStatus::Done);
        // Unknown id changes nothing.
        assert!(!db.set_task_status(9999, TaskStatus::Done).unwrap());
    }

    #[test]
    fn update_task_replaces_fields() {
        let db = Database::open_in_memory().unwrap();
        let id = db.create_task(&NewTask::local("old")).unwrap();
        let mut task = db.get_task(id).unwrap().unwrap();
        task.title = "new".into();
        task.status = TaskStatus::InProgress;
        task.action = Some(AutomationAction::Send {
            session_id: SessionId::default(),
        });
        db.update_task(&task).unwrap();
        let got = db.get_task(id).unwrap().unwrap();
        assert_eq!(got.title, "new");
        assert_eq!(got.status, TaskStatus::InProgress);
        assert!(matches!(got.action, Some(AutomationAction::Send { .. })));
    }

    #[test]
    fn soft_delete_is_soft() {
        let db = Database::open_in_memory().unwrap();
        let id = db.create_task(&NewTask::local("doomed")).unwrap();
        assert!(db.soft_delete_task(id).unwrap());
        // Hidden from the active views.
        assert!(db.get_task(id).unwrap().is_none());
        assert!(db.list_tasks().unwrap().is_empty());
        // But the row still physically exists.
        let raw: i64 = db
            .conn
            .query_row(
                "SELECT count(*) FROM tasks WHERE id = ?1",
                params![id],
                |r| r.get(0),
            )
            .unwrap();
        assert_eq!(raw, 1);
        // Deleting again is a no-op.
        assert!(!db.soft_delete_task(id).unwrap());
    }

    #[test]
    fn external_fields_round_trip() {
        let db = Database::open_in_memory().unwrap();
        let new = NewTask {
            title: "imported".into(),
            description: None,
            status: TaskStatus::Todo,
            action: None,
            source: "github".into(),
            external_id: Some("42".into()),
            external_url: Some("https://example.com/issues/42".into()),
        };
        let id = db.create_task(&new).unwrap();
        let got = db.get_task(id).unwrap().unwrap();
        assert_eq!(got.source, "github");
        assert_eq!(got.external_id.as_deref(), Some("42"));
        assert_eq!(
            got.external_url.as_deref(),
            Some("https://example.com/issues/42")
        );
    }

    #[test]
    fn get_task_by_external_id_finds_and_misses() {
        let db = Database::open_in_memory().unwrap();
        let new = NewTask {
            title: "imported".into(),
            source: "github".into(),
            external_id: Some("42".into()),
            external_url: Some("https://example.com/issues/42".into()),
            ..NewTask::local("imported")
        };
        let id = db.create_task(&new).unwrap();

        // Exact (source, external_id) pair finds the task.
        let got = db.get_task_by_external_id("github", "42").unwrap();
        assert_eq!(got.map(|t| t.id), Some(id));

        // Wrong source or wrong id misses.
        assert!(db
            .get_task_by_external_id("gitlab", "42")
            .unwrap()
            .is_none());
        assert!(db
            .get_task_by_external_id("github", "99")
            .unwrap()
            .is_none());

        // A soft-deleted match is excluded.
        assert!(db.soft_delete_task(id).unwrap());
        assert!(db
            .get_task_by_external_id("github", "42")
            .unwrap()
            .is_none());
    }

    #[test]
    fn description_round_trips_and_clears() {
        let db = Database::open_in_memory().unwrap();
        let new = NewTask {
            description: Some("# Notes\n- **bold** item".into()),
            ..NewTask::local("documented")
        };
        let id = db.create_task(&new).unwrap();
        let mut got = db.get_task(id).unwrap().unwrap();
        assert_eq!(got.description.as_deref(), Some("# Notes\n- **bold** item"));

        // Clearing the description persists as NULL.
        got.description = None;
        db.update_task(&got).unwrap();
        assert_eq!(db.get_task(id).unwrap().unwrap().description, None);
    }

    #[test]
    fn create_logs_audit() {
        let db = Database::open_in_memory().unwrap();
        let id = db.create_task(&NewTask::local("audited")).unwrap();
        let entries = db
            .get_audit_log(Some(EntityType::Task), Some(&id.to_string()), 10)
            .unwrap();
        assert_eq!(entries.len(), 1);
        assert_eq!(entries[0].action, "created");
    }

    #[test]
    fn canonical_task_upsert_and_board_lifecycle() {
        let db = Database::open_in_memory().unwrap();
        db.create_workspace("ws-tasks", "Task Board WS", "proj", "/tmp")
            .unwrap();

        let task1 = CanonicalTask {
            canonical_id: "TASK-042-01".into(),
            workspace_id: "ws-tasks".into(),
            rfc_id: "RFC-042".into(),
            prd_id: Some("PRD-042".into()),
            title: "Implement Token Generator".into(),
            description: Some("Must generate JWT tokens with expiry".into()),
            status: TaskStatus::Todo,
            assigned_agent: Some("dev".into()),
            assigned_model: None,
            worktree_branch: Some("feat/task-042-01".into()),
            base_branch: Some("main".into()),
            qa_attempts: 0,
            qa_score: None,
            created_at: 1000,
            updated_at: 1000,
        };

        db.upsert_canonical_task(&task1).unwrap();

        let fetched = db
            .get_canonical_task("ws-tasks", "TASK-042-01")
            .unwrap()
            .unwrap();
        assert_eq!(fetched.title, "Implement Token Generator");
        assert_eq!(fetched.status, TaskStatus::Todo);
        assert_eq!(fetched.rfc_id, "RFC-042");

        // Advance status to InProgress
        assert!(db.set_canonical_task_status("ws-tasks", "TASK-042-01", TaskStatus::InProgress).unwrap());
        let in_prog = db
            .get_canonical_task("ws-tasks", "TASK-042-01")
            .unwrap()
            .unwrap();
        assert_eq!(in_prog.status, TaskStatus::InProgress);

        // List tasks for board
        let board_tasks = db.list_canonical_tasks("ws-tasks").unwrap();
        assert_eq!(board_tasks.len(), 1);
        assert_eq!(board_tasks[0].canonical_id, "TASK-042-01");
    }
}
