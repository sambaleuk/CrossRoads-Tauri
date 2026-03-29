use std::collections::HashMap;
use std::io::{BufRead, BufReader, Write};
use std::path::{Path, PathBuf};
use std::process::{Child, Command, Stdio};
use std::sync::{Arc, Mutex};
use std::sync::atomic::{AtomicU64, Ordering};
use serde::{Deserialize, Serialize};
use serde_json::Value;

// ── US-001: MCP Server Spawner ──

/// Detect the node binary path, including NVM-managed installations
pub fn detect_node_path() -> Result<String, String> {
    // Try direct `which node`
    if let Ok(output) = Command::new("which").arg("node").output() {
        if output.status.success() {
            let path = String::from_utf8_lossy(&output.stdout).trim().to_string();
            if !path.is_empty() {
                return Ok(path);
            }
        }
    }

    // Try NVM default
    let home = std::env::var("HOME").unwrap_or_default();
    let nvm_paths = [
        format!("{}/.nvm/versions/node", home),
        format!("{}/.volta/bin", home),
        format!("{}/.local/share/fnm/node-versions", home),
    ];

    for nvm_dir in &nvm_paths {
        let p = Path::new(nvm_dir);
        if p.exists() {
            if nvm_dir.contains(".volta") || nvm_dir.contains("fnm") {
                let candidate = p.join("node");
                if candidate.exists() {
                    return Ok(candidate.to_string_lossy().to_string());
                }
            } else {
                // NVM: find latest version dir
                if let Ok(entries) = std::fs::read_dir(p) {
                    let mut versions: Vec<PathBuf> = entries
                        .filter_map(|e| e.ok())
                        .map(|e| e.path())
                        .filter(|p| p.is_dir())
                        .collect();
                    versions.sort();
                    if let Some(latest) = versions.last() {
                        let bin = latest.join("bin/node");
                        if bin.exists() {
                            return Ok(bin.to_string_lossy().to_string());
                        }
                    }
                }
            }
        }
    }

    Err("Node.js not found. Install Node.js or NVM.".into())
}

/// Search for the xroads-mcp server script
pub fn find_mcp_server(project_root: Option<&str>) -> Result<String, String> {
    let candidates: Vec<PathBuf> = {
        let mut v = Vec::new();
        // 1. Project root
        if let Some(root) = project_root {
            v.push(PathBuf::from(root).join("node_modules/.bin/xroads-mcp"));
            v.push(PathBuf::from(root).join("xroads-mcp/dist/index.js"));
            v.push(PathBuf::from(root).join(".crossroads/mcp-server/index.js"));
        }
        // 2. Global install
        let home = std::env::var("HOME").unwrap_or_default();
        v.push(PathBuf::from(format!("{}/.local/bin/xroads-mcp", home)));
        v.push(PathBuf::from("/usr/local/bin/xroads-mcp"));
        // 3. Bundled resources (Tauri)
        v.push(PathBuf::from("../resources/mcp-server/index.js"));
        v
    };

    for path in &candidates {
        if path.exists() {
            return Ok(path.to_string_lossy().to_string());
        }
    }

    Err(format!("xroads-mcp server not found. Searched: {:?}", candidates.iter().map(|p| p.display().to_string()).collect::<Vec<_>>()))
}

/// A spawned MCP server process with stdio pipes
pub struct McpServerProcess {
    child: Child,
    stdin_writer: std::process::ChildStdin,
    stdout_reader: BufReader<std::process::ChildStdout>,
}

/// Spawn the MCP server as a child process
pub fn spawn_mcp_server(node_path: &str, server_path: &str, env: Option<HashMap<String, String>>) -> Result<McpServerProcess, String> {
    let mut cmd = Command::new(node_path);
    cmd.arg(server_path)
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped());

    if let Some(vars) = env {
        for (k, v) in vars {
            cmd.env(k, v);
        }
    }

    let mut child = cmd.spawn().map_err(|e| format!("Failed to spawn MCP server: {}", e))?;

    let stdin = child.stdin.take().ok_or("Failed to open MCP server stdin")?;
    let stdout = child.stdout.take().ok_or("Failed to open MCP server stdout")?;

    Ok(McpServerProcess {
        child,
        stdin_writer: stdin,
        stdout_reader: BufReader::new(stdout),
    })
}

