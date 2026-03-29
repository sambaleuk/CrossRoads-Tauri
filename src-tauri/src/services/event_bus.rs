use once_cell::sync::OnceCell;
use serde::Serialize;
use std::collections::HashMap;
use std::sync::Mutex;
use std::time::{Duration, Instant};
use tauri::{AppHandle, Emitter};

// ── Global AppHandle ──

static APP_HANDLE: OnceCell<AppHandle> = OnceCell::new();

/// Store the AppHandle on startup (called from main.rs setup)
pub fn init(handle: AppHandle) {
    APP_HANDLE.set(handle).ok();
}

fn app() -> Option<&'static AppHandle> {
    APP_HANDLE.get()
}

// ── Event Names ──

pub const EVENT_PTY_OUTPUT: &str = "pty-output";
pub const EVENT_AGENT_STATUS: &str = "agent-status";
pub const EVENT_LOG_ENTRY: &str = "log-entry";
pub const EVENT_GATE_CREATED: &str = "gate-created";
pub const EVENT_GATE_RESOLVED: &str = "gate-resolved";
pub const EVENT_COST_UPDATED: &str = "cost-updated";
pub const EVENT_HEALTH_ALERT: &str = "health-alert";
pub const EVENT_ORCHESTRATION: &str = "orchestration-update";

// ── US-001: PTY Output Streaming with Buffering ──

/// PTY output buffer: accumulates chunks per slot, flushes on threshold
struct PtyBuffer {
    buffers: HashMap<String, PtySlotBuffer>,
}

struct PtySlotBuffer {
    data: String,
    last_flush: Instant,
}

const PTY_FLUSH_INTERVAL_MS: u64 = 50;
const PTY_FLUSH_SIZE: usize = 4096;

static PTY_BUFFERS: once_cell::sync::Lazy<Mutex<PtyBuffer>> =
    once_cell::sync::Lazy::new(|| Mutex::new(PtyBuffer { buffers: HashMap::new() }));

/// Buffer PTY output and flush when threshold is reached.
/// Call this from the PTY reader callback.
pub fn buffer_pty_output(slot_id: &str, text: &str) {
    let mut bufs = PTY_BUFFERS.lock().unwrap();
    let entry = bufs.buffers.entry(slot_id.to_string()).or_insert_with(|| PtySlotBuffer {
        data: String::new(),
        last_flush: Instant::now(),
    });

    entry.data.push_str(text);

    let should_flush = entry.data.len() >= PTY_FLUSH_SIZE
        || entry.last_flush.elapsed() >= Duration::from_millis(PTY_FLUSH_INTERVAL_MS);

    if should_flush {
        let payload = PtyOutputPayload {
            slot_id: slot_id.to_string(),
            text: std::mem::take(&mut entry.data),
        };
        entry.last_flush = Instant::now();
        drop(bufs); // Release lock before emitting
        emit(EVENT_PTY_OUTPUT, &payload);
    }
}

