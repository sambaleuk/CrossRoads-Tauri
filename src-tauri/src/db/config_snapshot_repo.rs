use rusqlite::{params, Result};
use uuid::Uuid;
use serde::{Deserialize, Serialize};
use crate::db::manager::with_db;

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ConfigSnapshot {
    pub id: String,
    pub session_id: Option<String>,
    pub workspace_id: Option<String>,
    pub config_type: String,
    pub version: i32,
    pub data: String,
    pub diff: Option<String>,
    pub changed_by: String,
    pub change_reason: Option<String>,
    pub created_at: String,
}

const COLS: &str = "id,sessionId,workspaceId,configType,version,data,diff,changedBy,changeReason,createdAt";

fn row_to_snapshot(row: &rusqlite::Row) -> rusqlite::Result<ConfigSnapshot> {
    Ok(ConfigSnapshot {
        id: row.get(0)?,
        session_id: row.get(1)?,
        workspace_id: row.get(2)?,
        config_type: row.get(3)?,
        version: row.get(4)?,
        data: row.get(5)?,
        diff: row.get(6)?,
        changed_by: row.get(7)?,
        change_reason: row.get(8)?,
        created_at: row.get(9)?,
    })
}

pub fn create_snapshot(
    session_id: Option<&str>,
    workspace_id: Option<&str>,
    config_type: &str,
    data_json: &str,
    changed_by: &str,
    reason: Option<&str>,
) -> Result<ConfigSnapshot> {
    with_db(|conn| {
        let id = Uuid::new_v4().to_string();
        let now = chrono::Utc::now().to_rfc3339();

        // Auto-increment version: get max version for this session+type combo
        let next_version: i32 = conn.query_row(
            "SELECT COALESCE(MAX(version), 0) + 1 FROM config_snapshot WHERE sessionId IS ?1 AND configType=?2",
            params![session_id, config_type],
            |row| row.get(0),
        )?;

        conn.execute(
            "INSERT INTO config_snapshot (id,sessionId,workspaceId,configType,version,data,changedBy,changeReason,createdAt) VALUES (?1,?2,?3,?4,?5,?6,?7,?8,?9)",
            params![id, session_id, workspace_id, config_type, next_version, data_json, changed_by, reason, now],
        )?;

        Ok(ConfigSnapshot {
            id,
            session_id: session_id.map(String::from),
            workspace_id: workspace_id.map(String::from),
            config_type: config_type.to_string(),
            version: next_version,
            data: data_json.to_string(),
            diff: None,
            changed_by: changed_by.to_string(),
            change_reason: reason.map(String::from),
            created_at: now,
        })
    })
}

pub fn fetch_snapshots(session_id: &str, config_type: &str) -> Result<Vec<ConfigSnapshot>> {
    with_db(|conn| {
        let mut stmt = conn.prepare(
            &format!("SELECT {} FROM config_snapshot WHERE sessionId=?1 AND configType=?2 ORDER BY version DESC", COLS),
        )?;
        let rows = stmt.query_map(params![session_id, config_type], row_to_snapshot)?;
        rows.collect()
    })
}

pub fn fetch_snapshot_by_version(session_id: &str, config_type: &str, version: i32) -> Result<Option<ConfigSnapshot>> {
    with_db(|conn| {
        Ok(conn.query_row(
            &format!("SELECT {} FROM config_snapshot WHERE sessionId=?1 AND configType=?2 AND version=?3", COLS),
            params![session_id, config_type, version],
            row_to_snapshot,
        ).ok())
    })
}

pub fn get_latest(session_id: &str, config_type: &str) -> Result<Option<ConfigSnapshot>> {
    with_db(|conn| {
        Ok(conn.query_row(
            &format!("SELECT {} FROM config_snapshot WHERE sessionId=?1 AND configType=?2 ORDER BY version DESC LIMIT 1", COLS),
            params![session_id, config_type],
            row_to_snapshot,
        ).ok())
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::db::{manager::initialize_memory, session_repo};

    #[test]
    fn test_snapshot_versioning() {
        initialize_memory().unwrap();
        let session = session_repo::create_session("/tmp/snap-test").unwrap();

        // Create first snapshot
        let s1 = create_snapshot(
            Some(&session.id), None, "roles",
            "{\"roles\":[\"ceo\"]}", "user", Some("Initial config"),
        ).unwrap();
        assert_eq!(s1.version, 1);

        // Create second snapshot — version auto-increments
        let s2 = create_snapshot(
            Some(&session.id), None, "roles",
            "{\"roles\":[\"ceo\",\"lead\"]}", "user", Some("Added lead"),
        ).unwrap();
        assert_eq!(s2.version, 2);

        // Create third snapshot
        let s3 = create_snapshot(
            Some(&session.id), None, "roles",
            "{\"roles\":[\"ceo\",\"lead\",\"eng\"]}", "system", None,
        ).unwrap();
        assert_eq!(s3.version, 3);

        // Different config_type starts at 1
        let budget_snap = create_snapshot(
            Some(&session.id), None, "budget",
            "{\"budget\":5000}", "user", None,
        ).unwrap();
        assert_eq!(budget_snap.version, 1);

        // Fetch all snapshots for roles
        let all = fetch_snapshots(&session.id, "roles").unwrap();
        assert_eq!(all.len(), 3);
        assert_eq!(all[0].version, 3); // DESC order

        // Fetch by version
        let v2 = fetch_snapshot_by_version(&session.id, "roles", 2).unwrap().unwrap();
        assert_eq!(v2.id, s2.id);

        // Get latest
        let latest = get_latest(&session.id, "roles").unwrap().unwrap();
        assert_eq!(latest.version, 3);
        assert_eq!(latest.id, s3.id);

        // Non-existent version
        assert!(fetch_snapshot_by_version(&session.id, "roles", 99).unwrap().is_none());
    }
}