// ── US-002: MCP JSON-RPC Client ──

/// JSON-RPC 2.0 request
#[derive(Debug, Clone, Serialize)]
pub struct JsonRpcRequest {
    pub jsonrpc: String,
    pub id: u64,
    pub method: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub params: Option<Value>,
}

/// JSON-RPC 2.0 response
#[derive(Debug, Clone, Deserialize)]
pub struct JsonRpcResponse {
    pub jsonrpc: String,
    pub id: Option<u64>,
    pub result: Option<Value>,
    pub error: Option<JsonRpcError>,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct JsonRpcError {
    pub code: i64,
    pub message: String,
    pub data: Option<Value>,
}

/// MCP Client wrapping a server process
pub struct McpClient {
    process: Mutex<McpServerProcess>,
    next_id: AtomicU64,
    server_capabilities: Mutex<Option<Value>>,
}

impl McpClient {
    pub fn new(process: McpServerProcess) -> Self {
        Self {
            process: Mutex::new(process),
            next_id: AtomicU64::new(1),
            server_capabilities: Mutex::new(None),
        }
    }

    /// Send a JSON-RPC request and read the response
    fn call_raw(&self, method: &str, params: Option<Value>) -> Result<JsonRpcResponse, String> {
        let id = self.next_id.fetch_add(1, Ordering::SeqCst);
        let request = JsonRpcRequest {
            jsonrpc: "2.0".into(),
            id,
            method: method.into(),
            params,
        };

        let mut proc = self.process.lock().unwrap();

        // Write request as a single line
        let json = serde_json::to_string(&request).map_err(|e| e.to_string())?;
        writeln!(proc.stdin_writer, "{}", json)
            .map_err(|e| format!("Failed to write to MCP server: {}", e))?;
        proc.stdin_writer.flush()
            .map_err(|e| format!("Failed to flush MCP server stdin: {}", e))?;

        // Read response line (with timeout via non-blocking would be ideal,
        // but for simplicity we use blocking read with a line-based protocol)
        let mut line = String::new();
        proc.stdout_reader.read_line(&mut line)
            .map_err(|e| format!("Failed to read from MCP server: {}", e))?;

        if line.trim().is_empty() {
            return Err("Empty response from MCP server".into());
        }

        let response: JsonRpcResponse = serde_json::from_str(line.trim())
            .map_err(|e| format!("Invalid JSON-RPC response: {} — raw: {}", e, line.trim()))?;

        // Verify response ID matches
        if response.id != Some(id) {
            return Err(format!("Response ID mismatch: expected {}, got {:?}", id, response.id));
        }

        Ok(response)
    }

    /// Initialize the MCP connection (handshake)
    pub fn initialize(&self) -> Result<Value, String> {
        let params = serde_json::json!({
            "protocolVersion": "2024-11-05",
            "capabilities": {},
            "clientInfo": {
                "name": "xroads-tauri",
                "version": env!("CARGO_PKG_VERSION")
            }
        });

        let response = self.call_raw("initialize", Some(params))?;

        if let Some(err) = response.error {
            return Err(format!("MCP initialize failed: {} (code {})", err.message, err.code));
        }

        let result = response.result.ok_or("No result in initialize response")?;

        // Store server capabilities
        if let Some(caps) = result.get("capabilities") {
            let mut stored = self.server_capabilities.lock().unwrap();
            *stored = Some(caps.clone());
        }

        // Send initialized notification (no response expected, but we send it)
        // For notifications we don't wait for a response
        {
            let mut proc = self.process.lock().unwrap();
            let notification = serde_json::json!({
                "jsonrpc": "2.0",
                "method": "notifications/initialized"
            });
            let json = serde_json::to_string(&notification).map_err(|e| e.to_string())?;
            let _ = writeln!(proc.stdin_writer, "{}", json);
            let _ = proc.stdin_writer.flush();
        }

        Ok(result)
    }

