use rusqlite::{params, Result};
use uuid::Uuid;
use crate::db::manager::with_db;
use crate::models::agent_slot::AgentSlot;

pub fn create_slot(session_id: &str, slot_index: i32, agent_type: &str, skill_id: Option<&str>, branch_name: Option<&str>) -> Result<AgentSlot> {
    with_db(|conn| {
        let id = Uuid::new_v4().to_string();
        let now = chrono::Utc::now().to_rfc3339();
        conn.execute(
            "INSERT INTO agent_slot (id, cockpitSessionId, slotIndex, status, agentType, skillId, branchName, createdAt, updatedAt) VALUES (?1,?2,?3,'empty',?4,?5,?6,?7,?7)",
            params![id, session_id, slot_index, agent_type, skill_id, branch_name, now],
        )?;
        Ok(AgentSlot {
            id, cockpit_session_id: session_id.to_string(), slot_index, status: "empty".to_string(),
            agent_type: agent_type.to_string(), worktree_path: None, branch_name: branch_name.map(String::from),
            skill_id: skill_id.map(String::from), current_task: None, claude_session_id: None,
            created_at: now.clone(), updated_at: now,
        })
    })
}

pub fn update_slot(id: &str, status: &str, current_task: Option<&str>) -> Result<()> {
    with_db(|conn| {
        let now = chrono::Utc::now().to_rfc3339();
        conn.execute(
            "UPDATE agent_slot SET status=?1, currentTask=?2, updatedAt=?3 WHERE id=?4",
            params![status, current_task, now, id],
        )?;
        Ok(())
    })
}

pub fn fetch_slots_for_session(session_id: &str) -> Result<Vec<AgentSlot>> {
    with_db(|conn| {
        let mut stmt = conn.prepare(
            "SELECT id,cockpitSessionId,slotIndex,status,agentType,worktreePath,branchName,skillId,currentTask,claudeSessionId,createdAt,updatedAt FROM agent_slot WHERE cockpitSessionId=?1 ORDER BY slotIndex"
        )?;
        let rows = stmt.query_map(params![session_id], |row| {
            Ok(AgentSlot {
                id: row.get(0)?, cockpit_session_id: row.get(1)?, slot_index: row.get(2)?,
                status: row.get(3)?, agent_type: row.get(4)?, worktree_path: row.get(5)?,
                branch_name: row.get(6)?, skill_id: row.get(7)?, current_task: row.get(8)?,
                claude_session_id: row.get(9)?,
                created_at: row.get(10)?, updated_at: row.get(11)?,
            })
        })?;
        rows.collect()
    })
}

pub fn update_claude_session_id(id: &str, claude_session_id: &str) -> Result<()> {
    with_db(|conn| {
        let now = chrono::Utc::now().to_rfc3339();
        conn.execute(
            "UPDATE agent_slot SET claudeSessionId=?1, updatedAt=?2 WHERE id=?3",
            params![claude_session_id, now, id],
        )?;
        Ok(())
    })
}

pub fn delete_slot(id: &str) -> Result<()> {
    with_db(|conn| {
        conn.execute("DELETE FROM agent_slot WHERE id=?1", params![id])?;
        Ok(())
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::db::{manager::initialize_memory, session_repo};

    #[test]
    fn test_slot_crud() {
        initialize_memory().unwrap();
        let session = session_repo::create_session("/tmp/slot-test").unwrap();
        let slot = create_slot(&session.id, 0, "claude", None, Some("feat/test")).unwrap();
        assert_eq!(slot.status, "empty");

        update_slot(&slot.id, "running", Some("Implement US-001")).unwrap();
        let slots = fetch_slots_for_session(&session.id).unwrap();
        assert_eq!(slots.len(), 1);
        assert_eq!(slots[0].status, "running");

        delete_slot(&slot.id).unwrap();
        assert_eq!(fetch_slots_for_session(&session.id).unwrap().len(), 0);
    }

    #[test]
    fn test_cascade_delete_with_session() {
        initialize_memory().unwrap();
        let session = session_repo::create_session("/tmp/cascade-test").unwrap();
        create_slot(&session.id, 0, "claude", None, None).unwrap();
        create_slot(&session.id, 1, "gemini", None, None).unwrap();
        assert_eq!(fetch_slots_for_session(&session.id).unwrap().len(), 2);

        session_repo::delete_session(&session.id).unwrap();
        assert_eq!(fetch_slots_for_session(&session.id).unwrap().len(), 0);
    }
}
