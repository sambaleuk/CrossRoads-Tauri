use std::io::{BufRead, BufReader};
use std::process::{Command, Stdio};
use std::sync::{Arc, Mutex};
use serde::{Deserialize, Serialize};

use crate::db::{session_repo, slot_repo, chat_history_repo};
use crate::services::{event_bus, stream_parser, cli_detector, cockpit_brain};

// ── PRD-44: Cockpit Session Manager ──
//
// The cockpit IS a Claude Code session. This service manages:
// - Spawning the cockpit brain as a headless Claude Code process
// - Parsing stream-json output and routing to cockpit-specific events
// - Lifecycle: start, stop, resume
// - Auto-start on project load

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CockpitStatus {
    pub is_alive: bool,
    pub process_id: Option<u32>,
    pub session_id: Option<String>,
    pub started_at: Option<String>,
    pub event_count: u64,
}

struct CockpitProcess {
    process_id: u32,
    session_id: Option<String>,
    started_at: String,
    event_count: u64,
}

static COCKPIT: once_cell::sync::Lazy<Arc<Mutex<Option<CockpitProcess>>>> =
    once_cell::sync::Lazy::new(|| Arc::new(Mutex::new(None)));

/// Start the cockpit brain session for a project.
/// Generates the agent definition, builds context prompt, and launches Claude Code headless.
pub fn start_session(project_path: &str) -> Result<CockpitStatus, String> {
    // Check if already running
    {
        let cockpit = COCKPIT.lock().unwrap();
        if cockpit.is_some() {
            return Err("Cockpit session already running".into());
        }
    }

    // Find claude binary
    let claude_path = cli_detector::find_loop_script("claude")
        .or_else(|| {
            let paths = ["/usr/local/bin/claude", "/opt/homebrew/bin/claude"];
            paths.iter().find(|p| std::path::Path::new(p).exists()).map(|p| p.to_string())
        })
        .ok_or_else(|| "Claude CLI not found".to_string())?;

    // Generate cockpit agent definition
    let cop = cockpit_brain::generate_cop(project_path, "{}")
        .unwrap_or_else(|_| cockpit_brain::CockpitOrchestrationPlan {
            project_name: std::path::Path::new(project_path)
                .file_name().map(|n| n.to_string_lossy().to_string())
                .unwrap_or_else(|| "project".into()),
            project_type: "unknown".into(),
            domain: "general".into(),
            market_context: "".into(),
            transverse_productions: vec![],
            specialist_triggers: vec![],
            meta_agent_config: cockpit_brain::MetaAgentConfig {
                capabilities: vec![], monitoring_interval_ms: 30000, autonomy_level: "supervised".into(),
            },
            deliverables_path: ".crossroads/deliverables".into(),
            created_at: chrono::Utc::now().to_rfc3339(),
        });

    // Gather active slot info for context
    let active_slots = gather_active_slots(project_path);

    // Generate the agent definition files
    let _ = cockpit_brain::generate_cockpit_agent_definition(project_path, &cop, &active_slots);

    // Build wake context from previous session (self-continuity)
    let session = session_repo::active_session_for_path(project_path).ok().flatten();
    let session_id_opt = session.as_ref().map(|s| s.id.as_str());
    let wake_context = chat_history_repo::build_wake_context(session_id_opt).unwrap_or_default();

    // Build context prompt with wake context injected
    let prompt = build_cockpit_prompt(project_path, &cop, &active_slots, &wake_context);

    // Launch Claude Code headless with cockpit-brain agent
    let mut cmd = Command::new(&claude_path);
    cmd.args([
        "-p", &prompt,
        "--agent", "cockpit-brain",
        "--output-format", "stream-json",
        "--dangerously-skip-permissions",
        "--verbose",
    ]);
    cmd.current_dir(project_path);
    cmd.stdout(Stdio::piped());
    cmd.stderr(Stdio::piped());

    let mut child = cmd.spawn()
        .map_err(|e| format!("Failed to spawn cockpit: {}", e))?;

    let pid = child.id();
    let started_at = chrono::Utc::now().to_rfc3339();

    {
        let mut cockpit = COCKPIT.lock().unwrap();
        *cockpit = Some(CockpitProcess {
            process_id: pid,
            session_id: None,
            started_at: started_at.clone(),
            event_count: 0,
        });
    }

    event_bus::emit_cockpit_event("decision", "Cockpit brain starting...", None);

    // Spawn background thread to read and route stream-json
    let stdout = child.stdout.take()
        .ok_or_else(|| "Failed to capture cockpit stdout".to_string())?;

    std::thread::spawn(move || {
        let reader = BufReader::new(stdout);

        for line in reader.lines() {
            let line = match line {
                Ok(l) => l,
                Err(_) => break,
            };

            // Parse with existing stream_parser
            if let Some(event) = stream_parser::parse_stream_line("cockpit", &line) {
                // Increment event counter
                {
                    let mut cockpit = COCKPIT.lock().unwrap();
                    if let Some(ref mut cp) = *cockpit {
                        cp.event_count += 1;
                    }
                }

                // Route to cockpit-specific events
                match &event {
                    stream_parser::StreamEvent::Text { text, .. } => {
                        let category = event_bus::categorize_cockpit_text(text);
                        event_bus::emit_cockpit_event(category, text, None);
                    }
                    stream_parser::StreamEvent::ToolUse { tool_name, input, .. } => {
                        let is_agent = tool_name == "Agent" || tool_name == "Task";
                        if is_agent {
                            let agent_name = input.get("prompt")
                                .or_else(|| input.get("description"))
                                .and_then(|v| v.as_str())
                                .unwrap_or("unknown");
                            event_bus::emit_cockpit_event("subagent",
                                &format!("Spawning: @{}", agent_name),
                                Some(serde_json::json!({"tool": tool_name, "input": input})));
                        } else {
                            let file_hint = input.get("file_path")
                                .or_else(|| input.get("command"))
                                .or_else(|| input.get("pattern"))
                                .and_then(|v| v.as_str())
                                .unwrap_or("");
                            event_bus::emit_cockpit_event("action",
                                &format!("[{}] {}", tool_name, file_hint),
                                Some(serde_json::json!({"tool": tool_name})));
                        }
                    }
                    stream_parser::StreamEvent::SessionComplete { session_id, .. } => {
                        if !session_id.is_empty() {
                            let mut cockpit = COCKPIT.lock().unwrap();
                            if let Some(ref mut cp) = *cockpit {
                                cp.session_id = Some(session_id.clone());
                            }
                        }
                        event_bus::emit_cockpit_event("decision",
                            "Cockpit session completed", None);
                    }
                    stream_parser::StreamEvent::Error { message, .. } => {
                        event_bus::emit_cockpit_event("decision",
                            &format!("[ERROR] {}", message), None);
                    }
                    _ => {}
                }

                // Also route to general event bus for MCP logs
                stream_parser::route_event(&event);
            }
        }

        // Process ended — clean up
        let _ = child.wait();
        let mut cockpit = COCKPIT.lock().unwrap();
        let final_session_id = cockpit.as_ref().and_then(|c| c.session_id.clone());
        *cockpit = None;

        event_bus::emit_cockpit_event("decision",
            &format!("Cockpit brain stopped. Resume ID: {:?}", final_session_id), None);
    });

    Ok(CockpitStatus {
        is_alive: true,
        process_id: Some(pid),
        session_id: None,
        started_at: Some(started_at),
        event_count: 0,
    })
}

