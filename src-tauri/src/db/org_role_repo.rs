use rusqlite::{params, Result};
use uuid::Uuid;
use serde::{Deserialize, Serialize};
use crate::db::manager::with_db;

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct OrgRole {
    pub id: String,
    pub session_id: String,
    pub name: String,
    pub role_type: String,
    pub parent_role_id: Option<String>,
    pub assigned_slot_id: Option<String>,
    pub goal_description: Option<String>,
    pub skill_names: Option<String>,
    pub authority: String,
    pub created_at: String,
    pub updated_at: String,
}

const SELECT_COLS: &str = "id,sessionId,name,roleType,parentRoleId,assignedSlotId,goalDescription,skillNames,authority,createdAt,updatedAt";

fn row_to_role(row: &rusqlite::Row) -> rusqlite::Result<OrgRole> {
    Ok(OrgRole {
        id: row.get(0)?,
        session_id: row.get(1)?,
        name: row.get(2)?,
        role_type: row.get(3)?,
        parent_role_id: row.get(4)?,
        assigned_slot_id: row.get(5)?,
        goal_description: row.get(6)?,
        skill_names: row.get(7)?,
        authority: row.get(8)?,
        created_at: row.get(9)?,
        updated_at: row.get(10)?,
    })
}

pub fn create_role(
    session_id: &str,
    name: &str,
    role_type: &str,
    parent_role_id: Option<&str>,
    goal: Option<&str>,
    skill_names_json: Option<&str>,
    authority: &str,
) -> Result<OrgRole> {
    with_db(|conn| {
        let id = Uuid::new_v4().to_string();
        let now = chrono::Utc::now().to_rfc3339();
        conn.execute(
            "INSERT INTO org_role (id,sessionId,name,roleType,parentRoleId,goalDescription,skillNames,authority,createdAt,updatedAt) VALUES (?1,?2,?3,?4,?5,?6,?7,?8,?9,?9)",
            params![id, session_id, name, role_type, parent_role_id, goal, skill_names_json, authority, now],
        )?;
        Ok(OrgRole {
            id,
            session_id: session_id.to_string(),
            name: name.to_string(),
            role_type: role_type.to_string(),
            parent_role_id: parent_role_id.map(String::from),
            assigned_slot_id: None,
            goal_description: goal.map(String::from),
            skill_names: skill_names_json.map(String::from),
            authority: authority.to_string(),
            created_at: now.clone(),
            updated_at: now,
        })
    })
}

pub fn fetch_role(id: &str) -> Result<Option<OrgRole>> {
    with_db(|conn| {
        Ok(conn.query_row(
            &format!("SELECT {} FROM org_role WHERE id=?1", SELECT_COLS),
            params![id],
            row_to_role,
        ).ok())
    })
}

pub fn fetch_roles_for_session(session_id: &str) -> Result<Vec<OrgRole>> {
    with_db(|conn| {
        let mut stmt = conn.prepare(
            &format!("SELECT {} FROM org_role WHERE sessionId=?1 ORDER BY createdAt", SELECT_COLS),
        )?;
        let rows = stmt.query_map(params![session_id], row_to_role)?;
        rows.collect()
    })
}

pub fn fetch_children(parent_id: &str) -> Result<Vec<OrgRole>> {
    with_db(|conn| {
        let mut stmt = conn.prepare(
            &format!("SELECT {} FROM org_role WHERE parentRoleId=?1 ORDER BY createdAt", SELECT_COLS),
        )?;
        let rows = stmt.query_map(params![parent_id], row_to_role)?;
        rows.collect()
    })
}

pub fn update_role(
    id: &str,
    goal: Option<&str>,
    assigned_slot_id: Option<&str>,
    authority: Option<&str>,
) -> Result<()> {
    with_db(|conn| {
        let now = chrono::Utc::now().to_rfc3339();
        conn.execute(
            "UPDATE org_role SET goalDescription=COALESCE(?1,goalDescription), assignedSlotId=COALESCE(?2,assignedSlotId), authority=COALESCE(?3,authority), updatedAt=?4 WHERE id=?5",
            params![goal, assigned_slot_id, authority, now, id],
        )?;
        Ok(())
    })
}

pub fn delete_role(id: &str) -> Result<()> {
    with_db(|conn| {
        conn.execute("DELETE FROM org_role WHERE id=?1", params![id])?;
        Ok(())
    })
}

pub fn find_ceo(session_id: &str) -> Result<Option<OrgRole>> {
    with_db(|conn| {
        Ok(conn.query_row(
            &format!("SELECT {} FROM org_role WHERE sessionId=?1 AND roleType='ceo' LIMIT 1", SELECT_COLS),
            params![session_id],
            row_to_role,
        ).ok())
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::db::{manager::initialize_memory, session_repo};

    #[test]
    fn test_org_role_hierarchy() {
        initialize_memory().unwrap();
        let session = session_repo::create_session("/tmp/org-role-test").unwrap();

        // Create CEO
        let ceo = create_role(
            &session.id, "Chairman", "ceo", None,
            Some("Orchestrate the team"), Some("[\"leadership\"]"), "full",
        ).unwrap();
        assert_eq!(ceo.role_type, "ceo");
        assert!(ceo.parent_role_id.is_none());

        // Create Lead under CEO
        let lead = create_role(
            &session.id, "Tech Lead", "lead", Some(&ceo.id),
            Some("Guide engineers"), None, "elevated",
        ).unwrap();
        assert_eq!(lead.parent_role_id.as_deref(), Some(ceo.id.as_str()));

        // Create Engineer under Lead
        let eng = create_role(
            &session.id, "Engineer", "engineer", Some(&lead.id),
            Some("Implement features"), Some("[\"coding\"]"), "limited",
        ).unwrap();

        // Verify tree queries
        let ceo_children = fetch_children(&ceo.id).unwrap();
        assert_eq!(ceo_children.len(), 1);
        assert_eq!(ceo_children[0].id, lead.id);

        let lead_children = fetch_children(&lead.id).unwrap();
        assert_eq!(lead_children.len(), 1);
        assert_eq!(lead_children[0].id, eng.id);

        // Find CEO
        let found_ceo = find_ceo(&session.id).unwrap().unwrap();
        assert_eq!(found_ceo.id, ceo.id);

        // Fetch all roles for session
        let all_roles = fetch_roles_for_session(&session.id).unwrap();
        assert_eq!(all_roles.len(), 3);

        // Update role
        update_role(&eng.id, Some("Build UI"), None, Some("elevated")).unwrap();
        let updated = fetch_role(&eng.id).unwrap().unwrap();
        assert_eq!(updated.goal_description.as_deref(), Some("Build UI"));
        assert_eq!(updated.authority, "elevated");

        // Delete role
        delete_role(&eng.id).unwrap();
        assert!(fetch_role(&eng.id).unwrap().is_none());
    }
}
