use serde::{Deserialize, Serialize};
use crate::db::{session_repo, slot_repo, cost_repo, orchestration_repo, metrics_repo};

// ── Learning + Intelligence Service ──

/// Agent recommendation based on task category
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AgentRecommendation {
    pub agent_type: String,
    pub confidence: f64,
    pub reason: String,
}

/// Predicted conflict between two stories
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ConflictPrediction {
    pub story_a: String,
    pub story_b: String,
    pub overlapping_patterns: Vec<String>,
    pub risk: String, // "low", "medium", "high"
}

// ── Agent Profiles ──

struct AgentProfile {
    agent_type: &'static str,
    strengths: &'static [&'static str],
    best_for: &'static [&'static str],
}

const AGENT_PROFILES: &[AgentProfile] = &[
    AgentProfile {
        agent_type: "claude",
        strengths: &["reasoning", "architecture", "debugging", "documentation"],
        best_for: &["backend", "api", "refactor", "architecture", "docs", "testing"],
    },
    AgentProfile {
        agent_type: "gemini",
        strengths: &["speed", "multimodal", "search", "breadth"],
        best_for: &["frontend", "ui", "design", "research", "exploration"],
    },
    AgentProfile {
        agent_type: "codex",
        strengths: &["code_generation", "completion", "patterns"],
        best_for: &["boilerplate", "crud", "migration", "scaffolding"],
    },
];

// ── Category Keywords ──

const CATEGORY_RULES: &[(&str, &[&str])] = &[
    ("backend", &["api", "server", "database", "db", "migration", "endpoint", "rest", "graphql", "service", "repo"]),
    ("frontend", &["ui", "component", "page", "view", "css", "style", "layout", "form", "button", "modal"]),
    ("testing", &["test", "spec", "assert", "mock", "fixture", "coverage", "e2e", "integration"]),
    ("devops", &["ci", "cd", "deploy", "docker", "pipeline", "build", "release", "infra"]),
    ("docs", &["doc", "readme", "guide", "tutorial", "changelog", "comment"]),
    ("refactor", &["refactor", "cleanup", "rename", "extract", "simplify", "optimize"]),
    ("architecture", &["arch", "design", "pattern", "module", "structure", "schema"]),
    ("config", &["config", "env", "setting", "option", "flag", "feature"]),
];

/// Infer task category from story title and file patterns.
pub fn categorize_story(story_title: &str, file_patterns: &[String]) -> String {
    let title_lower = story_title.to_lowercase();
    let patterns_lower: Vec<String> = file_patterns.iter()
        .map(|p| p.to_lowercase())
        .collect();

    let mut scores: Vec<(&str, u32)> = Vec::new();

    for (category, keywords) in CATEGORY_RULES {
        let mut score = 0u32;

        // Check title
        for kw in *keywords {
            if title_lower.contains(kw) {
                score += 2; // Title match is worth more
            }
        }

        // Check file patterns
        for pattern in &patterns_lower {
            for kw in *keywords {
                if pattern.contains(kw) {
                    score += 1;
                }
            }

            // File extension heuristics
            if *category == "frontend" && (pattern.ends_with(".tsx") || pattern.ends_with(".css") || pattern.ends_with(".scss")) {
                score += 1;
            }
            if *category == "backend" && (pattern.ends_with(".rs") || pattern.ends_with(".py") || pattern.ends_with(".go")) {
                score += 1;
            }
            if *category == "testing" && (pattern.contains("test") || pattern.contains("spec")) {
                score += 1;
            }
        }

        if score > 0 {
            scores.push((category, score));
        }
    }

    scores.sort_by(|a, b| b.1.cmp(&a.1));
    scores.first()
        .map(|(cat, _)| cat.to_string())
        .unwrap_or_else(|| "general".to_string())
}