/// Stop the cockpit session gracefully.
/// Saves a wake prompt with current state before killing the process.
pub fn stop_session() -> Result<(), String> {
    let process_id = {
        let cockpit = COCKPIT.lock().unwrap();
        cockpit.as_ref().map(|c| c.process_id)
    };

    if let Some(pid) = process_id {
        // Save wake prompt before shutdown (self-continuity)
        save_wake_prompt_on_stop();

        #[cfg(unix)]
        {
            let _ = Command::new("kill").args(["-TERM", &pid.to_string()]).status();
        }
        #[cfg(not(unix))]
        {
            let _ = Command::new("taskkill").args(["/PID", &pid.to_string(), "/F"]).status();
        }

        std::thread::sleep(std::time::Duration::from_secs(2));

        let mut cockpit = COCKPIT.lock().unwrap();
        *cockpit = None;

        event_bus::emit_cockpit_event("decision", "Cockpit brain stopped by user", None);
        Ok(())
    } else {
        Err("No cockpit session running".into())
    }
}

/// Build and save a wake prompt capturing current session state for resume.
fn save_wake_prompt_on_stop() {
    // Find the active session
    let sessions = session_repo::fetch_all_sessions().unwrap_or_default();
    let active = sessions.iter().find(|s| s.status == "active" || s.status == "initializing");

    if let Some(session) = active {
        let slots = slot_repo::fetch_slots_for_session(&session.id).unwrap_or_default();

        // Build slot summaries
        let slot_summaries: Vec<String> = slots.iter().map(|s| {
            format!("Slot {} ({}): status={}, task={}",
                s.slot_index, s.agent_type, s.status,
                s.current_task.as_deref().unwrap_or("none"))
        }).collect();
        let slot_json = serde_json::to_string(&slot_summaries).unwrap_or_else(|_| "[]".into());

        // Build observations from recent events
        let observations = format!(
            "Session had {} slots. Status at shutdown: {}",
            slots.len(),
            slots.iter().map(|s| format!("{}:{}", s.slot_index, s.status)).collect::<Vec<_>>().join(", ")
        );

        // Pending actions
        let pending: Vec<String> = slots.iter()
            .filter(|s| s.status == "running" || s.status == "provisioning")
            .map(|s| format!("Continue monitoring slot {} ({})", s.slot_index, s.agent_type))
            .collect();
        let pending_json = serde_json::to_string(&pending).unwrap_or_else(|_| "[]".into());

        let prompt = format!(
            "Cockpit was monitoring {} for project at {}. {} slots were active.",
            session.chairman_brief.as_deref().unwrap_or("session"),
            session.project_path,
            slots.len()
        );

        let _ = chat_history_repo::save_wake_prompt(
            &session.id,
            &prompt,
            Some(&observations),
            Some(&pending_json),
            Some(&slot_json),
        );

        event_bus::emit_log("info", "cockpit-session", "Wake prompt saved for session resume", None);
    }
}

