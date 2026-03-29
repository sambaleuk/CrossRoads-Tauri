use rusqlite::{params, Result};
use uuid::Uuid;
use crate::db::manager::with_db;
use crate::models::execution_gate::ExecutionGate;

pub fn create_gate(slot_id: &str, operation_type: &str, payload: &str, risk_level: &str) -> Result<ExecutionGate> {
    with_db(|conn| {
        let id = Uuid::new_v4().to_string();
        let now = chrono::Utc::now().to_rfc3339();
        conn.execute(
            "INSERT INTO execution_gate (id,agentSlotId,status,operationType,operationPayload,riskLevel,createdAt,updatedAt) VALUES (?1,?2,'pending',?3,?4,?5,?6,?6)",
            params![id, slot_id, operation_type, payload, risk_level, now],
        )?;
        Ok(ExecutionGate {
            id, agent_slot_id: slot_id.to_string(), status: "pending".to_string(),
            operation_type: operation_type.to_string(), operation_payload: payload.to_string(),
            risk_level: risk_level.to_string(), estimated_impact: None, approved_by: None,
            approved_at: None, denied_reason: None, rollback_payload: None, audit_entry: None,
            created_at: now.clone(), updated_at: now,
        })
    })
}

pub fn update_gate_status(id: &str, status: &str, approved_by: Option<&str>, denied_reason: Option<&str>) -> Result<()> {
    with_db(|conn| {
        let now = chrono::Utc::now().to_rfc3339();
        let approved_at: Option<String> = if approved_by.is_some() { Some(now.clone()) } else { None };
        conn.execute(
            "UPDATE execution_gate SET status=?1, approvedBy=?2, approvedAt=?3, deniedReason=?4, updatedAt=?5 WHERE id=?6",
            params![status, approved_by, approved_at, denied_reason, now, id],
        )?;
        Ok(())
    })
}

pub fn write_audit(id: &str, audit_json: &str) -> Result<()> {
    with_db(|conn| {
        // Immutable: only write if auditEntry is NULL
        let affected = conn.execute(
            "UPDATE execution_gate SET auditEntry=?1 WHERE id=?2 AND auditEntry IS NULL",
            params![audit_json, id],
        )?;
        if affected == 0 {
            return Err(rusqlite::Error::QueryReturnedNoRows);
        }
        Ok(())
    })
}

pub fn fetch_gates_for_slot(slot_id: &str) -> Result<Vec<ExecutionGate>> {
    with_db(|conn| {
        let mut stmt = conn.prepare(
            "SELECT id,agentSlotId,status,operationType,operationPayload,riskLevel,estimatedImpact,approvedBy,approvedAt,deniedReason,rollbackPayload,auditEntry,createdAt,updatedAt FROM execution_gate WHERE agentSlotId=?1 ORDER BY createdAt"
        )?;
        let rows = stmt.query_map(params![slot_id], |row| {
            Ok(ExecutionGate {
                id: row.get(0)?, agent_slot_id: row.get(1)?, status: row.get(2)?,
                operation_type: row.get(3)?, operation_payload: row.get(4)?, risk_level: row.get(5)?,
                estimated_impact: row.get(6)?, approved_by: row.get(7)?, approved_at: row.get(8)?,
                denied_reason: row.get(9)?, rollback_payload: row.get(10)?, audit_entry: row.get(11)?,
                created_at: row.get(12)?, updated_at: row.get(13)?,
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
    fn test_gate_lifecycle() {
        initialize_memory().unwrap();
        let session = session_repo::create_session("/tmp/gate").unwrap();
        let slot = slot_repo::create_slot(&session.id, 0, "claude", None, None).unwrap();

        let gate = create_gate(&slot.id, "rm -rf", "/tmp/danger", "critical").unwrap();
        assert_eq!(gate.status, "pending");

        update_gate_status(&gate.id, "awaiting_approval", None, None).unwrap();
        update_gate_status(&gate.id, "executing", Some("human"), None).unwrap();

        let gates = fetch_gates_for_slot(&slot.id).unwrap();
        assert_eq!(gates.len(), 1);
        assert_eq!(gates[0].status, "executing");
        assert_eq!(gates[0].approved_by.as_deref(), Some("human"));
    }

    #[test]
    fn test_audit_immutability() {
        initialize_memory().unwrap();
        let session = session_repo::create_session("/tmp/audit").unwrap();
        let slot = slot_repo::create_slot(&session.id, 0, "claude", None, None).unwrap();
        let gate = create_gate(&slot.id, "deploy", "{}", "high").unwrap();

        write_audit(&gate.id, r#"{"result":"ok"}"#).unwrap();
        // Second write should fail
        assert!(write_audit(&gate.id, r#"{"result":"tampered"}"#).is_err());
    }
}
