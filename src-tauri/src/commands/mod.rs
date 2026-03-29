use crate::db::{session_repo, slot_repo, cost_repo, gate_repo, message_repo, skill_repo, metrics_repo, orchestration_repo};
use crate::services::{cli_detector, git_service};
use crate::services::agent_lifecycle::{self, SpawnRequest, AgentHealth, HealthAlert};
use crate::services::orchestration_engine;
use crate::services::mcp_service;
use crate::services::event_bus;
use crate::services::cockpit_logic;
use crate::models::{cockpit_session::CockpitSession, agent_slot::AgentSlot, cost_event::{CostEvent, UsageSummary}, execution_gate::ExecutionGate, agent_message::AgentMessage, metier_skill::MetierSkill};
use once_cell::sync::Lazy;
use std::sync::Mutex;
use tokio::sync::mpsc;

// Global lifecycle manager — initialized lazily
static LIFECYCLE_CHANNELS: Lazy<(
    mpsc::UnboundedSender<HealthAlert>,
    mpsc::UnboundedSender<agent_lifecycle::LifecycleEvent>,
)> = Lazy::new(|| {
    let (alert_tx, _alert_rx) = mpsc::unbounded_channel();
    let (event_tx, _event_rx) = mpsc::unbounded_channel();
    (alert_tx, event_tx)
});

static LIFECYCLE_MANAGER: Lazy<Mutex<agent_lifecycle::AgentLifecycleManager>> = Lazy::new(|| {
    let (alert_tx, event_tx) = LIFECYCLE_CHANNELS.clone();
    Mutex::new(agent_lifecycle::AgentLifecycleManager::new(alert_tx, event_tx))
});

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

// Agent lifecycle commands (PRD-14)

#[tauri::command]
pub fn spawn_agent(req: SpawnRequest) -> Result<String, String> {
    let mgr = LIFECYCLE_MANAGER.lock().unwrap();
    mgr.spawn_agent(req)
}

#[tauri::command]
pub fn abort_agent(slot_id: String) -> Result<(), String> {
    let mgr = LIFECYCLE_MANAGER.lock().unwrap();
    mgr.abort_agent(&slot_id)
}

#[tauri::command]
pub fn agent_health(slot_id: String) -> Option<AgentHealth> {
    let mgr = LIFECYCLE_MANAGER.lock().unwrap();
    mgr.get_health(&slot_id)
}

#[tauri::command]
pub fn all_agent_health() -> Vec<AgentHealth> {
    let mgr = LIFECYCLE_MANAGER.lock().unwrap();
    mgr.get_all_health()
}

#[tauri::command]
pub fn failover_agent(slot_id: String) -> Result<String, String> {
    let mgr = LIFECYCLE_MANAGER.lock().unwrap();
    mgr.failover(&slot_id)
}

#[tauri::command]
pub fn handle_alert_action(slot_id: String, action: String) -> Result<(), String> {
    let mgr = LIFECYCLE_MANAGER.lock().unwrap();
    mgr.handle_alert_action(&slot_id, &action)
}

#[tauri::command]
pub fn check_agents_health() {
    let mgr = LIFECYCLE_MANAGER.lock().unwrap();
    mgr.check_health();
}

#[tauri::command]
pub fn fetch_agent_metrics(slot_id: String) -> Result<metrics_repo::AgentMetrics, String> {
    metrics_repo::get_or_create(&slot_id).map_err(|e| e.to_string())
}

#[tauri::command]
pub fn record_story_completed(slot_id: String, story_time_ms: i64) -> Result<(), String> {
    metrics_repo::record_story_completed(&slot_id, story_time_ms).map_err(|e| e.to_string())
}

#[tauri::command]
pub fn record_story_failed(slot_id: String) -> Result<(), String> {
    metrics_repo::record_story_failed(&slot_id).map_err(|e| e.to_string())
}

// Orchestration engine commands (PRD-15)

#[tauri::command]
pub fn parse_prd(path: String) -> Result<orchestration_engine::ParsedPrd, String> {
    orchestration_engine::parse_prd(&path)
}

#[tauri::command]
pub fn detect_prd_files(dir: String) -> Vec<String> {
    orchestration_engine::detect_prd_files(&dir)
}

#[tauri::command]
pub fn build_execution_layers(path: String) -> Result<Vec<orchestration_engine::ExecutionLayer>, String> {
    let prd = orchestration_engine::parse_prd(&path)?;
    orchestration_engine::build_layers(&prd.stories)
}

#[tauri::command]
pub fn create_dispatch_plans(path: String, num_slots: usize) -> Result<Vec<orchestration_engine::LayerDispatchPlan>, String> {
    let prd = orchestration_engine::parse_prd(&path)?;
    let layers = orchestration_engine::build_layers(&prd.stories)?;
    Ok(orchestration_engine::create_dispatch_plans(&layers, num_slots))
}

#[tauri::command]
pub fn start_orchestration(session_id: String, prd_path: String) -> Result<serde_json::Value, String> {
    let (record_id, prd, layers, plans, resume_layer) =
        orchestration_engine::start_orchestration(&session_id, &prd_path)?;
    Ok(serde_json::json!({
        "recordId": record_id,
        "featureName": prd.feature_name,
        "totalStories": prd.stories.len(),
        "totalLayers": layers.len(),
        "resumeLayer": resume_layer,
        "layers": layers,
        "plans": plans,
    }))
}