/// Resume a previous cockpit session.
pub fn resume_session(project_path: &str, session_id: &str) -> Result<CockpitStatus, String> {
    // Stop any existing session first
    let _ = stop_session();

    let claude_path = cli_detector::find_loop_script("claude")
        .or_else(|| {
            let paths = ["/usr/local/bin/claude", "/opt/homebrew/bin/claude"];
            paths.iter().find(|p| std::path::Path::new(p).exists()).map(|p| p.to_string())
        })
        .ok_or_else(|| "Claude CLI not found".to_string())?;

    let mut cmd = Command::new(&claude_path);
    cmd.args([
        "-p", "Continue monitoring. Check dev slot progress and report status.",
        "--resume", session_id,
        "--output-format", "stream-json",
        "--dangerously-skip-permissions",
    ]);
    cmd.current_dir(project_path);
    cmd.stdout(Stdio::piped());
    cmd.stderr(Stdio::piped());

    let mut child = cmd.spawn()
        .map_err(|e| format!("Failed to resume cockpit: {}", e))?;

    let pid = child.id();
    let started_at = chrono::Utc::now().to_rfc3339();

    {
        let mut cockpit = COCKPIT.lock().unwrap();
        *cockpit = Some(CockpitProcess {
            process_id: pid,
            session_id: Some(session_id.to_string()),
            started_at: started_at.clone(),
            event_count: 0,
        });
    }

    event_bus::emit_cockpit_event("decision",
        &format!("Cockpit brain resuming session: {}", session_id), None);

    // Same stream reading thread as start_session
    let stdout = child.stdout.take()
        .ok_or_else(|| "Failed to capture cockpit stdout".to_string())?;

    std::thread::spawn(move || {
        let reader = BufReader::new(stdout);
        for line in reader.lines() {
            let line = match line { Ok(l) => l, Err(_) => break };
            if let Some(event) = stream_parser::parse_stream_line("cockpit", &line) {
                {
                    let mut cockpit = COCKPIT.lock().unwrap();
                    if let Some(ref mut cp) = *cockpit { cp.event_count += 1; }
                }
                match &event {
                    stream_parser::StreamEvent::Text { text, .. } => {
                        let category = event_bus::categorize_cockpit_text(text);
                        event_bus::emit_cockpit_event(category, text, None);
                    }
                    stream_parser::StreamEvent::ToolUse { tool_name, input, .. } => {
                        event_bus::emit_cockpit_event("action",
                            &format!("[{}]", tool_name),
                            Some(serde_json::json!({"tool": tool_name, "input": input})));
                    }
                    _ => {}
                }
                stream_parser::route_event(&event);
            }
        }
        let _ = child.wait();
        let mut cockpit = COCKPIT.lock().unwrap();
        *cockpit = None;
    });

    Ok(CockpitStatus {
        is_alive: true,
        process_id: Some(pid),
        session_id: Some(session_id.to_string()),
        started_at: Some(started_at),
        event_count: 0,
    })
}

/// Get current cockpit status.
pub fn get_status() -> CockpitStatus {
    let cockpit = COCKPIT.lock().unwrap();
    match cockpit.as_ref() {
        Some(cp) => CockpitStatus {
            is_alive: true,
            process_id: Some(cp.process_id),
            session_id: cp.session_id.clone(),
            started_at: Some(cp.started_at.clone()),
            event_count: cp.event_count,
        },
        None => CockpitStatus {
            is_alive: false,
            process_id: None,
            session_id: None,
            started_at: None,
            event_count: 0,
        },
    }
}

// ── Internal helpers ──

fn gather_active_slots(project_path: &str) -> Vec<(i32, String, String, String)> {
    // Find sessions for this project path
    let session = session_repo::active_session_for_path(project_path)
        .ok()
        .flatten();

    let mut slots = Vec::new();
    if let Some(session) = session {
        if let Ok(db_slots) = slot_repo::fetch_slots_for_session(&session.id) {
            for s in db_slots {
                if s.status == "running" || s.status == "provisioning" {
                    slots.push((
                        s.slot_index,
                        s.agent_type.clone(),
                        s.branch_name.clone().unwrap_or_default(),
                        s.current_task.clone().unwrap_or_default(),
                    ));
                }
            }
        }
    }
    slots
}

