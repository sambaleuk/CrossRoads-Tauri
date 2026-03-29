use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Worktree {
    pub id: String,
    pub path: String,
    pub branch: String,
    pub agent_id: Option<String>,
    pub status: String,
    pub created_at: String,
}
