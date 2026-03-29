use rusqlite::{params, Result};
use uuid::Uuid;
use crate::db::manager::with_db;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct OrchestrationRecord {
    pub id: String,
    pub cockpit_session_id: String,
    pub prd_path: String,
    pub prd_name: String,
    pub status: String, // pending, running, paused, completed, failed
    pub total_stories: i32,
    pub completed_stories: i32,
    pub failed_stories: i32,
    pub current_layer: i32,
    pub layers_json: Option<String>,
    pub result_summary: Option<String>,
    pub merged_branches: Option<String>, // JSON array as string
    pub conflicts: Option<String>,       // JSON array as string
    pub total_cost_cents: i64,
    pub started_at: String,
    pub finished_at: Option<String>,
    pub updated_at: String,
}

pub fn create_record(session_id: &str, prd_path: &str, prd_name: &str, total_stories: i32, layers_json: &str) -> Result<OrchestrationRecord> {
    with_db(|conn| {
        let id = Uuid::new_v4().to_string();
        let now = chrono::Utc::now().to_rfc3339();
        conn.execute(
            "INSERT INTO orchestration_record (id,cockpitSessionId,prdPath,prdName,status,totalStories,completedStories,failedStories,currentLayer,layersJson,totalCostCents,startedAt,updatedAt) VALUES (?1,?2,?3,?4,'running',?5,0,0,0,?6,0,?7,?7)",
            params![id, session_id, prd_path, prd_name, total_stories, layers_json, now],
        )?;
        Ok(OrchestrationRecord {
            id, cockpit_session_id: session_id.to_string(),
            prd_path: prd_path.to_string(), prd_name: prd_name.to_string(),
            status: "running".into(), total_stories, completed_stories: 0,
            failed_stories: 0, current_layer: 0,
            layers_json: Some(layers_json.to_string()),
            result_summary: None, merged_branches: None, conflicts: None,
            total_cost_cents: 0, started_at: now.clone(), finished_at: None, updated_at: now,
        })
    })
}

pub fn update_progress(id: &str, completed: i32, failed: i32, current_layer: i32) -> Result<()> {
    with_db(|conn| {
        let now = chrono::Utc::now().to_rfc3339();
        conn.execute(
            "UPDATE orchestration_record SET completedStories=?1, failedStories=?2, currentLayer=?3, updatedAt=?4 WHERE id=?5",
            params![completed, failed, current_layer, now, id],
        )?;
        Ok(())
    })
}

pub fn complete_record(id: &str, summary: &str, merged_branches: &str, conflicts: &str, total_cost: i64) -> Result<()> {
    with_db(|conn| {
        let now = chrono::Utc::now().to_rfc3339();
        conn.execute(
            "UPDATE orchestration_record SET status='completed', resultSummary=?1, mergedBranches=?2, conflicts=?3, totalCostCents=?4, finishedAt=?5, updatedAt=?5 WHERE id=?6",
            params![summary, merged_branches, conflicts, total_cost, now, id],
        )?;
        Ok(())
    })
}

pub fn fail_record(id: &str, summary: &str) -> Result<()> {
    with_db(|conn| {
        let now = chrono::Utc::now().to_rfc3339();
        conn.execute(
            "UPDATE orchestration_record SET status='failed', resultSummary=?1, finishedAt=?2, updatedAt=?2 WHERE id=?3",
            params![summary, now, id],
        )?;
        Ok(())
    })
}

pub fn fetch_record(id: &str) -> Result<Option<OrchestrationRecord>> {
    with_db(|conn| {
        Ok(conn.query_row(
            "SELECT id,cockpitSessionId,prdPath,prdName,status,totalStories,completedStories,failedStories,currentLayer,layersJson,resultSummary,mergedBranches,conflicts,totalCostCents,startedAt,finishedAt,updatedAt FROM orchestration_record WHERE id=?1",
            params![id],
            |row| Ok(OrchestrationRecord {
                id: row.get(0)?, cockpit_session_id: row.get(1)?,
                prd_path: row.get(2)?, prd_name: row.get(3)?,
                status: row.get(4)?, total_stories: row.get(5)?,
                completed_stories: row.get(6)?, failed_stories: row.get(7)?,
                current_layer: row.get(8)?, layers_json: row.get(9)?,
                result_summary: row.get(10)?, merged_branches: row.get(11)?,
                conflicts: row.get(12)?, total_cost_cents: row.get(13)?,
                started_at: row.get(14)?, finished_at: row.get(15)?,
                updated_at: row.get(16)?,
            }),
        ).ok())
    })
}

