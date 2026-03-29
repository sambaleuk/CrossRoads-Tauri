use rusqlite::{params, Result};
use uuid::Uuid;
use crate::db::manager::with_db;
use crate::models::agent_message::AgentMessage;

pub fn publish(content: &str, message_type: &str, from_slot_id: &str, to_slot_id: Option<&str>, is_broadcast: bool) -> Result<AgentMessage> {
    with_db(|conn| {
        let id = Uuid::new_v4().to_string();
        let now = chrono::Utc::now().to_rfc3339();
        conn.execute(
            "INSERT INTO agent_message (id,content,messageType,fromSlotId,toSlotId,isBroadcast,createdAt) VALUES (?1,?2,?3,?4,?5,?6,?7)",
            params![id, content, message_type, from_slot_id, to_slot_id, is_broadcast as i32, now],
        )?;
        Ok(AgentMessage {
            id, content: content.to_string(), message_type: message_type.to_string(),
            from_slot_id: from_slot_id.to_string(), to_slot_id: to_slot_id.map(String::from),
            is_broadcast, read_at: None, created_at: now,
        })
    })
}

pub fn fetch_for_slot(slot_id: &str) -> Result<Vec<AgentMessage>> {
    with_db(|conn| {
        let mut stmt = conn.prepare(
            "SELECT id,content,messageType,fromSlotId,toSlotId,isBroadcast,readAt,createdAt FROM agent_message WHERE fromSlotId=?1 OR toSlotId=?1 OR isBroadcast=1 ORDER BY createdAt"
        )?;
        let rows = stmt.query_map(params![slot_id], |row| {
            Ok(AgentMessage {
                id: row.get(0)?, content: row.get(1)?, message_type: row.get(2)?,
                from_slot_id: row.get(3)?, to_slot_id: row.get(4)?,
                is_broadcast: row.get::<_, i32>(5)? != 0, read_at: row.get(6)?, created_at: row.get(7)?,
            })
        })?;
        rows.collect()
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::db::{manager::initialize_memory, session_repo, slot_repo};

    #[test]
    fn test_publish_and_fetch() {
        initialize_memory().unwrap();
        let session = session_repo::create_session("/tmp/msg").unwrap();
        let slot = slot_repo::create_slot(&session.id, 0, "claude", None, None).unwrap();

        publish("Hello from slot 0", "status", &slot.id, None, false).unwrap();
        publish("Broadcast!", "chairman_brief", &slot.id, None, true).unwrap();

        let msgs = fetch_for_slot(&slot.id).unwrap();
        assert_eq!(msgs.len(), 2);
    }
}
