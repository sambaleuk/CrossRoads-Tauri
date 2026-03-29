use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AgentMessage {
    pub id: String,
    pub content: String,
    pub message_type: String,
    pub from_slot_id: String,
    pub to_slot_id: Option<String>,
    pub is_broadcast: bool,
    pub read_at: Option<String>,
    pub created_at: String,
}
