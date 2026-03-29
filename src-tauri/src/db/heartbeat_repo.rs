use rusqlite::{params, Result};
use uuid::Uuid;
use serde::{Deserialize, Serialize};
use crate::db::manager::with_db;

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct HeartbeatConfig {
    pub id: String,
    pub session_id: String,
    pub slot_id: Option<String>,
    pub interval_ms: i64,
    pub enabled: bool,
    pub last_pulse_at: Option<String>,
    pub last_pulse_result: Option<String>,
    pub consecutive_failures: i32,
    pub max_failures: i32,
    pub created_at: String,
    pub updated_at: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ScheduledRun {
    pub id: String,
    pub project_path: String,
    pub prd_path: String,
    pub cron_expression: Option<String>,
    pub trigger_type: String,
    pub trigger_config: Option<String>,
    pub role_template_name: Option<String>,
    pub budget_preset: Option<String>,
    pub enabled: bool,
    pub last_run_at: Option<String>,
    pub last_run_result: Option<String>,
    pub next_run_at: Option<String>,
    pub created_at: String,
}

const HB_COLS: &str = "id,sessionId,slotId,intervalMs,enabled,lastPulseAt,lastPulseResult,consecutiveFailures,maxFailures,createdAt,updatedAt";
const SR_COLS: &str = "id,projectPath,prdPath,cronExpression,triggerType,triggerConfig,roleTemplateName,budgetPreset,enabled,lastRunAt,lastRunResult,nextRunAt,createdAt";

fn row_to_heartbeat(row: &rusqlite::Row) -> rusqlite::Result<HeartbeatConfig> {
    Ok(HeartbeatConfig {
        id: row.get(0)?,
        session_id: row.get(1)?,
        slot_id: row.get(2)?,
        interval_ms: row.get(3)?,
        enabled: row.get(4)?,
        last_pulse_at: row.get(5)?,
        last_pulse_result: row.get(6)?,
        consecutive_failures: row.get(7)?,
        max_failures: row.get(8)?,
        created_at: row.get(9)?,
        updated_at: row.get(10)?,
    })
}

fn row_to_scheduled_run(row: &rusqlite::Row) -> rusqlite::Result<ScheduledRun> {
    Ok(ScheduledRun {
        id: row.get(0)?,
        project_path: row.get(1)?,
        prd_path: row.get(2)?,
        cron_expression: row.get(3)?,
        trigger_type: row.get(4)?,
        trigger_config: row.get(5)?,
        role_template_name: row.get(6)?,
        budget_preset: row.get(7)?,
        enabled: row.get(8)?,
        last_run_at: row.get(9)?,
        last_run_result: row.get(10)?,
        next_run_at: row.get(11)?,
        created_at: row.get(12)?,
    })
}

pub fn create_heartbeat(session_id: &str, slot_id: Option<&str>, interval_ms: i64) -> Result<HeartbeatConfig> {
    with_db(|conn| {
        let id = Uuid::new_v4().to_string();
        let now = chrono::Utc::now().to_rfc3339();
        conn.execute(
            "INSERT INTO heartbeat_config (id,sessionId,slotId,intervalMs,createdAt,updatedAt) VALUES (?1,?2,?3,?4,?5,?5)",
            params![id, session_id, slot_id, interval_ms, now],
        )?;
        Ok(HeartbeatConfig {
            id,
            session_id: session_id.to_string(),
            slot_id: slot_id.map(String::from),
            interval_ms,
            enabled: true,
            last_pulse_at: None,
            last_pulse_result: None,
            consecutive_failures: 0,
            max_failures: 5,
            created_at: now.clone(),
            updated_at: now,
        })
    })
}

pub fn update_pulse(id: &str, result_json: &str) -> Result<()> {
    with_db(|conn| {
        let now = chrono::Utc::now().to_rfc3339();
        conn.execute(
            "UPDATE heartbeat_config SET lastPulseAt=?1, lastPulseResult=?2, consecutiveFailures=0, updatedAt=?1 WHERE id=?3",
            params![now, result_json, id],
        )?;
        Ok(())
    })
}

pub fn record_failure(id: &str) -> Result<()> {
    with_db(|conn| {
        let now = chrono::Utc::now().to_rfc3339();
        conn.execute(
            "UPDATE heartbeat_config SET consecutiveFailures=consecutiveFailures+1, updatedAt=?1 WHERE id=?2",
            params![now, id],
        )?;
        Ok(())
    })
}

pub fn create_scheduled_run(
    project_path: &str,
    prd_path: &str,
    cron: Option<&str>,
    trigger_type: &str,
) -> Result<ScheduledRun> {
    with_db(|conn| {
        let id = Uuid::new_v4().to_string();
        let now = chrono::Utc::now().to_rfc3339();
        conn.execute(
            "INSERT INTO scheduled_run (id,projectPath,prdPath,cronExpression,triggerType,createdAt) VALUES (?1,?2,?3,?4,?5,?6)",
            params![id, project_path, prd_path, cron, trigger_type, now],
        )?;
        Ok(ScheduledRun {
            id,
            project_path: project_path.to_string(),
            prd_path: prd_path.to_string(),
            cron_expression: cron.map(String::from),
            trigger_type: trigger_type.to_string(),
            trigger_config: None,
            role_template_name: None,
            budget_preset: None,
            enabled: true,
            last_run_at: None,
            last_run_result: None,
            next_run_at: None,
            created_at: now,
        })
    })
}

pub fn fetch_due_runs() -> Result<Vec<ScheduledRun>> {
    with_db(|conn| {
        let now = chrono::Utc::now().to_rfc3339();
        let mut stmt = conn.prepare(
            &format!("SELECT {} FROM scheduled_run WHERE enabled=1 AND nextRunAt IS NOT NULL AND nextRunAt<=?1", SR_COLS),
        )?;
        let rows = stmt.query_map(params![now], row_to_scheduled_run)?;
        rows.collect()
    })
}

pub fn update_run_result(id: &str, result: &str, next_run_at: Option<&str>) -> Result<()> {
    with_db(|conn| {
        let now = chrono::Utc::now().to_rfc3339();
        conn.execute(
            "UPDATE scheduled_run SET lastRunAt=?1, lastRunResult=?2, nextRunAt=?3 WHERE id=?4",
            params![now, result, next_run_at, id],
        )?;
        Ok(())
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::db::{manager::initialize_memory, session_repo};

    #[test]
    fn test_heartbeat_pulse() {
        initialize_memory().unwrap();
        let session = session_repo::create_session("/tmp/hb-test").unwrap();

        let hb = create_heartbeat(&session.id, None, 30000).unwrap();
        assert_eq!(hb.interval_ms, 30000);
        assert_eq!(hb.consecutive_failures, 0);
        assert!(hb.last_pulse_at.is_none());

        // Successful pulse
        update_pulse(&hb.id, "{\"status\":\"ok\"}").unwrap();
        let updated: HeartbeatConfig = with_db(|conn| {
            conn.query_row(
                &format!("SELECT {} FROM heartbeat_config WHERE id=?1", HB_COLS),
                params![hb.id],
                row_to_heartbeat,
            )
        }).unwrap();
        assert!(updated.last_pulse_at.is_some());
        assert_eq!(updated.consecutive_failures, 0);

        // Record failures
        record_failure(&hb.id).unwrap();
        record_failure(&hb.id).unwrap();
        let failed: HeartbeatConfig = with_db(|conn| {
            conn.query_row(
                &format!("SELECT {} FROM heartbeat_config WHERE id=?1", HB_COLS),
                params![hb.id],
                row_to_heartbeat,
            )
        }).unwrap();
        assert_eq!(failed.consecutive_failures, 2);

        // Successful pulse resets failures
        update_pulse(&hb.id, "{\"status\":\"recovered\"}").unwrap();
        let recovered: HeartbeatConfig = with_db(|conn| {
            conn.query_row(
                &format!("SELECT {} FROM heartbeat_config WHERE id=?1", HB_COLS),
                params![hb.id],
                row_to_heartbeat,
            )
        }).unwrap();
        assert_eq!(recovered.consecutive_failures, 0);
    }

    #[test]
    fn test_scheduled_run() {
        initialize_memory().unwrap();

        let run = create_scheduled_run("/tmp/proj", "/tmp/prd.json", Some("0 */6 * * *"), "cron").unwrap();
        assert_eq!(run.trigger_type, "cron");
        assert!(run.enabled);

        // No due runs yet (nextRunAt is NULL)
        let due = fetch_due_runs().unwrap();
        assert_eq!(due.len(), 0);

        // Set nextRunAt in the past
        with_db(|conn| {
            conn.execute(
                "UPDATE scheduled_run SET nextRunAt='2020-01-01T00:00:00+00:00' WHERE id=?1",
                params![run.id],
            )
        }).unwrap();

        let due2 = fetch_due_runs().unwrap();
        assert_eq!(due2.len(), 1);
        assert_eq!(due2[0].id, run.id);

        // Update result
        update_run_result(&run.id, "success", Some("2099-01-01T00:00:00+00:00")).unwrap();
        let due3 = fetch_due_runs().unwrap();
        assert_eq!(due3.len(), 0); // next run is in the future
    }
}
