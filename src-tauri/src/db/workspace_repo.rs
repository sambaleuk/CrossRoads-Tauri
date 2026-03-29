use rusqlite::{params, Result};
use uuid::Uuid;
use serde::{Deserialize, Serialize};
use crate::db::manager::with_db;

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Workspace {
    pub id: String,
    pub name: String,
    pub project_path: String,
    pub color: String,
    pub icon: Option<String>,
    pub is_active: bool,
    pub max_slots: i32,
    pub total_budget_cents: Option<i64>,
    pub metadata: Option<String>,
    pub last_accessed_at: String,
    pub created_at: String,
}

const COLS: &str = "id,name,projectPath,color,icon,isActive,maxSlots,totalBudgetCents,metadata,lastAccessedAt,createdAt";

fn row_to_workspace(row: &rusqlite::Row) -> rusqlite::Result<Workspace> {
    Ok(Workspace {
        id: row.get(0)?,
        name: row.get(1)?,
        project_path: row.get(2)?,
        color: row.get(3)?,
        icon: row.get(4)?,
        is_active: row.get(5)?,
        max_slots: row.get(6)?,
        total_budget_cents: row.get(7)?,
        metadata: row.get(8)?,
        last_accessed_at: row.get(9)?,
        created_at: row.get(10)?,
    })
}

pub fn create_workspace(name: &str, project_path: &str, color: Option<&str>) -> Result<Workspace> {
    with_db(|conn| {
        let id = Uuid::new_v4().to_string();
        let now = chrono::Utc::now().to_rfc3339();
        let c = color.unwrap_or("#0DF170");
        conn.execute(
            "INSERT INTO workspace (id,name,projectPath,color,lastAccessedAt,createdAt) VALUES (?1,?2,?3,?4,?5,?5)",
            params![id, name, project_path, c, now],
        )?;
        Ok(Workspace {
            id,
            name: name.to_string(),
            project_path: project_path.to_string(),
            color: c.to_string(),
            icon: None,
            is_active: false,
            max_slots: 6,
            total_budget_cents: None,
            metadata: None,
            last_accessed_at: now.clone(),
            created_at: now,
        })
    })
}

pub fn fetch_workspace(id: &str) -> Result<Option<Workspace>> {
    with_db(|conn| {
        Ok(conn.query_row(
            &format!("SELECT {} FROM workspace WHERE id=?1", COLS),
            params![id],
            row_to_workspace,
        ).ok())
    })
}

pub fn fetch_all_workspaces() -> Result<Vec<Workspace>> {
    with_db(|conn| {
        let mut stmt = conn.prepare(
            &format!("SELECT {} FROM workspace ORDER BY lastAccessedAt DESC", COLS),
        )?;
        let rows = stmt.query_map([], row_to_workspace)?;
        rows.collect()
    })
}

pub fn switch_active(id: &str) -> Result<()> {
    with_db(|conn| {
        let now = chrono::Utc::now().to_rfc3339();
        // Deactivate all
        conn.execute("UPDATE workspace SET isActive=0", [])?;
        // Activate the selected one and update last accessed
        conn.execute(
            "UPDATE workspace SET isActive=1, lastAccessedAt=?1 WHERE id=?2",
            params![now, id],
        )?;
        Ok(())
    })
}

pub fn delete_workspace(id: &str) -> Result<()> {
    with_db(|conn| {
        conn.execute("DELETE FROM workspace WHERE id=?1", params![id])?;
        Ok(())
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::db::manager::initialize_memory;

    #[test]
    fn test_workspace_crud() {
        initialize_memory().unwrap();

        let ws = create_workspace("My Project", "/tmp/proj", Some("#FF0000")).unwrap();
        assert_eq!(ws.name, "My Project");
        assert_eq!(ws.color, "#FF0000");
        assert!(!ws.is_active);

        let fetched = fetch_workspace(&ws.id).unwrap().unwrap();
        assert_eq!(fetched.project_path, "/tmp/proj");

        let all = fetch_all_workspaces().unwrap();
        assert_eq!(all.len(), 1);

        delete_workspace(&ws.id).unwrap();
        assert!(fetch_workspace(&ws.id).unwrap().is_none());
    }

    #[test]
    fn test_switch_active() {
        initialize_memory().unwrap();

        let ws1 = create_workspace("Project A", "/tmp/proj-a", None).unwrap();
        let ws2 = create_workspace("Project B", "/tmp/proj-b", None).unwrap();

        // Switch to ws1
        switch_active(&ws1.id).unwrap();
        let a = fetch_workspace(&ws1.id).unwrap().unwrap();
        let b = fetch_workspace(&ws2.id).unwrap().unwrap();
        assert!(a.is_active);
        assert!(!b.is_active);

        // Switch to ws2
        switch_active(&ws2.id).unwrap();
        let a = fetch_workspace(&ws1.id).unwrap().unwrap();
        let b = fetch_workspace(&ws2.id).unwrap().unwrap();
        assert!(!a.is_active);
        assert!(b.is_active);
    }
}
