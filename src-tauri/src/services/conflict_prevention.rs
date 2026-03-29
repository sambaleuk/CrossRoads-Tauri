use std::collections::HashMap;
use serde::{Deserialize, Serialize};
use crate::services::orchestration_engine::ExecutionLayer;
use crate::services::learning_engine;
use crate::services::ml_trainer::TrainedModels;

/// A pair of stories that can safely run in parallel (no predicted conflict).
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct StoryPair {
    pub story_a: String,
    pub story_b: String,
}

/// Result of conflict prevention analysis on a dispatch plan.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ConflictPreventionResult {
    pub safe_pairs: Vec<StoryPair>,
    pub risky_pairs: Vec<RiskyPair>,
    pub recommended_resequencing: Vec<ExecutionLayer>,
}

/// A pair of stories with predicted conflict risk.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct RiskyPair {
    pub story_a: String,
    pub story_b: String,
    pub risk: String,
    pub overlapping_patterns: Vec<String>,
}

/// Analyze a dispatch plan for potential conflicts, returning safe/risky pairs
/// and recommended layer resequencing.
pub fn analyze_dispatch_plan(
    stories: &[(String, Vec<String>)],
    file_predictions: &HashMap<String, Vec<String>>,
) -> ConflictPreventionResult {
    let mut safe_pairs = Vec::new();
    let mut risky_pairs = Vec::new();

    // Build effective file map: use predictions if available, fall back to provided patterns
    let effective_files: HashMap<&str, &Vec<String>> = stories.iter()
        .map(|(id, patterns)| {
            let files = file_predictions.get(id).unwrap_or(patterns);
            (id.as_str(), files)
        })
        .collect();

    for i in 0..stories.len() {
        for j in (i + 1)..stories.len() {
            let (id_a, _) = &stories[i];
            let (id_b, _) = &stories[j];

            let empty: Vec<String> = vec![];
            let files_a = effective_files.get(id_a.as_str()).copied().unwrap_or(&empty);
            let files_b = effective_files.get(id_b.as_str()).copied().unwrap_or(&empty);

            let overlapping: Vec<String> = files_a.iter()
                .filter(|fa| files_b.iter().any(|fb| paths_overlap(fa, fb)))
                .cloned()
                .collect();

            if overlapping.is_empty() {
                safe_pairs.push(StoryPair {
                    story_a: id_a.clone(),
                    story_b: id_b.clone(),
                });
            } else {
                let risk = match overlapping.len() {
                    1 => "low",
                    2..=3 => "medium",
                    _ => "high",
                };
                risky_pairs.push(RiskyPair {
                    story_a: id_a.clone(),
                    story_b: id_b.clone(),
                    risk: risk.to_string(),
                    overlapping_patterns: overlapping,
                });
            }
        }
    }

    // Build resequencing from the original analysis
    // Group stories into safe parallel layers
    let story_ids: Vec<String> = stories.iter().map(|(id, _)| id.clone()).collect();
    let resequenced = resequence_from_pairs(&story_ids, &risky_pairs);

    ConflictPreventionResult {
        safe_pairs,
        risky_pairs,
        recommended_resequencing: resequenced,
    }
}

/// If two stories in the same layer have high conflict risk, move one to the next layer.
pub fn resequence_for_safety(
    layers: &[ExecutionLayer],
    conflict_predictions: &[(String, String, String)], // (story_a, story_b, risk)
) -> Vec<ExecutionLayer> {
    let mut result: Vec<ExecutionLayer> = Vec::new();

    for layer in layers {
        let mut current_layer_ids: Vec<String> = Vec::new();
        let mut deferred_ids: Vec<String> = Vec::new();

        for story_id in &layer.story_ids {
            let has_high_risk = current_layer_ids.iter().any(|existing| {
                conflict_predictions.iter().any(|(a, b, risk)| {
                    (risk == "high" || risk == "medium")
                        && ((a == story_id && b == existing) || (b == story_id && a == existing))
                })
            });

            if has_high_risk {
                deferred_ids.push(story_id.clone());
            } else {
                current_layer_ids.push(story_id.clone());
            }
        }

        if !current_layer_ids.is_empty() {
            result.push(ExecutionLayer {
                index: result.len(),
                story_ids: current_layer_ids,
            });
        }

        if !deferred_ids.is_empty() {
            result.push(ExecutionLayer {
                index: result.len(),
                story_ids: deferred_ids,
            });
        }
    }

    // Renumber indices
    for (i, layer) in result.iter_mut().enumerate() {
        layer.index = i;
    }

    result
}

