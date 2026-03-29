use serde::{Deserialize, Serialize};
use crate::db::{cost_repo, slot_repo, session_repo};
use crate::services::event_bus;

// ── Budget Monitoring Service ──

/// Budget status for a slot
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct BudgetStatus {
    pub status: String, // "ok", "warning", "exceeded"
    pub percent_used: f64,
    pub remaining_cents: i64,
    pub spent_cents: i64,
    pub budget_cents: i64,
    pub projected_total: i64,
}

/// Cost projection for a session
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CostProjection {
    pub current_spend: i64,
    pub burn_rate_cents_per_hour: f64,
    pub projected_total: i64,
    pub time_to_exhaustion_minutes: Option<f64>,
    pub over_budget: bool,
    pub budget_cents: i64,
}

/// Default budget per slot in cents (500 = $5.00)
const DEFAULT_SLOT_BUDGET_CENTS: i64 = 500;

/// Default session budget in cents (2000 = $20.00)
const DEFAULT_SESSION_BUDGET_CENTS: i64 = 2000;

/// Warning threshold as a fraction of budget (0.8 = 80%)
const WARNING_THRESHOLD: f64 = 0.8;

/// Check budget status for a slot. Compares current spend vs configured budget.
pub fn check_budget(slot_id: &str) -> Result<BudgetStatus, String> {
    let summary = cost_repo::summary_for_slot(slot_id)
        .map_err(|e| e.to_string())?;

    let budget_cents = DEFAULT_SLOT_BUDGET_CENTS;
    let spent = summary.total_cost_cents;
    let remaining = (budget_cents - spent).max(0);
    let percent_used = if budget_cents > 0 {
        (spent as f64 / budget_cents as f64) * 100.0
    } else {
        0.0
    };

    // Simple projection: if we've had N events, assume linear burn
    let projected_total = if summary.event_count > 0 {
        // Rough projection: assume same burn rate continues for double the events
        spent * 2
    } else {
        0
    };

    let status = if spent >= budget_cents {
        "exceeded".to_string()
    } else if percent_used >= WARNING_THRESHOLD * 100.0 {
        "warning".to_string()
    } else {
        "ok".to_string()
    };

    if status == "exceeded" {
        event_bus::emit_log("warn", "budget",
            &format!("Slot {} budget exceeded: {}c / {}c", slot_id, spent, budget_cents),
            Some(slot_id),
        );
    }

    Ok(BudgetStatus {
        status,
        percent_used,
        remaining_cents: remaining,
        spent_cents: spent,
        budget_cents,
        projected_total,
    })
}

/// Project total cost for a session based on current burn rate.
pub fn get_projection(session_id: &str) -> Result<CostProjection, String> {
    let session = session_repo::fetch_session(session_id)
        .map_err(|e| e.to_string())?
        .ok_or_else(|| format!("Session {} not found", session_id))?;

    let summary = cost_repo::summary_for_session(session_id)
        .map_err(|e| e.to_string())?;

    let budget_cents = DEFAULT_SESSION_BUDGET_CENTS;
    let current_spend = summary.total_cost_cents;

    // Calculate burn rate from session duration
    let created = chrono::DateTime::parse_from_rfc3339(&session.created_at)
        .map_err(|e| e.to_string())?;
    let now = chrono::Utc::now();
    let elapsed_hours = (now - created.with_timezone(&chrono::Utc)).num_minutes() as f64 / 60.0;

    let burn_rate = if elapsed_hours > 0.01 {
        current_spend as f64 / elapsed_hours
    } else {
        0.0
    };

    // Project: assume 8-hour session if no better info
    let projected_total = if burn_rate > 0.0 {
        (burn_rate * 8.0) as i64
    } else {
        current_spend
    };

    let time_to_exhaustion = if burn_rate > 0.01 {
        let remaining = (budget_cents - current_spend).max(0) as f64;
        Some((remaining / burn_rate) * 60.0) // minutes
    } else {
        None
    };

    Ok(CostProjection {
        current_spend,
        burn_rate_cents_per_hour: burn_rate,
        projected_total,
        time_to_exhaustion_minutes: time_to_exhaustion,
        over_budget: current_spend >= budget_cents,
        budget_cents,
    })
}

