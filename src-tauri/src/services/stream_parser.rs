use serde::{Deserialize, Serialize};

// ── Claude Code stream-json Event Parser ──
//
// Parses the line-by-line JSON output from:
//   claude -p <prompt> --output-format stream-json
//
// Each line is a self-contained JSON object with a `type` field.

/// Parsed event from Claude Code stream-json output
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum StreamEvent {
    /// Text content from the assistant
    Text { slot_id: String, text: String },
    /// Tool use (Edit, Write, Bash, etc.)
    ToolUse { slot_id: String, tool_name: String, tool_id: String, input: serde_json::Value },
    /// Tool result after execution
    ToolResult { slot_id: String, tool_use_id: String, success: bool, output: String },
    /// Session completed with session_id for resume
    SessionComplete { slot_id: String, session_id: String, cost: Option<CostInfo> },
    /// Error from Claude Code
    Error { slot_id: String, message: String },
    /// System/status message
    System { slot_id: String, message: String },
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CostInfo {
    pub input_tokens: i64,
    pub output_tokens: i64,
}

/// Parse a single line of stream-json output into a StreamEvent.
/// Returns None for unrecognized or empty lines.
pub fn parse_stream_line(slot_id: &str, line: &str) -> Option<StreamEvent> {
    let line = line.trim();
    if line.is_empty() {
        return None;
    }

    let json: serde_json::Value = serde_json::from_str(line).ok()?;
    let event_type = json.get("type")?.as_str()?;

    match event_type {
        // Assistant message with text content
        "assistant" => {
            let message = json.get("message")?;
            let content = message.get("content")?.as_array()?;
            let mut text = String::new();
            for block in content {
                if block.get("type")?.as_str()? == "text" {
                    if let Some(t) = block.get("text").and_then(|t| t.as_str()) {
                        text.push_str(t);
                    }
                }
            }
            if text.is_empty() {
                return None;
            }
            Some(StreamEvent::Text {
                slot_id: slot_id.to_string(),
                text,
            })
        }

        // Content block delta (streaming text chunks)
        "content_block_delta" => {
            let delta = json.get("delta")?;
            let text = delta.get("text").and_then(|t| t.as_str())?;
            Some(StreamEvent::Text {
                slot_id: slot_id.to_string(),
                text: text.to_string(),
            })
        }

        // Tool use request
        "tool_use" => {
            let name = json.get("name").and_then(|n| n.as_str()).unwrap_or("unknown");
            let id = json.get("id").and_then(|i| i.as_str()).unwrap_or("");
            let input = json.get("input").cloned().unwrap_or(serde_json::Value::Null);
            Some(StreamEvent::ToolUse {
                slot_id: slot_id.to_string(),
                tool_name: name.to_string(),
                tool_id: id.to_string(),
                input,
            })
        }

        // Tool result
        "tool_result" => {
            let tool_use_id = json.get("tool_use_id").and_then(|i| i.as_str()).unwrap_or("").to_string();
            let content = json.get("content").and_then(|c| c.as_str()).unwrap_or("").to_string();
            let is_error = json.get("is_error").and_then(|e| e.as_bool()).unwrap_or(false);
            Some(StreamEvent::ToolResult {
                slot_id: slot_id.to_string(),
                tool_use_id,
                success: !is_error,
                output: content,
            })
        }

        // Message stop — contains session_id for resume
        "message_stop" => {
            let session_id = json.get("message")
                .and_then(|m| m.get("session_id"))
                .and_then(|s| s.as_str())
                .or_else(|| json.get("session_id").and_then(|s| s.as_str()))
                .unwrap_or("")
                .to_string();

            let cost = json.get("message")
                .and_then(|m| m.get("usage"))
                .and_then(|u| {
                    let input = u.get("input_tokens").and_then(|t| t.as_i64()).unwrap_or(0);
                    let output = u.get("output_tokens").and_then(|t| t.as_i64()).unwrap_or(0);
                    Some(CostInfo { input_tokens: input, output_tokens: output })
                });

            Some(StreamEvent::SessionComplete {
                slot_id: slot_id.to_string(),
                session_id,
                cost,
            })
        }

        // Result event (final)
        "result" => {
            let session_id = json.get("session_id")
                .and_then(|s| s.as_str())
                .unwrap_or("")
                .to_string();

            let cost = json.get("usage").and_then(|u| {
                let input = u.get("input_tokens").and_then(|t| t.as_i64()).unwrap_or(0);
                let output = u.get("output_tokens").and_then(|t| t.as_i64()).unwrap_or(0);
                Some(CostInfo { input_tokens: input, output_tokens: output })
            });

            Some(StreamEvent::SessionComplete {
                slot_id: slot_id.to_string(),
                session_id,
                cost,
            })
        }

        // Error
        "error" => {
            let message = json.get("error")
                .and_then(|e| e.get("message"))
                .and_then(|m| m.as_str())
                .or_else(|| json.get("message").and_then(|m| m.as_str()))
                .unwrap_or("Unknown error")
                .to_string();
            Some(StreamEvent::Error {
                slot_id: slot_id.to_string(),
                message,
            })
        }

        // System messages
        "system" => {
            let message = json.get("message").and_then(|m| m.as_str()).unwrap_or("").to_string();
            Some(StreamEvent::System {
                slot_id: slot_id.to_string(),
                message,
            })
        }

        _ => None,
    }
}

/// Route a StreamEvent to the XRoads event bus
pub fn route_event(event: &StreamEvent) {
    use crate::services::event_bus;

    match event {
        StreamEvent::Text { slot_id, text } => {
            event_bus::emit_log("debug", "agent-text", text, Some(slot_id));
        }
        StreamEvent::ToolUse { slot_id, tool_name, input, .. } => {
            let file_hint = input.get("file_path")
                .or_else(|| input.get("command"))
                .and_then(|v| v.as_str())
                .unwrap_or("");
            let msg = if file_hint.is_empty() {
                format!("[{}]", tool_name)
            } else {
                format!("[{}] {}", tool_name, file_hint)
            };
            event_bus::emit_log("info", "agent-tool", &msg, Some(slot_id));
        }
        StreamEvent::ToolResult { slot_id, success, output, .. } => {
            let level = if *success { "debug" } else { "warn" };
            let msg = if output.len() > 200 {
                format!("{}...", &output[..200])
            } else {
                output.clone()
            };
            event_bus::emit_log(level, "agent-tool-result", &msg, Some(slot_id));
        }
        StreamEvent::SessionComplete { slot_id, session_id, cost } => {
            let msg = match cost {
                Some(c) => format!("Session complete: {} ({}in/{}out tokens)", session_id, c.input_tokens, c.output_tokens),
                None => format!("Session complete: {}", session_id),
            };
            event_bus::emit_log("info", "agent-session", &msg, Some(slot_id));
        }
        StreamEvent::Error { slot_id, message } => {
            event_bus::emit_log("error", "agent-error", message, Some(slot_id));
        }
        StreamEvent::System { slot_id, message } => {
            event_bus::emit_log("info", "agent-system", message, Some(slot_id));
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_assistant_text() {
        let line = r#"{"type":"assistant","message":{"content":[{"type":"text","text":"Hello world"}]}}"#;
        let event = parse_stream_line("slot-0", line).unwrap();
        match event {
            StreamEvent::Text { slot_id, text } => {
                assert_eq!(slot_id, "slot-0");
                assert_eq!(text, "Hello world");
            }
            _ => panic!("Expected Text event"),
        }
    }

    #[test]
    fn test_parse_content_block_delta() {
        let line = r#"{"type":"content_block_delta","delta":{"text":"chunk "}}"#;
        let event = parse_stream_line("s1", line).unwrap();
        match event {
            StreamEvent::Text { text, .. } => assert_eq!(text, "chunk "),
            _ => panic!("Expected Text event"),
        }
    }

    #[test]
    fn test_parse_tool_use() {
        let line = r#"{"type":"tool_use","id":"tu_123","name":"Edit","input":{"file_path":"src/main.rs"}}"#;
        let event = parse_stream_line("s1", line).unwrap();
        match event {
            StreamEvent::ToolUse { tool_name, tool_id, input, .. } => {
                assert_eq!(tool_name, "Edit");
                assert_eq!(tool_id, "tu_123");
                assert_eq!(input["file_path"], "src/main.rs");
            }
            _ => panic!("Expected ToolUse event"),
        }
    }

    #[test]
    fn test_parse_tool_result() {
        let line = r#"{"type":"tool_result","tool_use_id":"tu_123","content":"OK","is_error":false}"#;
        let event = parse_stream_line("s1", line).unwrap();
        match event {
            StreamEvent::ToolResult { success, output, .. } => {
                assert!(success);
                assert_eq!(output, "OK");
            }
            _ => panic!("Expected ToolResult event"),
        }
    }

    #[test]
    fn test_parse_result_with_session_id() {
        let line = r#"{"type":"result","session_id":"sess_abc123","usage":{"input_tokens":100,"output_tokens":50}}"#;
        let event = parse_stream_line("s1", line).unwrap();
        match event {
            StreamEvent::SessionComplete { session_id, cost, .. } => {
                assert_eq!(session_id, "sess_abc123");
                let c = cost.unwrap();
                assert_eq!(c.input_tokens, 100);
                assert_eq!(c.output_tokens, 50);
            }
            _ => panic!("Expected SessionComplete event"),
        }
    }

    #[test]
    fn test_parse_error() {
        let line = r#"{"type":"error","error":{"message":"Rate limited"}}"#;
        let event = parse_stream_line("s1", line).unwrap();
        match event {
            StreamEvent::Error { message, .. } => assert_eq!(message, "Rate limited"),
            _ => panic!("Expected Error event"),
        }
    }

    #[test]
    fn test_parse_empty_line() {
        assert!(parse_stream_line("s1", "").is_none());
        assert!(parse_stream_line("s1", "   ").is_none());
    }

    #[test]
    fn test_parse_invalid_json() {
        assert!(parse_stream_line("s1", "not json").is_none());
    }

    #[test]
    fn test_parse_unknown_type() {
        let line = r#"{"type":"unknown_event","data":"foo"}"#;
        assert!(parse_stream_line("s1", line).is_none());
    }
}
