use rusqlite::{params, Result};
use uuid::Uuid;
use crate::db::manager::with_db;
use crate::models::cost_event::{CostEvent, UsageSummary, estimate_cost_cents};

pub fn record_usage(slot_id: &str, provider: &str, model: &str, input_tokens: i64, output_tokens: i64) -> Result<CostEvent> {
    with_db(|conn| {
        let id = Uuid::new_v4().to_string();
        let now = chrono::Utc::now().to_rfc3339();
        let cost_cents = estimate_cost_cents(model, input_tokens, output_tokens);
        conn.execute(
            "INSERT INTO cost_event (id,agentSlotId,provider,model,inputTokens,outputTokens,costCents,createdAt) VALUES (?1,?2,?3,?4,?5,?6,?7,?8)",
            params![id, slot_id, provider, model, input_tokens, output_tokens, cost_cents, now],
        )?;
        Ok(CostEvent {
            id, agent_slot_id: slot_id.to_string(), provider: provider.to_string(),
            model: model.to_string(), input_tokens, output_tokens, cost_cents, created_at: now,
        })
    })
}

pub fn summary_for_slot(slot_id: &str) -> Result<UsageSummary> {
    with_db(|conn| {
        conn.query_row(
            "SELECT COALESCE(SUM(inputTokens),0), COALESCE(SUM(outputTokens),0), COALESCE(SUM(costCents),0), COUNT(*) FROM cost_event WHERE agentSlotId=?1",
            params![slot_id],
            |row| Ok(UsageSummary {
                total_input_tokens: row.get(0)?,
                total_output_tokens: row.get(1)?,
                total_cost_cents: row.get(2)?,
                event_count: row.get(3)?,
            }),
        )
    })
}

pub fn summary_for_session(session_id: &str) -> Result<UsageSummary> {
    with_db(|conn| {
        conn.query_row(
            "SELECT COALESCE(SUM(ce.inputTokens),0), COALESCE(SUM(ce.outputTokens),0), COALESCE(SUM(ce.costCents),0), COUNT(*)
             FROM cost_event ce INNER JOIN agent_slot s ON ce.agentSlotId=s.id WHERE s.cockpitSessionId=?1",
            params![session_id],
            |row| Ok(UsageSummary {
                total_input_tokens: row.get(0)?,
                total_output_tokens: row.get(1)?,
                total_cost_cents: row.get(2)?,
                event_count: row.get(3)?,
            }),
        )
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::db::{manager::initialize_memory, session_repo, slot_repo};

    #[test]
    fn test_record_and_aggregate() {
        initialize_memory().unwrap();
        let session = session_repo::create_session("/tmp/cost").unwrap();
        let slot = slot_repo::create_slot(&session.id, 0, "claude", None, None).unwrap();

        record_usage(&slot.id, "anthropic", "sonnet", 1000, 500).unwrap();
        record_usage(&slot.id, "anthropic", "sonnet", 2000, 1000).unwrap();

        let summary = summary_for_slot(&slot.id).unwrap();
        assert_eq!(summary.total_input_tokens, 3000);
        assert_eq!(summary.total_output_tokens, 1500);
        assert_eq!(summary.event_count, 2);
        assert!(summary.total_cost_cents > 0);
    }

    #[test]
    fn test_session_aggregation() {
        initialize_memory().unwrap();
        let session = session_repo::create_session("/tmp/sess-cost").unwrap();
        let s1 = slot_repo::create_slot(&session.id, 0, "claude", None, None).unwrap();
        let s2 = slot_repo::create_slot(&session.id, 1, "gemini", None, None).unwrap();

        record_usage(&s1.id, "anthropic", "opus", 5000, 2000).unwrap();
        record_usage(&s2.id, "google", "gemini-2", 3000, 1000).unwrap();

        let summary = summary_for_session(&session.id).unwrap();
        assert_eq!(summary.total_input_tokens, 8000);
        assert_eq!(summary.total_output_tokens, 3000);
        assert_eq!(summary.event_count, 2);
    }
}