/// Recommend the best agent for a given task category.
pub fn recommend_agent(task_category: &str) -> Option<AgentRecommendation> {
    let category_lower = task_category.to_lowercase();

    // Find the best matching profile
    let mut best: Option<(&AgentProfile, f64)> = None;

    for profile in AGENT_PROFILES {
        let mut score = 0.0;

        for &best_for in profile.best_for {
            if category_lower == best_for || category_lower.contains(best_for) {
                score += 1.0;
            }
        }

        // Partial match on strengths
        for &strength in profile.strengths {
            if category_lower.contains(strength) {
                score += 0.5;
            }
        }

        if score > 0.0 {
            if best.is_none() || score > best.unwrap().1 {
                best = Some((profile, score));
            }
        }
    }

    best.map(|(profile, score)| {
        let max_score = profile.best_for.len() as f64 + profile.strengths.len() as f64 * 0.5;
        let confidence = (score / max_score).min(1.0);

        let reason = format!(
            "{} excels at {} tasks (strengths: {})",
            profile.agent_type,
            task_category,
            profile.strengths.join(", "),
        );

        AgentRecommendation {
            agent_type: profile.agent_type.to_string(),
            confidence,
            reason,
        }
    })
}

/// Estimate time for a story based on category and complexity.
/// Returns estimated milliseconds.
pub fn estimate_story_time(task_category: &str, complexity: &str) -> Option<i64> {
    // Base times in minutes per category
    let base_minutes = match task_category {
        "backend" => 45,
        "frontend" => 30,
        "testing" => 20,
        "devops" => 60,
        "docs" => 15,
        "refactor" => 40,
        "architecture" => 90,
        "config" => 10,
        _ => 30,
    };

    // Complexity multiplier
    let multiplier = match complexity {
        "trivial" => 0.5,
        "simple" => 0.75,
        "medium" => 1.0,
        "complex" => 2.0,
        "epic" => 4.0,
        _ => return None,
    };

    let estimated_ms = (base_minutes as f64 * multiplier * 60.0 * 1000.0) as i64;
    Some(estimated_ms)
}

/// Predict file conflicts between stories based on overlapping file patterns.
pub fn predict_conflicts(stories: &[(String, Vec<String>)]) -> Vec<ConflictPrediction> {
    let mut predictions = Vec::new();

    for i in 0..stories.len() {
        for j in (i + 1)..stories.len() {
            let (name_a, patterns_a) = &stories[i];
            let (name_b, patterns_b) = &stories[j];

            let overlapping: Vec<String> = patterns_a.iter()
                .filter(|pa| {
                    patterns_b.iter().any(|pb| paths_overlap(pa, pb))
                })
                .cloned()
                .collect();

            if !overlapping.is_empty() {
                let risk = match overlapping.len() {
                    1 => "low",
                    2..=3 => "medium",
                    _ => "high",
                };

                predictions.push(ConflictPrediction {
                    story_a: name_a.clone(),
                    story_b: name_b.clone(),
                    overlapping_patterns: overlapping,
                    risk: risk.to_string(),
                });
            }
        }
    }

    predictions
}

/// Check if two file patterns overlap (exact match or directory containment)
fn paths_overlap(a: &str, b: &str) -> bool {
    if a == b {
        return true;
    }
    // Check if one is a prefix of the other (directory containment)
    let a_norm = a.trim_end_matches('/');
    let b_norm = b.trim_end_matches('/');
    a_norm.starts_with(b_norm) || b_norm.starts_with(a_norm)
}

