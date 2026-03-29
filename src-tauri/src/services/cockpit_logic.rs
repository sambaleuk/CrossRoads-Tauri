use serde::{Deserialize, Serialize};
use crate::db::{session_repo, slot_repo, gate_repo, skill_repo};
use crate::services::{git_service, event_bus};

// ── US-001: Cockpit Lifecycle State Machine ──

/// Valid cockpit transitions from states.json
const TRANSITIONS: &[(&str, &str, &str)] = &[
    // (from, event, to)
    ("idle", "activate", "initializing"),
    ("initializing", "slots_assigned", "active"),
    ("active", "pause", "paused"),
    ("paused", "resume", "active"),
    ("active", "close", "closed"),
    ("paused", "close", "closed"),
];

/// Attempt a cockpit state transition. Returns the new status or an error.
pub fn transition(session_id: &str, event: &str) -> Result<String, String> {
    let session = session_repo::fetch_session(session_id)
        .map_err(|e| e.to_string())?
        .ok_or_else(|| format!("Session {} not found", session_id))?;

    let current = &session.status;

    // Find matching transition
    let (_, _, to) = TRANSITIONS.iter()
        .find(|(from, evt, _)| *from == current && *evt == event)
        .ok_or_else(|| format!("Invalid transition: {} -> {} (current: {})", current, event, current))?;

    // Run guards
    match event {
        "activate" => guard_has_valid_project(&session.project_path)?,
        "slots_assigned" => guard_at_least_one_slot(session_id)?,
        "close" if current == &"active" => guard_no_pending_gates(session_id)?,
        _ => {}
    }

    // Update status (preserve existing chairman_brief)
    let existing_brief = session.chairman_brief.as_deref();
    session_repo::update_session(session_id, to, existing_brief)
        .map_err(|e| e.to_string())?;

    // Emit event
    event_bus::emit_log("info", "cockpit", &format!("Session {} -> {}", event, to), None);

    Ok(to.to_string())
}

// Guards

fn guard_has_valid_project(project_path: &str) -> Result<(), String> {
    if project_path.is_empty() {
        return Err("No project path set".into());
    }
    if !git_service::is_git_repo(project_path) {
        return Err(format!("'{}' is not a git repository", project_path));
    }
    Ok(())
}

fn guard_at_least_one_slot(session_id: &str) -> Result<(), String> {
    let slots = slot_repo::fetch_slots_for_session(session_id)
        .map_err(|e| e.to_string())?;
    if slots.is_empty() {
        return Err("No slots configured for this session".into());
    }
    Ok(())
}

fn guard_no_pending_gates(session_id: &str) -> Result<(), String> {
    let slots = slot_repo::fetch_slots_for_session(session_id)
        .map_err(|e| e.to_string())?;
    for slot in &slots {
        let gates = gate_repo::fetch_gates_for_slot(&slot.id)
            .map_err(|e| e.to_string())?;
        let pending = gates.iter().any(|g| g.status == "pending" || g.status == "awaiting_approval");
        if pending {
            return Err(format!("Slot {} has pending gates — resolve them before closing", slot.slot_index));
        }
    }
    Ok(())
}

// ── US-002: Project Context Reader ──

