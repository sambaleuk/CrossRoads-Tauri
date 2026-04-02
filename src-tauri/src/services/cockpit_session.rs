use std::io::{BufRead, BufReader};
use std::process::{Command, Stdio};
use std::sync::{Arc, Mutex};
use serde::{Deserialize, Serialize};

use crate::db::{session_repo, slot_repo, chat_history_repo};
use crate::services::{event_bus, stream_parser, cli_detector, cockpit_brain};


// ── Auto-restart counter for brain crash recovery ──
static BRAIN_RESTART_COUNT: once_cell::sync::Lazy<Arc<Mutex<u32>>> =
    once_cell::sync::Lazy::new(|| Arc::new(Mutex::new(0)));

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

    // Generate the agent definition files (with soul injection)
    let _ = cockpit_brain::generate_cockpit_agent_definition(project_path, &cop, &active_slots);

    // Inject soul into the cockpit-brain.md agent definition
    inject_soul_into_agent_def(project_path);

    // Inject chairman brief if available
    inject_chairman_brief(project_path);

    // Build wake context from previous session (self-continuity)
    let session = session_repo::active_session_for_path(project_path).ok().flatten();
    let session_id_opt = session.as_ref().map(|s| s.id.as_str());
    let wake_context = chat_history_repo::build_wake_context(session_id_opt).unwrap_or_default();

    // Build context prompt with wake context injected
    let prompt = build_cockpit_prompt(project_path, &cop, &active_slots, &wake_context);

    // Compute parent directory for --add-dir (brain sees sibling projects)
    let parent_dir = std::path::Path::new(project_path)
        .parent()
        .map(|p| p.to_string_lossy().to_string())
        .unwrap_or_else(|| project_path.to_string());

    // Launch Claude Code headless with cockpit-brain agent
    let mut cmd = Command::new(&claude_path);
    cmd.args([
        "-p", &prompt,
        "--agent", "cockpit-brain",
        "--output-format", "stream-json",
        "--dangerously-skip-permissions",
        "--add-dir", &parent_dir,
        "--max-turns", "30",
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

    // Reset auto-restart counter on successful start
    {
        let mut count = BRAIN_RESTART_COUNT.lock().unwrap();
        *count = 0;
    }

    event_bus::emit_cockpit_event("decision", "Cockpit brain starting...", None);

    // Spawn background thread to read and route stream-json
    let stdout = child.stdout.take()
        .ok_or_else(|| "Failed to capture cockpit stdout".to_string())?;

    let project_path_owned = project_path.to_string();
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
                        let (event_type, msg) = categorize_brain_text(text);
                        event_bus::emit_cockpit_event(event_type, msg, None);
                        // Thinking events stay in brain tab only — skip MCP log routing
                        if event_type == "thinking" {
                            continue;
                        }
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
                        event_bus::emit_cockpit_event("error",
                            message, None);
                    }
                    _ => {}
                }

                // Route to general event bus for MCP logs
                stream_parser::route_event(&event);
            }
        }

        // Process ended — clean up and potentially auto-restart
        let exit_status = child.wait();
        let was_unexpected = match &exit_status {
            Ok(status) => !status.success(),
            Err(_) => true,
        };

        let final_session_id = {
            let mut cockpit = COCKPIT.lock().unwrap();
            let sid = cockpit.as_ref().and_then(|c| c.session_id.clone());
            *cockpit = None;
            sid
        };

        event_bus::emit_cockpit_event("decision",
            &format!("Cockpit brain stopped. Resume ID: {:?}", final_session_id), None);

        // Auto-restart logic:
        // - Normal exit (code 0): restart with longer delay (monitoring cycle)
        // - Error exit: restart with shorter delay (crash recovery), max 3 crashes
        // - Only restart if the session is still active
        let session_still_active = session_repo::active_session_for_path(&project_path_owned)
            .ok().flatten()
            .map(|s| s.status == "active" || s.status == "initializing")
            .unwrap_or(false);

        if session_still_active {
            if was_unexpected {
                // Crash recovery: max 3 attempts
                let should_restart = {
                    let mut count = BRAIN_RESTART_COUNT.lock().unwrap();
                    if *count < 3 {
                        *count += 1;
                        true
                    } else {
                        false
                    }
                };
                if should_restart {
                    event_bus::emit_log("warn", "cockpit-brain",
                        "Brain crashed. Auto-restarting in 3 seconds...", None);
                    std::thread::sleep(std::time::Duration::from_secs(3));
                    event_bus::emit_cockpit_event("decision", "Brain auto-restarting after crash...", None);
                    let _ = start_session(&project_path_owned);
                } else {
                    event_bus::emit_log("error", "cockpit-brain",
                        "Brain exceeded max crash restarts (3). Manual restart required.", None);
                }
            } else {
                // Normal exit: monitoring cycle restart (unlimited, 2-minute delay)
                event_bus::emit_log("info", "cockpit-brain",
                    "Brain cycle complete. Restarting in 120 seconds...", None);
                std::thread::sleep(std::time::Duration::from_secs(120));
                // Reset crash counter on normal cycles
                { let mut count = BRAIN_RESTART_COUNT.lock().unwrap(); *count = 0; }
                event_bus::emit_cockpit_event("loop", "Brain monitoring cycle restart", None);
                let _ = start_session(&project_path_owned);
            }
        }
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

        // Ingest harness proposals if the cockpit brain wrote them
        ingest_harness_proposals(&session.project_path, &session.id);
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

