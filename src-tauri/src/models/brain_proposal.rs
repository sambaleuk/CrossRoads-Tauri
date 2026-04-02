use serde::{Deserialize, Serialize};

/// The type of action the brain is proposing.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum BrainProposalType {
    Launch,    // [LAUNCH:agent:role:task] — spawn a new agent slot
    Suite,     // [SUITE:id] — switch active suite
    Decision,  // [DECISION] — strategic decision requiring confirmation
    Alert,     // [ALERT] — critical alert requiring acknowledgment
}

/// Status of a brain proposal.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum BrainProposalStatus {
    Pending,
    Approved,
    Rejected,
    Modified,
    Expired,
}

/// Represents an action proposed by the cockpit brain that requires operator
/// approval before execution. Displayed in the ReviewRibbon overlay.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct BrainProposal {
    pub id: String,
    pub proposal_type: BrainProposalType,
    pub status: BrainProposalStatus,
    pub title: String,
    pub detail: String,
    pub risk_level: String, // low, medium, high, critical

    // [LAUNCH]-specific fields
    pub agent_type: Option<String>,
    pub role: Option<String>,
    pub task: Option<String>,

    // [SUITE]-specific fields
    pub suite_id: Option<String>,

    // Context from scanner/advisor
    pub scanner_excerpt: Option<String>,
    pub advisor_rationale: Option<String>,

    pub created_at: String,
    pub resolved_at: Option<String>,
    pub resolved_by: Option<String>,
}

impl BrainProposal {
    /// Create a proposal from a parsed [LAUNCH:agent:role:task] command.
    pub fn from_launch(agent: &str, role: &str, task: &str) -> Self {
        let risk = if role.contains("security") || role.contains("devops") || role.contains("deploy") {
            "high"
        } else if role.contains("debug") || role.contains("review") {
            "medium"
        } else {
            "low"
        };

        Self {
            id: uuid::Uuid::new_v4().to_string(),
            proposal_type: BrainProposalType::Launch,
            status: BrainProposalStatus::Pending,
            title: format!("Launch {} as {}", agent, role),
            detail: task.to_string(),
            risk_level: risk.to_string(),
            agent_type: Some(agent.to_string()),
            role: Some(role.to_string()),
            task: Some(task.to_string()),
            suite_id: None,
            scanner_excerpt: None,
            advisor_rationale: None,
            created_at: chrono::Utc::now().to_rfc3339(),
            resolved_at: None,
            resolved_by: None,
        }
    }

    /// Create a proposal from a [SUITE:id] command.
    pub fn from_suite_switch(suite_id: &str) -> Self {
        Self {
            id: uuid::Uuid::new_v4().to_string(),
            proposal_type: BrainProposalType::Suite,
            status: BrainProposalStatus::Pending,
            title: format!("Switch to {} suite", suite_id),
            detail: format!("Brain wants to change the active mission suite to {}.", suite_id),
            risk_level: "low".to_string(),
            agent_type: None,
            role: None,
            task: None,
            suite_id: Some(suite_id.to_string()),
            scanner_excerpt: None,
            advisor_rationale: None,
            created_at: chrono::Utc::now().to_rfc3339(),
            resolved_at: None,
            resolved_by: None,
        }
    }
}