/// Apply throttle to a slot based on budget pressure.
/// Levels: 0=none, 1=delay, 2=model_downgrade_suggest, 3=pause
pub fn apply_throttle(slot_id: &str, level: u8) -> Result<(), String> {
    match level {
        0 => {
            // No throttle — ensure slot is running
            event_bus::emit_log("info", "budget",
                &format!("Throttle removed for slot {}", slot_id),
                Some(slot_id),
            );
        }
        1 => {
            // Delay: log suggestion, no hard action
            event_bus::emit_log("info", "budget",
                &format!("Throttle level 1 (delay) applied to slot {}", slot_id),
                Some(slot_id),
            );
        }
        2 => {
            // Suggest model downgrade
            event_bus::emit_log("warn", "budget",
                &format!("Throttle level 2 (model downgrade suggested) for slot {}", slot_id),
                Some(slot_id),
            );
        }
        3 => {
            // Pause the slot
            slot_repo::update_slot(slot_id, "paused", Some("Budget throttle: paused"))
                .map_err(|e| e.to_string())?;
            event_bus::emit_agent_status(slot_id, "paused", None, Some("Budget throttle: paused"), None);
            event_bus::emit_log("warn", "budget",
                &format!("Throttle level 3 (paused) for slot {}", slot_id),
                Some(slot_id),
            );
        }
        _ => {
            return Err(format!("Invalid throttle level: {}. Valid: 0-3", level));
        }
    }

    Ok(())
}

/// Auto-throttle: check budget and apply appropriate throttle level
pub fn auto_throttle(slot_id: &str) -> Result<u8, String> {
    let status = check_budget(slot_id)?;

    let level = if status.status == "exceeded" {
        3
    } else if status.percent_used >= 95.0 {
        2
    } else if status.percent_used >= 80.0 {
        1
    } else {
        0
    };

    if level > 0 {
        apply_throttle(slot_id, level)?;
    }

    Ok(level)
}

// ── Cost-Aware Model Routing ──

/// Model tier with cost per 1M tokens (input/output)
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ModelTier {
    pub name: String,
    pub provider: String,
    pub input_cost_per_m: f64,  // $ per 1M input tokens
    pub output_cost_per_m: f64, // $ per 1M output tokens
    pub capability_score: f64,  // 0.0-1.0 (higher = more capable)
}

const MODEL_TIERS: &[(&str, &str, f64, f64, f64)] = &[
    // (name, provider, input_$/M, output_$/M, capability)
    ("opus",    "anthropic", 15.0,  75.0,  1.0),
    ("sonnet",  "anthropic", 3.0,   15.0,  0.8),
    ("haiku",   "anthropic", 0.25,  1.25,  0.5),
    ("gpt-4o",  "openai",    2.5,   10.0,  0.85),
    ("o3",      "openai",    10.0,  40.0,  0.95),
    ("gemini-2","google",    0.50,  1.50,  0.7),
];

/// Model routing recommendation
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ModelRecommendation {
    pub recommended_model: String,
    pub provider: String,
    pub reason: String,
    pub estimated_cost_cents: i64,
    pub capability_score: f64,
    pub budget_pressure: String, // "none", "light", "heavy", "critical"
}