    /// Call an MCP tool
    pub fn call_tool(&self, tool_name: &str, arguments: Value) -> Result<Value, String> {
        let params = serde_json::json!({
            "name": tool_name,
            "arguments": arguments
        });

        let response = self.call_raw("tools/call", Some(params))?;

        if let Some(err) = response.error {
            return Err(format!("MCP tool '{}' failed: {} (code {})", tool_name, err.message, err.code));
        }

        response.result.ok_or(format!("No result from tool '{}'", tool_name))
    }

    /// List available tools from server
    pub fn list_tools(&self) -> Result<Value, String> {
        let response = self.call_raw("tools/list", None)?;

        if let Some(err) = response.error {
            return Err(format!("MCP tools/list failed: {} (code {})", err.message, err.code));
        }

        response.result.ok_or("No result from tools/list".into())
    }

    /// Kill the underlying server process
    pub fn shutdown(&self) -> Result<(), String> {
        let mut proc = self.process.lock().unwrap();
        proc.child.kill().map_err(|e| format!("Failed to kill MCP server: {}", e))?;
        Ok(())
    }

    /// Check if server process is still running
    pub fn is_alive(&self) -> bool {
        let mut proc = self.process.lock().unwrap();
        match proc.child.try_wait() {
            Ok(None) => true, // Still running
            _ => false,
        }
    }
}

// ── US-003: MCP Tools — emit_log + update_status ──

/// Process an emit_log tool call result and convert to a Tauri-compatible event
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct McpLogEvent {
    pub level: String, // debug, info, warn, error
    pub source: String,
    pub worktree: String,
    pub message: String,
    pub metadata: Option<Value>,
    pub timestamp: String,
}

/// Build emit_log tool arguments
pub fn build_emit_log_args(level: &str, source: &str, worktree: &str, message: &str, metadata: Option<Value>) -> Value {
    serde_json::json!({
        "level": level,
        "source": source,
        "worktree": worktree,
        "message": message,
        "metadata": metadata
    })
}

/// Process an update_status tool call result
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct McpStatusUpdate {
    pub agent: String,
    pub worktree: String,
    pub status: String,
    pub task: Option<String>,
    pub progress: Option<f64>,
    pub timestamp: String,
}

/// Build update_status tool arguments
pub fn build_update_status_args(agent: &str, worktree: &str, status: &str, task: Option<&str>, progress: Option<f64>) -> Value {
    serde_json::json!({
        "agent": agent,
        "worktree": worktree,
        "status": status,
        "task": task,
        "progress": progress
    })
}

// ── US-004: MCP Tools — Session + Handoff ──

/// Session data structure persisted to .crossroads/sessions.json
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct McpSession {
    pub session_id: String,
    pub project_path: String,
    pub agent_type: String,
    pub started_at: String,
    pub decisions: Vec<McpDecision>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct McpDecision {
    pub decision_type: String,
    pub description: String,
    pub context: Option<String>,
    pub timestamp: String,
}

/// Build register_session arguments
pub fn build_register_session_args(session_id: &str, project_path: &str, agent_type: &str) -> Value {
    serde_json::json!({
        "sessionId": session_id,
        "projectPath": project_path,
        "agentType": agent_type
    })
}

/// Build record_decision arguments
pub fn build_record_decision_args(session_id: &str, decision_type: &str, description: &str, context: Option<&str>) -> Value {
    serde_json::json!({
        "sessionId": session_id,
        "decisionType": decision_type,
        "description": description,
        "context": context
    })
}

/// Build generate_handoff arguments
pub fn build_generate_handoff_args(session_id: &str, max_tokens: Option<u32>) -> Value {
    serde_json::json!({
        "sessionId": session_id,
        "maxTokens": max_tokens.unwrap_or(4000)
    })
}

/// Persist a session to .crossroads/sessions.json
pub fn persist_session(worktree_path: &str, session: &McpSession) -> Result<(), String> {
    let sessions_dir = Path::new(worktree_path).join(".crossroads");
    std::fs::create_dir_all(&sessions_dir).map_err(|e| e.to_string())?;

    let file_path = sessions_dir.join("sessions.json");
    let mut sessions: Vec<McpSession> = if file_path.exists() {
        let content = std::fs::read_to_string(&file_path).map_err(|e| e.to_string())?;
        serde_json::from_str(&content).unwrap_or_default()
    } else {
        Vec::new()
    };

    // Update existing or append
    if let Some(existing) = sessions.iter_mut().find(|s| s.session_id == session.session_id) {
        *existing = session.clone();
    } else {
        sessions.push(session.clone());
    }

    let json = serde_json::to_string_pretty(&sessions).map_err(|e| e.to_string())?;
    std::fs::write(&file_path, json).map_err(|e| e.to_string())?;

    Ok(())
}

/// Load a session from .crossroads/sessions.json
pub fn load_session(worktree_path: &str, session_id: &str) -> Result<Option<McpSession>, String> {
    let file_path = Path::new(worktree_path).join(".crossroads/sessions.json");
    if !file_path.exists() {
        return Ok(None);
    }

    let content = std::fs::read_to_string(&file_path).map_err(|e| e.to_string())?;
    let sessions: Vec<McpSession> = serde_json::from_str(&content).map_err(|e| e.to_string())?;

    Ok(sessions.into_iter().find(|s| s.session_id == session_id))
}

/// Generate a compressed handoff document within a token budget.
/// Rough estimate: 1 token ≈ 4 chars.
pub fn generate_handoff(session: &McpSession, max_tokens: u32) -> String {
    let max_chars = (max_tokens * 4) as usize;

    let mut sections = Vec::new();
    sections.push(format!("# Handoff — {}\n", session.session_id));
    sections.push(format!("Project: {}\nAgent: {}\nStarted: {}\n",
        session.project_path, session.agent_type, session.started_at));

    if !session.decisions.is_empty() {
        sections.push("## Key Decisions\n".into());
        for d in &session.decisions {
            let entry = format!("- **{}**: {} ({})\n", d.decision_type, d.description, d.timestamp);
            sections.push(entry);
        }
    }

    let mut result = sections.join("");

    // Truncate to budget if needed
    if result.len() > max_chars {
        result.truncate(max_chars - 20);
        result.push_str("\n\n[...truncated]");
    }

    result
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_detect_node_path() {
        // This test may fail in CI without node, but should pass locally
        let result = detect_node_path();
        // We don't assert Ok because node might not be installed in test env
        if let Ok(path) = &result {
            assert!(path.contains("node"));
        }
    }

    #[test]
    fn test_json_rpc_request_serialization() {
        let req = JsonRpcRequest {
            jsonrpc: "2.0".into(),
            id: 1,
            method: "initialize".into(),
            params: Some(serde_json::json!({"key": "value"})),
        };
        let json = serde_json::to_string(&req).unwrap();
        assert!(json.contains("\"jsonrpc\":\"2.0\""));
        assert!(json.contains("\"id\":1"));
        assert!(json.contains("\"method\":\"initialize\""));
    }

    #[test]
    fn test_json_rpc_response_parsing() {
        let json = r#"{"jsonrpc":"2.0","id":1,"result":{"protocolVersion":"2024-11-05"}}"#;
        let resp: JsonRpcResponse = serde_json::from_str(json).unwrap();
        assert_eq!(resp.id, Some(1));
        assert!(resp.result.is_some());
        assert!(resp.error.is_none());
    }

    #[test]
    fn test_json_rpc_error_response_parsing() {
        let json = r#"{"jsonrpc":"2.0","id":2,"error":{"code":-32601,"message":"Method not found"}}"#;
        let resp: JsonRpcResponse = serde_json::from_str(json).unwrap();
        assert_eq!(resp.id, Some(2));
        assert!(resp.error.is_some());
        let err = resp.error.unwrap();
        assert_eq!(err.code, -32601);
        assert_eq!(err.message, "Method not found");
    }

    #[test]
    fn test_emit_log_args() {
        let args = build_emit_log_args("info", "slot-0", "/tmp/wt", "Hello", None);
        assert_eq!(args["level"], "info");
        assert_eq!(args["source"], "slot-0");
    }

    #[test]
    fn test_update_status_args() {
        let args = build_update_status_args("claude", "/tmp/wt", "running", Some("US-001"), Some(45.0));
        assert_eq!(args["agent"], "claude");
        assert_eq!(args["progress"], 45.0);
    }

    #[test]
    fn test_session_persistence() {
        let dir = format!("/tmp/xroads-mcp-test-{}", uuid::Uuid::new_v4());
        std::fs::create_dir_all(&dir).unwrap();

        let session = McpSession {
            session_id: "test-sess".into(),
            project_path: dir.clone(),
            agent_type: "claude".into(),
            started_at: "2026-03-29T10:00:00Z".into(),
            decisions: vec![
                McpDecision {
                    decision_type: "architecture".into(),
                    description: "Use layered dispatch".into(),
                    context: Some("PRD-15 requirement".into()),
                    timestamp: "2026-03-29T10:01:00Z".into(),
                },
            ],
        };

        persist_session(&dir, &session).unwrap();

        // Load it back
        let loaded = load_session(&dir, "test-sess").unwrap().unwrap();
        assert_eq!(loaded.session_id, "test-sess");
        assert_eq!(loaded.decisions.len(), 1);
        assert_eq!(loaded.decisions[0].decision_type, "architecture");

        // Not found
        let missing = load_session(&dir, "nonexistent").unwrap();
        assert!(missing.is_none());

        std::fs::remove_dir_all(&dir).ok();
    }

    #[test]
    fn test_generate_handoff() {
        let session = McpSession {
            session_id: "s1".into(),
            project_path: "/tmp/proj".into(),
            agent_type: "claude".into(),
            started_at: "2026-03-29T10:00:00Z".into(),
            decisions: vec![
                McpDecision {
                    decision_type: "fix".into(),
                    description: "Fixed auth bug".into(),
                    context: None,
                    timestamp: "2026-03-29T10:05:00Z".into(),
                },
                McpDecision {
                    decision_type: "refactor".into(),
                    description: "Split service layer".into(),
                    context: Some("Better separation".into()),
                    timestamp: "2026-03-29T10:10:00Z".into(),
                },
            ],
        };

        let handoff = generate_handoff(&session, 4000);
        assert!(handoff.contains("Handoff — s1"));
        assert!(handoff.contains("Fixed auth bug"));
        assert!(handoff.contains("Split service layer"));
    }

    #[test]
    fn test_generate_handoff_truncation() {
        let session = McpSession {
            session_id: "s1".into(),
            project_path: "/tmp/proj".into(),
            agent_type: "claude".into(),
            started_at: "2026-03-29T10:00:00Z".into(),
            decisions: (0..100).map(|i| McpDecision {
                decision_type: "decision".into(),
                description: format!("Decision number {} with some long description text to fill space", i),
                context: None,
                timestamp: "2026-03-29T10:00:00Z".into(),
            }).collect(),
        };

        let handoff = generate_handoff(&session, 100); // Very small budget: 400 chars
        assert!(handoff.len() <= 420); // 400 chars + "[...truncated]"
        assert!(handoff.contains("[...truncated]"));
    }

    #[test]
    fn test_mcp_log_event_serialization() {
        let event = McpLogEvent {
            level: "info".into(),
            source: "slot-0".into(),
            worktree: "/tmp/wt".into(),
            message: "Test message".into(),
            metadata: Some(serde_json::json!({"key": "val"})),
            timestamp: "2026-03-29T10:00:00Z".into(),
        };
        let json = serde_json::to_string(&event).unwrap();
        assert!(json.contains("\"level\":\"info\""));
        assert!(json.contains("\"source\":\"slot-0\""));
    }
}
