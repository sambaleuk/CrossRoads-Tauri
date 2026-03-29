use crate::db::{session_repo, slot_repo, cost_repo, gate_repo, message_repo, skill_repo};
use crate::services::{cli_detector, git_service};
use crate::models::{cockpit_session::CockpitSession, agent_slot::AgentSlot, cost_event::{CostEvent, UsageSummary}, execution_gate::ExecutionGate, agent_message::AgentMessage, metier_skill::MetierSkill};

// Session commands
#[tauri::command]
pub fn create_session(project_path: String) -> Result<CockpitSession, String> {
    session_repo::create_session(&project_path).map_err(|e| e.to_string())
}

#[tauri::command]
pub fn fetch_session(id: String) -> Result<Option<CockpitSession>, String> {
    session_repo::fetch_session(&id).map_err(|e| e.to_string())
}

#[tauri::command]
pub fn update_session(id: String, status: String, chairman_brief: Option<String>) -> Result<(), String> {
    session_repo::update_session(&id, &status, chairman_brief.as_deref()).map_err(|e| e.to_string())
}

#[tauri::command]
pub fn delete_session(id: String) -> Result<(), String> {
    session_repo::delete_session(&id).map_err(|e| e.to_string())
}

#[tauri::command]
pub fn active_session(project_path: String) -> Result<Option<CockpitSession>, String> {
    session_repo::active_session_for_path(&project_path).map_err(|e| e.to_string())
}

#[tauri::command]
pub fn fetch_all_sessions() -> Result<Vec<CockpitSession>, String> {
    session_repo::fetch_all_sessions().map_err(|e| e.to_string())
}

// Slot commands
#[tauri::command]
pub fn create_slot(session_id: String, slot_index: i32, agent_type: String, skill_id: Option<String>, branch_name: Option<String>) -> Result<AgentSlot, String> {
    slot_repo::create_slot(&session_id, slot_index, &agent_type, skill_id.as_deref(), branch_name.as_deref()).map_err(|e| e.to_string())
}

#[tauri::command]
pub fn update_slot(id: String, status: String, current_task: Option<String>) -> Result<(), String> {
    slot_repo::update_slot(&id, &status, current_task.as_deref()).map_err(|e| e.to_string())
}

#[tauri::command]
pub fn fetch_slots(session_id: String) -> Result<Vec<AgentSlot>, String> {
    slot_repo::fetch_slots_for_session(&session_id).map_err(|e| e.to_string())
}

// Cost commands
#[tauri::command]
pub fn record_usage(slot_id: String, provider: String, model: String, input_tokens: i64, output_tokens: i64) -> Result<CostEvent, String> {
    cost_repo::record_usage(&slot_id, &provider, &model, input_tokens, output_tokens).map_err(|e| e.to_string())
}

#[tauri::command]
pub fn cost_summary_slot(slot_id: String) -> Result<UsageSummary, String> {
    cost_repo::summary_for_slot(&slot_id).map_err(|e| e.to_string())
}

#[tauri::command]
pub fn cost_summary_session(session_id: String) -> Result<UsageSummary, String> {
    cost_repo::summary_for_session(&session_id).map_err(|e| e.to_string())
}

// Gate commands
#[tauri::command]
pub fn create_gate(slot_id: String, operation_type: String, payload: String, risk_level: String) -> Result<ExecutionGate, String> {
    gate_repo::create_gate(&slot_id, &operation_type, &payload, &risk_level).map_err(|e| e.to_string())
}

#[tauri::command]
pub fn approve_gate(id: String, approved_by: String) -> Result<(), String> {
    gate_repo::update_gate_status(&id, "executing", Some(&approved_by), None).map_err(|e| e.to_string())
}

#[tauri::command]
pub fn reject_gate(id: String, reason: String) -> Result<(), String> {
    gate_repo::update_gate_status(&id, "rejected", None, Some(&reason)).map_err(|e| e.to_string())
}

#[tauri::command]
pub fn fetch_gates(slot_id: String) -> Result<Vec<ExecutionGate>, String> {
    gate_repo::fetch_gates_for_slot(&slot_id).map_err(|e| e.to_string())
}

// Message commands
#[tauri::command]
pub fn publish_message(content: String, message_type: String, from_slot_id: String, to_slot_id: Option<String>, is_broadcast: bool) -> Result<AgentMessage, String> {
    message_repo::publish(&content, &message_type, &from_slot_id, to_slot_id.as_deref(), is_broadcast).map_err(|e| e.to_string())
}

#[tauri::command]
pub fn fetch_messages(slot_id: String) -> Result<Vec<AgentMessage>, String> {
    message_repo::fetch_for_slot(&slot_id).map_err(|e| e.to_string())
}

// Skill commands
#[tauri::command]
pub fn create_skill(name: String, family: String, skill_md_path: String, description: Option<String>) -> Result<MetierSkill, String> {
    skill_repo::create_skill(&name, &family, &skill_md_path, description.as_deref()).map_err(|e| e.to_string())
}

#[tauri::command]
pub fn find_skill(name: String) -> Result<Option<MetierSkill>, String> {
    skill_repo::find_by_name(&name).map_err(|e| e.to_string())
}

// CLI detection
#[tauri::command]
pub fn detect_cli_tools() -> Vec<cli_detector::CliStatus> {
    cli_detector::detect_all()
}

#[tauri::command]
pub fn find_loop_script(name: String) -> Option<String> {
    cli_detector::find_loop_script(&name)
}

// Git commands
#[tauri::command]
pub fn is_git_repo(path: String) -> bool {
    git_service::is_git_repo(&path)
}

#[tauri::command]
pub fn git_current_branch(path: String) -> Result<String, String> {
    git_service::get_current_branch(&path)
}

#[tauri::command]
pub fn git_recent_commits(path: String, count: usize) -> Result<Vec<git_service::GitCommit>, String> {
    git_service::get_recent_commits(&path, count)
}

#[tauri::command]
pub fn git_branches(path: String) -> Result<Vec<String>, String> {
    git_service::get_branches(&path)
}

#[tauri::command]
pub fn git_create_worktree(repo_path: String, worktree_path: String, branch: String) -> Result<(), String> {
    git_service::create_worktree(&repo_path, &worktree_path, &branch)
}

#[tauri::command]
pub fn git_delete_worktree(repo_path: String, worktree_path: String) -> Result<(), String> {
    git_service::delete_worktree(&repo_path, &worktree_path)
}

#[tauri::command]
pub fn git_coordinate_merge(repo_path: String, branches: Vec<String>) -> Result<git_service::MergeResult, String> {
    let refs: Vec<&str> = branches.iter().map(|s| s.as_str()).collect();
    git_service::coordinate_merge(&repo_path, &refs)
}

// Loop launcher commands
#[tauri::command]
pub fn resolve_loop_script(agent_type: String) -> Result<String, String> {
    crate::services::loop_launcher::resolve_loop_script(&agent_type)
}

#[tauri::command]
pub fn parse_progress(progress_path: String) -> Vec<crate::services::loop_launcher::ProgressEntry> {
    crate::services::loop_launcher::parse_progress(&progress_path)
}

#[tauri::command]
pub fn list_iteration_logs(log_dir: String) -> Result<Vec<crate::services::loop_launcher::LogFileInfo>, String> {
    crate::services::loop_launcher::list_iteration_logs(&log_dir)
}

#[tauri::command]
pub fn read_log_file(path: String) -> Result<String, String> {
    crate::services::loop_launcher::read_log_file(&path)
}