/// Generate a retrospective document for a session.
pub fn generate_retro(session_id: &str) -> Result<String, String> {
    let session = session_repo::fetch_session(session_id)
        .map_err(|e| e.to_string())?
        .ok_or_else(|| format!("Session {} not found", session_id))?;

    let slots = slot_repo::fetch_slots_for_session(session_id)
        .map_err(|e| e.to_string())?;

    let cost_summary = cost_repo::summary_for_session(session_id)
        .map_err(|e| e.to_string())?;

    let orchestrations = orchestration_repo::fetch_records_for_session(session_id)
        .unwrap_or_default();

    let mut md = String::new();
    md.push_str(&format!("# Session Retrospective\n\n"));
    md.push_str(&format!("**Session ID:** {}\n", session_id));
    md.push_str(&format!("**Project:** {}\n", session.project_path));
    md.push_str(&format!("**Status:** {}\n", session.status));
    md.push_str(&format!("**Created:** {}\n\n", session.created_at));

    // Slot summary
    md.push_str("## Agent Slots\n\n");
    md.push_str("| Slot | Agent | Status | Task |\n");
    md.push_str("|------|-------|--------|------|\n");
    for slot in &slots {
        md.push_str(&format!(
            "| {} | {} | {} | {} |\n",
            slot.slot_index,
            slot.agent_type,
            slot.status,
            slot.current_task.as_deref().unwrap_or("-"),
        ));
    }
    md.push('\n');

    // Metrics per slot
    md.push_str("## Agent Metrics\n\n");
    for slot in &slots {
        if let Ok(metrics) = metrics_repo::get_or_create(&slot.id) {
            md.push_str(&format!(
                "- **Slot {}** ({}): {} completed, {} failed, avg {}ms, {} conflicts\n",
                slot.slot_index, slot.agent_type,
                metrics.total_stories_completed, metrics.total_stories_failed,
                metrics.avg_story_time_ms, metrics.conflicts_encountered,
            ));
        }
    }
    md.push('\n');

    // Cost summary
    md.push_str("## Cost Summary\n\n");
    md.push_str(&format!("- **Total Input Tokens:** {}\n", cost_summary.total_input_tokens));
    md.push_str(&format!("- **Total Output Tokens:** {}\n", cost_summary.total_output_tokens));
    md.push_str(&format!("- **Total Cost:** {}c (${:.2})\n", cost_summary.total_cost_cents, cost_summary.total_cost_cents as f64 / 100.0));
    md.push_str(&format!("- **API Calls:** {}\n\n", cost_summary.event_count));

    // Orchestration summary
    if !orchestrations.is_empty() {
        md.push_str("## Orchestration Runs\n\n");
        for orch in &orchestrations {
            md.push_str(&format!(
                "- **{}** ({}): {}/{} stories completed, {} failed\n",
                orch.prd_name, orch.status,
                orch.completed_stories, orch.total_stories,
                orch.failed_stories,
            ));
        }
        md.push('\n');
    }

    // Recommendations
    md.push_str("## Recommendations\n\n");
    let total_completed: i32 = orchestrations.iter().map(|o| o.completed_stories).sum();
    let total_failed: i32 = orchestrations.iter().map(|o| o.failed_stories).sum();

    if total_failed > 0 && total_completed > 0 {
        let failure_rate = total_failed as f64 / (total_completed + total_failed) as f64 * 100.0;
        if failure_rate > 20.0 {
            md.push_str(&format!("- High failure rate ({:.0}%). Consider breaking stories into smaller units.\n", failure_rate));
        }
    }

    if cost_summary.total_cost_cents > 1000 {
        md.push_str("- Session cost is high. Consider using model downgrade for simpler tasks.\n");
    }

    if slots.len() > 3 {
        md.push_str("- Many concurrent agents. Monitor for merge conflicts.\n");
    }

    if md.ends_with("## Recommendations\n\n") {
        md.push_str("- No specific recommendations. Session ran within normal parameters.\n");
    }

    md.push_str("\n---\n*Generated by XRoads Learning Engine*\n");

    Ok(md)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::db::manager::initialize_memory;

    #[test]
    fn test_categorize() {
        // Backend story
        assert_eq!(
            categorize_story("Create REST API endpoint for users", &["src/api/users.rs".into()]),
            "backend"
        );

        // Frontend story
        assert_eq!(
            categorize_story("Build login form component", &["src/components/LoginForm.tsx".into()]),
            "frontend"
        );

        // Testing story
        assert_eq!(
            categorize_story("Add unit tests for auth module", &["tests/auth_test.rs".into()]),
            "testing"
        );

        // Unknown falls back to general
        assert_eq!(
            categorize_story("Do something vague", &[]),
            "general"
        );
    }

    #[test]
    fn test_recommend() {
        let rec = recommend_agent("backend").unwrap();
        assert_eq!(rec.agent_type, "claude");
        assert!(rec.confidence > 0.0);

        let rec = recommend_agent("frontend").unwrap();
        assert_eq!(rec.agent_type, "gemini");

        let rec = recommend_agent("scaffolding").unwrap();
        assert_eq!(rec.agent_type, "codex");

        // Unknown category: no recommendation
        let rec = recommend_agent("quantum_physics");
        assert!(rec.is_none());
    }

    #[test]
    fn test_estimate_time() {
        let t = estimate_story_time("backend", "medium").unwrap();
        assert_eq!(t, 45 * 60 * 1000); // 45 minutes in ms

        let t = estimate_story_time("backend", "complex").unwrap();
        assert_eq!(t, 45 * 2 * 60 * 1000); // 90 minutes

        let t = estimate_story_time("testing", "simple").unwrap();
        assert_eq!(t, (20.0 * 0.75 * 60.0 * 1000.0) as i64); // 15 minutes

        // Invalid complexity
        assert!(estimate_story_time("backend", "impossible").is_none());
    }

    #[test]
    fn test_conflict_prediction() {
        let stories = vec![
            ("Story A".into(), vec!["src/api/users.rs".into(), "src/models/user.rs".into()]),
            ("Story B".into(), vec!["src/api/users.rs".into(), "src/api/orders.rs".into()]),
            ("Story C".into(), vec!["src/ui/dashboard.tsx".into()]),
        ];

        let conflicts = predict_conflicts(&stories);
        assert_eq!(conflicts.len(), 1); // A and B overlap on users.rs
        assert_eq!(conflicts[0].story_a, "Story A");
        assert_eq!(conflicts[0].story_b, "Story B");
        assert!(conflicts[0].overlapping_patterns.contains(&"src/api/users.rs".to_string()));
    }

    #[test]
    fn test_conflict_prediction_no_overlap() {
        let stories = vec![
            ("Story A".into(), vec!["src/api/users.rs".into()]),
            ("Story B".into(), vec!["src/ui/dashboard.tsx".into()]),
        ];

        let conflicts = predict_conflicts(&stories);
        assert!(conflicts.is_empty());
    }

    #[test]
    fn test_conflict_risk_levels() {
        let stories = vec![
            ("Story A".into(), vec![
                "src/api/a.rs".into(),
                "src/api/b.rs".into(),
                "src/api/c.rs".into(),
                "src/api/d.rs".into(),
            ]),
            ("Story B".into(), vec![
                "src/api/a.rs".into(),
                "src/api/b.rs".into(),
                "src/api/c.rs".into(),
                "src/api/d.rs".into(),
            ]),
        ];

        let conflicts = predict_conflicts(&stories);
        assert_eq!(conflicts.len(), 1);
        assert_eq!(conflicts[0].risk, "high"); // 4 overlapping files
    }

    #[test]
    fn test_retro() {
        initialize_memory().unwrap();
        let session = session_repo::create_session("/tmp/retro-test").unwrap();
        slot_repo::create_slot(&session.id, 0, "claude", None, None).unwrap();
        slot_repo::create_slot(&session.id, 1, "gemini", None, None).unwrap();

        let retro = generate_retro(&session.id).unwrap();
        assert!(retro.contains("# Session Retrospective"));
        assert!(retro.contains(&session.id));
        assert!(retro.contains("/tmp/retro-test"));
        assert!(retro.contains("Agent Slots"));
        assert!(retro.contains("claude"));
        assert!(retro.contains("gemini"));
        assert!(retro.contains("Cost Summary"));
    }

    #[test]
    fn test_retro_nonexistent_session() {
        initialize_memory().unwrap();
        let result = generate_retro("nonexistent-session-id");
        assert!(result.is_err());
    }

    #[test]
    fn test_paths_overlap() {
        assert!(paths_overlap("src/api/users.rs", "src/api/users.rs"));
        assert!(paths_overlap("src/api", "src/api/users.rs"));
        assert!(!paths_overlap("src/api/users.rs", "src/ui/dashboard.tsx"));
    }
}
