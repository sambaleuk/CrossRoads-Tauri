use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CostEvent {
    pub id: String,
    pub agent_slot_id: String,
    pub provider: String,
    pub model: String,
    pub input_tokens: i64,
    pub output_tokens: i64,
    pub cost_cents: i64,
    pub created_at: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct UsageSummary {
    pub total_input_tokens: i64,
    pub total_output_tokens: i64,
    pub total_cost_cents: i64,
    pub event_count: i64,
}

impl UsageSummary {
    pub fn zero() -> Self {
        Self {
            total_input_tokens: 0,
            total_output_tokens: 0,
            total_cost_cents: 0,
            event_count: 0,
        }
    }
}

/// Estimate cost in cents based on provider/model pricing.
pub fn estimate_cost_cents(model: &str, input_tokens: i64, output_tokens: i64) -> i64 {
    let m = model.to_lowercase();
    let (input_rate, output_rate) = if m.contains("opus") {
        (15.0, 75.0)
    } else if m.contains("sonnet") {
        (3.0, 15.0)
    } else if m.contains("haiku") {
        (0.25, 1.25)
    } else if m.contains("gpt-4o") {
        (2.5, 10.0)
    } else if m.contains("gpt-4") {
        (10.0, 30.0)
    } else if m.contains("o3") || m.contains("o4") {
        (10.0, 40.0)
    } else if m.contains("gemini-2") {
        (0.50, 1.50)
    } else if m.contains("gemini") {
        (0.35, 1.05)
    } else {
        (3.0, 15.0) // default
    };
    let cost = (input_tokens as f64 / 1_000_000.0 * input_rate)
        + (output_tokens as f64 / 1_000_000.0 * output_rate);
    (cost * 100.0) as i64
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_opus_pricing() {
        let cost = estimate_cost_cents("claude-opus-4", 1_000_000, 1_000_000);
        assert_eq!(cost, 9000); // $15 + $75 = $90
    }

    #[test]
    fn test_sonnet_pricing() {
        let cost = estimate_cost_cents("claude-sonnet-4", 1_000_000, 1_000_000);
        assert_eq!(cost, 1800); // $3 + $15 = $18
    }

    #[test]
    fn test_haiku_pricing() {
        let cost = estimate_cost_cents("claude-haiku-4", 1_000_000, 1_000_000);
        assert_eq!(cost, 150); // $0.25 + $1.25 = $1.50
    }
}