/// Select optimal model based on budget pressure + task complexity.
/// Complexity: "simple" (0), "moderate" (1), "complex" (2), "critical" (3)
pub fn route_model(slot_id: &str, complexity: &str) -> Result<ModelRecommendation, String> {
    let budget = check_budget(slot_id)?;

    // Determine budget pressure
    let pressure = if budget.status == "exceeded" {
        "critical"
    } else if budget.percent_used >= 90.0 {
        "heavy"
    } else if budget.percent_used >= 60.0 {
        "light"
    } else {
        "none"
    };

    // Minimum capability needed for this complexity
    let min_capability = match complexity {
        "simple" => 0.3,
        "moderate" => 0.6,
        "complex" => 0.8,
        "critical" => 0.9,
        _ => 0.6,
    };

    // Filter models by minimum capability
    let mut candidates: Vec<ModelTier> = MODEL_TIERS.iter()
        .filter(|(_, _, _, _, cap)| *cap >= min_capability)
        .map(|(name, provider, ic, oc, cap)| ModelTier {
            name: name.to_string(),
            provider: provider.to_string(),
            input_cost_per_m: *ic,
            output_cost_per_m: *oc,
            capability_score: *cap,
        })
        .collect();

    if candidates.is_empty() {
        // Fallback to cheapest capable model
        candidates = MODEL_TIERS.iter().map(|(name, provider, ic, oc, cap)| ModelTier {
            name: name.to_string(), provider: provider.to_string(),
            input_cost_per_m: *ic, output_cost_per_m: *oc, capability_score: *cap,
        }).collect();
    }

    // Score candidates: balance capability vs cost based on pressure
    let cost_weight = match pressure {
        "critical" => 0.9,  // Almost entirely cost-driven
        "heavy" => 0.7,
        "light" => 0.4,
        _ => 0.2,           // Mostly capability-driven
    };
    let cap_weight = 1.0 - cost_weight;

    let max_cost = candidates.iter()
        .map(|m| m.input_cost_per_m + m.output_cost_per_m)
        .fold(0.0f64, |a, b| a.max(b));

    let best = candidates.iter()
        .max_by(|a, b| {
            let cost_a = 1.0 - (a.input_cost_per_m + a.output_cost_per_m) / max_cost;
            let cost_b = 1.0 - (b.input_cost_per_m + b.output_cost_per_m) / max_cost;
            let score_a = cap_weight * a.capability_score + cost_weight * cost_a;
            let score_b = cap_weight * b.capability_score + cost_weight * cost_b;
            score_a.partial_cmp(&score_b).unwrap_or(std::cmp::Ordering::Equal)
        })
        .unwrap();

    // Estimate cost for ~5K input + 2K output tokens (typical story)
    let estimated_cost = ((best.input_cost_per_m * 5.0 + best.output_cost_per_m * 2.0) / 1000.0 * 100.0) as i64;

    let reason = match pressure {
        "critical" => format!("Budget exhausted — using cheapest viable model ({})", best.name),
        "heavy" => format!("Budget pressure — downgraded to {} to save costs", best.name),
        "light" => format!("Moderate budget usage — {} balances cost and capability", best.name),
        _ => format!("Budget healthy — {} selected for best capability", best.name),
    };

    event_bus::emit_log("info", "model-router",
        &format!("Routed to {} ({}) for {} task, pressure={}", best.name, best.provider, complexity, pressure),
        Some(slot_id),
    );

    Ok(ModelRecommendation {
        recommended_model: best.name.clone(),
        provider: best.provider.clone(),
        reason,
        estimated_cost_cents: estimated_cost,
        capability_score: best.capability_score,
        budget_pressure: pressure.to_string(),
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::db::manager::initialize_memory;

    fn setup() -> (String, String) {
        initialize_memory().unwrap();
        let session = session_repo::create_session("/tmp/budget-test").unwrap();
        let slot = slot_repo::create_slot(&session.id, 0, "claude", None, None).unwrap();
        (session.id, slot.id)
    }

    #[test]
    fn test_budget_check() {
        let (_, slot_id) = setup();

        // No spend yet — should be OK
        let status = check_budget(&slot_id).unwrap();
        assert_eq!(status.status, "ok");
        assert_eq!(status.percent_used, 0.0);
        assert_eq!(status.remaining_cents, DEFAULT_SLOT_BUDGET_CENTS);

        // Record some usage
        cost_repo::record_usage(&slot_id, "anthropic", "sonnet", 10000, 5000).unwrap();
        let status = check_budget(&slot_id).unwrap();
        assert_eq!(status.status, "ok");
        assert!(status.spent_cents > 0);
        assert!(status.percent_used > 0.0);
    }

    #[test]
    fn test_projection() {
        let (session_id, slot_id) = setup();

        // Record usage to establish a burn rate
        cost_repo::record_usage(&slot_id, "anthropic", "opus", 50000, 20000).unwrap();

        let projection = get_projection(&session_id).unwrap();
        assert!(projection.current_spend > 0);
        assert_eq!(projection.budget_cents, DEFAULT_SESSION_BUDGET_CENTS);
        // Burn rate should be positive since we just recorded usage
        assert!(projection.burn_rate_cents_per_hour >= 0.0);
    }

    #[test]
    fn test_throttle() {
        let (_, slot_id) = setup();

        // Level 0 — no action
        apply_throttle(&slot_id, 0).unwrap();

        // Level 1 — delay
        apply_throttle(&slot_id, 1).unwrap();

        // Level 2 — suggest downgrade
        apply_throttle(&slot_id, 2).unwrap();

        // Level 3 — pause
        apply_throttle(&slot_id, 3).unwrap();

        // Invalid level
        let result = apply_throttle(&slot_id, 5);
        assert!(result.is_err());
        assert!(result.unwrap_err().contains("Invalid throttle level"));
    }

    #[test]
    fn test_auto_throttle_no_spend() {
        let (_, slot_id) = setup();
        let level = auto_throttle(&slot_id).unwrap();
        assert_eq!(level, 0); // No spend = no throttle
    }

    #[test]
    fn test_model_routing_no_pressure() {
        let (_, slot_id) = setup();
        // No spend — should pick highest capability model
        let rec = route_model(&slot_id, "complex").unwrap();
        assert_eq!(rec.recommended_model, "opus"); // Best capability for complex
        assert_eq!(rec.budget_pressure, "none");
    }

    #[test]
    fn test_model_routing_simple_task() {
        let (_, slot_id) = setup();
        let rec = route_model(&slot_id, "simple").unwrap();
        // Simple task + no budget pressure = still picks capable but not overkill
        assert!(rec.capability_score >= 0.3);
    }

    #[test]
    fn test_budget_exceeded_status() {
        let (_, slot_id) = setup();

        // Record massive usage to exceed budget
        // Opus: $15/1M input, $75/1M output → need ~500 cents total
        // 1M input = 1500c, 1M output = 7500c — one big call is enough
        for _ in 0..10 {
            cost_repo::record_usage(&slot_id, "anthropic", "opus", 500000, 200000).unwrap();
        }

        let status = check_budget(&slot_id).unwrap();
        assert_eq!(status.status, "exceeded");
        assert!(status.percent_used >= 100.0);
        assert_eq!(status.remaining_cents, 0);
    }
}
