// Prevents additional console window on Windows in release
#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

use tauri::Manager;

mod commands;
mod db;
mod models;
mod services;

fn main() {
    env_logger::init();

    tauri::Builder::default()
        .plugin(tauri_plugin_shell::init())
        .plugin(tauri_plugin_dialog::init())
        .setup(|app| {
            // Initialize database on app startup
            let app_dir = app.path().app_data_dir().expect("failed to get app data dir");
            std::fs::create_dir_all(&app_dir).ok();
            let db_path = app_dir.join("xroads.sqlite");
            db::manager::initialize(db_path.to_str().unwrap())
                .expect("failed to initialize database");
            log::info!("Database initialized at {:?}", db_path);

            // Initialize event bus with app handle
            services::event_bus::init(app.handle().clone());
            log::info!("Event bus initialized");

            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            commands::create_session,
            commands::fetch_session,
            commands::update_session,
            commands::delete_session,
            commands::active_session,
            commands::fetch_all_sessions,
            commands::create_slot,
            commands::update_slot,
            commands::fetch_slots,
            commands::record_usage,
            commands::cost_summary_slot,
            commands::cost_summary_session,
            commands::create_gate,
            commands::approve_gate,
            commands::reject_gate,
            commands::fetch_gates,
            commands::publish_message,
            commands::fetch_messages,
            commands::create_skill,
            commands::find_skill,
            commands::detect_cli_tools,
            commands::find_loop_script,
            commands::is_git_repo,
            commands::git_current_branch,
            commands::git_recent_commits,
            commands::git_branches,
            commands::git_create_worktree,
            commands::git_delete_worktree,
            commands::git_coordinate_merge,
            commands::resolve_loop_script,
            commands::parse_progress,
            commands::list_iteration_logs,
            commands::read_log_file,
            // PRD-14: Agent Lifecycle
            commands::spawn_agent,
            commands::abort_agent,
            commands::agent_health,
            commands::all_agent_health,
            commands::failover_agent,
            commands::handle_alert_action,
            commands::check_agents_health,
            commands::fetch_agent_metrics,
            commands::record_story_completed,
            commands::record_story_failed,
            // PRD-15: Orchestration Engine
            commands::parse_prd,
            commands::detect_prd_files,
            commands::build_execution_layers,
            commands::create_dispatch_plans,
            commands::start_orchestration,
            commands::update_orchestration_progress,
            commands::complete_orchestration,
            commands::fetch_orchestration_record,
            commands::fetch_orchestration_records,
            // PRD-16: MCP Client + Server
            commands::mcp_detect_node,
            commands::mcp_find_server,
            commands::mcp_persist_session,
            commands::mcp_load_session,
            commands::mcp_record_decision,
            commands::mcp_generate_handoff,
            // PRD-17: Event Bus
            commands::emit_agent_status,
            commands::emit_log_entry,
            commands::emit_gate_event,
            commands::flush_pty_buffers,
            // PRD-18: Cockpit Logic
            commands::cockpit_activate,
            commands::cockpit_pause,
            commands::cockpit_resume,
            commands::cockpit_close,
            commands::cockpit_read_context,
            commands::cockpit_deliberate,
            commands::generate_agent_definitions,
            // PRD-43: Headless Claude Code
            commands::launch_headless,
            commands::abort_headless,
            commands::list_headless_sessions,
            commands::generate_hooks_config,
            commands::generate_claude_md,
            commands::generate_rules,
            commands::generate_all_claude_config,
            commands::inject_agent_memories,
            commands::sync_agent_memories,
            // PRD-44: Cockpit as Claude Code session
            commands::start_cockpit_session,
            commands::stop_cockpit_session,
            commands::resume_cockpit_session,
            commands::get_cockpit_status,
            commands::generate_cockpit_agent_definition,
            // PRD-19: SafeExecutor
            commands::detect_dangerous_ops,
            commands::safe_trigger_gate,
            commands::safe_approve_gate,
            commands::safe_reject_gate,
            commands::evaluate_policy,
            // PRD-20: Skill System
            commands::scan_skills,
            commands::register_skills,
            commands::adapt_skill_for_cli,
            commands::prepare_skill_injection,
            // PRD-21: Session Persistence + Recovery
            commands::save_session_state,
            commands::load_session_state,
            commands::detect_recoverable_sessions,
            commands::discard_session,
            commands::save_orchestration_history,
            commands::load_orchestration_history,
            commands::detect_orphaned_worktrees,
            commands::cleanup_worktrees,
            commands::detect_stale_sessions,
            commands::cleanup_stale_sessions,
            // PRD-23: Terminal PTY input
            commands::run_claude_cli,
            commands::send_pty_input,
            // Phase 5: Org Chart (PRD-35)
            commands::create_org_role,
            commands::fetch_org_roles,
            commands::apply_role_template,
            commands::get_org_tree,
            commands::cascade_goals,
            // Phase 5: Budget (PRD-36)
            commands::create_budget_config,
            commands::check_budget,
            commands::get_cost_projection,
            commands::fetch_budget_alerts,
            commands::route_model,
            // Phase 5: Heartbeat (PRD-37)
            commands::create_heartbeat,
            commands::create_scheduled_run,
            commands::fetch_scheduled_runs,
            // Phase 5: Workspace (PRD-38)
            commands::create_workspace,
            commands::fetch_workspaces,
            commands::switch_workspace,
            commands::delete_workspace,
            // Phase 5: Runtime (PRD-39)
            commands::register_runtime,
            commands::fetch_runtimes,
            commands::register_builtin_runtimes,
            // Phase 5: Config (PRD-40)
            commands::create_config_snapshot,
            commands::fetch_config_snapshots,
            commands::rollback_config,
            // Phase 5: Learning (PRD-41)
            commands::fetch_performance_profiles,
            commands::recommend_agent,
            commands::estimate_story_time,
            commands::predict_conflicts,
            commands::generate_retro,
            // P0-1: Persistent Agent Memory
            commands::store_agent_memory,
            commands::recall_agent_memories,
            commands::search_memories,
            // P0-2: Trust Scoring + Auto-Merge
            commands::compute_trust_score,
            commands::fetch_trust_scores,
            commands::set_auto_merge_policy,
            commands::should_auto_merge,
            // P0-3: Predictive Conflict Prevention
            commands::analyze_conflicts_before_dispatch,
            // Cockpit Brain: Autonomous Orchestration
            commands::generate_cockpit_plan,
            commands::get_cockpit_plan,
            commands::generate_brain_meta_skill,
            commands::generate_brain_transverse_skill,
            commands::create_brain_deliverables_structure,
            commands::get_brain_adaptation_actions,
            // Chat History + Wake Prompts
            commands::save_chat_message,
            commands::fetch_chat_history,
            commands::get_wake_context,
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
