use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum CockpitSessionStatus {
    Idle,
    Initializing,
    Active,
    Paused,
    Closed,
}

impl CockpitSessionStatus {
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::Idle => "idle",
            Self::Initializing => "initializing",
            Self::Active => "active",
            Self::Paused => "paused",
            Self::Closed => "closed",
        }
    }

    pub fn from_str(s: &str) -> Option<Self> {
        match s {
            "idle" => Some(Self::Idle),
            "initializing" => Some(Self::Initializing),
            "active" => Some(Self::Active),
            "paused" => Some(Self::Paused),
            "closed" => Some(Self::Closed),
            _ => None,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum AgentSlotStatus {
    Empty,
    Provisioning,
    Running,
    WaitingApproval,
    Paused,
    Done,
    Error,
}

impl AgentSlotStatus {
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::Empty => "empty",
            Self::Provisioning => "provisioning",
            Self::Running => "running",
            Self::WaitingApproval => "waiting_approval",
            Self::Paused => "paused",
            Self::Done => "done",
            Self::Error => "error",
        }
    }

    pub fn from_str(s: &str) -> Option<Self> {
        match s {
            "empty" => Some(Self::Empty),
            "provisioning" => Some(Self::Provisioning),
            "running" => Some(Self::Running),
            "waiting_approval" => Some(Self::WaitingApproval),
            "paused" => Some(Self::Paused),
            "done" => Some(Self::Done),
            "error" => Some(Self::Error),
            _ => None,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ExecutionGateStatus {
    Pending,
    DryRun,
    AwaitingApproval,
    Executing,
    Completed,
    Rejected,
    RolledBack,
}

impl ExecutionGateStatus {
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::Pending => "pending",
            Self::DryRun => "dry_run",
            Self::AwaitingApproval => "awaiting_approval",
            Self::Executing => "executing",
            Self::Completed => "completed",
            Self::Rejected => "rejected",
            Self::RolledBack => "rolled_back",
        }
    }

    pub fn from_str(s: &str) -> Option<Self> {
        match s {
            "pending" => Some(Self::Pending),
            "dry_run" => Some(Self::DryRun),
            "awaiting_approval" => Some(Self::AwaitingApproval),
            "executing" => Some(Self::Executing),
            "completed" => Some(Self::Completed),
            "rejected" => Some(Self::Rejected),
            "rolled_back" => Some(Self::RolledBack),
            _ => None,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum RiskLevel {
    Low,
    Medium,
    High,
    Critical,
}

impl RiskLevel {
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::Low => "low",
            Self::Medium => "medium",
            Self::High => "high",
            Self::Critical => "critical",
        }
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum MessageType {
    Status,
    Log,
    Error,
    Blocker,
    ChairmanBrief,
    Handoff,
}

impl MessageType {
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::Status => "status",
            Self::Log => "log",
            Self::Error => "error",
            Self::Blocker => "blocker",
            Self::ChairmanBrief => "chairman_brief",
            Self::Handoff => "handoff",
        }
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum AgentType {
    Claude,
    Gemini,
    Codex,
}

impl AgentType {
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::Claude => "claude",
            Self::Gemini => "gemini",
            Self::Codex => "codex",
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_cockpit_session_status_roundtrip() {
        let statuses = vec!["idle", "initializing", "active", "paused", "closed"];
        for s in statuses {
            let status = CockpitSessionStatus::from_str(s).unwrap();
            assert_eq!(status.as_str(), s);
        }
    }

    #[test]
    fn test_agent_slot_status_roundtrip() {
        let statuses = vec!["empty", "provisioning", "running", "waiting_approval", "paused", "done", "error"];
        for s in statuses {
            let status = AgentSlotStatus::from_str(s).unwrap();
            assert_eq!(status.as_str(), s);
        }
    }

    #[test]
    fn test_execution_gate_status_roundtrip() {
        let statuses = vec!["pending", "dry_run", "awaiting_approval", "executing", "completed", "rejected", "rolled_back"];
        for s in statuses {
            let status = ExecutionGateStatus::from_str(s).unwrap();
            assert_eq!(status.as_str(), s);
        }
    }

    #[test]
    fn test_serde_roundtrip() {
        let status = CockpitSessionStatus::Active;
        let json = serde_json::to_string(&status).unwrap();
        assert_eq!(json, "\"active\"");
        let back: CockpitSessionStatus = serde_json::from_str(&json).unwrap();
        assert_eq!(back, status);
    }
}