/// Generate file predictions for stories using ML model if available, else keyword heuristics.
pub fn generate_file_predictions(
    stories: &[(String, String)], // (story_id, story_title)
    trained_models: Option<&TrainedModels>,
    project_path: Option<&str>,
) -> HashMap<String, Vec<String>> {
    // Try to load trained models
    let loaded_models;
    let models = match trained_models {
        Some(m) => Some(m),
        None => {
            loaded_models = project_path.and_then(TrainedModels::load);
            loaded_models.as_ref()
        }
    };

    let mut predictions = HashMap::new();

    for (story_id, title) in stories {
        let title_lower = title.to_lowercase();

        // If we have a trained category model, use it to predict the domain
        let category = if let Some(m) = models {
            let tokens: Vec<String> = title_lower.split_whitespace().map(String::from).collect();
            m.classify_story(&tokens).unwrap_or_else(|| heuristic_category(&title_lower))
        } else {
            heuristic_category(&title_lower)
        };

        // Generate file pattern predictions based on category + title keywords
        let predicted_files = predict_files_for_category(&category, &title_lower);
        predictions.insert(story_id.clone(), predicted_files);
    }

    predictions
}

/// Heuristic category detection from title keywords
fn heuristic_category(title: &str) -> String {
    learning_engine::categorize_story(title, &[])
}

/// Predict likely file patterns based on task category and title
fn predict_files_for_category(category: &str, title: &str) -> Vec<String> {
    let mut patterns = Vec::new();

    match category {
        "backend" => {
            patterns.push("src/api/".to_string());
            patterns.push("src/services/".to_string());
            if title.contains("db") || title.contains("database") || title.contains("migration") {
                patterns.push("src/db/".to_string());
            }
        }
        "frontend" => {
            patterns.push("src/components/".to_string());
            patterns.push("src/views/".to_string());
            if title.contains("style") || title.contains("css") {
                patterns.push("src/styles/".to_string());
            }
        }
        "testing" => {
            patterns.push("tests/".to_string());
            patterns.push("src/tests/".to_string());
        }
        "devops" => {
            patterns.push("ci/".to_string());
            patterns.push(".github/".to_string());
            patterns.push("Dockerfile".to_string());
        }
        "docs" => {
            patterns.push("docs/".to_string());
            patterns.push("README.md".to_string());
        }
        "config" => {
            patterns.push("config/".to_string());
            patterns.push(".env".to_string());
        }
        _ => {
            patterns.push("src/".to_string());
        }
    }

    // Extract specific keywords for more precise predictions
    let keywords = ["auth", "user", "login", "api", "dashboard", "settings", "payment", "order"];
    for kw in keywords {
        if title.contains(kw) {
            patterns.push(format!("src/**/{}", kw));
        }
    }

    patterns
}

/// Check if two file paths overlap (exact match or directory containment)
fn paths_overlap(a: &str, b: &str) -> bool {
    if a == b {
        return true;
    }
    let a_norm = a.trim_end_matches('/');
    let b_norm = b.trim_end_matches('/');
    a_norm.starts_with(b_norm) || b_norm.starts_with(a_norm)
}

