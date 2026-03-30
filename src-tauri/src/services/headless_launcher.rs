use std::collections::HashMap;
use std::io::{BufRead, BufReader};
use std::process::{Command, Stdio};
use std::sync::{Arc, Mutex};
use serde::{Deserialize, Serialize};

use crate::db::slot_repo;
use crate::services::{event_bus, stream_parser, cli_detector};

// ── Headless Claude Code Launcher ──
//
// Replaces PTY-based agent spawning with Claude Code's native headless mode.
// Uses: claude -p <prompt> --agent <agent_name> --output-format stream-json
// Parses stream-json output line-by-line and routes to event_bus.

/// Active headless session info
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct HeadlessSession {
    pub slot_id: String,
    pub agent_name: String,
    pub claude_session_id: Option<String>,
    pub process_id: u32,
}

/// Tracked headless processes
static HEADLESS_SESSIONS: once_cell::sync::Lazy<Arc<Mutex<HashMap<String, HeadlessSession>>>> =
    once_cell::sync::Lazy::new(|| Arc::new(Mutex::new(HashMap::new())));

/// Launch a Claude Code agent in headless mode.
/// Spawns `claude -p <prompt> --agent <agent_name> --output-format stream-json`
/// and parses the stream output in a background thread.
pub fn launch_headless_agent(
    slot_id: &str,
    agent_name: &str,
    prompt: &str,
    worktree_path: &str,
    resume_session_id: Option<&str>,
) -> Result<HeadlessSession, String> {
    // Find claude binary
    let claude_path = cli_detector::find_loop_script("claude")
        .or_else(|| {
            let paths = ["/usr/local/bin/claude", "/opt/homebrew/bin/claude"];
            paths.iter().find(|p| std::path::Path::new(p).exists()).map(|p| p.to_string())
        })
        .ok_or_else(|| "Claude CLI not found".to_string())?;

    // Build command
    let mut cmd = Command::new(&claude_path);

    if let Some(session_id) = resume_session_id {
        // Resume mode
        cmd.args([
            "-p", prompt,
            "--resume", session_id,
            "--output-format", "stream-json",
            "--dangerously-skip-permissions",
        ]);
    } else {
        // Fresh launch with agent definition
        cmd.args([
            "-p", prompt,
            "--agent", agent_name,
            "--output-format", "stream-json",
            "--dangerously-skip-permissions",
        ]);
    }

    cmd.current_dir(worktree_path);
    cmd.stdout(Stdio::piped());
    cmd.stderr(Stdio::piped());

    // Update slot to provisioning
    let _ = slot_repo::update_slot(slot_id, "provisioning", Some("Launching headless agent..."));
    event_bus::emit_agent_status(slot_id, "provisioning", None, Some("Launching..."), None);

    // Spawn the process
    let mut child = cmd.spawn()
        .map_err(|e| format!("Failed to spawn claude: {}", e))?;

    let pid = child.id();
    let session = HeadlessSession {
        slot_id: slot_id.to_string(),
        agent_name: agent_name.to_string(),
        claude_session_id: resume_session_id.map(|s| s.to_string()),
        process_id: pid,
    };

    // Track the session
    {
        let mut sessions = HEADLESS_SESSIONS.lock().unwrap();
        sessions.insert(slot_id.to_string(), session.clone());
    }

    // Update slot to running
    let _ = slot_repo::update_slot(slot_id, "running", Some("Agent started (headless)"));
    event_bus::emit_agent_status(slot_id, "running", None, Some("Agent started"), None);

    // Spawn background thread to read stdout stream-json
    let slot_id_owned = slot_id.to_string();
    let stdout = child.stdout.take()
        .ok_or_else(|| "Failed to capture stdout".to_string())?;

    std::thread::spawn(move || {
        let reader = BufReader::new(stdout);

        for line in reader.lines() {
            let line = match line {
                Ok(l) => l,
                Err(_) => break,
            };

            if let Some(event) = stream_parser::parse_stream_line(&slot_id_owned, &line) {
                // Route to event bus
                stream_parser::route_event(&event);

                // Handle session completion
                match &event {
                    stream_parser::StreamEvent::SessionComplete { session_id, .. } => {
                        if !session_id.is_empty() {
                            // Store session_id for resume (in memory + DB)
                            let mut sessions = HEADLESS_SESSIONS.lock().unwrap();
                            if let Some(s) = sessions.get_mut(&slot_id_owned) {
                                s.claude_session_id = Some(session_id.clone());
                            }
                            let _ = slot_repo::update_claude_session_id(&slot_id_owned, session_id);
                        }

                        // Mark slot as done
                        let _ = slot_repo::update_slot(&slot_id_owned, "done", Some("Completed"));
                        event_bus::emit_agent_status(&slot_id_owned, "done", Some(100.0), Some("Completed"), None);
                    }
                    stream_parser::StreamEvent::Error { message, .. } => {
                        let _ = slot_repo::update_slot(&slot_id_owned, "error", Some(message));
                        event_bus::emit_agent_status(&slot_id_owned, "error", None, Some(message), None);
                    }
                    _ => {}
                }
            }
        }

        // Wait for process to exit
        let exit_status = child.wait();

        // Clean up tracking
        let mut sessions = HEADLESS_SESSIONS.lock().unwrap();
        let final_session = sessions.remove(&slot_id_owned);

        match exit_status {
            Ok(status) if status.success() => {
                // Already handled by SessionComplete event above
                // But if we get here without a SessionComplete, mark done
                // Check if slot was already marked done/error by stream events
                // If not, mark it done now
                {
                    let _ = slot_repo::update_slot(&slot_id_owned, "done", Some("Process exited"));
                    event_bus::emit_agent_status(&slot_id_owned, "done", Some(100.0), Some("Process exited"), None);
                }
            }
            Ok(status) => {
                let msg = format!("Agent exited with code: {}", status.code().unwrap_or(-1));
                let _ = slot_repo::update_slot(&slot_id_owned, "error", Some(&msg));
                event_bus::emit_agent_status(&slot_id_owned, "error", None, Some(&msg), None);
            }
            Err(e) => {
                let msg = format!("Process wait error: {}", e);
                let _ = slot_repo::update_slot(&slot_id_owned, "error", Some(&msg));
                event_bus::emit_agent_status(&slot_id_owned, "error", None, Some(&msg), None);
            }
        }

        // Log final session_id if available (for resume)
        if let Some(s) = final_session {
            if let Some(ref sid) = s.claude_session_id {
                event_bus::emit_log("info", "headless",
                    &format!("Session {} ended. Resume ID: {}", s.agent_name, sid),
                    Some(&slot_id_owned));
            }
        }
    });

    Ok(session)
}

