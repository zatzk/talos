//! Workspaces persistence and repository association.

use rusqlite::{params, OptionalExtension};
use serde::{Deserialize, Serialize};

use super::Database;
use crate::sync::current_time_millis;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Workspace {
    pub id: String,
    pub name: String,
    pub project_id: String,
    pub control_plane_path: String,
    pub active_thread_id: Option<String>,
    pub created_at: u64,
    pub updated_at: u64,
    pub deleted_at: Option<u64>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WorkspaceRepo {
    pub workspace_id: String,
    pub repo_name: String,
    pub repo_path: String,
    pub is_primary: bool,
}

impl Database {
    pub fn create_workspace(
        &self,
        id: &str,
        name: &str,
        project_id: &str,
        control_plane_path: &str,
    ) -> rusqlite::Result<Workspace> {
        let now = current_time_millis();
        self.conn.execute(
            "INSERT INTO workspaces (id, name, project_id, control_plane_path, active_thread_id, created_at, updated_at, deleted_at)
             VALUES (?1, ?2, ?3, ?4, NULL, ?5, ?5, NULL)",
            params![id, name, project_id, control_plane_path, now as i64],
        )?;
        Ok(Workspace {
            id: id.to_string(),
            name: name.to_string(),
            project_id: project_id.to_string(),
            control_plane_path: control_plane_path.to_string(),
            active_thread_id: None,
            created_at: now,
            updated_at: now,
            deleted_at: None,
        })
    }

    pub fn get_workspace(&self, id: &str) -> rusqlite::Result<Option<Workspace>> {
        self.conn
            .query_row(
                "SELECT id, name, project_id, control_plane_path, active_thread_id, created_at, updated_at, deleted_at
                 FROM workspaces WHERE id = ?1 AND deleted_at IS NULL",
                params![id],
                |row| {
                    Ok(Workspace {
                        id: row.get(0)?,
                        name: row.get(1)?,
                        project_id: row.get(2)?,
                        control_plane_path: row.get(3)?,
                        active_thread_id: row.get(4)?,
                        created_at: row.get::<_, i64>(5)? as u64,
                        updated_at: row.get::<_, i64>(6)? as u64,
                        deleted_at: row.get::<_, Option<i64>>(7)?.map(|v| v as u64),
                    })
                },
            )
            .optional()
    }

    pub fn list_workspaces(&self) -> rusqlite::Result<Vec<Workspace>> {
        let mut stmt = self.conn.prepare_cached(
            "SELECT id, name, project_id, control_plane_path, active_thread_id, created_at, updated_at, deleted_at
             FROM workspaces WHERE deleted_at IS NULL ORDER BY updated_at DESC",
        )?;
        let rows = stmt.query_map([], |row| {
            Ok(Workspace {
                id: row.get(0)?,
                name: row.get(1)?,
                project_id: row.get(2)?,
                control_plane_path: row.get(3)?,
                active_thread_id: row.get(4)?,
                created_at: row.get::<_, i64>(5)? as u64,
                updated_at: row.get::<_, i64>(6)? as u64,
                deleted_at: row.get::<_, Option<i64>>(7)?.map(|v| v as u64),
            })
        })?;
        rows.collect()
    }

    pub fn set_active_thread(&self, workspace_id: &str, thread_id: Option<&str>) -> rusqlite::Result<bool> {
        let now = current_time_millis() as i64;
        let count = self.conn.execute(
            "UPDATE workspaces SET active_thread_id = ?2, updated_at = ?3 WHERE id = ?1 AND deleted_at IS NULL",
            params![workspace_id, thread_id, now],
        )?;
        Ok(count > 0)
    }

    pub fn add_workspace_repo(
        &self,
        workspace_id: &str,
        repo_name: &str,
        repo_path: &str,
        is_primary: bool,
    ) -> rusqlite::Result<()> {
        self.conn.execute(
            "INSERT OR REPLACE INTO workspace_repos (workspace_id, repo_name, repo_path, is_primary)
             VALUES (?1, ?2, ?3, ?4)",
            params![workspace_id, repo_name, repo_path, if is_primary { 1 } else { 0 }],
        )?;
        Ok(())
    }

    pub fn list_workspace_repos(&self, workspace_id: &str) -> rusqlite::Result<Vec<WorkspaceRepo>> {
        let mut stmt = self.conn.prepare_cached(
            "SELECT workspace_id, repo_name, repo_path, is_primary
             FROM workspace_repos WHERE workspace_id = ?1 ORDER BY is_primary DESC, repo_name ASC",
        )?;
        let rows = stmt.query_map(params![workspace_id], |row| {
            Ok(WorkspaceRepo {
                workspace_id: row.get(0)?,
                repo_name: row.get(1)?,
                repo_path: row.get(2)?,
                is_primary: row.get::<_, i64>(3)? != 0,
            })
        })?;
        rows.collect()
    }

    pub fn delete_workspace(&self, id: &str) -> rusqlite::Result<bool> {
        let now = current_time_millis() as i64;
        let count = self.conn.execute(
            "UPDATE workspaces SET deleted_at = ?2 WHERE id = ?1 AND deleted_at IS NULL",
            params![id, now],
        )?;
        Ok(count > 0)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn create_and_get_workspace() {
        let db = Database::open_in_memory().unwrap();
        let ws = db
            .create_workspace("ws-1", "Aton Core", "aton-core", "/tmp/cp")
            .unwrap();
        assert_eq!(ws.id, "ws-1");
        assert_eq!(ws.name, "Aton Core");

        let fetched = db.get_workspace("ws-1").unwrap().unwrap();
        assert_eq!(fetched.id, "ws-1");
        assert_eq!(fetched.project_id, "aton-core");
        assert_eq!(fetched.active_thread_id, None);

        db.set_active_thread("ws-1", Some("thread-42")).unwrap();
        let updated = db.get_workspace("ws-1").unwrap().unwrap();
        assert_eq!(updated.active_thread_id.as_deref(), Some("thread-42"));

        db.add_workspace_repo("ws-1", "backend", "/code/backend", true).unwrap();
        db.add_workspace_repo("ws-1", "frontend", "/code/frontend", false).unwrap();
        let repos = db.list_workspace_repos("ws-1").unwrap();
        assert_eq!(repos.len(), 2);
        assert_eq!(repos[0].repo_name, "backend");
        assert!(repos[0].is_primary);
    }
}
