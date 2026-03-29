use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AgentSlot {
    pub id: String,
    pub cockpit_session_id: String,
    pub slot_index: i32,
    pub status: String,
    pub agent_type: String,
    pub worktree_path: Option<String>,
    pub branch_name: Option<String>,
    pub skill_id: Option<String>,
    pub current_task: Option<String>,
    pub created_at: String,
    pub updated_at: String,
}
