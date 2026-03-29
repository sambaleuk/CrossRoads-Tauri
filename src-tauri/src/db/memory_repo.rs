use rusqlite::{params, Result};
use uuid::Uuid;
use serde::{Deserialize, Serialize};
use crate::db::manager::with_db;
use crate::db::learning_repo::LearningRecord;

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AgentMemory {
    pub id: String,
    pub agent_type: String,
    pub domain: String,
    pub memory_type: String,
    pub content: String,
    pub confidence: f64,
    pub source_session_id: Option<String>,
    pub source_story_id: Option<String>,
    pub tags: String,
    pub access_count: i32,
    pub last_accessed_at: Option<String>,
    pub created_at: String,
}

const AM_COLS: &str = "id,agentType,domain,memoryType,content,confidence,sourceSessionId,sourceStoryId,tags,accessCount,lastAccessedAt,createdAt";

fn row_to_memory(row: &rusqlite::Row) -> rusqlite::Result<AgentMemory> {
    Ok(AgentMemory {
        id: row.get(0)?,
        agent_type: row.get(1)?,
        domain: row.get(2)?,
        memory_type: row.get(3)?,
        content: row.get(4)?,
        confidence: row.get(5)?,
        source_session_id: row.get(6)?,
        source_story_id: row.get(7)?,
        tags: row.get(8)?,
        access_count: row.get(9)?,
        last_accessed_at: row.get(10)?,
        created_at: row.get(11)?,
    })
}

/// Store a new memory for an agent in a specific domain.
pub fn store_memory(
    agent_type: &str,
    domain: &str,
    memory_type: &str,
    content: &str,
    confidence: f64,
    session_id: Option<&str>,
    story_id: Option<&str>,
    tags_json: &str,
) -> Result<AgentMemory> {
    with_db(|conn| {
        let id = Uuid::new_v4().to_string();
        let now = chrono::Utc::now().to_rfc3339();
        conn.execute(
            "INSERT INTO agent_memory (id,agentType,domain,memoryType,content,confidence,sourceSessionId,sourceStoryId,tags,createdAt) VALUES (?1,?2,?3,?4,?5,?6,?7,?8,?9,?10)",
            params![id, agent_type, domain, memory_type, content, confidence, session_id, story_id, tags_json, now],
        )?;
        Ok(AgentMemory {
            id,
            agent_type: agent_type.to_string(),
            domain: domain.to_string(),
            memory_type: memory_type.to_string(),
            content: content.to_string(),
            confidence,
            source_session_id: session_id.map(String::from),
            source_story_id: story_id.map(String::from),
            tags: tags_json.to_string(),
            access_count: 0,
            last_accessed_at: None,
            created_at: now,
        })
    })
}

/// Recall memories for an agent in a domain, ordered by confidence and access frequency.
pub fn recall_memories(agent_type: &str, domain: &str, limit: i32) -> Result<Vec<AgentMemory>> {
    with_db(|conn| {
        let mut stmt = conn.prepare(
            &format!(
                "SELECT {} FROM agent_memory WHERE agentType=?1 AND domain=?2 ORDER BY confidence DESC, accessCount DESC LIMIT ?3",
                AM_COLS
            ),
        )?;
        let rows = stmt.query_map(params![agent_type, domain, limit], row_to_memory)?;
        rows.collect()
    })
}

/// Search memories by content or tags using LIKE.
pub fn search_memories(query: &str) -> Result<Vec<AgentMemory>> {
    with_db(|conn| {
        let pattern = format!("%{}%", query);
        let mut stmt = conn.prepare(
            &format!(
                "SELECT {} FROM agent_memory WHERE content LIKE ?1 OR tags LIKE ?1 ORDER BY confidence DESC",
                AM_COLS
            ),
        )?;
        let rows = stmt.query_map(params![pattern], row_to_memory)?;
        rows.collect()
    })
}

