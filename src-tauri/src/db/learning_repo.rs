use rusqlite::{params, Result};
use uuid::Uuid;
use serde::{Deserialize, Serialize};
use crate::db::manager::with_db;

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct LearningRecord {
    pub id: String,
    pub session_id: String,
    pub story_id: String,
    pub story_title: String,
    pub story_complexity: String,
    pub agent_type: String,
    pub runtime_id: Option<String>,
    pub model: Option<String>,
    pub duration_ms: i64,
    pub cost_cents: i64,
    pub files_changed: i32,
    pub lines_added: i32,
    pub lines_removed: i32,
    pub tests_run: i32,
    pub tests_passed: i32,
    pub tests_failed: i32,
    pub conflicts_encountered: i32,
    pub retries_needed: i32,
    pub success: bool,
    pub failure_reason: Option<String>,
    pub file_patterns: String,
    pub created_at: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PerformanceProfile {
    pub id: String,
    pub agent_type: String,
    pub runtime_id: Option<String>,
    pub task_category: String,
    pub total_executions: i32,
    pub success_rate: f64,
    pub avg_duration_ms: i64,
    pub avg_cost_cents: i64,
    pub avg_files_changed: f64,
    pub avg_test_pass_rate: f64,
    pub conflict_rate: f64,
    pub last_updated_at: String,
}

const LR_COLS: &str = "id,sessionId,storyId,storyTitle,storyComplexity,agentType,runtimeId,model,durationMs,costCents,filesChanged,linesAdded,linesRemoved,testsRun,testsPassed,testsFailed,conflictsEncountered,retriesNeeded,success,failureReason,filePatterns,createdAt";
const PP_COLS: &str = "id,agentType,runtimeId,taskCategory,totalExecutions,successRate,avgDurationMs,avgCostCents,avgFilesChanged,avgTestPassRate,conflictRate,lastUpdatedAt";

fn row_to_learning(row: &rusqlite::Row) -> rusqlite::Result<LearningRecord> {
    Ok(LearningRecord {
        id: row.get(0)?,
        session_id: row.get(1)?,
        story_id: row.get(2)?,
        story_title: row.get(3)?,
        story_complexity: row.get(4)?,
        agent_type: row.get(5)?,
        runtime_id: row.get(6)?,
        model: row.get(7)?,
        duration_ms: row.get(8)?,
        cost_cents: row.get(9)?,
        files_changed: row.get(10)?,
        lines_added: row.get(11)?,
        lines_removed: row.get(12)?,
        tests_run: row.get(13)?,
        tests_passed: row.get(14)?,
        tests_failed: row.get(15)?,
        conflicts_encountered: row.get(16)?,
        retries_needed: row.get(17)?,
        success: row.get(18)?,
        failure_reason: row.get(19)?,
        file_patterns: row.get(20)?,
        created_at: row.get(21)?,
    })
}

fn row_to_profile(row: &rusqlite::Row) -> rusqlite::Result<PerformanceProfile> {
    Ok(PerformanceProfile {
        id: row.get(0)?,
        agent_type: row.get(1)?,
        runtime_id: row.get(2)?,
        task_category: row.get(3)?,
        total_executions: row.get(4)?,
        success_rate: row.get(5)?,
        avg_duration_ms: row.get(6)?,
        avg_cost_cents: row.get(7)?,
        avg_files_changed: row.get(8)?,
        avg_test_pass_rate: row.get(9)?,
        conflict_rate: row.get(10)?,
        last_updated_at: row.get(11)?,
    })
}

pub fn record_learning(
    session_id: &str,
    story_id: &str,
    story_title: &str,
    complexity: &str,
    agent_type: &str,
    duration_ms: i64,
    cost_cents: i64,
    files_changed: i32,
    lines_added: i32,
    lines_removed: i32,
    tests_run: i32,
    tests_passed: i32,
    conflicts: i32,
    success: bool,
    file_patterns_json: &str,
) -> Result<LearningRecord> {
    with_db(|conn| {
        let id = Uuid::new_v4().to_string();
        let now = chrono::Utc::now().to_rfc3339();
        let tests_failed = tests_run - tests_passed;
        conn.execute(
            "INSERT INTO learning_record (id,sessionId,storyId,storyTitle,storyComplexity,agentType,durationMs,costCents,filesChanged,linesAdded,linesRemoved,testsRun,testsPassed,testsFailed,conflictsEncountered,success,filePatterns,createdAt) VALUES (?1,?2,?3,?4,?5,?6,?7,?8,?9,?10,?11,?12,?13,?14,?15,?16,?17,?18)",
            params![id, session_id, story_id, story_title, complexity, agent_type, duration_ms, cost_cents, files_changed, lines_added, lines_removed, tests_run, tests_passed, tests_failed, conflicts, success, file_patterns_json, now],
        )?;
        Ok(LearningRecord {
            id,
            session_id: session_id.to_string(),
            story_id: story_id.to_string(),
            story_title: story_title.to_string(),
            story_complexity: complexity.to_string(),
            agent_type: agent_type.to_string(),
            runtime_id: None,
            model: None,
            duration_ms,
            cost_cents,
            files_changed,
            lines_added,
            lines_removed,
            tests_run,
            tests_passed,
            tests_failed,
            conflicts_encountered: conflicts,
            retries_needed: 0,
            success,
            failure_reason: None,
            file_patterns: file_patterns_json.to_string(),
            created_at: now,
        })
    })
}

/// Update (or create) a performance profile using a running average calculation.
pub fn update_performance_profile(
    agent_type: &str,
    task_category: &str,
    record: &LearningRecord,
) -> Result<()> {
    with_db(|conn| {
        let now = chrono::Utc::now().to_rfc3339();

        // Check if profile exists
        let existing: Option<PerformanceProfile> = conn.query_row(
            &format!("SELECT {} FROM performance_profile WHERE agentType=?1 AND taskCategory=?2", PP_COLS),
            params![agent_type, task_category],
            row_to_profile,
        ).ok();

        match existing {
            Some(prof) => {
                let n = prof.total_executions as f64;
                let new_n = n + 1.0;

                // Running averages
                let new_success_rate = (prof.success_rate * n + if record.success { 1.0 } else { 0.0 }) / new_n;
                let new_avg_duration = ((prof.avg_duration_ms as f64 * n + record.duration_ms as f64) / new_n) as i64;
                let new_avg_cost = ((prof.avg_cost_cents as f64 * n + record.cost_cents as f64) / new_n) as i64;
                let new_avg_files = (prof.avg_files_changed * n + record.files_changed as f64) / new_n;
                let test_pass_rate = if record.tests_run > 0 {
                    record.tests_passed as f64 / record.tests_run as f64
                } else {
                    1.0
                };
                let new_avg_test_pass = (prof.avg_test_pass_rate * n + test_pass_rate) / new_n;
                let had_conflict = if record.conflicts_encountered > 0 { 1.0 } else { 0.0 };
                let new_conflict_rate = (prof.conflict_rate * n + had_conflict) / new_n;

                conn.execute(
                    "UPDATE performance_profile SET totalExecutions=?1, successRate=?2, avgDurationMs=?3, avgCostCents=?4, avgFilesChanged=?5, avgTestPassRate=?6, conflictRate=?7, lastUpdatedAt=?8 WHERE id=?9",
                    params![
                        prof.total_executions + 1,
                        new_success_rate, new_avg_duration, new_avg_cost,
                        new_avg_files, new_avg_test_pass, new_conflict_rate,
                        now, prof.id
                    ],
                )?;
            }
            None => {
                let id = Uuid::new_v4().to_string();
                let test_pass_rate = if record.tests_run > 0 {
                    record.tests_passed as f64 / record.tests_run as f64
                } else {
                    1.0
                };
                let conflict_rate = if record.conflicts_encountered > 0 { 1.0 } else { 0.0 };

                conn.execute(
                    "INSERT INTO performance_profile (id,agentType,taskCategory,totalExecutions,successRate,avgDurationMs,avgCostCents,avgFilesChanged,avgTestPassRate,conflictRate,lastUpdatedAt) VALUES (?1,?2,?3,1,?4,?5,?6,?7,?8,?9,?10)",
                    params![
                        id, agent_type, task_category,
                        if record.success { 1.0 } else { 0.0 },
                        record.duration_ms, record.cost_cents,
                        record.files_changed as f64,
                        test_pass_rate, conflict_rate, now
                    ],
                )?;
            }
        }
        Ok(())
    })
}

pub fn fetch_profile(agent_type: &str, task_category: &str) -> Result<Option<PerformanceProfile>> {
    with_db(|conn| {
        Ok(conn.query_row(
            &format!("SELECT {} FROM performance_profile WHERE agentType=?1 AND taskCategory=?2", PP_COLS),
            params![agent_type, task_category],
            row_to_profile,
        ).ok())
    })
}

pub fn fetch_all_profiles() -> Result<Vec<PerformanceProfile>> {
    with_db(|conn| {
        let mut stmt = conn.prepare(
            &format!("SELECT {} FROM performance_profile ORDER BY agentType, taskCategory", PP_COLS),
        )?;
        let rows = stmt.query_map([], row_to_profile)?;
        rows.collect()
    })
}

pub fn fetch_learning_records(session_id: &str) -> Result<Vec<LearningRecord>> {
    with_db(|conn| {
        let mut stmt = conn.prepare(
            &format!("SELECT {} FROM learning_record WHERE sessionId=?1 ORDER BY createdAt DESC", LR_COLS),
        )?;
        let rows = stmt.query_map(params![session_id], row_to_learning)?;
        rows.collect()
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::db::{manager::initialize_memory, session_repo};

    #[test]
    fn test_learning_and_profiles() {
        initialize_memory().unwrap();
        let session = session_repo::create_session("/tmp/learning-test").unwrap();

        // Record first learning
        let lr1 = record_learning(
            &session.id, "US-001", "Add login", "moderate", "claude",
            5000, 150, 3, 100, 20, 10, 9, 0, true, "[\"src/auth\"]",
        ).unwrap();
        assert!(lr1.success);
        assert_eq!(lr1.tests_failed, 1); // 10 - 9

        // Update performance profile
        update_performance_profile("claude", "feature", &lr1).unwrap();

        let prof = fetch_profile("claude", "feature").unwrap().unwrap();
        assert_eq!(prof.total_executions, 1);
        assert_eq!(prof.success_rate, 1.0);
        assert_eq!(prof.avg_duration_ms, 5000);
        assert_eq!(prof.avg_cost_cents, 150);

        // Record second learning (failed)
        let lr2 = record_learning(
            &session.id, "US-002", "Fix bug", "simple", "claude",
            3000, 80, 1, 20, 5, 5, 5, 1, false, "[\"src/core\"]",
        ).unwrap();
        assert!(!lr2.success);

        // Update profile again — running averages
        update_performance_profile("claude", "feature", &lr2).unwrap();

        let prof2 = fetch_profile("claude", "feature").unwrap().unwrap();
        assert_eq!(prof2.total_executions, 2);
        assert_eq!(prof2.success_rate, 0.5); // 1 success out of 2
        assert_eq!(prof2.avg_duration_ms, 4000); // (5000 + 3000) / 2
        assert_eq!(prof2.avg_cost_cents, 115); // (150 + 80) / 2

        // Fetch all records
        let records = fetch_learning_records(&session.id).unwrap();
        assert_eq!(records.len(), 2);

        // Fetch all profiles
        let all_profiles = fetch_all_profiles().unwrap();
        assert_eq!(all_profiles.len(), 1);

        // Different agent type creates separate profile
        update_performance_profile("gemini", "feature", &lr1).unwrap();
        let all_profiles2 = fetch_all_profiles().unwrap();
        assert_eq!(all_profiles2.len(), 2);
    }
}
