use rusqlite::{params, Result};
use uuid::Uuid;
use serde::{Deserialize, Serialize};
use crate::db::manager::with_db;
use crate::db::learning_repo;

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct TrustScore {
    pub id: String,
    pub agent_type: String,
    pub domain: String,
    pub score: f64,
    pub total_stories: i32,
    pub successful_stories: i32,
    pub total_tests_passed: i32,
    pub total_tests_failed: i32,
    pub auto_merge_enabled: bool,
    pub auto_merge_threshold: f64,
    pub last_computed_at: String,
}

const TS_COLS: &str = "id,agentType,domain,score,totalStories,successfulStories,totalTestsPassed,totalTestsFailed,autoMergeEnabled,autoMergeThreshold,lastComputedAt";

fn row_to_trust(row: &rusqlite::Row) -> rusqlite::Result<TrustScore> {
    Ok(TrustScore {
        id: row.get(0)?,
        agent_type: row.get(1)?,
        domain: row.get(2)?,
        score: row.get(3)?,
        total_stories: row.get(4)?,
        successful_stories: row.get(5)?,
        total_tests_passed: row.get(6)?,
        total_tests_failed: row.get(7)?,
        auto_merge_enabled: row.get::<_, i32>(8)? != 0,
        auto_merge_threshold: row.get(9)?,
        last_computed_at: row.get(10)?,
    })
}

/// Compute trust score for an agent in a domain from PerformanceProfile and LearningRecords.
/// Score formula: (successRate * 0.4) + (testPassRate * 0.3) + (1 - conflictRate) * 0.2 + min(totalStories/50, 1.0) * 0.1
pub fn compute_trust(agent_type: &str, domain: &str) -> Result<TrustScore> {
    // Fetch profile outside with_db to avoid nested lock
    let profile = learning_repo::fetch_profile(agent_type, domain)?;

    with_db(|conn| {
        let now = chrono::Utc::now().to_rfc3339();

        let (success_rate, conflict_rate, total_executions, avg_test_pass_rate) = match &profile {
            Some(p) => (p.success_rate, p.conflict_rate, p.total_executions, p.avg_test_pass_rate),
            None => (0.5, 0.0, 0, 1.0),
        };

        // Compute aggregate test stats from learning records that match this agent+domain category
        let (total_passed, total_failed, successful_count): (i32, i32, i32) = conn.query_row(
            "SELECT COALESCE(SUM(testsPassed),0), COALESCE(SUM(testsFailed),0), COALESCE(SUM(CASE WHEN success=1 THEN 1 ELSE 0 END),0) FROM learning_record WHERE agentType=?1",
            params![agent_type],
            |row| Ok((row.get(0)?, row.get(1)?, row.get(2)?)),
        ).unwrap_or((0, 0, 0));

        let test_pass_rate = if total_passed + total_failed > 0 {
            total_passed as f64 / (total_passed + total_failed) as f64
        } else {
            avg_test_pass_rate
        };

        let experience_factor = (total_executions as f64 / 50.0).min(1.0);

        let score = (success_rate * 0.4)
            + (test_pass_rate * 0.3)
            + ((1.0 - conflict_rate) * 0.2)
            + (experience_factor * 0.1);

        // Clamp score to [0, 1]
        let score = score.max(0.0).min(1.0);

        // Upsert trust_score
        let existing: Option<TrustScore> = conn.query_row(
            &format!("SELECT {} FROM trust_score WHERE agentType=?1 AND domain=?2", TS_COLS),
            params![agent_type, domain],
            row_to_trust,
        ).ok();

        match existing {
            Some(ts) => {
                conn.execute(
                    "UPDATE trust_score SET score=?1, totalStories=?2, successfulStories=?3, totalTestsPassed=?4, totalTestsFailed=?5, lastComputedAt=?6 WHERE id=?7",
                    params![score, total_executions, successful_count, total_passed, total_failed, now, ts.id],
                )?;
                Ok(TrustScore {
                    score,
                    total_stories: total_executions,
                    successful_stories: successful_count,
                    total_tests_passed: total_passed,
                    total_tests_failed: total_failed,
                    last_computed_at: now,
                    ..ts
                })
            }
            None => {
                let id = Uuid::new_v4().to_string();
                conn.execute(
                    "INSERT INTO trust_score (id,agentType,domain,score,totalStories,successfulStories,totalTestsPassed,totalTestsFailed,lastComputedAt) VALUES (?1,?2,?3,?4,?5,?6,?7,?8,?9)",
                    params![id, agent_type, domain, score, total_executions, successful_count, total_passed, total_failed, now],
                )?;
                Ok(TrustScore {
                    id,
                    agent_type: agent_type.to_string(),
                    domain: domain.to_string(),
                    score,
                    total_stories: total_executions,
                    successful_stories: successful_count,
                    total_tests_passed: total_passed,
                    total_tests_failed: total_failed,
                    auto_merge_enabled: false,
                    auto_merge_threshold: 0.9,
                    last_computed_at: now,
                })
            }
        }
    })
}

/// Fetch an existing trust score for an agent+domain.
pub fn fetch_trust(agent_type: &str, domain: &str) -> Result<Option<TrustScore>> {
    with_db(|conn| {
        Ok(conn.query_row(
            &format!("SELECT {} FROM trust_score WHERE agentType=?1 AND domain=?2", TS_COLS),
            params![agent_type, domain],
            row_to_trust,
        ).ok())
    })
}