/// Find the latest incomplete orchestration for a session
pub fn find_incomplete(session_id: &str) -> Result<Option<OrchestrationRecord>> {
    with_db(|conn| {
        Ok(conn.query_row(
            "SELECT id,cockpitSessionId,prdPath,prdName,status,totalStories,completedStories,failedStories,currentLayer,layersJson,resultSummary,mergedBranches,conflicts,totalCostCents,startedAt,finishedAt,updatedAt FROM orchestration_record WHERE cockpitSessionId=?1 AND status='running' ORDER BY startedAt DESC LIMIT 1",
            params![session_id],
            |row| Ok(OrchestrationRecord {
                id: row.get(0)?, cockpit_session_id: row.get(1)?,
                prd_path: row.get(2)?, prd_name: row.get(3)?,
                status: row.get(4)?, total_stories: row.get(5)?,
                completed_stories: row.get(6)?, failed_stories: row.get(7)?,
                current_layer: row.get(8)?, layers_json: row.get(9)?,
                result_summary: row.get(10)?, merged_branches: row.get(11)?,
                conflicts: row.get(12)?, total_cost_cents: row.get(13)?,
                started_at: row.get(14)?, finished_at: row.get(15)?,
                updated_at: row.get(16)?,
            }),
        ).ok())
    })
}

pub fn fetch_records_for_session(session_id: &str) -> Result<Vec<OrchestrationRecord>> {
    with_db(|conn| {
        let mut stmt = conn.prepare(
            "SELECT id,cockpitSessionId,prdPath,prdName,status,totalStories,completedStories,failedStories,currentLayer,layersJson,resultSummary,mergedBranches,conflicts,totalCostCents,startedAt,finishedAt,updatedAt FROM orchestration_record WHERE cockpitSessionId=?1 ORDER BY startedAt DESC"
        )?;
        let rows = stmt.query_map(params![session_id], |row| {
            Ok(OrchestrationRecord {
                id: row.get(0)?, cockpit_session_id: row.get(1)?,
                prd_path: row.get(2)?, prd_name: row.get(3)?,
                status: row.get(4)?, total_stories: row.get(5)?,
                completed_stories: row.get(6)?, failed_stories: row.get(7)?,
                current_layer: row.get(8)?, layers_json: row.get(9)?,
                result_summary: row.get(10)?, merged_branches: row.get(11)?,
                conflicts: row.get(12)?, total_cost_cents: row.get(13)?,
                started_at: row.get(14)?, finished_at: row.get(15)?,
                updated_at: row.get(16)?,
            })
        })?;
        rows.collect()
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::db::{manager::initialize_memory, session_repo};

    #[test]
    fn test_orchestration_record_lifecycle() {
        initialize_memory().unwrap();
        let session = session_repo::create_session("/tmp/orch-test").unwrap();

        let record = create_record(&session.id, "/tmp/prd.json", "Test Feature", 5, "[[\"US-001\"],[\"US-002\",\"US-003\"]]").unwrap();
        assert_eq!(record.status, "running");
        assert_eq!(record.total_stories, 5);

        update_progress(&record.id, 2, 0, 1).unwrap();
        let r = fetch_record(&record.id).unwrap().unwrap();
        assert_eq!(r.completed_stories, 2);
        assert_eq!(r.current_layer, 1);

        // Find incomplete
        let inc = find_incomplete(&session.id).unwrap().unwrap();
        assert_eq!(inc.id, record.id);

        // Complete
        complete_record(&record.id, "All done", "[\"feat/a\"]", "[]", 1500).unwrap();
        let r = fetch_record(&record.id).unwrap().unwrap();
        assert_eq!(r.status, "completed");
        assert!(r.finished_at.is_some());

        // No more incomplete
        assert!(find_incomplete(&session.id).unwrap().is_none());
    }
}