/// Build resequenced layers from story IDs and risky pairs
fn resequence_from_pairs(story_ids: &[String], risky_pairs: &[RiskyPair]) -> Vec<ExecutionLayer> {
    let single_layer = vec![ExecutionLayer {
        index: 0,
        story_ids: story_ids.to_vec(),
    }];

    let conflict_tuples: Vec<(String, String, String)> = risky_pairs.iter()
        .map(|rp| (rp.story_a.clone(), rp.story_b.clone(), rp.risk.clone()))
        .collect();

    resequence_for_safety(&single_layer, &conflict_tuples)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_resequencing() {
        let layers = vec![
            ExecutionLayer {
                index: 0,
                story_ids: vec!["A".into(), "B".into(), "C".into()],
            },
        ];

        // A and B conflict with high risk
        let conflicts = vec![
            ("A".to_string(), "B".to_string(), "high".to_string()),
        ];

        let resequenced = resequence_for_safety(&layers, &conflicts);
        // A stays in layer 0, B is deferred to layer 1, C stays in layer 0
        assert!(resequenced.len() >= 2);
        assert!(resequenced[0].story_ids.contains(&"A".to_string()));
        assert!(resequenced[0].story_ids.contains(&"C".to_string()));
        assert!(resequenced[1].story_ids.contains(&"B".to_string()));
    }

    #[test]
    fn test_file_prediction() {
        let stories = vec![
            ("US-001".to_string(), "Build REST API for user authentication".to_string()),
            ("US-002".to_string(), "Create dashboard UI component".to_string()),
            ("US-003".to_string(), "Add database migration for orders".to_string()),
        ];

        let predictions = generate_file_predictions(&stories, None, None);
        assert_eq!(predictions.len(), 3);

        // Backend story should predict api/services patterns
        let api_pred = &predictions["US-001"];
        assert!(api_pred.iter().any(|p| p.contains("api") || p.contains("services")));

        // Frontend story should predict components/views
        let ui_pred = &predictions["US-002"];
        assert!(ui_pred.iter().any(|p| p.contains("components") || p.contains("views")));

        // DB story should predict db patterns
        let db_pred = &predictions["US-003"];
        assert!(db_pred.iter().any(|p| p.contains("db")));
    }

    #[test]
    fn test_integration_with_dispatch() {
        // Simulate stories with overlapping file patterns (high risk: 4+ overlapping files)
        let stories = vec![
            ("US-001".to_string(), vec![
                "src/api/users.rs".to_string(), "src/models/user.rs".to_string(),
                "src/api/auth.rs".to_string(), "src/api/session.rs".to_string(),
            ]),
            ("US-002".to_string(), vec![
                "src/api/users.rs".to_string(), "src/api/orders.rs".to_string(),
                "src/api/auth.rs".to_string(), "src/api/session.rs".to_string(),
            ]),
            ("US-003".to_string(), vec!["src/ui/dashboard.tsx".to_string()]),
        ];

        let file_predictions = HashMap::new(); // Use provided patterns

        let result = analyze_dispatch_plan(&stories, &file_predictions);

        // US-001 and US-002 overlap on multiple files
        assert_eq!(result.risky_pairs.len(), 1);
        assert_eq!(result.risky_pairs[0].story_a, "US-001");
        assert_eq!(result.risky_pairs[0].story_b, "US-002");
        // 3 overlapping files → medium risk
        assert_eq!(result.risky_pairs[0].risk, "medium");

        // US-001+US-003 and US-002+US-003 are safe
        assert_eq!(result.safe_pairs.len(), 2);

        // Resequencing should separate US-001 and US-002 (high risk)
        assert!(result.recommended_resequencing.len() >= 2);

        // Verify no high/medium risky pair stories are in the same layer
        for layer in &result.recommended_resequencing {
            let in_layer: Vec<&String> = layer.story_ids.iter().collect();
            for rp in &result.risky_pairs {
                let both_in = in_layer.contains(&&rp.story_a) && in_layer.contains(&&rp.story_b);
                if rp.risk == "high" || rp.risk == "medium" {
                    assert!(!both_in, "Risky pair {} and {} should not be in the same layer", rp.story_a, rp.story_b);
                }
            }
        }
    }

    #[test]
    fn test_no_conflicts() {
        let stories = vec![
            ("US-001".to_string(), vec!["src/api/users.rs".to_string()]),
            ("US-002".to_string(), vec!["src/ui/dashboard.tsx".to_string()]),
        ];

        let result = analyze_dispatch_plan(&stories, &HashMap::new());
        assert!(result.risky_pairs.is_empty());
        assert_eq!(result.safe_pairs.len(), 1);
        assert_eq!(result.recommended_resequencing.len(), 1);
    }
}
