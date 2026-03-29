use std::collections::{HashMap, HashSet};
use serde::{Deserialize, Serialize};
use crate::db::{orchestration_repo, learning_repo};
use crate::services::{ml_trainer, conflict_prevention};

// ── US-001: PRD Parser ──

/// A parsed user story from a PRD
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ParsedStory {
    pub id: String,
    pub title: String,
    pub description: String,
    pub status: String,
    pub tests: Vec<String>,
    pub dependencies: Vec<String>,
    pub priority: String,
}

/// A fully parsed PRD
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ParsedPrd {
    pub feature_name: String,
    pub description: String,
    pub branch: Option<String>,
    pub status: String,
    pub stories: Vec<ParsedStory>,
}

/// Validation error
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ValidationError {
    pub field: String,
    pub message: String,
}

/// Parse a PRD JSON file and validate it
pub fn parse_prd(path: &str) -> Result<ParsedPrd, String> {
    let content = std::fs::read_to_string(path)
        .map_err(|e| format!("Failed to read PRD at {}: {}", path, e))?;
    parse_prd_content(&content)
}

/// Parse PRD from JSON string (for testing)
pub fn parse_prd_content(content: &str) -> Result<ParsedPrd, String> {
    let raw: serde_json::Value = serde_json::from_str(content)
        .map_err(|e| format!("Invalid JSON: {}", e))?;

    let errors = validate_prd(&raw);
    if !errors.is_empty() {
        let msgs: Vec<String> = errors.iter().map(|e| format!("{}: {}", e.field, e.message)).collect();
        return Err(format!("PRD validation failed: {}", msgs.join("; ")));
    }

    let feature_name = raw.get("feature_name").and_then(|v| v.as_str()).unwrap_or("").to_string();
    let description = raw.get("description").and_then(|v| v.as_str()).unwrap_or("").to_string();
    let branch = raw.get("branch").and_then(|v| v.as_str()).map(String::from);
    let status = raw.get("status").and_then(|v| v.as_str()).unwrap_or("pending").to_string();

    let stories = raw.get("user_stories")
        .and_then(|v| v.as_array())
        .map(|arr| {
            arr.iter().filter_map(|s| {
                Some(ParsedStory {
                    id: s.get("id")?.as_str()?.to_string(),
                    title: s.get("title")?.as_str()?.to_string(),
                    description: s.get("description").and_then(|v| v.as_str()).unwrap_or("").to_string(),
                    status: s.get("status").and_then(|v| v.as_str()).unwrap_or("pending").to_string(),
                    tests: s.get("tests").and_then(|v| v.as_array())
                        .map(|a| a.iter().filter_map(|t| t.as_str().map(String::from)).collect())
                        .unwrap_or_default(),
                    dependencies: s.get("dependencies").and_then(|v| v.as_array())
                        .map(|a| a.iter().filter_map(|d| d.as_str().map(String::from)).collect())
                        .unwrap_or_default(),
                    priority: s.get("priority").and_then(|v| v.as_str()).unwrap_or("medium").to_string(),
                })
            }).collect()
        })
        .unwrap_or_default();

    Ok(ParsedPrd { feature_name, description, branch, status, stories })
}

/// Validate PRD structure
fn validate_prd(raw: &serde_json::Value) -> Vec<ValidationError> {
    let mut errors = Vec::new();

    if raw.get("feature_name").and_then(|v| v.as_str()).unwrap_or("").is_empty() {
        errors.push(ValidationError { field: "feature_name".into(), message: "Required field".into() });
    }

    let valid_statuses = ["pending", "in_progress", "done", "complete", "completed", "failed"];

    match raw.get("user_stories").and_then(|v| v.as_array()) {
        None => errors.push(ValidationError { field: "user_stories".into(), message: "Required array".into() }),
        Some(stories) => {
            for (i, story) in stories.iter().enumerate() {
                if story.get("id").and_then(|v| v.as_str()).unwrap_or("").is_empty() {
                    errors.push(ValidationError { field: format!("user_stories[{}].id", i), message: "Required".into() });
                }
                if story.get("title").and_then(|v| v.as_str()).unwrap_or("").is_empty() {
                    errors.push(ValidationError { field: format!("user_stories[{}].title", i), message: "Required".into() });
                }
                if let Some(status) = story.get("status").and_then(|v| v.as_str()) {
                    if !valid_statuses.contains(&status) {
                        errors.push(ValidationError {
                            field: format!("user_stories[{}].status", i),
                            message: format!("Invalid status '{}'. Valid: {:?}", status, valid_statuses),
                        });
                    }
                }
            }
        }
    }

    errors
}