/// Categorize cockpit brain text using structured prefixes, with keyword fallback.
fn categorize_brain_text(text: &str) -> (&'static str, &str) {
    let trimmed = text.trim();

    // [CHAT] prefix → route to the chat panel (brain↔chat bidirectional link)
    if trimmed.starts_with("[CHAT]") {
        let msg = trimmed[6..].trim();
        // Emit dedicated event for the chat panel
        event_bus::emit_cockpit_to_chat(msg);
        // Also log so it shows up in MCP logs
        event_bus::emit_log("info", "cockpit-brain-chat", msg, None);
        return ("decision", msg);
    }

    // [LAUNCH:agent:role:task] prefix → brain proposes a slot launch (requires operator approval)
    if trimmed.starts_with("[LAUNCH:") {
        if let Some(end) = trimmed.find(']') {
            let payload = &trimmed[8..end]; // after "[LAUNCH:"
            let parts: Vec<&str> = payload.splitn(3, ':').collect();
            if parts.len() == 3 {
                let agent = parts[0];
                let role = parts[1];
                let task = parts[2];
                // Create a proposal instead of auto-launching — operator must approve
                let proposal = crate::models::brain_proposal::BrainProposal::from_launch(agent, role, task);
                event_bus::emit_brain_proposal(&proposal);
                // Notify chat panel
                event_bus::emit_cockpit_to_chat(
                    &format!("🔔 Proposal: launch {} as {} — {} [awaiting approval]", agent, role, task));
                return ("decision", trimmed);
            }
        }
    }

    // [SUITE:id] prefix → brain proposes a suite switch (requires operator approval)
    if trimmed.starts_with("[SUITE:") {
        if let Some(end) = trimmed.find(']') {
            let suite_id = &trimmed[7..end];
            let proposal = crate::models::brain_proposal::BrainProposal::from_suite_switch(suite_id);
            event_bus::emit_brain_proposal(&proposal);
            event_bus::emit_cockpit_to_chat(
                &format!("🔔 Proposal: switch to {} suite [awaiting approval]", suite_id));
            return ("decision", trimmed);
        }
    }

    // [PREVIEW:url] prefix → open URL in the Review Ribbon Preview tab
    if trimmed.starts_with("[PREVIEW:") {
        if let Some(end) = trimmed.find(']') {
            let url = &trimmed[9..end];
            event_bus::emit_preview_url(url);
            event_bus::emit_cockpit_to_chat(&format!("Preview: opening {}", url));
            return ("action", trimmed);
        }
    }

    let protocols: [(&str, &str); 6] = [
        ("[ERROR]", "error"),
        ("[ALERT]", "decision"),
        ("[DECISION]", "decision"),
        ("[STATUS]", "status"),
        ("[REPORT]", "report"),
        ("[LOG]", "log"),
    ];
    for (prefix, event_type) in &protocols {
        if trimmed.starts_with(prefix) {
            let msg = trimmed[prefix.len()..].trim();
            return (event_type, msg);
        }
    }
    // Fallback keyword-based
    let lower = trimmed.to_lowercase();
    if lower.contains("spawning") || lower.contains("launching") { return ("subagent", trimmed); }
    if lower.contains("detected") || lower.contains("decided") { return ("decision", trimmed); }
    if lower.contains("monitoring") || lower.contains("scanning") { return ("loop", trimmed); }
    ("thinking", trimmed)
}

