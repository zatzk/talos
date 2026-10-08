//! Threads and chat messages storage.

use rusqlite::{params, OptionalExtension};
use serde::{Deserialize, Serialize};

use super::Database;
use crate::sync::current_time_millis;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Thread {
    pub id: String,
    pub workspace_id: String,
    pub title: String,
    pub target_kind: String, // "auto" | "api" | "cli"
    pub target_agent: Option<String>,
    pub target_model: Option<String>,
    pub created_at: u64,
    pub updated_at: u64,
    pub deleted_at: Option<u64>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ChatMessage {
    pub id: String,
    pub thread_id: String,
    pub workspace_id: String,
    pub role: String, // "user" | "agent" | "system"
    pub agent: Option<String>,
    pub backend: Option<String>,
    pub model: Option<String>,
    pub content: String,
    pub redacted: bool,
    pub created_at: u64,
}

impl Database {
    pub fn create_thread(&self, id: &str, workspace_id: &str, title: &str) -> rusqlite::Result<Thread> {
        let now = current_time_millis();
        self.conn.execute(
            "INSERT INTO threads (id, workspace_id, title, target_kind, target_agent, target_model, created_at, updated_at, deleted_at)
             VALUES (?1, ?2, ?3, 'auto', NULL, NULL, ?4, ?4, NULL)",
            params![id, workspace_id, title, now as i64],
        )?;
        Ok(Thread {
            id: id.to_string(),
            workspace_id: workspace_id.to_string(),
            title: title.to_string(),
            target_kind: "auto".to_string(),
            target_agent: None,
            target_model: None,
            created_at: now,
            updated_at: now,
            deleted_at: None,
        })
    }

    pub fn get_thread(&self, id: &str) -> rusqlite::Result<Option<Thread>> {
        self.conn
            .query_row(
                "SELECT id, workspace_id, title, target_kind, target_agent, target_model, created_at, updated_at, deleted_at
                 FROM threads WHERE id = ?1 AND deleted_at IS NULL",
                params![id],
                |row| {
                    Ok(Thread {
                        id: row.get(0)?,
                        workspace_id: row.get(1)?,
                        title: row.get(2)?,
                        target_kind: row.get(3)?,
                        target_agent: row.get(4)?,
                        target_model: row.get(5)?,
                        created_at: row.get::<_, i64>(6)? as u64,
                        updated_at: row.get::<_, i64>(7)? as u64,
                        deleted_at: row.get::<_, Option<i64>>(8)?.map(|v| v as u64),
                    })
                },
            )
            .optional()
    }

    pub fn list_threads_by_workspace(&self, workspace_id: &str) -> rusqlite::Result<Vec<Thread>> {
        let mut stmt = self.conn.prepare_cached(
            "SELECT id, workspace_id, title, target_kind, target_agent, target_model, created_at, updated_at, deleted_at
             FROM threads WHERE workspace_id = ?1 AND deleted_at IS NULL ORDER BY updated_at DESC",
        )?;
        let rows = stmt.query_map(params![workspace_id], |row| {
            Ok(Thread {
                id: row.get(0)?,
                workspace_id: row.get(1)?,
                title: row.get(2)?,
                target_kind: row.get(3)?,
                target_agent: row.get(4)?,
                target_model: row.get(5)?,
                created_at: row.get::<_, i64>(6)? as u64,
                updated_at: row.get::<_, i64>(7)? as u64,
                deleted_at: row.get::<_, Option<i64>>(8)?.map(|v| v as u64),
            })
        })?;
        rows.collect()
    }

    pub fn set_thread_target(
        &self,
        id: &str,
        target_kind: &str,
        agent: Option<&str>,
        model: Option<&str>,
    ) -> rusqlite::Result<bool> {
        let now = current_time_millis() as i64;
        let count = self.conn.execute(
            "UPDATE threads SET target_kind = ?2, target_agent = ?3, target_model = ?4, updated_at = ?5
             WHERE id = ?1 AND deleted_at IS NULL",
            params![id, target_kind, agent, model, now],
        )?;
        Ok(count > 0)
    }

    pub fn update_thread_title(&self, id: &str, title: &str) -> rusqlite::Result<bool> {
        let now = current_time_millis() as i64;
        let count = self.conn.execute(
            "UPDATE threads SET title = ?2, updated_at = ?3 WHERE id = ?1 AND deleted_at IS NULL",
            params![id, title, now],
        )?;
        Ok(count > 0)
    }

    pub fn insert_chat_message(&self, msg: &ChatMessage) -> rusqlite::Result<()> {
        let now = msg.created_at as i64;
        self.conn.execute(
            "INSERT INTO chat_messages (id, thread_id, workspace_id, role, agent, backend, model, content, redacted, created_at)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10)",
            params![
                msg.id,
                msg.thread_id,
                msg.workspace_id,
                msg.role,
                msg.agent,
                msg.backend,
                msg.model,
                msg.content,
                if msg.redacted { 1 } else { 0 },
                now
            ],
        )?;
        // Update thread updated_at
        self.conn.execute(
            "UPDATE threads SET updated_at = ?2 WHERE id = ?1",
            params![msg.thread_id, now],
        )?;
        Ok(())
    }

    pub fn list_chat_messages(&self, thread_id: &str) -> rusqlite::Result<Vec<ChatMessage>> {
        let mut stmt = self.conn.prepare_cached(
            "SELECT id, thread_id, workspace_id, role, agent, backend, model, content, redacted, created_at
             FROM chat_messages WHERE thread_id = ?1 ORDER BY created_at ASC",
        )?;
        let rows = stmt.query_map(params![thread_id], |row| {
            Ok(ChatMessage {
                id: row.get(0)?,
                thread_id: row.get(1)?,
                workspace_id: row.get(2)?,
                role: row.get(3)?,
                agent: row.get(4)?,
                backend: row.get(5)?,
                model: row.get(6)?,
                content: row.get(7)?,
                redacted: row.get::<_, i64>(8)? != 0,
                created_at: row.get::<_, i64>(9)? as u64,
            })
        })?;
        rows.collect()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn create_thread_and_messages() {
        let db = Database::open_in_memory().unwrap();
        db.create_workspace("ws-1", "Test", "proj", "/tmp").unwrap();

        let thread = db.create_thread("th-1", "ws-1", "Architecture Discussion").unwrap();
        assert_eq!(thread.id, "th-1");
        assert_eq!(thread.target_kind, "auto");

        db.set_thread_target("th-1", "api", Some("architect"), Some("anthropic/claude-3.7-sonnet")).unwrap();
        let updated = db.get_thread("th-1").unwrap().unwrap();
        assert_eq!(updated.target_kind, "api");
        assert_eq!(updated.target_model.as_deref(), Some("anthropic/claude-3.7-sonnet"));

        let msg = ChatMessage {
            id: "msg-1".into(),
            thread_id: "th-1".into(),
            workspace_id: "ws-1".into(),
            role: "user".into(),
            agent: None,
            backend: None,
            model: None,
            content: "Please review the RFC".into(),
            redacted: false,
            created_at: 1000,
        };
        db.insert_chat_message(&msg).unwrap();

        let messages = db.list_chat_messages("th-1").unwrap();
        assert_eq!(messages.len(), 1);
        assert_eq!(messages[0].content, "Please review the RFC");

        let updated = db.update_thread_title("th-1", "Updated Title").unwrap();
        assert!(updated);
        let thread = db.get_thread("th-1").unwrap().unwrap();
        assert_eq!(thread.title, "Updated Title");
        let threads = db.list_threads_by_workspace("ws-1").unwrap();
        assert_eq!(threads[0].title, "Updated Title");
    }
}