/// Full project context for chairman deliberation
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ChairmanInput {
    pub project_path: String,
    pub current_branch: String,
    pub recent_commits: Vec<CommitSummary>,
    pub branches: Vec<String>,
    pub prd_summary: Option<String>,
    pub previous_session: Option<PreviousSessionSummary>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CommitSummary {
    pub short_sha: String,
    pub message: String,
    pub author: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PreviousSessionSummary {
    pub session_id: String,
    pub status: String,
    pub slot_count: usize,
}

/// Read project context for chairman deliberation
pub fn read_project_context(project_path: &str) -> Result<ChairmanInput, String> {
    if !git_service::is_git_repo(project_path) {
        return Err(format!("'{}' is not a git repository", project_path));
    }

    let current_branch = git_service::get_current_branch(project_path)
        .unwrap_or_else(|_| "unknown".into());

    let commits = git_service::get_recent_commits(project_path, 20)
        .unwrap_or_default()
        .into_iter()
        .map(|c| CommitSummary {
            short_sha: c.short_sha,
            message: c.message,
            author: c.author,
        })
        .collect();

    let branches = git_service::get_branches(project_path)
        .unwrap_or_default();

    // Try to find PRD summary from prd.json in project root
    let prd_summary = {
        let prd_path = std::path::Path::new(project_path).join("prd.json");
        if prd_path.exists() {
            crate::services::orchestration_engine::parse_prd(prd_path.to_str().unwrap())
                .ok()
                .map(|prd| format!("{} — {} stories, status: {}", prd.feature_name, prd.stories.len(), prd.status))
        } else {
            None
        }
    };

    // Find previous session
    let previous_session = session_repo::active_session_for_path(project_path)
        .ok()
        .flatten()
        .map(|s| {
            let slot_count = slot_repo::fetch_slots_for_session(&s.id)
                .map(|slots| slots.len())
                .unwrap_or(0);
            PreviousSessionSummary {
                session_id: s.id,
                status: s.status,
                slot_count,
            }
        });

    Ok(ChairmanInput {
        project_path: project_path.to_string(),
        current_branch,
        recent_commits: commits,
        branches,
        prd_summary,
        previous_session,
    })
}

// ── US-003: Chairman Deliberation ──

/// A slot assignment recommended by the chairman
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SlotAssignment {
    pub slot_index: u32,
    pub agent_type: String,
    pub skill_name: String,
    pub branch_name: String,
    pub task_description: String,
}

/// Chairman deliberation result
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ChairmanOutput {
    pub brief: String,
    pub assignments: Vec<SlotAssignment>,
}

/// Demo chairman: analyzes context and returns sensible slot assignments.
/// In production, this would call the cockpit-council Python module.
pub fn chairman_deliberate(input: &ChairmanInput) -> ChairmanOutput {
    let project_name = std::path::Path::new(&input.project_path)
        .file_name()
        .map(|n| n.to_string_lossy().to_string())
        .unwrap_or_else(|| "project".into());

    // Build brief
    let mut brief_parts = Vec::new();
    brief_parts.push(format!("Project: {} (branch: {})", project_name, input.current_branch));
    brief_parts.push(format!("{} branches, {} recent commits", input.branches.len(), input.recent_commits.len()));
    if let Some(ref prd) = input.prd_summary {
        brief_parts.push(format!("PRD: {}", prd));
    }
    if let Some(ref prev) = input.previous_session {
        brief_parts.push(format!("Previous session: {} ({}, {} slots)", prev.session_id, prev.status, prev.slot_count));
    }

    let brief = brief_parts.join("\n");

    // Demo assignments: 3 slots with common agent configs
    let assignments = vec![
        SlotAssignment {
            slot_index: 0,
            agent_type: "claude".into(),
            skill_name: "backend".into(),
            branch_name: format!("agent/slot-0-backend"),
            task_description: "Backend implementation and API work".into(),
        },
        SlotAssignment {
            slot_index: 1,
            agent_type: "claude".into(),
            skill_name: "frontend".into(),
            branch_name: format!("agent/slot-1-frontend"),
            task_description: "Frontend UI components and state management".into(),
        },
        SlotAssignment {
            slot_index: 2,
            agent_type: "claude".into(),
            skill_name: "testing".into(),
            branch_name: format!("agent/slot-2-testing"),
            task_description: "Test writing and quality assurance".into(),
        },
    ];

    ChairmanOutput { brief, assignments }
}

// ── US-004: Conductor Service ──

/// Execute chairman assignments: create slots, resolve skills, transition session.
pub fn conduct(session_id: &str, output: &ChairmanOutput) -> Result<(), String> {
    // 1. Store the chairman brief
    session_repo::update_session(session_id, "initializing", Some(&output.brief))
        .map_err(|e| e.to_string())?;

    // 2. Create slots from assignments
    for assignment in &output.assignments {
        // Ensure skill exists (create if missing)
        let skill_id = ensure_skill(&assignment.skill_name)?;

        let slot = slot_repo::create_slot(
            session_id,
            assignment.slot_index as i32,
            &assignment.agent_type,
            Some(&skill_id),
            Some(&assignment.branch_name),
        ).map_err(|e| e.to_string())?;

        // Update slot with task
        slot_repo::update_slot(&slot.id, "empty", Some(&assignment.task_description))
            .map_err(|e| e.to_string())?;

        event_bus::emit_agent_status(
            &slot.id, "empty", None,
            Some(&assignment.task_description),
            Some(&assignment.agent_type),
        );
    }

    // 3. Transition to active (will check guard: at_least_one_slot_configured)
    transition(session_id, "slots_assigned")?;

    event_bus::emit_log("info", "conductor",
        &format!("Session activated with {} slots", output.assignments.len()), None);

    Ok(())
}

/// Activate a session: full pipeline from idle → read context → deliberate → conduct → active
pub fn activate_session(session_id: &str) -> Result<ChairmanOutput, String> {
    // 1. Transition idle → initializing (runs has_valid_project guard)
    let session = session_repo::fetch_session(session_id)
        .map_err(|e| e.to_string())?
        .ok_or_else(|| format!("Session {} not found", session_id))?;

    transition(session_id, "activate")?;

    // 2. Read project context
    let context = read_project_context(&session.project_path)?;

    // 3. Chairman deliberation
    let output = chairman_deliberate(&context);

    // 4. Conduct (create slots, transition to active)
    conduct(session_id, &output)?;

    Ok(output)
}

/// Pause an active session
pub fn pause_session(session_id: &str) -> Result<(), String> {
    transition(session_id, "pause")?;
    // Pause all running slots
    let slots = slot_repo::fetch_slots_for_session(session_id).map_err(|e| e.to_string())?;
    for slot in &slots {
        if slot.status == "running" {
            slot_repo::update_slot(&slot.id, "paused", slot.current_task.as_deref())
                .map_err(|e| e.to_string())?;
            event_bus::emit_agent_status(&slot.id, "paused", None, slot.current_task.as_deref(), Some(&slot.agent_type));
        }
    }
    Ok(())
}

/// Resume a paused session
pub fn resume_session(session_id: &str) -> Result<(), String> {
    transition(session_id, "resume")?;
    let slots = slot_repo::fetch_slots_for_session(session_id).map_err(|e| e.to_string())?;
    for slot in &slots {
        if slot.status == "paused" {
            slot_repo::update_slot(&slot.id, "running", slot.current_task.as_deref())
                .map_err(|e| e.to_string())?;
            event_bus::emit_agent_status(&slot.id, "running", None, slot.current_task.as_deref(), Some(&slot.agent_type));
        }
    }
    Ok(())
}

/// Close a session
pub fn close_session(session_id: &str) -> Result<(), String> {
    transition(session_id, "close")?;
    let slots = slot_repo::fetch_slots_for_session(session_id).map_err(|e| e.to_string())?;
    for slot in &slots {
        if slot.status != "done" && slot.status != "error" {
            slot_repo::update_slot(&slot.id, "done", Some("Session closed"))
                .map_err(|e| e.to_string())?;
        }
    }
    Ok(())
}

// ── Helpers ──

/// Ensure a skill record exists, creating a placeholder if needed.
fn ensure_skill(skill_name: &str) -> Result<String, String> {
    if let Some(skill) = skill_repo::find_by_name(skill_name).map_err(|e| e.to_string())? {
        return Ok(skill.id);
    }
    // Create placeholder skill
    let skill = skill_repo::create_skill(
        skill_name,
        "auto",
        &format!("skills/{}.md", skill_name),
        Some(&format!("Auto-created skill: {}", skill_name)),
    ).map_err(|e| e.to_string())?;
    Ok(skill.id)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::db::manager::initialize_memory;

    fn setup_test_session(project_path: &str) -> String {
        initialize_memory().unwrap();
        let session = session_repo::create_session(project_path).unwrap();
        session.id
    }

    #[test]
    fn test_valid_transitions() {
        // Idle -> activate requires valid project (git repo), so we test the transition table directly
        assert!(TRANSITIONS.iter().any(|(f, e, _)| *f == "idle" && *e == "activate"));
        assert!(TRANSITIONS.iter().any(|(f, e, _)| *f == "initializing" && *e == "slots_assigned"));
        assert!(TRANSITIONS.iter().any(|(f, e, _)| *f == "active" && *e == "pause"));
        assert!(TRANSITIONS.iter().any(|(f, e, _)| *f == "paused" && *e == "resume"));
        assert!(TRANSITIONS.iter().any(|(f, e, _)| *f == "active" && *e == "close"));
        assert!(TRANSITIONS.iter().any(|(f, e, _)| *f == "paused" && *e == "close"));
    }

    #[test]
    fn test_invalid_transition_rejected() {
        let sid = setup_test_session("/tmp/cockpit-test");
        // Session starts idle — "pause" is invalid from idle
        let result = transition(&sid, "pause");
        assert!(result.is_err());
        assert!(result.unwrap_err().contains("Invalid transition"));
    }

    #[test]
    fn test_activate_guard_rejects_non_git() {
        let sid = setup_test_session("/tmp/not-a-git-repo-12345");
        let result = transition(&sid, "activate");
        assert!(result.is_err());
        assert!(result.unwrap_err().contains("not a git repository"));
    }

    #[test]
    fn test_slots_assigned_guard_rejects_empty() {
        let sid = setup_test_session("/tmp/cockpit-guard-test");
        // Force status to initializing to test the guard
        session_repo::update_session(&sid, "initializing", None).unwrap();
        let result = transition(&sid, "slots_assigned");
        assert!(result.is_err());
        assert!(result.unwrap_err().contains("No slots configured"));
    }

    #[test]
    fn test_slots_assigned_succeeds_with_slots() {
        let sid = setup_test_session("/tmp/cockpit-slots-test");
        session_repo::update_session(&sid, "initializing", None).unwrap();
        slot_repo::create_slot(&sid, 0, "claude", None, None).unwrap();
        let result = transition(&sid, "slots_assigned");
        assert!(result.is_ok());
        assert_eq!(result.unwrap(), "active");
    }

    #[test]
    fn test_chairman_deliberate_returns_3_slots() {
        let input = ChairmanInput {
            project_path: "/tmp/test-proj".into(),
            current_branch: "main".into(),
            recent_commits: vec![
                CommitSummary { short_sha: "abc1234".into(), message: "init".into(), author: "dev".into() },
            ],
            branches: vec!["main".into(), "feat/a".into()],
            prd_summary: Some("Test Feature — 5 stories".into()),
            previous_session: None,
        };

        let output = chairman_deliberate(&input);
        assert_eq!(output.assignments.len(), 3);
        assert!(output.brief.contains("test-proj"));
        assert!(output.brief.contains("Test Feature"));
        assert_eq!(output.assignments[0].slot_index, 0);
        assert_eq!(output.assignments[0].agent_type, "claude");
    }

    #[test]
    fn test_conduct_creates_slots() {
        let sid = setup_test_session("/tmp/conduct-test");
        session_repo::update_session(&sid, "initializing", None).unwrap();

        let output = ChairmanOutput {
            brief: "Test brief".into(),
            assignments: vec![
                SlotAssignment {
                    slot_index: 0, agent_type: "claude".into(),
                    skill_name: "backend".into(), branch_name: "agent/slot-0".into(),
                    task_description: "Backend work".into(),
                },
                SlotAssignment {
                    slot_index: 1, agent_type: "gemini".into(),
                    skill_name: "frontend".into(), branch_name: "agent/slot-1".into(),
                    task_description: "Frontend work".into(),
                },
            ],
        };

        conduct(&sid, &output).unwrap();

        let slots = slot_repo::fetch_slots_for_session(&sid).unwrap();
        assert_eq!(slots.len(), 2);
        assert_eq!(slots[0].agent_type, "claude");
        assert_eq!(slots[1].agent_type, "gemini");

        // Session should now be active
        let session = session_repo::fetch_session(&sid).unwrap().unwrap();
        assert_eq!(session.status, "active");
        assert_eq!(session.chairman_brief.unwrap(), "Test brief");
    }

    #[test]
    fn test_ensure_skill_creates_if_missing() {
        initialize_memory().unwrap();
        let id = ensure_skill("test-skill-unique").unwrap();
        assert!(!id.is_empty());
        // Second call returns same
        let id2 = ensure_skill("test-skill-unique").unwrap();
        assert_eq!(id, id2);
    }

    #[test]
    fn test_pause_and_resume_session() {
        let sid = setup_test_session("/tmp/pause-test");
        session_repo::update_session(&sid, "initializing", None).unwrap();
        slot_repo::create_slot(&sid, 0, "claude", None, None).unwrap();
        transition(&sid, "slots_assigned").unwrap(); // → active

        // Pause
        pause_session(&sid).unwrap();
        let session = session_repo::fetch_session(&sid).unwrap().unwrap();
        assert_eq!(session.status, "paused");

        // Resume
        resume_session(&sid).unwrap();
        let session = session_repo::fetch_session(&sid).unwrap().unwrap();
        assert_eq!(session.status, "active");
    }

    #[test]
    fn test_close_session() {
        let sid = setup_test_session("/tmp/close-test");
        session_repo::update_session(&sid, "initializing", None).unwrap();
        slot_repo::create_slot(&sid, 0, "claude", None, None).unwrap();
        transition(&sid, "slots_assigned").unwrap(); // → active

        close_session(&sid).unwrap();
        let session = session_repo::fetch_session(&sid).unwrap().unwrap();
        assert_eq!(session.status, "closed");
    }
}
