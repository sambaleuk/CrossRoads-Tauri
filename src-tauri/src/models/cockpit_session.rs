use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CockpitSession {
    pub id: String,
    pub project_path: String,
    pub status: String,
    pub chairman_brief: Option<String>,
    pub created_at: String,
    pub updated_at: String,
}
