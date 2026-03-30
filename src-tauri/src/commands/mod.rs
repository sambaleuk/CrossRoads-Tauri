use crate::db::{session_repo, slot_repo, cost_repo, gate_repo, message_repo, skill_repo, metrics_repo, orchestration_repo};
use crate::db::{org_role_repo, budget_repo, heartbeat_repo, workspace_repo, runtime_repo, config_snapshot_repo, learning_repo, memory_repo, trust_repo};
use crate::services::{cli_detector, git_service};
use crate::services::agent_lifecycle::{self, SpawnRequest, AgentHealth, HealthAlert};
use crate::services::orchestration_engine;
use crate::services::mcp_service;
use crate::services::event_bus;
use crate::services::cockpit_logic;
use crate::services::safe_executor;
use crate::services::skill_system;
use crate::services::session_persistence;
use crate::services::{org_chart, budget_engine, heartbeat_engine, learning_engine, conflict_prevention};
use crate::services::cockpit_brain;
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
    // If there's already an active (non-closed) session for this path, return it
    if let Ok(Some(existing)) = session_repo::active_session_for_path(&project_path) {
        return Ok(existing);
    }
    // Clean up any stale sessions from previous crashes before creating
    let _ = session_repo::cleanup_stale_sessions(&project_path);
    session_repo::create_session(&project_path).map_err(|e| e.to_string())
}