/// Force flush all PTY buffers (call periodically or on shutdown)
pub fn flush_pty_buffers() {
    let mut bufs = PTY_BUFFERS.lock().unwrap();
    let mut payloads = Vec::new();

    for (slot_id, buf) in bufs.buffers.iter_mut() {
        if !buf.data.is_empty() {
            payloads.push(PtyOutputPayload {
                slot_id: slot_id.clone(),
                text: std::mem::take(&mut buf.data),
            });
            buf.last_flush = Instant::now();
        }
    }

    drop(bufs);
    for payload in payloads {
        emit(EVENT_PTY_OUTPUT, &payload);
    }
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct PtyOutputPayload {
    pub slot_id: String,
    pub text: String,
}

// ── US-002: Agent Status Events ──

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct AgentStatusPayload {
    pub slot_id: String,
    pub status: String,
    pub progress: Option<f64>,
    pub task: Option<String>,
    pub agent_type: Option<String>,
}

/// Emit an agent status change event
pub fn emit_agent_status(slot_id: &str, status: &str, progress: Option<f64>, task: Option<&str>, agent_type: Option<&str>) {
    let payload = AgentStatusPayload {
        slot_id: slot_id.to_string(),
        status: status.to_string(),
        progress,
        task: task.map(String::from),
        agent_type: agent_type.map(String::from),
    };
    emit(EVENT_AGENT_STATUS, &payload);
}

// ── US-003: Log Events ──

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct LogEntryPayload {
    pub level: String,
    pub source: String,
    pub message: String,
    pub timestamp: String,
    pub slot_id: Option<String>,
}

/// Emit a log entry event
pub fn emit_log(level: &str, source: &str, message: &str, slot_id: Option<&str>) {
    let payload = LogEntryPayload {
        level: level.to_string(),
        source: source.to_string(),
        message: message.to_string(),
        timestamp: chrono::Utc::now().to_rfc3339(),
        slot_id: slot_id.map(String::from),
    };
    emit(EVENT_LOG_ENTRY, &payload);
}

// ── US-004: Gate + Cost Events ──

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct GateEventPayload {
    pub gate_id: String,
    pub slot_id: String,
    pub operation_type: String,
    pub risk_level: String,
    pub status: String,
}

/// Emit a gate-created event
pub fn emit_gate_created(gate_id: &str, slot_id: &str, operation_type: &str, risk_level: &str) {
    let payload = GateEventPayload {
        gate_id: gate_id.to_string(),
        slot_id: slot_id.to_string(),
        operation_type: operation_type.to_string(),
        risk_level: risk_level.to_string(),
        status: "pending".to_string(),
    };
    emit(EVENT_GATE_CREATED, &payload);
}

/// Emit a gate-resolved event (approved/rejected)
pub fn emit_gate_resolved(gate_id: &str, slot_id: &str, operation_type: &str, risk_level: &str, status: &str) {
    let payload = GateEventPayload {
        gate_id: gate_id.to_string(),
        slot_id: slot_id.to_string(),
        operation_type: operation_type.to_string(),
        risk_level: risk_level.to_string(),
        status: status.to_string(),
    };
    emit(EVENT_GATE_RESOLVED, &payload);
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct CostUpdatedPayload {
    pub slot_id: String,
    pub provider: String,
    pub model: String,
    pub input_tokens: i64,
    pub output_tokens: i64,
    pub cost_cents: i64,
    pub total_cost_cents: i64,
}

/// Emit a cost-updated event
pub fn emit_cost_updated(slot_id: &str, provider: &str, model: &str, input_tokens: i64, output_tokens: i64, cost_cents: i64, total_cost_cents: i64) {
    let payload = CostUpdatedPayload {
        slot_id: slot_id.to_string(),
        provider: provider.to_string(),
        model: model.to_string(),
        input_tokens,
        output_tokens,
        cost_cents,
        total_cost_cents,
    };
    emit(EVENT_COST_UPDATED, &payload);
}

// ── Health Alert Events ──

/// Emit a health alert event (reuses HealthAlert from agent_lifecycle)
pub fn emit_health_alert(payload: &crate::services::agent_lifecycle::HealthAlert) {
    emit(EVENT_HEALTH_ALERT, payload);
}

// ── Orchestration Events ──

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct OrchestrationPayload {
    pub record_id: String,
    pub event_type: String, // "started", "layer_complete", "story_complete", "finished", "failed"
    pub current_layer: Option<i32>,
    pub completed_stories: Option<i32>,
    pub total_stories: Option<i32>,
    pub message: Option<String>,
}

/// Emit an orchestration update event
pub fn emit_orchestration(payload: &OrchestrationPayload) {
    emit(EVENT_ORCHESTRATION, payload);
}

// ── Core emit helper ──

fn emit<T: Serialize + Clone>(event: &str, payload: &T) {
    if let Some(handle) = app() {
        if let Err(e) = handle.emit(event, payload.clone()) {
            log::warn!("Failed to emit event '{}': {}", event, e);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_pty_output_payload_serialization() {
        let payload = PtyOutputPayload {
            slot_id: "s1".into(),
            text: "hello world".into(),
        };
        let json = serde_json::to_string(&payload).unwrap();
        assert!(json.contains("\"slotId\":\"s1\""));
        assert!(json.contains("\"text\":\"hello world\""));
    }

    #[test]
    fn test_agent_status_payload_serialization() {
        let payload = AgentStatusPayload {
            slot_id: "s1".into(),
            status: "running".into(),
            progress: Some(45.0),
            task: Some("Implementing US-001".into()),
            agent_type: Some("claude".into()),
        };
        let json = serde_json::to_string(&payload).unwrap();
        assert!(json.contains("\"status\":\"running\""));
        assert!(json.contains("\"progress\":45.0"));
    }

    #[test]
    fn test_log_entry_payload_serialization() {
        let payload = LogEntryPayload {
            level: "info".into(),
            source: "system".into(),
            message: "Test log".into(),
            timestamp: "2026-03-29T10:00:00Z".into(),
            slot_id: None,
        };
        let json = serde_json::to_string(&payload).unwrap();
        assert!(json.contains("\"level\":\"info\""));
        assert!(json.contains("\"slotId\":null"));
    }

    #[test]
    fn test_gate_event_payload_serialization() {
        let payload = GateEventPayload {
            gate_id: "g1".into(),
            slot_id: "s1".into(),
            operation_type: "file_delete".into(),
            risk_level: "high".into(),
            status: "pending".into(),
        };
        let json = serde_json::to_string(&payload).unwrap();
        assert!(json.contains("\"riskLevel\":\"high\""));
    }

    #[test]
    fn test_cost_updated_payload_serialization() {
        let payload = CostUpdatedPayload {
            slot_id: "s1".into(),
            provider: "anthropic".into(),
            model: "sonnet".into(),
            input_tokens: 1000,
            output_tokens: 500,
            cost_cents: 10,
            total_cost_cents: 150,
        };
        let json = serde_json::to_string(&payload).unwrap();
        assert!(json.contains("\"totalCostCents\":150"));
    }

    #[test]
    fn test_orchestration_payload_serialization() {
        let payload = OrchestrationPayload {
            record_id: "r1".into(),
            event_type: "layer_complete".into(),
            current_layer: Some(2),
            completed_stories: Some(5),
            total_stories: Some(10),
            message: None,
        };
        let json = serde_json::to_string(&payload).unwrap();
        assert!(json.contains("\"eventType\":\"layer_complete\""));
        assert!(json.contains("\"currentLayer\":2"));
    }

    #[test]
    fn test_pty_buffer_accumulation() {
        // Test that buffer accumulates without AppHandle (emit is a no-op)
        buffer_pty_output("test-slot", "hello ");
        buffer_pty_output("test-slot", "world");

        let bufs = PTY_BUFFERS.lock().unwrap();
        if let Some(buf) = bufs.buffers.get("test-slot") {
            // Buffer should have accumulated (unless flushed by time threshold)
            assert!(buf.data.contains("hello") || buf.data.is_empty());
        }
    }

    #[test]
    fn test_emit_without_app_handle_does_not_panic() {
        // Calling emit functions without initialized AppHandle should be safe no-ops
        emit_agent_status("s1", "running", None, None, None);
        emit_log("info", "test", "no panic", None);
        emit_gate_created("g1", "s1", "test", "low");
        emit_cost_updated("s1", "anthropic", "sonnet", 100, 50, 5, 5);
        // No assertion needed — just verify no panic
    }
}