#[tauri::command]
pub fn update_orchestration_progress(record_id: String, completed: i32, failed: i32, current_layer: i32) -> Result<(), String> {
    orchestration_repo::update_progress(&record_id, completed, failed, current_layer).map_err(|e| e.to_string())
}

#[tauri::command]
pub fn complete_orchestration(record_id: String, summary: String, merged_branches: Vec<String>, conflicts: Vec<String>, total_cost: i64) -> Result<(), String> {
    let branches_json = serde_json::to_string(&merged_branches).map_err(|e| e.to_string())?;
    let conflicts_json = serde_json::to_string(&conflicts).map_err(|e| e.to_string())?;
    orchestration_repo::complete_record(&record_id, &summary, &branches_json, &conflicts_json, total_cost).map_err(|e| e.to_string())
}

#[tauri::command]
pub fn fetch_orchestration_record(record_id: String) -> Result<Option<orchestration_repo::OrchestrationRecord>, String> {
    orchestration_repo::fetch_record(&record_id).map_err(|e| e.to_string())
}

#[tauri::command]
pub fn fetch_orchestration_records(session_id: String) -> Result<Vec<orchestration_repo::OrchestrationRecord>, String> {
    orchestration_repo::fetch_records_for_session(&session_id).map_err(|e| e.to_string())
}

// MCP commands (PRD-16)

#[tauri::command]
pub fn mcp_detect_node() -> Result<String, String> {
    mcp_service::detect_node_path()
}

#[tauri::command]
pub fn mcp_find_server(project_root: Option<String>) -> Result<String, String> {
    mcp_service::find_mcp_server(project_root.as_deref())
}

#[tauri::command]
pub fn mcp_persist_session(worktree_path: String, session_id: String, project_path: String, agent_type: String) -> Result<(), String> {
    let session = mcp_service::McpSession {
        session_id,
        project_path,
        agent_type,
        started_at: chrono::Utc::now().to_rfc3339(),
        decisions: Vec::new(),
    };
    mcp_service::persist_session(&worktree_path, &session)
}

#[tauri::command]
pub fn mcp_load_session(worktree_path: String, session_id: String) -> Result<Option<mcp_service::McpSession>, String> {
    mcp_service::load_session(&worktree_path, &session_id)
}

#[tauri::command]
pub fn mcp_record_decision(worktree_path: String, session_id: String, decision_type: String, description: String, context: Option<String>) -> Result<(), String> {
    let mut session = mcp_service::load_session(&worktree_path, &session_id)?
        .ok_or_else(|| format!("Session {} not found", session_id))?;

    session.decisions.push(mcp_service::McpDecision {
        decision_type,
        description,
        context,
        timestamp: chrono::Utc::now().to_rfc3339(),
    });

    mcp_service::persist_session(&worktree_path, &session)
}

#[tauri::command]
pub fn mcp_generate_handoff(worktree_path: String, session_id: String, max_tokens: Option<u32>) -> Result<String, String> {
    let session = mcp_service::load_session(&worktree_path, &session_id)?
        .ok_or_else(|| format!("Session {} not found", session_id))?;
    Ok(mcp_service::generate_handoff(&session, max_tokens.unwrap_or(4000)))
}

// Event bus commands (PRD-17)

#[tauri::command]
pub fn emit_agent_status(slot_id: String, status: String, progress: Option<f64>, task: Option<String>, agent_type: Option<String>) {
    event_bus::emit_agent_status(&slot_id, &status, progress, task.as_deref(), agent_type.as_deref());
}

#[tauri::command]
pub fn emit_log_entry(level: String, source: String, message: String, slot_id: Option<String>) {
    event_bus::emit_log(&level, &source, &message, slot_id.as_deref());
}

#[tauri::command]
pub fn emit_gate_event(gate_id: String, slot_id: String, operation_type: String, risk_level: String, status: String) {
    if status == "pending" {
        event_bus::emit_gate_created(&gate_id, &slot_id, &operation_type, &risk_level);
    } else {
        event_bus::emit_gate_resolved(&gate_id, &slot_id, &operation_type, &risk_level, &status);
    }
}

#[tauri::command]
pub fn flush_pty_buffers() {
    event_bus::flush_pty_buffers();
}

// Cockpit logic commands (PRD-18)

#[tauri::command]
pub fn cockpit_activate(session_id: String) -> Result<cockpit_logic::ChairmanOutput, String> {
    cockpit_logic::activate_session(&session_id)
}

#[tauri::command]
pub fn cockpit_pause(session_id: String) -> Result<(), String> {
    cockpit_logic::pause_session(&session_id)
}

#[tauri::command]
pub fn cockpit_resume(session_id: String) -> Result<(), String> {
    cockpit_logic::resume_session(&session_id)
}

#[tauri::command]
pub fn cockpit_close(session_id: String) -> Result<(), String> {
    cockpit_logic::close_session(&session_id)
}

#[tauri::command]
pub fn cockpit_read_context(project_path: String) -> Result<cockpit_logic::ChairmanInput, String> {
    cockpit_logic::read_project_context(&project_path)
}

#[tauri::command]
pub fn cockpit_deliberate(project_path: String) -> cockpit_logic::ChairmanOutput {
    let input = cockpit_logic::read_project_context(&project_path).unwrap_or(cockpit_logic::ChairmanInput {
        project_path: project_path.clone(),
        current_branch: "unknown".into(),
        recent_commits: vec![],
        branches: vec![],
        prd_summary: None,
        previous_session: None,
    });
    cockpit_logic::chairman_deliberate(&input)
}
