use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct OrchestrationRecord {
    pub id: String,
    pub started_at: String,
    pub finished_at: Option<String>,
    pub prd_name: String,
    pub result_summary: String,
    pub merged_branches: Vec<String>,
    pub conflicts: Vec<String>,
    pub total_stories: i32,
    pub completed_stories: i32,
    pub total_cost_cents: i64,
}
