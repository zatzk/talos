//! 4-layer memory storage: episodic, semantic (embeddings), procedural, and entity graph.

use rusqlite::{params, OptionalExtension};
use serde::{Deserialize, Serialize};

use super::Database;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MemoryFact {
    pub id: String,
    pub workspace_id: String,
    pub subject: String,
    pub predicate: String,
    pub object: String,
    pub confidence: f64,
    pub source_message_id: Option<String>,
    pub superseded_by: Option<String>,
    pub created_at: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MemoryPreference {
    pub workspace_id: String,
    pub key: String,
    pub value: String,
    pub source: Option<String>,
    pub updated_at: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MemoryEntity {
    pub id: String,
    pub workspace_id: String,
    pub kind: String,
    pub name: String,
    pub ref_target: Option<String>,
    pub updated_at: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MemoryEdge {
    pub from_id: String,
    pub relation: String,
    pub to_id: String,
    pub created_at: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FtsSearchResult {
    pub message_id: String,
    pub thread_id: String,
    pub workspace_id: String,
    pub content: String,
    pub role: String,
    pub score: f64,
}

impl Database {
    pub fn store_embedding(
        &self,
        message_id: &str,
        model: &str,
        dim: usize,
        vector: &[f32],
    ) -> rusqlite::Result<()> {
        let bytes: Vec<u8> = vector
            .iter()
            .flat_map(|val| val.to_ne_bytes())
            .collect();

        self.conn.execute(
            "INSERT OR REPLACE INTO memory_embedding (message_id, model, dim, vector)
             VALUES (?1, ?2, ?3, ?4)",
            params![message_id, model, dim as i64, bytes],
        )?;
        Ok(())
    }

    pub fn get_embedding(&self, message_id: &str, model: &str) -> rusqlite::Result<Option<Vec<f32>>> {
        let blob: Option<Vec<u8>> = self
            .conn
            .query_row(
                "SELECT vector FROM memory_embedding WHERE message_id = ?1 AND model = ?2",
                params![message_id, model],
                |row| row.get(0),
            )
            .optional()?;

        Ok(blob.map(|bytes| {
            bytes
                .chunks_exact(4)
                .map(|chunk| f32::from_ne_bytes(chunk.try_into().unwrap()))
                .collect()
        }))
    }

    pub fn insert_fact(&self, fact: &MemoryFact) -> rusqlite::Result<()> {
        self.conn.execute(
            "INSERT OR REPLACE INTO memory_fact (id, workspace_id, subject, predicate, object, confidence, source_message_id, superseded_by, created_at)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9)",
            params![
                fact.id,
                fact.workspace_id,
                fact.subject,
                fact.predicate,
                fact.object,
                fact.confidence,
                fact.source_message_id,
                fact.superseded_by,
                fact.created_at as i64,
            ],
        )?;
        Ok(())
    }

    pub fn list_facts(&self, workspace_id: &str) -> rusqlite::Result<Vec<MemoryFact>> {
        let mut stmt = self.conn.prepare_cached(
            "SELECT id, workspace_id, subject, predicate, object, confidence, source_message_id, superseded_by, created_at
             FROM memory_fact WHERE workspace_id = ?1 AND superseded_by IS NULL ORDER BY created_at DESC",
        )?;
        let rows = stmt.query_map(params![workspace_id], |row| {
            Ok(MemoryFact {
                id: row.get(0)?,
                workspace_id: row.get(1)?,
                subject: row.get(2)?,
                predicate: row.get(3)?,
                object: row.get(4)?,
                confidence: row.get(5)?,
                source_message_id: row.get(6)?,
                superseded_by: row.get(7)?,
                created_at: row.get::<_, i64>(8)? as u64,
            })
        })?;
        rows.collect()
    }

    pub fn set_preference(&self, pref: &MemoryPreference) -> rusqlite::Result<()> {
        self.conn.execute(
            "INSERT OR REPLACE INTO memory_preference (workspace_id, key, value, source, updated_at)
             VALUES (?1, ?2, ?3, ?4, ?5)",
            params![
                pref.workspace_id,
                pref.key,
                pref.value,
                pref.source,
                pref.updated_at as i64
            ],
        )?;
        Ok(())
    }

    pub fn get_preference(&self, workspace_id: &str, key: &str) -> rusqlite::Result<Option<String>> {
        self.conn
            .query_row(
                "SELECT value FROM memory_preference WHERE workspace_id = ?1 AND key = ?2",
                params![workspace_id, key],
                |row| row.get(0),
            )
            .optional()
    }

    pub fn list_preferences(&self, workspace_id: &str) -> rusqlite::Result<Vec<MemoryPreference>> {
        let mut stmt = self.conn.prepare_cached(
            "SELECT workspace_id, key, value, source, updated_at
             FROM memory_preference WHERE workspace_id = ?1 ORDER BY key ASC",
        )?;
        let rows = stmt.query_map(params![workspace_id], |row| {
            Ok(MemoryPreference {
                workspace_id: row.get(0)?,
                key: row.get(1)?,
                value: row.get(2)?,
                source: row.get(3)?,
                updated_at: row.get::<_, i64>(4)? as u64,
            })
        })?;
        rows.collect()
    }

    pub fn upsert_entity(&self, entity: &MemoryEntity) -> rusqlite::Result<()> {
        self.conn.execute(
            "INSERT OR REPLACE INTO memory_entity (id, workspace_id, kind, name, ref, updated_at)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6)",
            params![
                entity.id,
                entity.workspace_id,
                entity.kind,
                entity.name,
                entity.ref_target,
                entity.updated_at as i64
            ],
        )?;
        Ok(())
    }

    pub fn add_edge(&self, edge: &MemoryEdge) -> rusqlite::Result<()> {
        self.conn.execute(
            "INSERT OR REPLACE INTO memory_edge (from_id, relation, to_id, created_at)
             VALUES (?1, ?2, ?3, ?4)",
            params![
                edge.from_id,
                edge.relation,
                edge.to_id,
                edge.created_at as i64
            ],
        )?;
        Ok(())
    }

    pub fn list_entities(&self, workspace_id: &str) -> rusqlite::Result<Vec<MemoryEntity>> {
        let mut stmt = self.conn.prepare_cached(
            "SELECT id, workspace_id, kind, name, ref, updated_at
             FROM memory_entity WHERE workspace_id = ?1 ORDER BY name ASC",
        )?;
        let rows = stmt.query_map(params![workspace_id], |row| {
            Ok(MemoryEntity {
                id: row.get(0)?,
                workspace_id: row.get(1)?,
                kind: row.get(2)?,
                name: row.get(3)?,
                ref_target: row.get(4)?,
                updated_at: row.get::<_, i64>(5)? as u64,
            })
        })?;
        rows.collect()
    }

    pub fn search_memory_fts(
        &self,
        workspace_id: &str,
        query: &str,
        limit: usize,
    ) -> rusqlite::Result<Vec<FtsSearchResult>> {
        // Sanitize query for FTS5 (avoid syntax errors on raw punctuation)
        let sanitized = query
            .chars()
            .map(|c| if c.is_alphanumeric() || c.is_whitespace() { c } else { ' ' })
            .collect::<String>();
        let trimmed = sanitized
            .split_whitespace()
            .collect::<Vec<_>>()
            .join(" OR ");
        if trimmed.is_empty() {
            return Ok(Vec::new());
        }

        let mut stmt = self.conn.prepare_cached(
            "SELECT m.id, m.thread_id, m.workspace_id, m.content, m.role, rank
             FROM chat_messages m
             JOIN memory_fts f ON m.rowid = f.rowid
             WHERE memory_fts MATCH ?1 AND m.workspace_id = ?2
             ORDER BY rank
             LIMIT ?3",
        )?;

        let rows = stmt.query_map(params![trimmed, workspace_id, limit as i64], |row| {
            Ok(FtsSearchResult {
                message_id: row.get(0)?,
                thread_id: row.get(1)?,
                workspace_id: row.get(2)?,
                content: row.get(3)?,
                role: row.get(4)?,
                score: row.get::<_, f64>(5)?.abs(), // bm25 rank
            })
        })?;

        rows.collect()
    }

    /// List all message embeddings for a given workspace and model to perform in-memory vector search.
    pub fn list_workspace_embeddings(
        &self,
        workspace_id: &str,
        model: &str,
    ) -> rusqlite::Result<Vec<(String, String, Vec<f32>)>> {
        let mut stmt = self.conn.prepare_cached(
            "SELECT m.id, m.content, e.vector
             FROM chat_messages m
             JOIN memory_embedding e ON m.id = e.message_id
             WHERE m.workspace_id = ?1 AND e.model = ?2",
        )?;
        let rows = stmt.query_map(params![workspace_id, model], |row| {
            let msg_id: String = row.get(0)?;
            let content: String = row.get(1)?;
            let bytes: Vec<u8> = row.get(2)?;
            let vector = bytes
                .chunks_exact(4)
                .map(|chunk| f32::from_ne_bytes(chunk.try_into().unwrap()))
                .collect();
            Ok((msg_id, content, vector))
        })?;
        rows.collect()
    }

    /// Reciprocal Rank Fusion (RRF) combining FTS5 BM25 search and cosine vector search.
    /// Formula: RRF_score(d) = sum_{m in {fts, vec}} 1.0 / (k + rank_m(d))
    pub fn search_hybrid_rrf(
        &self,
        workspace_id: &str,
        query_text: &str,
        query_vector: Option<(&str, &[f32])>,
        limit: usize,
    ) -> rusqlite::Result<Vec<FtsSearchResult>> {
        use std::collections::HashMap;

        const K: f64 = 60.0;
        let mut scores: HashMap<String, (f64, String, String, String)> = HashMap::new();

        // 1. FTS5 BM25 search
        let fts_results = self.search_memory_fts(workspace_id, query_text, limit * 2)?;
        for (rank, res) in fts_results.into_iter().enumerate() {
            let rrf_score = 1.0 / (K + (rank as f64) + 1.0);
            scores.insert(
                res.message_id,
                (rrf_score, res.thread_id, res.content, res.role),
            );
        }

        // 2. Vector Cosine Similarity (if query vector provided)
        if let Some((model, q_vec)) = query_vector {
            let embeddings = self.list_workspace_embeddings(workspace_id, model)?;
            let mut vector_ranks: Vec<(f32, String, String)> = embeddings
                .into_iter()
                .map(|(id, content, vec)| {
                    let sim = cosine_similarity(q_vec, &vec);
                    (sim, id, content)
                })
                .collect();

            // Sort descending by similarity
            vector_ranks.sort_by(|a, b| b.0.partial_cmp(&a.0).unwrap_or(std::cmp::Ordering::Equal));

            for (rank, (_, msg_id, content)) in vector_ranks.into_iter().take(limit * 2).enumerate() {
                let rrf_score = 1.0 / (K + (rank as f64) + 1.0);
                scores
                    .entry(msg_id)
                    .and_modify(|entry| entry.0 += rrf_score)
                    .or_insert_with(|| (rrf_score, "".into(), content, "user".into()));
            }
        }

        let mut combined: Vec<FtsSearchResult> = scores
            .into_iter()
            .map(|(msg_id, (score, thread_id, content, role))| FtsSearchResult {
                message_id: msg_id,
                thread_id,
                workspace_id: workspace_id.to_string(),
                content,
                role,
                score,
            })
            .collect();

        // Sort descending by combined RRF score
        combined.sort_by(|a, b| b.score.partial_cmp(&a.score).unwrap_or(std::cmp::Ordering::Equal));
        combined.truncate(limit);

        Ok(combined)
    }
}

/// Compute cosine similarity between two vector slices.
pub fn cosine_similarity(a: &[f32], b: &[f32]) -> f32 {
    if a.len() != b.len() || a.is_empty() {
        return 0.0;
    }
    let mut dot = 0.0f32;
    let mut norm_a = 0.0f32;
    let mut norm_b = 0.0f32;
    for (x, y) in a.iter().zip(b.iter()) {
        dot += x * y;
        norm_a += x * x;
        norm_b += y * y;
    }
    if norm_a == 0.0 || norm_b == 0.0 {
        0.0
    } else {
        dot / (norm_a.sqrt() * norm_b.sqrt())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::storage::threads::ChatMessage;

    #[test]
    fn memory_embedding_and_fts_round_trip() {
        let db = Database::open_in_memory().unwrap();
        db.create_workspace("ws-mem", "Memory Test", "proj", "/tmp")
            .unwrap();
        db.create_thread("th-mem", "ws-mem", "Topic").unwrap();

        let msg = ChatMessage {
            id: "msg-embed-1".into(),
            thread_id: "th-mem".into(),
            workspace_id: "ws-mem".into(),
            role: "user".into(),
            agent: None,
            backend: None,
            model: None,
            content: "Authentication architecture using JWT guards".into(),
            redacted: false,
            created_at: 1000,
        };
        db.insert_chat_message(&msg).unwrap();

        // 1. Test FTS5 trigger and search
        let results = db
            .search_memory_fts("ws-mem", "authentication jwt", 10)
            .unwrap();
        assert_eq!(results.len(), 1);
        assert_eq!(results[0].message_id, "msg-embed-1");
        assert!(results[0].content.contains("Authentication"));

        // 2. Test Vector storage
        let vector = vec![0.1f32, 0.2f32, 0.3f32, 0.4f32];
        db.store_embedding("msg-embed-1", "test-model", 4, &vector)
            .unwrap();
        let fetched = db
            .get_embedding("msg-embed-1", "test-model")
            .unwrap()
            .unwrap();
        assert_eq!(fetched, vector);

        // 3. Test Facts
        let fact = MemoryFact {
            id: "fact-1".into(),
            workspace_id: "ws-mem".into(),
            subject: "Auth".into(),
            predicate: "uses".into(),
            object: "JWT".into(),
            confidence: 0.95,
            source_message_id: Some("msg-embed-1".into()),
            superseded_by: None,
            created_at: 1000,
        };
        db.insert_fact(&fact).unwrap();
        let facts = db.list_facts("ws-mem").unwrap();
        assert_eq!(facts.len(), 1);
        assert_eq!(facts[0].subject, "Auth");

        // 4. Test Preferences
        let pref = MemoryPreference {
            workspace_id: "ws-mem".into(),
            key: "framework".into(),
            value: "actix-web".into(),
            source: Some("user".into()),
            updated_at: 1000,
        };
        db.set_preference(&pref).unwrap();
        assert_eq!(
            db.get_preference("ws-mem", "framework").unwrap().as_deref(),
            Some("actix-web")
        );

        // 5. Test Hybrid RRF Search
        let q_vec = vec![0.1f32, 0.2f32, 0.3f32, 0.4f32];
        let hybrid_results = db
            .search_hybrid_rrf("ws-mem", "jwt", Some(("test-model", &q_vec)), 5)
            .unwrap();
        assert_eq!(hybrid_results.len(), 1);
        assert_eq!(hybrid_results[0].message_id, "msg-embed-1");
        assert!(hybrid_results[0].score > 0.0);
    }
}
