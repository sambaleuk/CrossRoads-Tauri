use rusqlite::{params, Result};
use uuid::Uuid;
use crate::db::manager::with_db;

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AgentMetrics {
    pub id: String,
    pub agent_slot_id: String,
    pub total_stories_completed: i32,
    pub total_stories_failed: i32,
    pub avg_story_time_ms: i64,
    pub conflicts_encountered: i32,
    pub failover_attempts: i32,
    pub last_story_started_at: Option<String>,
    pub updated_at: String,
}

/// Get or create metrics for a slot
pub fn get_or_create(slot_id: &str) -> Result<AgentMetrics> {
    with_db(|conn| {
        let existing: Option<AgentMetrics> = conn.query_row(
            "SELECT id,agentSlotId,totalStoriesCompleted,totalStoriesFailed,avgStoryTimeMs,conflictsEncountered,failoverAttempts,lastStoryStartedAt,updatedAt FROM agent_metrics WHERE agentSlotId=?1",
            params![slot_id],
            |row| Ok(AgentMetrics {
                id: row.get(0)?, agent_slot_id: row.get(1)?,
                total_stories_completed: row.get(2)?, total_stories_failed: row.get(3)?,
                avg_story_time_ms: row.get(4)?, conflicts_encountered: row.get(5)?,
                failover_attempts: row.get(6)?, last_story_started_at: row.get(7)?,
                updated_at: row.get(8)?,
            }),
        ).ok();

        if let Some(m) = existing {
            return Ok(m);
        }

        let id = Uuid::new_v4().to_string();
        let now = chrono::Utc::now().to_rfc3339();
        conn.execute(
            "INSERT INTO agent_metrics (id,agentSlotId,totalStoriesCompleted,totalStoriesFailed,avgStoryTimeMs,conflictsEncountered,failoverAttempts,updatedAt) VALUES (?1,?2,0,0,0,0,0,?3)",
            params![id, slot_id, now],
        )?;
        Ok(AgentMetrics {
            id, agent_slot_id: slot_id.to_string(),
            total_stories_completed: 0, total_stories_failed: 0,
            avg_story_time_ms: 0, conflicts_encountered: 0,
            failover_attempts: 0, last_story_started_at: None,
            updated_at: now,
        })
    })
}

/// Record a story completion (updates running average)
pub fn record_story_completed(slot_id: &str, story_time_ms: i64) -> Result<()> {
    with_db(|conn| {
        let now = chrono::Utc::now().to_rfc3339();
        // Update with running average: new_avg = (old_avg * count + new_time) / (count + 1)
        conn.execute(
            "UPDATE agent_metrics SET
                totalStoriesCompleted = totalStoriesCompleted + 1,
                avgStoryTimeMs = (avgStoryTimeMs * totalStoriesCompleted + ?1) / (totalStoriesCompleted + 1),
                updatedAt = ?2
            WHERE agentSlotId = ?3",
            params![story_time_ms, now, slot_id],
        )?;
        Ok(())
    })
}

/// Record a story failure
pub fn record_story_failed(slot_id: &str) -> Result<()> {
    with_db(|conn| {
        let now = chrono::Utc::now().to_rfc3339();
        conn.execute(
            "UPDATE agent_metrics SET totalStoriesFailed = totalStoriesFailed + 1, updatedAt = ?1 WHERE agentSlotId = ?2",
            params![now, slot_id],
        )?;
        Ok(())
    })
}

/// Record a conflict encountered
pub fn record_conflict(slot_id: &str) -> Result<()> {
    with_db(|conn| {
        let now = chrono::Utc::now().to_rfc3339();
        conn.execute(
            "UPDATE agent_metrics SET conflictsEncountered = conflictsEncountered + 1, updatedAt = ?1 WHERE agentSlotId = ?2",
            params![now, slot_id],
        )?;
        Ok(())
    })
}

/// Increment failover attempts
pub fn record_failover(slot_id: &str) -> Result<()> {
    with_db(|conn| {
        let now = chrono::Utc::now().to_rfc3339();
        conn.execute(
            "UPDATE agent_metrics SET failoverAttempts = failoverAttempts + 1, updatedAt = ?1 WHERE agentSlotId = ?2",
            params![now, slot_id],
        )?;
        Ok(())
    })
}

/// Mark story start time
pub fn mark_story_start(slot_id: &str) -> Result<()> {
    with_db(|conn| {
        let now = chrono::Utc::now().to_rfc3339();
        conn.execute(
            "UPDATE agent_metrics SET lastStoryStartedAt = ?1, updatedAt = ?1 WHERE agentSlotId = ?2",
            params![now, slot_id],
        )?;
        Ok(())
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::db::{manager::initialize_memory, session_repo, slot_repo};

    #[test]
    fn test_metrics_crud() {
        initialize_memory().unwrap();
        let session = session_repo::create_session("/tmp/metrics-test").unwrap();
        let slot = slot_repo::create_slot(&session.id, 0, "claude", None, None).unwrap();

        let m = get_or_create(&slot.id).unwrap();
        assert_eq!(m.total_stories_completed, 0);
        assert_eq!(m.failover_attempts, 0);

        // Idempotent get_or_create
        let m2 = get_or_create(&slot.id).unwrap();
        assert_eq!(m.id, m2.id);

        record_story_completed(&slot.id, 5000).unwrap();
        let m3 = get_or_create(&slot.id).unwrap();
        assert_eq!(m3.total_stories_completed, 1);
        assert_eq!(m3.avg_story_time_ms, 5000);

        record_story_completed(&slot.id, 3000).unwrap();
        let m4 = get_or_create(&slot.id).unwrap();
        assert_eq!(m4.total_stories_completed, 2);
        assert_eq!(m4.avg_story_time_ms, 4000); // (5000*1 + 3000) / 2

        record_story_failed(&slot.id).unwrap();
        record_conflict(&slot.id).unwrap();
        record_failover(&slot.id).unwrap();

        let m5 = get_or_create(&slot.id).unwrap();
        assert_eq!(m5.total_stories_failed, 1);
        assert_eq!(m5.conflicts_encountered, 1);
        assert_eq!(m5.failover_attempts, 1);
    }
}