/// Detect PRD files in a directory (glob for prd-*.json or prd.json)
pub fn detect_prd_files(dir: &str) -> Vec<String> {
    let path = std::path::Path::new(dir);
    let mut files = Vec::new();

    // Check for prd.json
    let single = path.join("prd.json");
    if single.exists() {
        files.push(single.to_string_lossy().to_string());
    }

    // Glob for prd-*.json
    if let Ok(entries) = std::fs::read_dir(path) {
        for entry in entries.filter_map(|e| e.ok()) {
            let name = entry.file_name().to_string_lossy().to_string();
            if name.starts_with("prd-") && name.ends_with(".json") {
                files.push(entry.path().to_string_lossy().to_string());
            }
        }
    }

    files.sort();
    files
}

// ── US-002: Layer Builder ──

/// An execution layer: a group of stories that can run in parallel
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ExecutionLayer {
    pub index: usize,
    pub story_ids: Vec<String>,
}

/// Build execution layers from stories based on dependencies.
/// Layer 0 = stories with no deps, Layer N = depends on stories in layers < N.
pub fn build_layers(stories: &[ParsedStory]) -> Result<Vec<ExecutionLayer>, String> {
    let story_ids: HashSet<&str> = stories.iter().map(|s| s.id.as_str()).collect();

    // Validate all dependency references exist
    for story in stories {
        for dep in &story.dependencies {
            if !story_ids.contains(dep.as_str()) {
                return Err(format!("Story {} depends on non-existent story {}", story.id, dep));
            }
        }
    }

    // Detect circular dependencies via topological sort (Kahn's algorithm)
    let mut in_degree: HashMap<&str, usize> = HashMap::new();
    let mut dependents: HashMap<&str, Vec<&str>> = HashMap::new();

    for story in stories {
        in_degree.entry(story.id.as_str()).or_insert(0);
        for dep in &story.dependencies {
            *in_degree.entry(story.id.as_str()).or_insert(0) += 1;
            dependents.entry(dep.as_str()).or_default().push(story.id.as_str());
        }
    }

    let mut layers: Vec<ExecutionLayer> = Vec::new();
    let mut remaining: HashSet<&str> = story_ids.clone();
    let mut layer_idx = 0;

    loop {
        // Find all stories with no unsatisfied deps
        let ready: Vec<&str> = remaining.iter()
            .filter(|&&id| {
                let story = stories.iter().find(|s| s.id == id).unwrap();
                story.dependencies.iter().all(|dep| !remaining.contains(dep.as_str()))
            })
            .cloned()
            .collect();

        if ready.is_empty() {
            if remaining.is_empty() {
                break;
            } else {
                let stuck: Vec<&str> = remaining.iter().cloned().collect();
                return Err(format!("Circular dependency detected involving: {:?}", stuck));
            }
        }

        let mut sorted_ready: Vec<String> = ready.iter().map(|s| s.to_string()).collect();
        sorted_ready.sort();

        layers.push(ExecutionLayer {
            index: layer_idx,
            story_ids: sorted_ready,
        });

        for id in &ready {
            remaining.remove(id);
        }
        layer_idx += 1;
    }

    Ok(layers)
}

// ── US-003: Dispatch Plan ──

/// A dispatch assignment: which story goes to which slot
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct DispatchAssignment {
    pub story_id: String,
    pub slot_index: usize,
    pub status: String, // pending, dispatched, completed, failed
}

/// A dispatch plan for one layer
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct LayerDispatchPlan {
    pub layer_index: usize,
    pub assignments: Vec<DispatchAssignment>,
}