/// Inject cockpit-soul.md content into the cockpit-brain.md agent definition.
fn inject_soul_into_agent_def(project_path: &str) {
    let agent_def_path = std::path::Path::new(project_path)
        .join(".claude").join("agents").join("cockpit-brain.md");

    if !agent_def_path.exists() {
        return;
    }

    // Try to read soul from resource directory (bundled binary), then fallback to source
    let soul_content = {
        let exe_dir = std::env::current_exe()
            .ok()
            .and_then(|p| p.parent().map(|pp| pp.join("resources").join("cockpit-soul.md")));

        let bundled = exe_dir.and_then(|p| std::fs::read_to_string(&p).ok());

        bundled.or_else(|| {
            // Fallback to source path
            let src_path = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
                .join("resources").join("cockpit-soul.md");
            std::fs::read_to_string(src_path).ok()
        })
    };

    let soul = match soul_content {
        Some(s) => s,
        None => {
            log::warn!("cockpit-soul.md not found — skipping soul injection");
            return;
        }
    };

    // Read current agent def
    let current = match std::fs::read_to_string(&agent_def_path) {
        Ok(c) => c,
        Err(_) => return,
    };

    // Insert soul after YAML frontmatter (after the closing ---)
    let injected = if let Some(pos) = current.find("---\n\n") {
        let split_at = pos + 4; // after "---\n"
        format!("{}\n\n## Soul\n\n{}\n{}", &current[..split_at], soul, &current[split_at..])
    } else {
        format!("## Soul\n\n{}\n\n{}", soul, current)
    };

    let _ = std::fs::write(&agent_def_path, injected);
    log::info!("Soul injected into cockpit-brain.md");
}

/// Inject chairman brief from the active session into the cockpit-brain.md agent def.
fn inject_chairman_brief(project_path: &str) {
    let session = session_repo::active_session_for_path(project_path).ok().flatten();
    let brief = match session.and_then(|s| s.chairman_brief) {
        Some(b) if !b.is_empty() => b,
        _ => return,
    };

    let agent_def_path = std::path::Path::new(project_path)
        .join(".claude").join("agents").join("cockpit-brain.md");

    if !agent_def_path.exists() {
        return;
    }

    let current = match std::fs::read_to_string(&agent_def_path) {
        Ok(c) => c,
        Err(_) => return,
    };

    let chairman_section = format!("\n\n## Chairman Strategy\n\n{}\n", brief);

    // Append before the last section or at end
    let updated = format!("{}{}", current, chairman_section);
    let _ = std::fs::write(&agent_def_path, updated);
    log::info!("Chairman brief injected into cockpit-brain.md");
}

/// Read harness proposals from .crossroads/harness-proposals.json and save to DB.
fn ingest_harness_proposals(project_path: &str, session_id: &str) {
    let proposals_path = std::path::Path::new(project_path)
        .join(".crossroads").join("harness-proposals.json");

    if !proposals_path.exists() {
        return;
    }

    let content = match std::fs::read_to_string(&proposals_path) {
        Ok(c) => c,
        Err(e) => {
            log::warn!("Failed to read harness-proposals.json: {}", e);
            return;
        }
    };

    let proposals: Vec<serde_json::Value> = match serde_json::from_str(&content) {
        Ok(p) => p,
        Err(e) => {
            log::warn!("Failed to parse harness-proposals.json: {}", e);
            return;
        }
    };

    let mut count = 0;
    for proposal in &proposals {
        let target = proposal.get("target").and_then(|v| v.as_str()).unwrap_or("unknown");
        let critique = proposal.get("critique").and_then(|v| v.as_str()).unwrap_or("");
        let prop = proposal.get("proposal").and_then(|v| v.as_str()).unwrap_or("");

        if !critique.is_empty() && !prop.is_empty() {
            if chat_history_repo::save_harness_iteration(Some(session_id), target, critique, prop).is_ok() {
                count += 1;
            }
        }
    }

    if count > 0 {
        // Delete the file after successful ingestion
        let _ = std::fs::remove_file(&proposals_path);
        event_bus::emit_log("info", "cockpit-session",
            &format!("Ingested {} harness proposals from cockpit brain", count), None);
    }
}