/// Fetch all trust scores.
pub fn fetch_all_trust() -> Result<Vec<TrustScore>> {
    with_db(|conn| {
        let mut stmt = conn.prepare(
            &format!("SELECT {} FROM trust_score ORDER BY agentType, domain", TS_COLS),
        )?;
        let rows = stmt.query_map([], row_to_trust)?;
        rows.collect()
    })
}

/// Update auto-merge policy for an agent+domain.
pub fn update_auto_merge(agent_type: &str, domain: &str, enabled: bool, threshold: f64) -> Result<()> {
    with_db(|conn| {
        let enabled_int = if enabled { 1 } else { 0 };
        let updated = conn.execute(
            "UPDATE trust_score SET autoMergeEnabled=?1, autoMergeThreshold=?2 WHERE agentType=?3 AND domain=?4",
            params![enabled_int, threshold, agent_type, domain],
        )?;
        if updated == 0 {
            // Create a trust score first if it doesn't exist, then update
            let id = Uuid::new_v4().to_string();
            let now = chrono::Utc::now().to_rfc3339();
            conn.execute(
                "INSERT INTO trust_score (id,agentType,domain,score,totalStories,successfulStories,totalTestsPassed,totalTestsFailed,autoMergeEnabled,autoMergeThreshold,lastComputedAt) VALUES (?1,?2,?3,0.5,0,0,0,0,?4,?5,?6)",
                params![id, agent_type, domain, enabled_int, threshold, now],
            )?;
        }
        Ok(())
    })
}

/// Check if an agent+domain should auto-merge based on trust score and policy.
pub fn should_auto_merge(agent_type: &str, domain: &str) -> Result<bool> {
    with_db(|conn| {
        let result: Option<(f64, bool, f64)> = conn.query_row(
            "SELECT score, autoMergeEnabled, autoMergeThreshold FROM trust_score WHERE agentType=?1 AND domain=?2",
            params![agent_type, domain],
            |row| {
                let enabled: i32 = row.get(1)?;
                Ok((row.get(0)?, enabled != 0, row.get(2)?))
            },
        ).ok();

        match result {
            Some((score, enabled, threshold)) => Ok(enabled && score >= threshold),
            None => Ok(false),
        }
    })
}

/// Recompute trust scores for all known agent+domain combinations.
pub fn refresh_all_trust() -> Result<Vec<TrustScore>> {
    // Get all unique agent types from performance profiles
    let profiles = learning_repo::fetch_all_profiles()?;
    let mut results = Vec::new();

    for profile in &profiles {
        let ts = compute_trust(&profile.agent_type, &profile.task_category)?;
        results.push(ts);
    }

    Ok(results)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::db::{manager::initialize_memory, session_repo, learning_repo};

    #[test]
    fn test_trust_computation() {
        initialize_memory().unwrap();
        let session = session_repo::create_session("/tmp/trust-test").unwrap();

        // Record some learning data
        let lr1 = learning_repo::record_learning(
            &session.id, "US-001", "Build API", "moderate", "claude",
            5000, 150, 3, 100, 20, 10, 9, 0, true, "[\"src/api\"]",
        ).unwrap();
        learning_repo::update_performance_profile("claude", "backend", &lr1).unwrap();

        let lr2 = learning_repo::record_learning(
            &session.id, "US-002", "Fix endpoint", "simple", "claude",
            3000, 80, 1, 20, 5, 5, 5, 0, true, "[\"src/api\"]",
        ).unwrap();
        learning_repo::update_performance_profile("claude", "backend", &lr2).unwrap();

        // Compute trust
        let trust = compute_trust("claude", "backend").unwrap();
        assert_eq!(trust.agent_type, "claude");
        assert_eq!(trust.domain, "backend");
        assert!(trust.score > 0.0 && trust.score <= 1.0);
        assert_eq!(trust.total_stories, 2);
        assert_eq!(trust.successful_stories, 2);
        assert!(!trust.auto_merge_enabled);

        // Fetch trust
        let fetched = fetch_trust("claude", "backend").unwrap().unwrap();
        assert_eq!(fetched.id, trust.id);
        assert_eq!(fetched.score, trust.score);

        // Fetch all
        let all = fetch_all_trust().unwrap();
        assert_eq!(all.len(), 1);

        // Non-existent
        let none = fetch_trust("gemini", "backend").unwrap();
        assert!(none.is_none());
    }

    #[test]
    fn test_auto_merge_policy() {
        initialize_memory().unwrap();
        let session = session_repo::create_session("/tmp/merge-test").unwrap();

        // Record data with high success
        let lr = learning_repo::record_learning(
            &session.id, "US-001", "API work", "moderate", "claude",
            5000, 150, 3, 100, 20, 10, 10, 0, true, "[\"src/api\"]",
        ).unwrap();
        learning_repo::update_performance_profile("claude", "backend", &lr).unwrap();

        // Compute trust first
        let trust = compute_trust("claude", "backend").unwrap();

        // Auto-merge should be false by default
        assert!(!should_auto_merge("claude", "backend").unwrap());

        // Enable auto-merge with a threshold the score meets
        update_auto_merge("claude", "backend", true, trust.score - 0.01).unwrap();
        assert!(should_auto_merge("claude", "backend").unwrap());

        // Set threshold above current score
        update_auto_merge("claude", "backend", true, 0.99).unwrap();
        assert!(!should_auto_merge("claude", "backend").unwrap());

        // Disable auto-merge
        update_auto_merge("claude", "backend", false, 0.0).unwrap();
        assert!(!should_auto_merge("claude", "backend").unwrap());

        // Non-existent agent — should be false
        assert!(!should_auto_merge("unknown", "unknown").unwrap());

        // Refresh all trust
        let all = refresh_all_trust().unwrap();
        assert!(!all.is_empty());
    }
}
