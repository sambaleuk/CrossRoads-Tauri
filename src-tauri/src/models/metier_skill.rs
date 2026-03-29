use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct MetierSkill {
    pub id: String,
    pub name: String,
    pub family: String,
    pub skill_md_path: String,
    pub required_mcps: Option<String>,
    pub description: Option<String>,
    pub created_at: String,
}