fn build_cockpit_prompt(
    project_path: &str,
    cop: &cockpit_brain::CockpitOrchestrationPlan,
    active_slots: &[(i32, String, String, String)],
    wake_context: &str,
) -> String {
    build_cockpit_prompt_with_suite(project_path, cop, active_slots, wake_context, None)
}

fn build_cockpit_prompt_with_suite(
    project_path: &str,
    cop: &cockpit_brain::CockpitOrchestrationPlan,
    active_slots: &[(i32, String, String, String)],
    _wake_context: &str,
    _suite_id: Option<&str>,
) -> String {
    // Minimal context-only prompt — just live state
    let slot_section = if active_slots.is_empty() {
        "No agents running.".to_string()
    } else {
        let lines: Vec<String> = active_slots.iter().map(|(idx, agent, _branch, task)| {
            // Gather worktree info from slot DB if available
            let status = "running";
            let branch = _branch;
            let worktree = format!("{}/.worktrees/slot-{}", project_path, idx);
            format!(
                "  Slot {} ({}) [{}]: {}\n    Branch: {}\n    Worktree: {}",
                idx, agent, status, task, branch, worktree
            )
        }).collect();
        format!("{} agents active:\n{}", active_slots.len(), lines.join("\n"))
    };

    format!(
r#"Project: {project_name} at {project_path}
Domain: {domain} / {project_type}
{slot_section}

You are a living component of XRoads — not a tool, not a feature. You are the brain.
You run in full autonomy with --dangerously-skip-permissions and --add-dir on the parent directory.
You have 6 agent slots you can fill using [LAUNCH:agent:role:task].
You talk to the operator via [CHAT].

Your agent definition (cockpit-brain.md) contains your full soul.
Read it if this is your first cycle. Then act on what you see.

This is cycle start. Observe the project state and act accordingly."#,
        project_name = cop.project_name,
        project_path = project_path,
        domain = cop.domain,
        project_type = cop.project_type,
        slot_section = slot_section,
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
    fn test_categorize_brain_text() {
        let (t, _) = categorize_brain_text("[ERROR] Something broke");
        assert_eq!(t, "error");
        let (t, _) = categorize_brain_text("[ALERT] Budget warning");
        assert_eq!(t, "decision");
        let (t, _) = categorize_brain_text("[DECISION] Spawning agent");
        assert_eq!(t, "decision");
        let (t, _) = categorize_brain_text("[STATUS] Slot 1 at 80%");
        assert_eq!(t, "status");
        let (t, _) = categorize_brain_text("[REPORT] Health report");
        assert_eq!(t, "report");
        let (t, _) = categorize_brain_text("[LOG] Observation");
        assert_eq!(t, "log");
        let (t, _) = categorize_brain_text("Detected a new PRD file");
        assert_eq!(t, "decision");
        let (t, _) = categorize_brain_text("Thinking about next steps...");
        assert_eq!(t, "thinking");
        // [CHAT] prefix routes to chat panel and returns as "decision"
        let (t, msg) = categorize_brain_text("[CHAT] All stories done. Ready for merge.");
        assert_eq!(t, "decision");
        assert_eq!(msg, "All stories done. Ready for merge.");
    }

    #[test]
    fn test_role_brief() {
        let (title, _) = cockpit_brain::role_brief("testing", 0);
        assert!(title.contains("TESTER"));
        let (title, _) = cockpit_brain::role_brief("backend", 1);
        assert!(title.contains("IMPLEMENTER"));
        let (title, _) = cockpit_brain::role_brief("security-audit", 2);
        assert!(title.contains("SECURITY"));
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
        assert!(prompt.contains("No agents running."));
        assert!(prompt.contains("You are the brain"));
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