/// Create dispatch plans for all layers, assigning stories round-robin to available slots
pub fn create_dispatch_plans(layers: &[ExecutionLayer], num_slots: usize) -> Vec<LayerDispatchPlan> {
    layers.iter().map(|layer| {
        let assignments = layer.story_ids.iter().enumerate().map(|(i, story_id)| {
            DispatchAssignment {
                story_id: story_id.clone(),
                slot_index: i % num_slots,
                status: "pending".into(),
            }
        }).collect();
        LayerDispatchPlan {
            layer_index: layer.index,
            assignments,
        }
    }).collect()
}

// ── US-004: Resume ──

/// Determine the resume point: which layer to start from and which stories to skip
pub fn compute_resume_point(prd: &ParsedPrd, layers: &[ExecutionLayer]) -> (usize, HashSet<String>) {
    let completed: HashSet<&str> = prd.stories.iter()
        .filter(|s| s.status == "done" || s.status == "complete" || s.status == "completed")
        .map(|s| s.id.as_str())
        .collect();

    let mut resume_layer = 0;
    let mut skip_stories = HashSet::new();

    for layer in layers {
        let all_done = layer.story_ids.iter().all(|id| completed.contains(id.as_str()));
        if all_done {
            resume_layer = layer.index + 1;
            for id in &layer.story_ids {
                skip_stories.insert(id.clone());
            }
        } else {
            // Add individually completed stories in this layer to skip set
            for id in &layer.story_ids {
                if completed.contains(id.as_str()) {
                    skip_stories.insert(id.clone());
                }
            }
            break;
        }
    }

    (resume_layer, skip_stories)
}

/// Start or resume an orchestration. Returns record ID and dispatch plans.
pub fn start_orchestration(
    session_id: &str,
    prd_path: &str,
) -> Result<(String, ParsedPrd, Vec<ExecutionLayer>, Vec<LayerDispatchPlan>, usize), String> {
    // Check for existing incomplete orchestration
    if let Ok(Some(existing)) = orchestration_repo::find_incomplete(session_id) {
        // Resume mode
        let prd = parse_prd(&existing.prd_path)?;
        let layers = build_layers(&prd.stories)?;
        let (resume_layer, _skip) = compute_resume_point(&prd, &layers);
        let plans = create_dispatch_plans(&layers, 6); // max 6 slots
        return Ok((existing.id, prd, layers, plans, resume_layer));
    }

    // Fresh start
    let prd = parse_prd(prd_path)?;
    let mut layers = build_layers(&prd.stories)?;

    // WIRING 3: Conflict prevention before dispatch
    let story_patterns: Vec<(String, Vec<String>)> = prd.stories.iter().map(|s| {
        (s.id.clone(), s.tests.clone()) // Use test names as proxy for file patterns
    }).collect();
    let prevention = conflict_prevention::analyze_dispatch_plan(&story_patterns, &HashMap::new());
    if !prevention.risky_pairs.is_empty() {
        // Use resequenced layers instead
        layers = prevention.recommended_resequencing;
    }

    let layers_json = serde_json::to_string(&layers).map_err(|e| e.to_string())?;

    let record = orchestration_repo::create_record(
        session_id, prd_path, &prd.feature_name,
        prd.stories.len() as i32, &layers_json,
    ).map_err(|e| e.to_string())?;

    let plans = create_dispatch_plans(&layers, 6);
    Ok((record.id, prd, layers, plans, 0))
}