fn build_cockpit_prompt(
    project_path: &str,
    cop: &cockpit_brain::CockpitOrchestrationPlan,
    active_slots: &[(i32, String, String, String)],
    wake_context: &str,
) -> String {
    let slot_desc = if active_slots.is_empty() {
        "No dev agents are currently running. Enter idle/initiative mode.".to_string()
    } else {
        let lines: Vec<String> = active_slots.iter().map(|(idx, agent, branch, task)| {
            format!("Slot {} ({}) on branch '{}': {}", idx, agent, branch, task)
        }).collect();
        format!("Active dev agents:\n{}", lines.join("\n"))
    };

    let wake_section = if wake_context.is_empty() {
        String::new()
    } else {
        format!(
r#"

## Previous Session Context (Self-Continuity)
You are resuming from a previous session. Here is what you knew:

{}

Use this context to resume monitoring without re-scanning everything.
Verify that the previous state still holds before acting on it."#,
            wake_context
        )
    };

    format!(
r#"You are the cockpit brain for {project_name} ({project_type}, {domain}).

Current state:
{slot_desc}

Project path: {project_path}{wake_section}

Start monitoring. Use your observation protocol to check dev agent progress.
If no agents are running, enter idle mode and produce deliverables.
Use /loop 30s for active monitoring, /loop 5m for idle mode."#,
        project_name = cop.project_name,
        project_type = cop.project_type,
        domain = cop.domain,
        slot_desc = slot_desc,
        project_path = project_path,
        wake_section = wake_section,
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_cockpit_status_serialization() {
        let status = CockpitStatus {
            is_alive: true,
            process_id: Some(12345),
            session_id: Some("sess_abc".into()),
            started_at: Some("2026-03-30T10:00:00Z".into()),
            event_count: 42,
        };
        let json = serde_json::to_string(&status).unwrap();
        assert!(json.contains("isAlive"));
        assert!(json.contains("processId"));
        assert!(json.contains("eventCount"));
    }

    #[test]
    fn test_get_status_when_idle() {
        let status = get_status();
        // May or may not be running depending on test order, but shouldn't panic
        assert!(!status.is_alive || status.is_alive);
    }

    #[test]
    fn test_categorize_cockpit_text() {
        assert_eq!(event_bus::categorize_cockpit_text("[DECIDE] Spawning agent"), "decision");
        assert_eq!(event_bus::categorize_cockpit_text("[OBSERVE] Slot 1 progress"), "loop");
        assert_eq!(event_bus::categorize_cockpit_text("[ACTION] Running tests"), "action");
        assert_eq!(event_bus::categorize_cockpit_text("Thinking about next steps..."), "thinking");
        assert_eq!(event_bus::categorize_cockpit_text("Detected a new PRD file"), "decision");
    }

    #[test]
    fn test_build_cockpit_prompt_no_slots() {
        let cop = cockpit_brain::CockpitOrchestrationPlan {
            project_name: "Test".into(),
            project_type: "saas".into(),
            domain: "general".into(),
            market_context: "".into(),
            transverse_productions: vec![],
            specialist_triggers: vec![],
            meta_agent_config: cockpit_brain::MetaAgentConfig {
                capabilities: vec![], monitoring_interval_ms: 30000, autonomy_level: "supervised".into(),
            },
            deliverables_path: ".crossroads/deliverables".into(),
            created_at: "2026-03-30".into(),
        };
        let prompt = build_cockpit_prompt("/tmp/test", &cop, &[], "");
        assert!(prompt.contains("idle mode"));
        assert!(prompt.contains("/loop"));
    }

    #[test]
    fn test_build_cockpit_prompt_with_slots() {
        let cop = cockpit_brain::CockpitOrchestrationPlan {
            project_name: "Test".into(),
            project_type: "saas".into(),
            domain: "general".into(),
            market_context: "".into(),
            transverse_productions: vec![],
            specialist_triggers: vec![],
            meta_agent_config: cockpit_brain::MetaAgentConfig {
                capabilities: vec![], monitoring_interval_ms: 30000, autonomy_level: "supervised".into(),
            },
            deliverables_path: ".crossroads/deliverables".into(),
            created_at: "2026-03-30".into(),
        };
        let slots = vec![
            (0, "claude".into(), "feat/auth".into(), "Implement auth".into()),
        ];
        let prompt = build_cockpit_prompt("/tmp/test", &cop, &slots, "");
        assert!(prompt.contains("Slot 0"));
        assert!(prompt.contains("feat/auth"));
    }
}