#[tauri::command]
pub fn cleanup_stale_sessions(project_path: String) -> Result<usize, String> {
    session_repo::cleanup_stale_sessions(&project_path).map_err(|e| e.to_string())
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
    let event = cost_repo::record_usage(&slot_id, &provider, &model, input_tokens, output_tokens).map_err(|e| e.to_string())?;

    // WIRING 4: Check budget after recording cost
    if let Ok(status) = budget_engine::check_budget(&slot_id) {
        if status.status == "exceeded" {
            let _ = slot_repo::update_slot(&slot_id, "paused", Some("Budget exceeded"));
            event_bus::emit_agent_status(&slot_id, "paused", None, Some("Budget exceeded"), None);
        } else if status.status == "warning" {
            event_bus::emit_log("warn", "budget", &format!("Slot {} at {:.0}% of budget", slot_id, status.percent_used), Some(&slot_id));
        }
    }

    Ok(event)
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
    let output = cockpit_logic::activate_session(&session_id)?;

    // Auto-launch: create worktrees and spawn agents
    match cockpit_logic::prepare_auto_launch(&session_id, &output) {
        Ok(spawn_requests) => {
            let mgr = LIFECYCLE_MANAGER.lock().unwrap();
            for req in spawn_requests {
                match mgr.spawn_agent(req) {
                    Ok(pid) => {
                        event_bus::emit_log("info", "auto-launch",
                            &format!("Agent spawned: {}", pid), None);
                    }
                    Err(e) => {
                        event_bus::emit_log("warn", "auto-launch",
                            &format!("Failed to spawn agent: {}", e), None);
                    }
                }
            }
        }
        Err(e) => {
            event_bus::emit_log("warn", "auto-launch",
                &format!("Auto-launch preparation failed: {}", e), None);
        }
    }

    Ok(output)
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

// SafeExecutor commands (PRD-19)

#[tauri::command]
pub fn detect_dangerous_ops(text: String) -> Vec<safe_executor::DangerousOperation> {
    safe_executor::detect_dangerous_ops(&text)
}

#[tauri::command]
pub fn safe_trigger_gate(slot_id: String, pattern: String, matched_text: String, risk_level: String, description: String) -> Result<String, String> {
    let op = safe_executor::DangerousOperation { pattern, matched_text, risk_level, description };
    safe_executor::trigger_gate(&slot_id, &op, None)
}

#[tauri::command]
pub fn safe_approve_gate(gate_id: String, slot_id: String, approved_by: String) -> Result<(), String> {
    safe_executor::approve_gate(&gate_id, &slot_id, &approved_by, None)
}

#[tauri::command]
pub fn safe_reject_gate(gate_id: String, slot_id: String, reason: String) -> Result<(), String> {
    safe_executor::reject_gate(&gate_id, &slot_id, &reason, None)
}

#[tauri::command]
pub fn evaluate_policy(risk_level: String, pattern: String) -> String {
    let decision = safe_executor::evaluate_policy(&risk_level, &[], &pattern);
    match decision {
        safe_executor::PolicyDecision::AutoApprove => "auto_approve".into(),
        safe_executor::PolicyDecision::RequireDryRun => "require_dry_run".into(),
        safe_executor::PolicyDecision::RequireHumanApproval => "require_human_approval".into(),
    }
}

// Skill system commands (PRD-20)

#[tauri::command]
pub fn scan_skills(project_path: Option<String>) -> Vec<skill_system::LoadedSkill> {
    skill_system::scan_skills(project_path.as_deref())
}

#[tauri::command]
pub fn register_skills(project_path: Option<String>) -> Result<Vec<String>, String> {
    let skills = skill_system::scan_skills(project_path.as_deref());
    skill_system::register_skills(&skills)
}

#[tauri::command]
pub fn adapt_skill_for_cli(skill_name: String, cli_type: String, project_path: Option<String>) -> Result<String, String> {
    let skills = skill_system::scan_skills(project_path.as_deref());
    let skill = skills.iter().find(|s| s.name == skill_name)
        .ok_or_else(|| format!("Skill '{}' not found", skill_name))?;
    Ok(skill_system::adapt_for_cli(skill, &cli_type))
}

#[tauri::command]
pub fn prepare_skill_injection(
    skill_names: Vec<String>,
    cli_type: String,
    project_name: String,
    branch: String,
    story_title: Option<String>,
    slot_index: u32,
    agent_type: String,
    project_path: Option<String>,
) -> Result<String, String> {
    let all_skills = skill_system::scan_skills(project_path.as_deref());
    let selected: Vec<skill_system::LoadedSkill> = all_skills.into_iter()
        .filter(|s| skill_names.contains(&s.name))
        .collect();

    if selected.is_empty() {
        return Err("No matching skills found".into());
    }

    let ctx = skill_system::SkillContext {
        project_name,
        branch,
        story_title,
        story_id: None,
        slot_index,
        agent_type: agent_type.clone(),
    };

    Ok(skill_system::prepare_skill_injection(&selected, &cli_type, &ctx))
}

// Session persistence commands (PRD-21)

#[tauri::command]
pub fn save_session_state(project_path: String) -> Result<(), String> {
    session_persistence::save_session_state(&project_path)
}

#[tauri::command]
pub fn load_session_state(project_path: String, session_id: String) -> Result<Option<session_persistence::PersistedSession>, String> {
    session_persistence::load_session_state(&project_path, &session_id)
}

#[tauri::command]
pub fn detect_recoverable_sessions() -> Result<Vec<session_persistence::RecoveryInfo>, String> {
    session_persistence::detect_recoverable_sessions()
}

#[tauri::command]
pub fn discard_session(session_id: String) -> Result<(), String> {
    session_persistence::discard_session(&session_id)
}

#[tauri::command]
pub fn save_orchestration_history(entry: session_persistence::HistoryEntry) -> Result<(), String> {
    session_persistence::save_to_history(&entry)
}

#[tauri::command]
pub fn load_orchestration_history() -> Result<Vec<session_persistence::HistoryEntry>, String> {
    session_persistence::load_history()
}

#[tauri::command]
pub fn detect_orphaned_worktrees(repo_path: String) -> Result<Vec<session_persistence::OrphanedWorktree>, String> {
    session_persistence::detect_orphaned_worktrees(&repo_path)
}

#[tauri::command]
pub fn cleanup_worktrees(repo_path: String, worktree_paths: Vec<String>) -> Result<u32, String> {
    session_persistence::cleanup_worktrees(&repo_path, &worktree_paths)
}

#[tauri::command]
pub fn detect_stale_sessions() -> Result<Vec<session_persistence::RecoveryInfo>, String> {
    session_persistence::detect_stale_sessions()
}

// Chat: Run Claude CLI for real conversation

#[tauri::command]
pub async fn run_claude_cli(prompt: String, system_prompt: String, working_directory: Option<String>) -> Result<String, String> {
    // Find claude binary
    let claude_path = cli_detector::find_loop_script("claude")
        .or_else(|| {
            // Try common paths
            let paths = ["/usr/local/bin/claude", "/opt/homebrew/bin/claude"];
            paths.iter().find(|p| std::path::Path::new(p).exists()).map(|p| p.to_string())
        })
        .ok_or_else(|| "Claude CLI not found. Install: npm install -g @anthropic-ai/claude-code".to_string())?;

    let mut cmd = std::process::Command::new(&claude_path);
    cmd.args([
        "--dangerously-skip-permissions",
        "--output-format", "text",
        "--system-prompt", &system_prompt,
        "-p", &prompt,
    ]);

    if let Some(ref dir) = working_directory {
        cmd.current_dir(dir);
    }

    let output = cmd.output().map_err(|e| format!("Failed to run claude: {}", e))?;

    if output.status.success() {
        let stdout = String::from_utf8_lossy(&output.stdout).to_string();
        Ok(stdout.trim().to_string())
    } else {
        let stderr = String::from_utf8_lossy(&output.stderr).to_string();
        Err(format!("Claude exited with error: {}", stderr.trim()))
    }
}

// PTY input command (PRD-23)

#[tauri::command]
pub fn send_pty_input(slot_id: String, text: String) -> Result<(), String> {
    // Route input to the agent's PTY process via the lifecycle manager
    let mgr = LIFECYCLE_MANAGER.lock().unwrap();
    if let Some(health) = mgr.get_health(&slot_id) {
        if let Some(process_id) = health.process_id {
            // ProcessRunner is inside the lifecycle manager — emit event for now
            // TODO: expose process_runner.send_input through lifecycle manager
            event_bus::emit_log("debug", "pty-input",
                &format!("Input to slot {} (pid {}): {}", slot_id, process_id, text.escape_debug()),
                Some(&slot_id));
        }
    }
    Ok(())
}

// ── Phase 5: Org Chart (PRD-35) ──

#[tauri::command]
pub fn create_org_role(session_id: String, name: String, role_type: String, parent_role_id: Option<String>, goal_description: Option<String>, authority: Option<String>) -> Result<org_role_repo::OrgRole, String> {
    org_role_repo::create_role(&session_id, &name, &role_type, parent_role_id.as_deref(), goal_description.as_deref(), None, authority.as_deref().unwrap_or("limited")).map_err(|e| e.to_string())
}

#[tauri::command]
pub fn fetch_org_roles(session_id: String) -> Result<Vec<org_role_repo::OrgRole>, String> {
    org_role_repo::fetch_roles_for_session(&session_id).map_err(|e| e.to_string())
}

#[tauri::command]
pub fn apply_role_template(session_id: String, template_name: String) -> Result<Vec<org_chart::OrgRole>, String> {
    org_chart::apply_template(&session_id, &template_name)
}

#[tauri::command]
pub fn get_org_tree(session_id: String) -> Result<Vec<org_chart::OrgRoleNode>, String> {
    org_chart::get_role_tree(&session_id)
}

#[tauri::command]
pub fn cascade_goals(session_id: String, ceo_goal: String) -> Result<serde_json::Value, String> {
    let result = org_chart::cascade_goals(&session_id, &ceo_goal)?;
    Ok(serde_json::to_value(result).unwrap_or_default())
}

// ── Phase 5: Budget (PRD-36) ──

#[tauri::command]
pub fn create_budget_config(session_id: String, slot_id: Option<String>, budget_cents: Option<i32>) -> Result<budget_repo::BudgetConfig, String> {
    budget_repo::create_budget_config(&session_id, slot_id.as_deref(), budget_cents.unwrap_or(2500) as i64, 80, true, true).map_err(|e| e.to_string())
}

#[tauri::command]
pub fn check_budget(slot_id: String) -> Result<budget_engine::BudgetStatus, String> {
    budget_engine::check_budget(&slot_id)
}

#[tauri::command]
pub fn get_cost_projection(session_id: String) -> Result<budget_engine::CostProjection, String> {
    budget_engine::get_projection(&session_id)
}

#[tauri::command]
pub fn fetch_budget_alerts(config_id: String) -> Result<Vec<budget_repo::BudgetAlert>, String> {
    budget_repo::fetch_alerts(&config_id).map_err(|e| e.to_string())
}

#[tauri::command]
pub fn route_model(slot_id: String, complexity: String) -> Result<budget_engine::ModelRecommendation, String> {
    budget_engine::route_model(&slot_id, &complexity)
}

// ── Phase 5: Heartbeat (PRD-37) ──

#[tauri::command]
pub fn create_heartbeat(session_id: String, slot_id: Option<String>, interval_ms: Option<i32>) -> Result<heartbeat_repo::HeartbeatConfig, String> {
    heartbeat_repo::create_heartbeat(&session_id, slot_id.as_deref(), interval_ms.unwrap_or(30000) as i64).map_err(|e| e.to_string())
}

#[tauri::command]
pub fn create_scheduled_run(project_path: String, prd_path: String, cron_expression: Option<String>, trigger_type: Option<String>) -> Result<heartbeat_repo::ScheduledRun, String> {
    heartbeat_repo::create_scheduled_run(&project_path, &prd_path, cron_expression.as_deref(), trigger_type.as_deref().unwrap_or("manual")).map_err(|e| e.to_string())
}

#[tauri::command]
pub fn fetch_scheduled_runs(project_path: String) -> Result<Vec<heartbeat_repo::ScheduledRun>, String> {
    heartbeat_repo::fetch_due_runs().map_err(|e| e.to_string())
}

// ── Phase 5: Workspace (PRD-38) ──

#[tauri::command]
pub fn create_workspace(name: String, project_path: String, color: Option<String>) -> Result<workspace_repo::Workspace, String> {
    workspace_repo::create_workspace(&name, &project_path, color.as_deref()).map_err(|e| e.to_string())
}

#[tauri::command]
pub fn fetch_workspaces() -> Result<Vec<workspace_repo::Workspace>, String> {
    workspace_repo::fetch_all_workspaces().map_err(|e| e.to_string())
}

#[tauri::command]
pub fn switch_workspace(id: String) -> Result<(), String> {
    workspace_repo::switch_active(&id).map_err(|e| e.to_string())
}

#[tauri::command]
pub fn delete_workspace(id: String) -> Result<(), String> {
    workspace_repo::delete_workspace(&id).map_err(|e| e.to_string())
}

// ── Phase 5: Runtime (PRD-39) ──

#[tauri::command]
pub fn register_runtime(name: String, runtime_type: String, command: Option<String>, url: Option<String>, capabilities: Option<String>) -> Result<runtime_repo::AgentRuntime, String> {
    runtime_repo::register_runtime(&name, &runtime_type, command.as_deref(), url.as_deref(), &capabilities.unwrap_or_else(|| "[]".into()), false).map_err(|e| e.to_string())
}

#[tauri::command]
pub fn fetch_runtimes() -> Result<Vec<runtime_repo::AgentRuntime>, String> {
    runtime_repo::fetch_all_runtimes().map_err(|e| e.to_string())
}

#[tauri::command]
pub fn register_builtin_runtimes() -> Result<(), String> {
    runtime_repo::register_builtins().map_err(|e| e.to_string())
}

// ── Phase 5: Config Versioning (PRD-40) ──

#[tauri::command]
pub fn create_config_snapshot(session_id: String, config_type: String, data: String, changed_by: Option<String>, reason: Option<String>) -> Result<config_snapshot_repo::ConfigSnapshot, String> {
    config_snapshot_repo::create_snapshot(Some(&session_id), None, &config_type, &data, &changed_by.unwrap_or_else(|| "user".into()), reason.as_deref()).map_err(|e| e.to_string())
}

#[tauri::command]
pub fn fetch_config_snapshots(session_id: String, config_type: String) -> Result<Vec<config_snapshot_repo::ConfigSnapshot>, String> {
    config_snapshot_repo::fetch_snapshots(&session_id, &config_type).map_err(|e| e.to_string())
}

#[tauri::command]
pub fn rollback_config(session_id: String, config_type: String, version: i32) -> Result<(), String> {
    let snapshot = config_snapshot_repo::fetch_snapshot_by_version(&session_id, &config_type, version)
        .map_err(|e| e.to_string())?
        .ok_or_else(|| format!("Snapshot v{} not found for {}", version, config_type))?;
    // Create new snapshot with rollback data
    config_snapshot_repo::create_snapshot(Some(&session_id), None, &config_type, &snapshot.data, "system", Some(&format!("Rollback to v{}", version)))
        .map_err(|e| e.to_string())?;
    Ok(())
}

// ── Phase 5: Learning (PRD-41) ──

#[tauri::command]
pub fn fetch_performance_profiles() -> Result<Vec<learning_repo::PerformanceProfile>, String> {
    learning_repo::fetch_all_profiles().map_err(|e| e.to_string())
}

#[tauri::command]
pub fn recommend_agent(task_category: String) -> Option<learning_engine::AgentRecommendation> {
    learning_engine::recommend_agent(&task_category)
}

#[tauri::command]
pub fn estimate_story_time(task_category: String, complexity: String) -> Option<i64> {
    learning_engine::estimate_story_time(&task_category, &complexity)
}

#[tauri::command]
pub fn predict_conflicts(stories: Vec<serde_json::Value>) -> Vec<learning_engine::ConflictPrediction> {
    let parsed: Vec<(String, Vec<String>)> = stories.iter().filter_map(|s| {
        let id = s.get("id")?.as_str()?.to_string();
        let patterns: Vec<String> = s.get("patterns")?.as_array()?
            .iter().filter_map(|p| p.as_str().map(String::from)).collect();
        Some((id, patterns))
    }).collect();
    learning_engine::predict_conflicts(&parsed)
}

#[tauri::command]
pub fn generate_retro(session_id: String) -> Result<String, String> {
    learning_engine::generate_retro(&session_id)
}

// ── P0-1: Persistent Agent Memory ──

#[tauri::command]
pub fn store_agent_memory(
    agent_type: String,
    domain: String,
    memory_type: String,
    content: String,
    confidence: f64,
    session_id: Option<String>,
    story_id: Option<String>,
    tags: Option<String>,
) -> Result<memory_repo::AgentMemory, String> {
    memory_repo::store_memory(
        &agent_type, &domain, &memory_type, &content, confidence,
        session_id.as_deref(), story_id.as_deref(),
        &tags.unwrap_or_else(|| "[]".into()),
    ).map_err(|e| e.to_string())
}

#[tauri::command]
pub fn recall_agent_memories(agent_type: String, domain: String, limit: Option<i32>) -> Result<Vec<memory_repo::AgentMemory>, String> {
    memory_repo::recall_memories(&agent_type, &domain, limit.unwrap_or(20)).map_err(|e| e.to_string())
}

#[tauri::command]
pub fn search_memories(query: String) -> Result<Vec<memory_repo::AgentMemory>, String> {
    memory_repo::search_memories(&query).map_err(|e| e.to_string())
}

// ── P0-2: Trust Scoring + Auto-Merge Policy ──

#[tauri::command]
pub fn compute_trust_score(agent_type: String, domain: String) -> Result<trust_repo::TrustScore, String> {
    trust_repo::compute_trust(&agent_type, &domain).map_err(|e| e.to_string())
}

#[tauri::command]
pub fn fetch_trust_scores() -> Result<Vec<trust_repo::TrustScore>, String> {
    trust_repo::fetch_all_trust().map_err(|e| e.to_string())
}

#[tauri::command]
pub fn set_auto_merge_policy(agent_type: String, domain: String, enabled: bool, threshold: f64) -> Result<(), String> {
    trust_repo::update_auto_merge(&agent_type, &domain, enabled, threshold).map_err(|e| e.to_string())
}

#[tauri::command]
pub fn should_auto_merge(agent_type: String, domain: String) -> Result<bool, String> {
    trust_repo::should_auto_merge(&agent_type, &domain).map_err(|e| e.to_string())
}

// ── P0-3: Predictive Conflict Prevention ──

#[tauri::command]
pub fn analyze_conflicts_before_dispatch(
    stories: Vec<serde_json::Value>,
    project_path: Option<String>,
) -> Result<conflict_prevention::ConflictPreventionResult, String> {
    let parsed_stories: Vec<(String, Vec<String>)> = stories.iter().filter_map(|s| {
        let id = s.get("id")?.as_str()?.to_string();
        let patterns: Vec<String> = s.get("patterns")
            .and_then(|v| v.as_array())
            .map(|a| a.iter().filter_map(|p| p.as_str().map(String::from)).collect())
            .unwrap_or_default();
        Some((id, patterns))
    }).collect();

    // Generate file predictions from titles
    let story_titles: Vec<(String, String)> = stories.iter().filter_map(|s| {
        let id = s.get("id")?.as_str()?.to_string();
        let title = s.get("title").and_then(|v| v.as_str()).unwrap_or("").to_string();
        Some((id, title))
    }).collect();

    let file_predictions = conflict_prevention::generate_file_predictions(
        &story_titles, None, project_path.as_deref(),
    );

    Ok(conflict_prevention::analyze_dispatch_plan(&parsed_stories, &file_predictions))
}

// ── Cockpit Brain: Autonomous Orchestration ──

#[tauri::command]
pub fn generate_cockpit_plan(project_path: String, prd_json: String) -> Result<cockpit_brain::CockpitOrchestrationPlan, String> {
    cockpit_brain::generate_cop(&project_path, &prd_json)
}

#[tauri::command]
pub fn get_cockpit_plan(project_path: String) -> Result<Option<cockpit_brain::CockpitOrchestrationPlan>, String> {
    cockpit_brain::get_cop(&project_path)
}

#[tauri::command]
pub fn generate_brain_meta_skill(project_path: String) -> Result<String, String> {
    let cop = cockpit_brain::get_cop(&project_path)?
        .ok_or_else(|| "No COP found — run generate_cockpit_plan first".to_string())?;
    Ok(cockpit_brain::generate_meta_skill(&cop))
}

#[tauri::command]
pub fn generate_brain_transverse_skill(project_path: String, category: String) -> Result<String, String> {
    let cop = cockpit_brain::get_cop(&project_path)?
        .ok_or_else(|| "No COP found — run generate_cockpit_plan first".to_string())?;
    cockpit_brain::generate_transverse_skill(&cop, &category)
}

#[tauri::command]
pub fn create_brain_deliverables_structure(project_path: String) -> Result<(), String> {
    let cop = cockpit_brain::get_cop(&project_path)?
        .ok_or_else(|| "No COP found — run generate_cockpit_plan first".to_string())?;
    cockpit_brain::create_deliverables_structure(&project_path, &cop)
}

#[tauri::command]
pub fn get_brain_adaptation_actions(session_id: String) -> Result<Vec<cockpit_brain::AdaptationAction>, String> {
    cockpit_brain::get_adaptation_actions(&session_id)
}