/// Record an access to a memory: increment access_count and update lastAccessedAt.
pub fn record_access(id: &str) -> Result<()> {
    with_db(|conn| {
        let now = chrono::Utc::now().to_rfc3339();
        conn.execute(
            "UPDATE agent_memory SET accessCount = accessCount + 1, lastAccessedAt = ?1 WHERE id = ?2",
            params![now, id],
        )?;
        Ok(())
    })
}

/// Delete memories older than N days with low confidence (< 0.3).
pub fn forget_old(days: i32) -> Result<u32> {
    with_db(|conn| {
        let cutoff = chrono::Utc::now() - chrono::Duration::days(days as i64);
        let cutoff_str = cutoff.to_rfc3339();
        let deleted = conn.execute(
            "DELETE FROM agent_memory WHERE createdAt < ?1 AND confidence < 0.3",
            params![cutoff_str],
        )?;
        Ok(deleted as u32)
    })
}

/// Auto-extract memories from learning records for a session.
/// Detects struggles, slow performance, and conflicts from past executions.
pub fn auto_extract_memories(
    session_id: &str,
    learning_records: &[LearningRecord],
) -> Result<Vec<AgentMemory>> {
    // Compute average duration across all records for comparison
    let avg_duration: f64 = if learning_records.is_empty() {
        0.0
    } else {
        learning_records.iter().map(|r| r.duration_ms as f64).sum::<f64>()
            / learning_records.len() as f64
    };

    let mut memories = Vec::new();

    for record in learning_records {
        let category = crate::services::learning_engine::categorize_story(
            &record.story_title,
            &serde_json::from_str::<Vec<String>>(&record.file_patterns).unwrap_or_default(),
        );

        // If tests failed, create a "struggled with" memory
        if record.tests_failed > 0 {
            let content = format!("struggled with {} (tests failed: {})", category, record.tests_failed);
            let confidence = 0.6 + (record.tests_failed as f64 * 0.05).min(0.3);
            let mem = store_memory(
                &record.agent_type,
                &category,
                "observation",
                &content,
                confidence,
                Some(session_id),
                Some(&record.story_id),
                &format!("[\"struggle\",\"{}\"]", category),
            )?;
            memories.push(mem);
        }

        // If story took 3x average, create a "slow on" memory
        if avg_duration > 0.0 && record.duration_ms as f64 > avg_duration * 3.0 {
            let content = format!(
                "slow on {} (took {}ms vs avg {}ms)",
                category, record.duration_ms, avg_duration as i64
            );
            let mem = store_memory(
                &record.agent_type,
                &category,
                "observation",
                &content,
                0.7,
                Some(session_id),
                Some(&record.story_id),
                &format!("[\"slow\",\"{}\"]", category),
            )?;
            memories.push(mem);
        }

        // If conflicts encountered, create a "conflicts with" memory
        if record.conflicts_encountered > 0 {
            let content = format!("conflicts with {} (patterns: {})", category, record.file_patterns);
            let mem = store_memory(
                &record.agent_type,
                &category,
                "observation",
                &content,
                0.8,
                Some(session_id),
                Some(&record.story_id),
                &format!("[\"conflict\",\"{}\"]", category),
            )?;
            memories.push(mem);
        }
    }

    Ok(memories)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::db::manager::initialize_memory;

    #[test]
    fn test_memory_crud() {
        initialize_memory().unwrap();

        // Store a memory
        let mem = store_memory(
            "claude", "backend", "observation",
            "Excels at Rust API design",
            0.9, Some("session-1"), Some("US-001"),
            "[\"rust\",\"api\"]",
        ).unwrap();
        assert_eq!(mem.agent_type, "claude");
        assert_eq!(mem.domain, "backend");
        assert_eq!(mem.confidence, 0.9);
        assert_eq!(mem.access_count, 0);

        // Record access
        record_access(&mem.id).unwrap();
        let recalled = recall_memories("claude", "backend", 10).unwrap();
        assert_eq!(recalled.len(), 1);
        assert_eq!(recalled[0].access_count, 1);
        assert!(recalled[0].last_accessed_at.is_some());

        // Search
        let found = search_memories("Rust API").unwrap();
        assert_eq!(found.len(), 1);
        assert_eq!(found[0].id, mem.id);

        // Search by tag
        let found_tag = search_memories("rust").unwrap();
        assert_eq!(found_tag.len(), 1);

        // Forget old — nothing should be deleted (memory is fresh)
        let deleted = forget_old(1).unwrap();
        assert_eq!(deleted, 0);
    }

    #[test]
    fn test_recall_by_domain() {
        initialize_memory().unwrap();

        // Store memories in different domains
        store_memory("claude", "backend", "observation", "Good at APIs", 0.9, None, None, "[]").unwrap();
        store_memory("claude", "frontend", "observation", "Struggles with CSS", 0.4, None, None, "[]").unwrap();
        store_memory("claude", "backend", "observation", "Fast at DB queries", 0.8, None, None, "[]").unwrap();
        store_memory("gemini", "backend", "observation", "Good at prototyping", 0.7, None, None, "[]").unwrap();

        // Recall only claude + backend
        let memories = recall_memories("claude", "backend", 10).unwrap();
        assert_eq!(memories.len(), 2);
        // Should be ordered by confidence DESC
        assert!(memories[0].confidence >= memories[1].confidence);

        // Recall gemini + backend
        let gemini_mem = recall_memories("gemini", "backend", 10).unwrap();
        assert_eq!(gemini_mem.len(), 1);

        // Recall claude + frontend
        let fe_mem = recall_memories("claude", "frontend", 10).unwrap();
        assert_eq!(fe_mem.len(), 1);
    }

    #[test]
    fn test_auto_extract() {
        initialize_memory().unwrap();

        let records = vec![
            LearningRecord {
                id: "lr-1".into(),
                session_id: "sess-1".into(),
                story_id: "US-001".into(),
                story_title: "Build REST API endpoint".into(),
                story_complexity: "moderate".into(),
                agent_type: "claude".into(),
                runtime_id: None,
                model: None,
                duration_ms: 5000,
                cost_cents: 100,
                files_changed: 3,
                lines_added: 100,
                lines_removed: 10,
                tests_run: 10,
                tests_passed: 7,
                tests_failed: 3,
                conflicts_encountered: 1,
                retries_needed: 0,
                success: false,
                failure_reason: None,
                file_patterns: "[\"src/api/users.rs\"]".into(),
                created_at: chrono::Utc::now().to_rfc3339(),
            },
            LearningRecord {
                id: "lr-2".into(),
                session_id: "sess-1".into(),
                story_id: "US-002".into(),
                story_title: "Add unit tests".into(),
                story_complexity: "simple".into(),
                agent_type: "claude".into(),
                runtime_id: None,
                model: None,
                duration_ms: 1000,
                cost_cents: 50,
                files_changed: 1,
                lines_added: 30,
                lines_removed: 0,
                tests_run: 5,
                tests_passed: 5,
                tests_failed: 0,
                conflicts_encountered: 0,
                retries_needed: 0,
                success: true,
                failure_reason: None,
                file_patterns: "[\"tests/\"]".into(),
                created_at: chrono::Utc::now().to_rfc3339(),
            },
        ];

        let memories = auto_extract_memories("sess-1", &records).unwrap();
        // lr-1: tests_failed=3 → "struggled" memory
        // lr-1: 5000 > 3000 * 3.0? avg = 3000, 5000 < 9000 → no slow memory
        // lr-1: conflicts=1 → "conflicts with" memory
        // lr-2: all good, no memories
        assert_eq!(memories.len(), 2);
        assert!(memories[0].content.contains("struggled"));
        assert!(memories[1].content.contains("conflicts"));
    }
}
