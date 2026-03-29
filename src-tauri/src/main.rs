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
        .setup(|app| {
            // Initialize database on app startup
            let app_dir = app.path().app_data_dir().expect("failed to get app data dir");
            std::fs::create_dir_all(&app_dir).ok();
            let db_path = app_dir.join("xroads.sqlite");
            db::manager::initialize(db_path.to_str().unwrap())
                .expect("failed to initialize database");
            log::info!("Database initialized at {:?}", db_path);
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
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
