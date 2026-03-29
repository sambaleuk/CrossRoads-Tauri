use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ExecutionGate {
    pub id: String,
    pub agent_slot_id: String,
    pub status: String,
    pub operation_type: String,
    pub operation_payload: String,
    pub risk_level: String,
    pub estimated_impact: Option<String>,
    pub approved_by: Option<String>,
    pub approved_at: Option<String>,
    pub denied_reason: Option<String>,
    pub rollback_payload: Option<String>,
    pub audit_entry: Option<String>,
    pub created_at: String,
    pub updated_at: String,
}
