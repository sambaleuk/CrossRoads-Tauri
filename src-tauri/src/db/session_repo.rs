use rusqlite::{params, Result};
use uuid::Uuid;
use crate::db::manager::with_db;
use crate::models::cockpit_session::CockpitSession;

pub fn create_session(project_path: &str) -> Result<CockpitSession> {
    with_db(|conn| {
        let id = Uuid::new_v4().to_string();
        let now = chrono::Utc::now().to_rfc3339();
        conn.execute(
            "INSERT INTO cockpit_session (id, projectPath, status, createdAt, updatedAt) VALUES (?1, ?2, 'idle', ?3, ?3)",
            params![id, project_path, now],
        )?;
        Ok(CockpitSession {
            id,
            project_path: project_path.to_string(),
            status: "idle".to_string(),
            chairman_brief: None,
            created_at: now.clone(),
            updated_at: now,
        })
    })
}

pub fn fetch_session(id: &str) -> Result<Option<CockpitSession>> {
    with_db(|conn| {
        let mut stmt = conn.prepare(
            "SELECT id, projectPath, status, chairmanBrief, createdAt, updatedAt FROM cockpit_session WHERE id = ?1"
        )?;
        let mut rows = stmt.query_map(params![id], |row| {
            Ok(CockpitSession {
                id: row.get(0)?,
                project_path: row.get(1)?,
                status: row.get(2)?,
                chairman_brief: row.get(3)?,
                created_at: row.get(4)?,
                updated_at: row.get(5)?,
            })
        })?;
        match rows.next() {
            Some(Ok(s)) => Ok(Some(s)),
            _ => Ok(None),
        }
    })
}

pub fn update_session(id: &str, status: &str, chairman_brief: Option<&str>) -> Result<()> {
    with_db(|conn| {
        let now = chrono::Utc::now().to_rfc3339();
        conn.execute(
            "UPDATE cockpit_session SET status = ?1, chairmanBrief = ?2, updatedAt = ?3 WHERE id = ?4",
            params![status, chairman_brief, now, id],
        )?;
        Ok(())
    })
}

pub fn delete_session(id: &str) -> Result<()> {
    with_db(|conn| {
        conn.execute("DELETE FROM cockpit_session WHERE id = ?1", params![id])?;
        Ok(())
    })
}

pub fn active_session_for_path(project_path: &str) -> Result<Option<CockpitSession>> {
    with_db(|conn| {
        let mut stmt = conn.prepare(
            "SELECT id, projectPath, status, chairmanBrief, createdAt, updatedAt FROM cockpit_session WHERE projectPath = ?1 AND status != 'closed' LIMIT 1"
        )?;
        let mut rows = stmt.query_map(params![project_path], |row| {
            Ok(CockpitSession {
                id: row.get(0)?,
                project_path: row.get(1)?,
                status: row.get(2)?,
                chairman_brief: row.get(3)?,
                created_at: row.get(4)?,
                updated_at: row.get(5)?,
            })
        })?;
        match rows.next() {
            Some(Ok(s)) => Ok(Some(s)),
            _ => Ok(None),
        }
    })
}

/// Close any non-closed sessions for a project path (cleanup after crash/force-quit).
/// Returns the number of sessions that were closed.
pub fn cleanup_stale_sessions(project_path: &str) -> Result<usize> {
    with_db(|conn| {
        let now = chrono::Utc::now().to_rfc3339();
        let count = conn.execute(
            "UPDATE cockpit_session SET status = 'closed', updatedAt = ?1 WHERE projectPath = ?2 AND status != 'closed'",
            params![now, project_path],
        )?;
        Ok(count)
    })
}

/// Close any non-closed sessions for a project path EXCEPT a specific session ID.
/// Used during activation to clean up stale siblings without affecting the current session.
pub fn cleanup_stale_sessions_except(project_path: &str, except_id: &str) -> Result<usize> {
    with_db(|conn| {
        let now = chrono::Utc::now().to_rfc3339();
        let count = conn.execute(
            "UPDATE cockpit_session SET status = 'closed', updatedAt = ?1 WHERE projectPath = ?2 AND status != 'closed' AND id != ?3",
            params![now, project_path, except_id],
        )?;
        Ok(count)
    })
}

pub fn fetch_all_sessions() -> Result<Vec<CockpitSession>> {
    with_db(|conn| {
        let mut stmt = conn.prepare(
            "SELECT id, projectPath, status, chairmanBrief, createdAt, updatedAt FROM cockpit_session ORDER BY createdAt DESC"
        )?;
        let rows = stmt.query_map([], |row| {
            Ok(CockpitSession {
                id: row.get(0)?,
                project_path: row.get(1)?,
                status: row.get(2)?,
                chairman_brief: row.get(3)?,
                created_at: row.get(4)?,
                updated_at: row.get(5)?,
            })
        })?;
        rows.collect()
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::db::manager::initialize_memory;

    #[test]
    fn test_session_crud_lifecycle() {
        initialize_memory().unwrap();
        let session = create_session("/tmp/test").unwrap();
        assert_eq!(session.status, "idle");

        let fetched = fetch_session(&session.id).unwrap().unwrap();
        assert_eq!(fetched.project_path, "/tmp/test");

        update_session(&session.id, "active", Some("Brief text")).unwrap();
        let updated = fetch_session(&session.id).unwrap().unwrap();
        assert_eq!(updated.status, "active");
        assert_eq!(updated.chairman_brief.as_deref(), Some("Brief text"));

        delete_session(&session.id).unwrap();
        assert!(fetch_session(&session.id).unwrap().is_none());
    }

    #[test]
    fn test_active_session_unique_per_project() {
        initialize_memory().unwrap();
        let s1 = create_session("/tmp/proj").unwrap();
        let active = active_session_for_path("/tmp/proj").unwrap();
        assert!(active.is_some());

        // Close first session, create new one
        update_session(&s1.id, "closed", None).unwrap();
        let s2 = create_session("/tmp/proj").unwrap();
        let active2 = active_session_for_path("/tmp/proj").unwrap().unwrap();
        assert_eq!(active2.id, s2.id);
    }
}
