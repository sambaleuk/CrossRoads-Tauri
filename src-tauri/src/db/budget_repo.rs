use rusqlite::{params, Result};
use uuid::Uuid;
use serde::{Deserialize, Serialize};
use crate::db::manager::with_db;

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct BudgetConfig {
    pub id: String,
    pub session_id: String,
    pub slot_id: Option<String>,
    pub budget_cents: i64,
    pub warning_threshold_pct: i32,
    pub hard_stop_enabled: bool,
    pub throttle_enabled: bool,
    pub daily_limit_cents: Option<i64>,
    pub per_story_limit_cents: Option<i64>,
    pub created_at: String,
    pub updated_at: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct BudgetAlert {
    pub id: String,
    pub budget_config_id: String,
    pub alert_type: String,
    pub current_spend_cents: i64,
    pub budget_cents: i64,
    pub percent_used: f64,
    pub message: String,
    pub acknowledged: bool,
    pub created_at: String,
}

fn row_to_config(row: &rusqlite::Row) -> rusqlite::Result<BudgetConfig> {
    Ok(BudgetConfig {
        id: row.get(0)?,
        session_id: row.get(1)?,
        slot_id: row.get(2)?,
        budget_cents: row.get(3)?,
        warning_threshold_pct: row.get(4)?,
        hard_stop_enabled: row.get(5)?,
        throttle_enabled: row.get(6)?,
        daily_limit_cents: row.get(7)?,
        per_story_limit_cents: row.get(8)?,
        created_at: row.get(9)?,
        updated_at: row.get(10)?,
    })
}

const CONFIG_COLS: &str = "id,sessionId,slotId,budgetCents,warningThresholdPct,hardStopEnabled,throttleEnabled,dailyLimitCents,perStoryLimitCents,createdAt,updatedAt";

fn row_to_alert(row: &rusqlite::Row) -> rusqlite::Result<BudgetAlert> {
    Ok(BudgetAlert {
        id: row.get(0)?,
        budget_config_id: row.get(1)?,
        alert_type: row.get(2)?,
        current_spend_cents: row.get(3)?,
        budget_cents: row.get(4)?,
        percent_used: row.get(5)?,
        message: row.get(6)?,
        acknowledged: row.get(7)?,
        created_at: row.get(8)?,
    })
}

const ALERT_COLS: &str = "id,budgetConfigId,alertType,currentSpendCents,budgetCents,percentUsed,message,acknowledged,createdAt";

pub fn create_budget_config(
    session_id: &str,
    slot_id: Option<&str>,
    budget_cents: i64,
    warning_pct: i32,
    hard_stop: bool,
    throttle: bool,
) -> Result<BudgetConfig> {
    with_db(|conn| {
        let id = Uuid::new_v4().to_string();
        let now = chrono::Utc::now().to_rfc3339();
        conn.execute(
            "INSERT INTO budget_config (id,sessionId,slotId,budgetCents,warningThresholdPct,hardStopEnabled,throttleEnabled,createdAt,updatedAt) VALUES (?1,?2,?3,?4,?5,?6,?7,?8,?8)",
            params![id, session_id, slot_id, budget_cents, warning_pct, hard_stop, throttle, now],
        )?;
        Ok(BudgetConfig {
            id,
            session_id: session_id.to_string(),
            slot_id: slot_id.map(String::from),
            budget_cents,
            warning_threshold_pct: warning_pct,
            hard_stop_enabled: hard_stop,
            throttle_enabled: throttle,
            daily_limit_cents: None,
            per_story_limit_cents: None,
            created_at: now.clone(),
            updated_at: now,
        })
    })
}

pub fn fetch_budget_config(session_id: &str, slot_id: Option<&str>) -> Result<Option<BudgetConfig>> {
    with_db(|conn| {
        match slot_id {
            Some(sid) => {
                let sql = format!("SELECT {} FROM budget_config WHERE sessionId=?1 AND slotId=?2 LIMIT 1", CONFIG_COLS);
                Ok(conn.query_row(&sql, params![session_id, sid], row_to_config).ok())
            }
            None => {
                let sql = format!("SELECT {} FROM budget_config WHERE sessionId=?1 AND slotId IS NULL LIMIT 1", CONFIG_COLS);
                Ok(conn.query_row(&sql, params![session_id], row_to_config).ok())
            }
        }
    })
}

pub fn update_budget_config(
    id: &str,
    budget_cents: Option<i64>,
    warning_pct: Option<i32>,
    hard_stop: Option<bool>,
    throttle: Option<bool>,
) -> Result<()> {
    with_db(|conn| {
        let now = chrono::Utc::now().to_rfc3339();
        conn.execute(
            "UPDATE budget_config SET budgetCents=COALESCE(?1,budgetCents), warningThresholdPct=COALESCE(?2,warningThresholdPct), hardStopEnabled=COALESCE(?3,hardStopEnabled), throttleEnabled=COALESCE(?4,throttleEnabled), updatedAt=?5 WHERE id=?6",
            params![budget_cents, warning_pct, hard_stop, throttle, now, id],
        )?;
        Ok(())
    })
}

pub fn create_alert(
    config_id: &str,
    alert_type: &str,
    current_spend: i64,
    budget: i64,
    pct: f64,
    message: &str,
) -> Result<BudgetAlert> {
    with_db(|conn| {
        let id = Uuid::new_v4().to_string();
        let now = chrono::Utc::now().to_rfc3339();
        conn.execute(
            "INSERT INTO budget_alert (id,budgetConfigId,alertType,currentSpendCents,budgetCents,percentUsed,message,createdAt) VALUES (?1,?2,?3,?4,?5,?6,?7,?8)",
            params![id, config_id, alert_type, current_spend, budget, pct, message, now],
        )?;
        Ok(BudgetAlert {
            id,
            budget_config_id: config_id.to_string(),
            alert_type: alert_type.to_string(),
            current_spend_cents: current_spend,
            budget_cents: budget,
            percent_used: pct,
            message: message.to_string(),
            acknowledged: false,
            created_at: now,
        })
    })
}

pub fn fetch_alerts(config_id: &str) -> Result<Vec<BudgetAlert>> {
    with_db(|conn| {
        let mut stmt = conn.prepare(
            &format!("SELECT {} FROM budget_alert WHERE budgetConfigId=?1 ORDER BY createdAt DESC", ALERT_COLS),
        )?;
        let rows = stmt.query_map(params![config_id], row_to_alert)?;
        rows.collect()
    })
}

pub fn acknowledge_alert(id: &str) -> Result<()> {
    with_db(|conn| {
        conn.execute(
            "UPDATE budget_alert SET acknowledged=1 WHERE id=?1",
            params![id],
        )?;
        Ok(())
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::db::{manager::initialize_memory, session_repo};

    #[test]
    fn test_budget_lifecycle() {
        initialize_memory().unwrap();
        let session = session_repo::create_session("/tmp/budget-test").unwrap();

        // Create session-level budget config
        let config = create_budget_config(&session.id, None, 5000, 80, true, true).unwrap();
        assert_eq!(config.budget_cents, 5000);
        assert!(config.hard_stop_enabled);

        // Fetch it back
        let fetched = fetch_budget_config(&session.id, None).unwrap().unwrap();
        assert_eq!(fetched.id, config.id);

        // Update budget
        update_budget_config(&config.id, Some(10000), None, None, Some(false)).unwrap();
        let updated = fetch_budget_config(&session.id, None).unwrap().unwrap();
        assert_eq!(updated.budget_cents, 10000);
        assert!(!updated.throttle_enabled);

        // Create alert
        let alert = create_alert(&config.id, "warning", 8500, 10000, 85.0, "Budget at 85%").unwrap();
        assert_eq!(alert.alert_type, "warning");
        assert!(!alert.acknowledged);

        // Fetch alerts
        let alerts = fetch_alerts(&config.id).unwrap();
        assert_eq!(alerts.len(), 1);

        // Acknowledge
        acknowledge_alert(&alert.id).unwrap();
        let alerts2 = fetch_alerts(&config.id).unwrap();
        assert!(alerts2[0].acknowledged);
    }
}