/// WIRING 1: Auto-train ML models after orchestration completes.
/// Call this when all stories in a session have finished.
pub fn complete_orchestration_ml(session_id: &str, project_path: &str) {
    let records = learning_repo::fetch_learning_records(session_id).unwrap_or_default();
    if records.len() >= 10 {
        let training_data: Vec<_> = records.iter().map(|r| {
            (ml_trainer::StoryFeatures {
                files_changed: r.files_changed as f64,
                lines_added: r.lines_added as f64,
                lines_removed: r.lines_removed as f64,
                tests_run: r.tests_run as f64,
                complexity_score: match r.story_complexity.as_str() {
                    "simple" => 0.0, "moderate" => 1.0, "complex" => 2.0, _ => 3.0,
                },
            }, r.duration_ms as f64, vec![], r.story_complexity.clone(), r.conflicts_encountered > 0)
        }).collect();
        let models = ml_trainer::TrainedModels::train_from_records(&training_data);
        let _ = models.save(project_path);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn sample_prd_json() -> &'static str {
        r#"{
            "feature_name": "Test Feature",
            "description": "A test feature",
            "branch": "feat/test",
            "status": "pending",
            "user_stories": [
                {"id": "US-001", "title": "First story", "description": "Do first", "status": "pending", "tests": ["test1"], "priority": "high"},
                {"id": "US-002", "title": "Second story", "description": "Do second", "status": "pending", "tests": ["test2"], "dependencies": ["US-001"], "priority": "high"},
                {"id": "US-003", "title": "Third story", "description": "Do third", "status": "pending", "tests": ["test3"], "priority": "medium"},
                {"id": "US-004", "title": "Fourth story", "description": "Do fourth", "status": "pending", "tests": ["test4"], "dependencies": ["US-002", "US-003"], "priority": "low"}
            ]
        }"#
    }

    #[test]
    fn test_parse_valid_prd() {
        let prd = parse_prd_content(sample_prd_json()).unwrap();
        assert_eq!(prd.feature_name, "Test Feature");
        assert_eq!(prd.stories.len(), 4);
        assert_eq!(prd.stories[0].id, "US-001");
        assert_eq!(prd.stories[1].dependencies, vec!["US-001"]);
        assert_eq!(prd.stories[3].dependencies, vec!["US-002", "US-003"]);
    }

    #[test]
    fn test_parse_invalid_prd_missing_name() {
        let json = r#"{"user_stories": [{"id": "US-001", "title": "ok", "status": "pending"}]}"#;
        let result = parse_prd_content(json);
        assert!(result.is_err());
        assert!(result.unwrap_err().contains("feature_name"));
    }

    #[test]
    fn test_parse_invalid_prd_bad_status() {
        let json = r#"{"feature_name": "Test", "user_stories": [{"id": "US-001", "title": "ok", "status": "banana"}]}"#;
        let result = parse_prd_content(json);
        assert!(result.is_err());
        assert!(result.unwrap_err().contains("Invalid status"));
    }

    #[test]
    fn test_build_layers_no_deps() {
        let stories = vec![
            ParsedStory { id: "A".into(), title: "a".into(), description: "".into(), status: "pending".into(), tests: vec![], dependencies: vec![], priority: "high".into() },
            ParsedStory { id: "B".into(), title: "b".into(), description: "".into(), status: "pending".into(), tests: vec![], dependencies: vec![], priority: "high".into() },
        ];
        let layers = build_layers(&stories).unwrap();
        assert_eq!(layers.len(), 1);
        assert_eq!(layers[0].story_ids.len(), 2);
    }

    #[test]
    fn test_build_layers_with_deps() {
        let prd = parse_prd_content(sample_prd_json()).unwrap();
        let layers = build_layers(&prd.stories).unwrap();

        // Layer 0: US-001, US-003 (no deps)
        // Layer 1: US-002 (deps: US-001)
        // Layer 2: US-004 (deps: US-002, US-003)
        assert_eq!(layers.len(), 3);
        assert!(layers[0].story_ids.contains(&"US-001".to_string()));
        assert!(layers[0].story_ids.contains(&"US-003".to_string()));
        assert_eq!(layers[1].story_ids, vec!["US-002"]);
        assert_eq!(layers[2].story_ids, vec!["US-004"]);
    }

    #[test]
    fn test_circular_dependency_detection() {
        let stories = vec![
            ParsedStory { id: "A".into(), title: "a".into(), description: "".into(), status: "pending".into(), tests: vec![], dependencies: vec!["B".into()], priority: "high".into() },
            ParsedStory { id: "B".into(), title: "b".into(), description: "".into(), status: "pending".into(), tests: vec![], dependencies: vec!["A".into()], priority: "high".into() },
        ];
        let result = build_layers(&stories);
        assert!(result.is_err());
        assert!(result.unwrap_err().contains("Circular dependency"));
    }

    #[test]
    fn test_dispatch_plans() {
        let layers = vec![
            ExecutionLayer { index: 0, story_ids: vec!["A".into(), "B".into(), "C".into()] },
            ExecutionLayer { index: 1, story_ids: vec!["D".into()] },
        ];
        let plans = create_dispatch_plans(&layers, 2);
        assert_eq!(plans.len(), 2);
        // Layer 0: A→slot0, B→slot1, C→slot0
        assert_eq!(plans[0].assignments[0].slot_index, 0);
        assert_eq!(plans[0].assignments[1].slot_index, 1);
        assert_eq!(plans[0].assignments[2].slot_index, 0);
        // Layer 1: D→slot0
        assert_eq!(plans[1].assignments[0].slot_index, 0);
    }

    #[test]
    fn test_resume_point() {
        let mut prd = parse_prd_content(sample_prd_json()).unwrap();
        // Mark first two done
        prd.stories[0].status = "done".into();
        prd.stories[2].status = "done".into();

        let layers = build_layers(&prd.stories).unwrap();
        let (resume_layer, skip) = compute_resume_point(&prd, &layers);

        // Layer 0 (US-001, US-003) is fully done → resume from layer 1
        assert_eq!(resume_layer, 1);
        assert!(skip.contains("US-001"));
        assert!(skip.contains("US-003"));
        assert!(!skip.contains("US-002"));
    }

    #[test]
    fn test_resume_partial_layer() {
        let mut prd = parse_prd_content(sample_prd_json()).unwrap();
        // Only US-001 done (US-003 still pending) → layer 0 not fully done
        prd.stories[0].status = "done".into();

        let layers = build_layers(&prd.stories).unwrap();
        let (resume_layer, skip) = compute_resume_point(&prd, &layers);

        assert_eq!(resume_layer, 0); // Still on layer 0
        assert!(skip.contains("US-001")); // But skip the completed one
        assert!(!skip.contains("US-003"));
    }

    #[test]
    fn test_detect_prd_files() {
        let dir = format!("/tmp/xroads-prd-detect-{}", uuid::Uuid::new_v4());
        std::fs::create_dir_all(&dir).unwrap();
        std::fs::write(format!("{}/prd.json", dir), "{}").unwrap();
        std::fs::write(format!("{}/prd-14-test.json", dir), "{}").unwrap();
        std::fs::write(format!("{}/prd-15-test.json", dir), "{}").unwrap();
        std::fs::write(format!("{}/readme.md", dir), "ignore").unwrap();

        let files = detect_prd_files(&dir);
        assert_eq!(files.len(), 3);
        assert!(files.iter().any(|f| f.ends_with("prd.json")));
        assert!(files.iter().any(|f| f.ends_with("prd-14-test.json")));

        std::fs::remove_dir_all(&dir).ok();
    }

    #[test]
    fn test_orchestration_resume_with_db() {
        crate::db::manager::initialize_memory().unwrap();
        let session = crate::db::session_repo::create_session("/tmp/orch-resume").unwrap();

        // Write a temp PRD file
        let dir = format!("/tmp/xroads-orch-test-{}", uuid::Uuid::new_v4());
        std::fs::create_dir_all(&dir).unwrap();
        let prd_path = format!("{}/prd.json", dir);
        std::fs::write(&prd_path, sample_prd_json()).unwrap();

        let (record_id, prd, layers, plans, resume) = start_orchestration(&session.id, &prd_path).unwrap();
        assert_eq!(resume, 0); // Fresh start
        assert_eq!(prd.stories.len(), 4);
        assert_eq!(layers.len(), 3);
        assert!(!record_id.is_empty());

        // Verify record exists
        let rec = orchestration_repo::fetch_record(&record_id).unwrap().unwrap();
        assert_eq!(rec.status, "running");

        std::fs::remove_dir_all(&dir).ok();
    }
}