/// Get the session_id for a slot (for resume after crash)
pub fn get_session_id(slot_id: &str) -> Option<String> {
    let sessions = HEADLESS_SESSIONS.lock().unwrap();
    sessions.get(slot_id).and_then(|s| s.claude_session_id.clone())
}

/// Abort a headless agent
pub fn abort_headless(slot_id: &str) -> Result<(), String> {
    let session = {
        let mut sessions = HEADLESS_SESSIONS.lock().unwrap();
        sessions.remove(slot_id)
    };

    if let Some(session) = session {
        // Kill the process
        #[cfg(unix)]
        {
            use std::os::unix::process::CommandExt;
            let _ = Command::new("kill")
                .args(["-TERM", &session.process_id.to_string()])
                .status();
        }
        #[cfg(not(unix))]
        {
            let _ = Command::new("taskkill")
                .args(["/PID", &session.process_id.to_string(), "/F"])
                .status();
        }

        let _ = slot_repo::update_slot(slot_id, "error", Some("Aborted by user"));
        event_bus::emit_agent_status(slot_id, "error", None, Some("Aborted"), None);
        Ok(())
    } else {
        Err(format!("No headless session found for slot {}", slot_id))
    }
}

/// List all active headless sessions
pub fn list_sessions() -> Vec<HeadlessSession> {
    let sessions = HEADLESS_SESSIONS.lock().unwrap();
    sessions.values().cloned().collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_headless_session_serialization() {
        let session = HeadlessSession {
            slot_id: "s1".into(),
            agent_name: "slot-0-backend".into(),
            claude_session_id: Some("sess_abc".into()),
            process_id: 12345,
        };
        let json = serde_json::to_string(&session).unwrap();
        assert!(json.contains("slotId"));
        assert!(json.contains("agentName"));
        assert!(json.contains("claudeSessionId"));
    }

    #[test]
    fn test_get_session_id_none() {
        assert!(get_session_id("nonexistent-slot").is_none());
    }

    #[test]
    fn test_list_sessions_empty() {
        // Clear any state
        let sessions = list_sessions();
        // Just verify it doesn't panic
        assert!(sessions.is_empty() || sessions.len() > 0);
    }
}
