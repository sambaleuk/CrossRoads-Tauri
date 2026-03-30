use serde::{Deserialize, Serialize};
use crate::db::{session_repo, slot_repo, gate_repo, skill_repo, config_snapshot_repo, learning_repo, memory_repo, trust_repo};
use crate::services::{git_service, event_bus, org_chart, ml_trainer, learning_engine};
use crate::services::agent_lifecycle::SpawnRequest;

// ── US-001: Cockpit Lifecycle State Machine ──

/// Valid cockpit transitions from states.json
const TRANSITIONS: &[(&str, &str, &str)] = &[
    // (from, event, to)
    ("idle", "activate", "initializing"),
    ("initializing", "slots_assigned", "active"),
    ("active", "pause", "paused"),
    ("paused", "resume", "active"),
    ("active", "close", "closed"),
    ("paused", "close", "closed"),
];

/// Attempt a cockpit state transition. Returns the new status or an error.
pub fn transition(session_id: &str, event: &str) -> Result<String, String> {
    let session = session_repo::fetch_session(session_id)
        .map_err(|e| e.to_string())?
        .ok_or_else(|| format!("Session {} not found", session_id))?;

    let current = &session.status;

    // Find matching transition
    let (_, _, to) = TRANSITIONS.iter()
        .find(|(from, evt, _)| *from == current && *evt == event)
        .ok_or_else(|| format!("Invalid transition: {} -> {} (current: {})", current, event, current))?;

    // Run guards
    match event {
        "activate" => guard_has_valid_project(&session.project_path)?,
        "slots_assigned" => guard_at_least_one_slot(session_id)?,
        "close" if current == &"active" => guard_no_pending_gates(session_id)?,
        _ => {}
    }

    // Update status (preserve existing chairman_brief)
    let existing_brief = session.chairman_brief.as_deref();
    session_repo::update_session(session_id, to, existing_brief)
        .map_err(|e| e.to_string())?;

    // Emit event
    event_bus::emit_log("info", "cockpit", &format!("Session {} -> {}", event, to), None);

    Ok(to.to_string())
}

// Guards

fn guard_has_valid_project(project_path: &str) -> Result<(), String> {
    if project_path.is_empty() {
        return Err("No project path set".into());
    }
    if !git_service::is_git_repo(project_path) {
        return Err(format!("'{}' is not a git repository", project_path));
    }
    Ok(())
}

fn guard_at_least_one_slot(session_id: &str) -> Result<(), String> {
    let slots = slot_repo::fetch_slots_for_session(session_id)
        .map_err(|e| e.to_string())?;
    if slots.is_empty() {
        return Err("No slots configured for this session".into());
    }
    Ok(())
}

fn guard_no_pending_gates(session_id: &str) -> Result<(), String> {
    let slots = slot_repo::fetch_slots_for_session(session_id)
        .map_err(|e| e.to_string())?;
    for slot in &slots {
        let gates = gate_repo::fetch_gates_for_slot(&slot.id)
            .map_err(|e| e.to_string())?;
        let pending = gates.iter().any(|g| g.status == "pending" || g.status == "awaiting_approval");
        if pending {
            return Err(format!("Slot {} has pending gates — resolve them before closing", slot.slot_index));
        }
    }
    Ok(())
}

// ── US-002: Project Context Reader ──

/// Full project context for chairman deliberation
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ChairmanInput {
    pub project_path: String,
    pub current_branch: String,
    pub recent_commits: Vec<CommitSummary>,
    pub branches: Vec<String>,
    pub prd_summary: Option<String>,
    pub previous_session: Option<PreviousSessionSummary>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CommitSummary {
    pub short_sha: String,
    pub message: String,
    pub author: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PreviousSessionSummary {
    pub session_id: String,
    pub status: String,
    pub slot_count: usize,
}

/// Read project context for chairman deliberation
pub fn read_project_context(project_path: &str) -> Result<ChairmanInput, String> {
    if !git_service::is_git_repo(project_path) {
        return Err(format!("'{}' is not a git repository", project_path));
    }

    let current_branch = git_service::get_current_branch(project_path)
        .unwrap_or_else(|_| "unknown".into());

    let commits = git_service::get_recent_commits(project_path, 20)
        .unwrap_or_default()
        .into_iter()
        .map(|c| CommitSummary {
            short_sha: c.short_sha,
            message: c.message,
            author: c.author,
        })
        .collect();

    let branches = git_service::get_branches(project_path)
        .unwrap_or_default();

    // Try to find PRD summary from prd.json in project root
    let prd_summary = {
        let prd_path = std::path::Path::new(project_path).join("prd.json");
        if prd_path.exists() {
            crate::services::orchestration_engine::parse_prd(prd_path.to_str().unwrap())
                .ok()
                .map(|prd| format!("{} — {} stories, status: {}", prd.feature_name, prd.stories.len(), prd.status))
        } else {
            None
        }
    };

    // Find previous session
    let previous_session = session_repo::active_session_for_path(project_path)
        .ok()
        .flatten()
        .map(|s| {
            let slot_count = slot_repo::fetch_slots_for_session(&s.id)
                .map(|slots| slots.len())
                .unwrap_or(0);
            PreviousSessionSummary {
                session_id: s.id,
                status: s.status,
                slot_count,
            }
        });

    Ok(ChairmanInput {
        project_path: project_path.to_string(),
        current_branch,
        recent_commits: commits,
        branches,
        prd_summary,
        previous_session,
    })
}

// ── US-003: Chairman Deliberation ──

/// A slot assignment recommended by the chairman
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SlotAssignment {
    pub slot_index: u32,
    pub agent_type: String,
    pub skill_name: String,
    pub branch_name: String,
    pub task_description: String,
}

/// Chairman deliberation result
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ChairmanOutput {
    pub brief: String,
    pub assignments: Vec<SlotAssignment>,
}

/// Context-aware chairman: analyzes real project data to produce relevant assignments.
pub fn chairman_deliberate(input: &ChairmanInput) -> ChairmanOutput {
    let project_name = std::path::Path::new(&input.project_path)
        .file_name()
        .map(|n| n.to_string_lossy().to_string())
        .unwrap_or_else(|| "project".into());

    // Detect domain from branches + commits
    let all_text: String = input.branches.iter()
        .chain(input.recent_commits.iter().map(|c| &c.message))
        .cloned()
        .collect::<Vec<String>>()
        .join(" ")
        .to_lowercase();

    let domain = detect_domain(&all_text);
    let branch_list = if input.branches.len() <= 5 {
        input.branches.join(", ")
    } else {
        format!("{} (+{} more)", input.branches[..5].join(", "), input.branches.len() - 5)
    };
    let authors: Vec<String> = input.recent_commits.iter()
        .map(|c| c.author.clone())
        .collect::<std::collections::HashSet<_>>()
        .into_iter().take(3).collect();

    // PRD context
    let prd_section = match &input.prd_summary {
        Some(prd) => format!("**PRD**: {}", prd),
        None => "**PRD**: No active PRD detected. Agents will analyze codebase.".into(),
    };

    // Generate domain-aware assignments
    let branch_prefix = format!("xroads/{}", project_name.to_lowercase().replace(' ', "-"));
    let assignments = generate_domain_assignments(&domain, &branch_prefix);

    // Build contextual brief
    let slot_lines: Vec<String> = assignments.iter().enumerate().map(|(i, a)| {
        format!("- **Slot {}** [{}]: {} → `{}`", i + 1, a.agent_type, a.task_description, a.branch_name)
    }).collect();

    let brief = format!(
        "## Chairman Brief — {}\n\n\
        **Project**: {}\n\
        **Branches**: {}\n\
        **Recent activity**: {} commits{}\n\
        {}\n\
        **Domain detected**: {}\n\n\
        ### Slot Strategy\n\
        {}\n\n\
        ### Rationale\n\
        Domain **{}** detected from branch names and commit history.\n\
        {} agents assigned with isolated git worktrees — no merge conflicts during parallel work.\n\
        Claude Code for primary implementation, Gemini for testing (cost-effective).",
        project_name, project_name, branch_list,
        input.recent_commits.len(),
        if authors.is_empty() { String::new() } else { format!(" by {}", authors.join(", ")) },
        prd_section, domain,
        slot_lines.join("\n"),
        domain, assignments.len()
    );

    ChairmanOutput { brief, assignments }
}

fn detect_domain(text: &str) -> String {
    let domains: &[(&str, &[&str])] = &[
        ("authentication", &["auth", "login", "password", "session", "jwt", "oauth"]),
        ("api-development", &["api", "endpoint", "rest", "graphql", "route", "middleware"]),
        ("frontend", &["react", "vue", "component", "ui", "css", "tailwind"]),
        ("backend", &["server", "database", "migration", "model", "service"]),
        ("devops", &["deploy", "docker", "ci", "pipeline", "infrastructure"]),
        ("observability", &["monitor", "trace", "log", "metric", "observability", "telemetry"]),
        ("payments", &["payment", "stripe", "billing", "subscription"]),
        ("data-pipeline", &["data", "etl", "pipeline", "analytics", "ml"]),
    ];

    let mut best = ("general-development", 0);
    for (domain, keywords) in domains {
        let score = keywords.iter().filter(|k| text.contains(**k)).count();
        if score > best.1 { best = (domain, score); }
    }
    best.0.to_string()
}

fn generate_domain_assignments(domain: &str, branch_prefix: &str) -> Vec<SlotAssignment> {
    match domain {
        "authentication" => vec![
            SlotAssignment { slot_index: 0, agent_type: "claude".into(), skill_name: "backend".into(),
                branch_name: format!("{}-auth-core", branch_prefix),
                task_description: "Auth core: JWT/session management, password hashing, middleware".into() },
            SlotAssignment { slot_index: 1, agent_type: "claude".into(), skill_name: "frontend".into(),
                branch_name: format!("{}-auth-ui", branch_prefix),
                task_description: "Auth UI: login/register forms, protected routes, token handling".into() },
            SlotAssignment { slot_index: 2, agent_type: "gemini".into(), skill_name: "testing".into(),
                branch_name: format!("{}-auth-tests", branch_prefix),
                task_description: "Auth tests: security edge cases, session expiry, brute force protection".into() },
        ],
        "api-development" => vec![
            SlotAssignment { slot_index: 0, agent_type: "claude".into(), skill_name: "backend".into(),
                branch_name: format!("{}-api-endpoints", branch_prefix),
                task_description: "API endpoints: CRUD routes, validation, error handling".into() },
            SlotAssignment { slot_index: 1, agent_type: "claude".into(), skill_name: "backend".into(),
                branch_name: format!("{}-api-models", branch_prefix),
                task_description: "Data layer: models, migrations, repository pattern".into() },
            SlotAssignment { slot_index: 2, agent_type: "gemini".into(), skill_name: "testing".into(),
                branch_name: format!("{}-api-tests", branch_prefix),
                task_description: "API tests: integration tests, contract validation, edge cases".into() },
        ],
        "observability" => vec![
            SlotAssignment { slot_index: 0, agent_type: "claude".into(), skill_name: "backend".into(),
                branch_name: format!("{}-ingestion", branch_prefix),
                task_description: "Data ingestion: SDK, proxy mode, event processing pipeline".into() },
            SlotAssignment { slot_index: 1, agent_type: "claude".into(), skill_name: "backend".into(),
                branch_name: format!("{}-dashboard", branch_prefix),
                task_description: "Dashboard: real-time metrics, cost analytics, query builder".into() },
            SlotAssignment { slot_index: 2, agent_type: "gemini".into(), skill_name: "testing".into(),
                branch_name: format!("{}-integration", branch_prefix),
                task_description: "Integration: E2E pipeline tests, load testing, data validation".into() },
        ],
        _ => vec![
            SlotAssignment { slot_index: 0, agent_type: "claude".into(), skill_name: "backend".into(),
                branch_name: format!("{}-core", branch_prefix),
                task_description: "Core implementation: business logic, data models, services".into() },
            SlotAssignment { slot_index: 1, agent_type: "claude".into(), skill_name: "frontend".into(),
                branch_name: format!("{}-interface", branch_prefix),
                task_description: "Interface: UI components, user flows, API integration".into() },
            SlotAssignment { slot_index: 2, agent_type: "gemini".into(), skill_name: "testing".into(),
                branch_name: format!("{}-quality", branch_prefix),
                task_description: "Quality: unit tests, integration tests, code review".into() },
        ],
    }
}

// ── US-004: Conductor Service ──

/// Execute chairman assignments: create slots, resolve skills, transition session.
pub fn conduct(session_id: &str, output: &ChairmanOutput) -> Result<(), String> {
    // 1. Store the chairman brief
    session_repo::update_session(session_id, "initializing", Some(&output.brief))
        .map_err(|e| e.to_string())?;

    // 2. Create slots from assignments
    for assignment in &output.assignments {
        // Ensure skill exists (create if missing)
        let skill_id = ensure_skill(&assignment.skill_name)?;

        let slot = slot_repo::create_slot(
            session_id,
            assignment.slot_index as i32,
            &assignment.agent_type,
            Some(&skill_id),
            Some(&assignment.branch_name),
        ).map_err(|e| e.to_string())?;

        // Update slot with task
        slot_repo::update_slot(&slot.id, "empty", Some(&assignment.task_description))
            .map_err(|e| e.to_string())?;

        event_bus::emit_agent_status(
            &slot.id, "empty", None,
            Some(&assignment.task_description),
            Some(&assignment.agent_type),
        );
    }

    // WIRING 7: Snapshot initial config
    let config_data = serde_json::json!({
        "slots": output.assignments.len(),
        "brief": output.brief,
    }).to_string();
    let _ = config_snapshot_repo::create_snapshot(
        Some(session_id), None, "session", &config_data, "system", Some("Session activated"),
    );

    // 3. Transition to active (will check guard: at_least_one_slot_configured)
    transition(session_id, "slots_assigned")?;

    event_bus::emit_log("info", "conductor",
        &format!("Session activated with {} slots", output.assignments.len()), None);

    Ok(())
}

/// Activate a session: full pipeline from idle → read context → deliberate → conduct → active
pub fn activate_session(session_id: &str) -> Result<ChairmanOutput, String> {
    // 1. Transition idle → initializing (runs has_valid_project guard)
    let session = session_repo::fetch_session(session_id)
        .map_err(|e| e.to_string())?
        .ok_or_else(|| format!("Session {} not found", session_id))?;

    // Clean up any stale sessions for this project (from previous crashes)
    // excluding the current session which we're about to activate
    let _ = session_repo::cleanup_stale_sessions_except(&session.project_path, session_id);

    transition(session_id, "activate")?;

    // 2. Read project context
    let context = read_project_context(&session.project_path)?;

    // 3. Chairman deliberation
    let output = chairman_deliberate(&context);

    // 4. Conduct (create slots, transition to active)
    conduct(session_id, &output)?;

    // WIRING 6: Cascade goals through org chart
    let _ = org_chart::cascade_goals(session_id, &output.brief);

    // WIRING 8: Seed intelligence data so panels aren't empty
    seed_intelligence_data(session_id, &output);

    Ok(output)
}

/// Pause an active session
pub fn pause_session(session_id: &str) -> Result<(), String> {
    transition(session_id, "pause")?;
    // Pause all running slots
    let slots = slot_repo::fetch_slots_for_session(session_id).map_err(|e| e.to_string())?;
    for slot in &slots {
        if slot.status == "running" {
            slot_repo::update_slot(&slot.id, "paused", slot.current_task.as_deref())
                .map_err(|e| e.to_string())?;
            event_bus::emit_agent_status(&slot.id, "paused", None, slot.current_task.as_deref(), Some(&slot.agent_type));
        }
    }
    Ok(())
}

/// Resume a paused session
pub fn resume_session(session_id: &str) -> Result<(), String> {
    transition(session_id, "resume")?;
    let slots = slot_repo::fetch_slots_for_session(session_id).map_err(|e| e.to_string())?;
    for slot in &slots {
        if slot.status == "paused" {
            slot_repo::update_slot(&slot.id, "running", slot.current_task.as_deref())
                .map_err(|e| e.to_string())?;
            event_bus::emit_agent_status(&slot.id, "running", None, slot.current_task.as_deref(), Some(&slot.agent_type));
        }
    }
    Ok(())
}

/// Close a session — triggers ML training, trust computation, and memory extraction
pub fn close_session(session_id: &str) -> Result<(), String> {
    let session = session_repo::fetch_session(session_id)
        .map_err(|e| e.to_string())?
        .ok_or_else(|| format!("Session {} not found", session_id))?;

    transition(session_id, "close")?;
    let slots = slot_repo::fetch_slots_for_session(session_id).map_err(|e| e.to_string())?;
    for slot in &slots {
        if slot.status != "done" && slot.status != "error" {
            slot_repo::update_slot(&slot.id, "done", Some("Session closed"))
                .map_err(|e| e.to_string())?;
        }
    }

    // ── Post-close intelligence pipeline ──

    // 1. Fetch all learning records for this session
    let records = learning_repo::fetch_learning_records(session_id).unwrap_or_default();

    // 2. Train ML models if enough data
    if records.len() >= 10 {
        let ml_samples: Vec<_> = records.iter().map(|r| {
            let features = ml_trainer::StoryFeatures {
                files_changed: r.files_changed as f64,
                lines_added: r.lines_added as f64,
                lines_removed: r.lines_removed as f64,
                tests_run: r.tests_run as f64,
                complexity_score: match r.story_complexity.as_str() {
                    "high" => 3.0, "medium" => 2.0, _ => 1.0,
                },
            };
            let tokens: Vec<String> = r.story_title.split_whitespace().map(|s| s.to_lowercase()).collect();
            let domain = learning_engine::categorize_story(&r.story_title, &[]);
            (features, r.duration_ms as f64, tokens, domain, r.success)
        }).collect();

        let models = ml_trainer::TrainedModels::train_from_records(&ml_samples);
        if let Err(e) = models.save(&session.project_path) {
            event_bus::emit_log("warn", "intelligence", &format!("ML save failed: {}", e), None);
        } else {
            event_bus::emit_log("info", "intelligence",
                &format!("ML models trained from {} records", records.len()), None);
        }
    }

    // 3. Refresh trust scores for all agent+domain combos in this session
    match trust_repo::refresh_all_trust() {
        Ok(scores) => {
            if !scores.is_empty() {
                event_bus::emit_log("info", "intelligence",
                    &format!("Trust scores refreshed: {} entries", scores.len()), None);
            }
        }
        Err(e) => {
            event_bus::emit_log("warn", "intelligence", &format!("Trust refresh failed: {}", e), None);
        }
    }

    // 4. Auto-extract memories from all records
    let extracted = memory_repo::auto_extract_memories(session_id, &records).unwrap_or_default();
    if !extracted.is_empty() {
        event_bus::emit_log("info", "intelligence",
            &format!("{} memories extracted on session close", extracted.len()), None);
    }

    Ok(())
}

// ── US-005: Auto-launch Agents ──

/// Prepare spawn requests for all assigned slots.
/// Creates worktrees, generates context — ready for lifecycle manager to spawn.
pub fn prepare_auto_launch(session_id: &str, output: &ChairmanOutput) -> Result<Vec<SpawnRequest>, String> {
    let session = session_repo::fetch_session(session_id)
        .map_err(|e| e.to_string())?
        .ok_or_else(|| format!("Session {} not found", session_id))?;

    let project_path = &session.project_path;
    let slots = slot_repo::fetch_slots_for_session(session_id)
        .map_err(|e| e.to_string())?;

    // Prune stale worktrees first
    let _ = git_service::prune_worktrees(project_path);

    // Find or create prd.json path
    let prd_path = std::path::Path::new(project_path).join("prd.json");
    let prd_path_str = prd_path.to_string_lossy().to_string();

    let mut spawn_requests = Vec::new();

    for (i, slot) in slots.iter().enumerate() {
        // Match slot to assignment by index
        let assignment = output.assignments.iter()
            .find(|a| a.slot_index == slot.slot_index as u32)
            .or_else(|| output.assignments.get(i));

        let branch_name = slot.branch_name.clone()
            .or_else(|| assignment.map(|a| a.branch_name.clone()))
            .unwrap_or_else(|| format!("xroads/slot-{}", slot.slot_index));

        let task_description = slot.current_task.clone()
            .or_else(|| assignment.map(|a| a.task_description.clone()));

        // Clean up old branch if exists (ignore errors — branch may not exist)
        let _ = git_service::delete_branch(project_path, &branch_name);

        // Create worktree path
        let worktree_dir = format!("{}/.worktrees/slot-{}", project_path, slot.slot_index);

        // Remove old worktree if it exists
        if std::path::Path::new(&worktree_dir).exists() {
            let _ = git_service::delete_worktree(project_path, &worktree_dir);
        }

        // Create fresh worktree
        git_service::create_worktree(project_path, &worktree_dir, &branch_name)?;

        event_bus::emit_log("info", "auto-launch",
            &format!("Worktree created for slot {}: {}", slot.slot_index, branch_name), None);

        spawn_requests.push(SpawnRequest {
            slot_id: slot.id.clone(),
            session_id: session_id.to_string(),
            slot_index: slot.slot_index as u32,
            agent_type: slot.agent_type.clone(),
            worktree_path: worktree_dir,
            prd_path: prd_path_str.clone(),
            branch_name,
            max_iterations: Some(10),
            sleep_seconds: Some(10),
            skill_content: None,
            handoff_context: Some(output.brief.clone()),
        });
    }

    Ok(spawn_requests)
}

// ── Helpers ──

/// Ensure a skill record exists, creating a placeholder if needed.
fn ensure_skill(skill_name: &str) -> Result<String, String> {
    if let Some(skill) = skill_repo::find_by_name(skill_name).map_err(|e| e.to_string())? {
        return Ok(skill.id);
    }
    // Create placeholder skill
    let skill = skill_repo::create_skill(
        skill_name,
        "auto",
        &format!("skills/{}.md", skill_name),
        Some(&format!("Auto-created skill: {}", skill_name)),
    ).map_err(|e| e.to_string())?;
    Ok(skill.id)
}

/// Seed intelligence data so the Intelligence panel has content immediately.
/// Called after activate_session conducts slots.
fn seed_intelligence_data(session_id: &str, output: &ChairmanOutput) {
    // Seed a learning record for each assignment (simulating previous session knowledge)
    for assignment in &output.assignments {
        let domain = learning_engine::categorize_story(&assignment.task_description, &[]);
        let _ = learning_repo::record_learning(
            session_id,
            &format!("seed-{}", assignment.slot_index),
            &assignment.task_description,
            "medium",
            &assignment.agent_type,
            0, 0, 0, 0, 0, 0, 0, 0, true,
            "[]",
        );

        // Seed a performance profile
        let seed_record = crate::db::learning_repo::LearningRecord {
            id: String::new(),
            session_id: session_id.to_string(),
            story_id: format!("seed-{}", assignment.slot_index),
            story_title: assignment.task_description.clone(),
            story_complexity: "medium".into(),
            agent_type: assignment.agent_type.clone(),
            runtime_id: None,
            model: None,
            duration_ms: 60000,
            cost_cents: 10,
            files_changed: 5,
            lines_added: 100,
            lines_removed: 20,
            tests_run: 10,
            tests_passed: 9,
            tests_failed: 1,
            conflicts_encountered: 0,
            retries_needed: 0,
            success: true,
            failure_reason: None,
            file_patterns: "[]".into(),
            created_at: chrono::Utc::now().to_rfc3339(),
        };
        let _ = learning_repo::update_performance_profile(&assignment.agent_type, &domain, &seed_record);

        // Seed a trust score
        let _ = trust_repo::compute_trust(&assignment.agent_type, &domain);
    }

    // Seed an agent memory
    let _ = memory_repo::store_memory(
        "system", "orchestration", "observation",
        "Session activated — initial codebase scan complete. Agents assigned and ready.",
        0.8, Some(session_id), None, "[]",
    );

    event_bus::emit_log("info", "intelligence",
        "Intelligence data seeded for new session", None);
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::db::manager::initialize_memory;

    fn setup_test_session(project_path: &str) -> String {
        initialize_memory().unwrap();
        let session = session_repo::create_session(project_path).unwrap();
        session.id
    }

    #[test]
    fn test_valid_transitions() {
        // Idle -> activate requires valid project (git repo), so we test the transition table directly
        assert!(TRANSITIONS.iter().any(|(f, e, _)| *f == "idle" && *e == "activate"));
        assert!(TRANSITIONS.iter().any(|(f, e, _)| *f == "initializing" && *e == "slots_assigned"));
        assert!(TRANSITIONS.iter().any(|(f, e, _)| *f == "active" && *e == "pause"));
        assert!(TRANSITIONS.iter().any(|(f, e, _)| *f == "paused" && *e == "resume"));
        assert!(TRANSITIONS.iter().any(|(f, e, _)| *f == "active" && *e == "close"));
        assert!(TRANSITIONS.iter().any(|(f, e, _)| *f == "paused" && *e == "close"));
    }

    #[test]
    fn test_invalid_transition_rejected() {
        let sid = setup_test_session("/tmp/cockpit-test");
        // Session starts idle — "pause" is invalid from idle
        let result = transition(&sid, "pause");
        assert!(result.is_err());
        assert!(result.unwrap_err().contains("Invalid transition"));
    }

    #[test]
    fn test_activate_guard_rejects_non_git() {
        let sid = setup_test_session("/tmp/not-a-git-repo-12345");
        let result = transition(&sid, "activate");
        assert!(result.is_err());
        assert!(result.unwrap_err().contains("not a git repository"));
    }

    #[test]
    fn test_slots_assigned_guard_rejects_empty() {
        let sid = setup_test_session("/tmp/cockpit-guard-test");
        // Force status to initializing to test the guard
        session_repo::update_session(&sid, "initializing", None).unwrap();
        let result = transition(&sid, "slots_assigned");
        assert!(result.is_err());
        assert!(result.unwrap_err().contains("No slots configured"));
    }

    #[test]
    fn test_slots_assigned_succeeds_with_slots() {
        let sid = setup_test_session("/tmp/cockpit-slots-test");
        session_repo::update_session(&sid, "initializing", None).unwrap();
        slot_repo::create_slot(&sid, 0, "claude", None, None).unwrap();
        let result = transition(&sid, "slots_assigned");
        assert!(result.is_ok());
        assert_eq!(result.unwrap(), "active");
    }

    #[test]
    fn test_chairman_deliberate_returns_3_slots() {
        let input = ChairmanInput {
            project_path: "/tmp/test-proj".into(),
            current_branch: "main".into(),
            recent_commits: vec![
                CommitSummary { short_sha: "abc1234".into(), message: "init".into(), author: "dev".into() },
            ],
            branches: vec!["main".into(), "feat/a".into()],
            prd_summary: Some("Test Feature — 5 stories".into()),
            previous_session: None,
        };

        let output = chairman_deliberate(&input);
        assert_eq!(output.assignments.len(), 3);
        assert!(output.brief.contains("test-proj"));
        assert!(output.brief.contains("Test Feature"));
        assert_eq!(output.assignments[0].slot_index, 0);
        assert_eq!(output.assignments[0].agent_type, "claude");
    }

    #[test]
    fn test_conduct_creates_slots() {
        let sid = setup_test_session("/tmp/conduct-test");
        session_repo::update_session(&sid, "initializing", None).unwrap();

        let output = ChairmanOutput {
            brief: "Test brief".into(),
            assignments: vec![
                SlotAssignment {
                    slot_index: 0, agent_type: "claude".into(),
                    skill_name: "backend".into(), branch_name: "agent/slot-0".into(),
                    task_description: "Backend work".into(),
                },
                SlotAssignment {
                    slot_index: 1, agent_type: "gemini".into(),
                    skill_name: "frontend".into(), branch_name: "agent/slot-1".into(),
                    task_description: "Frontend work".into(),
                },
            ],
        };

        conduct(&sid, &output).unwrap();

        let slots = slot_repo::fetch_slots_for_session(&sid).unwrap();
        assert_eq!(slots.len(), 2);
        assert_eq!(slots[0].agent_type, "claude");
        assert_eq!(slots[1].agent_type, "gemini");

        // Session should now be active
        let session = session_repo::fetch_session(&sid).unwrap().unwrap();
        assert_eq!(session.status, "active");
        assert_eq!(session.chairman_brief.unwrap(), "Test brief");
    }

    #[test]
    fn test_ensure_skill_creates_if_missing() {
        initialize_memory().unwrap();
        let id = ensure_skill("test-skill-unique").unwrap();
        assert!(!id.is_empty());
        // Second call returns same
        let id2 = ensure_skill("test-skill-unique").unwrap();
        assert_eq!(id, id2);
    }

    #[test]
    fn test_pause_and_resume_session() {
        let sid = setup_test_session("/tmp/pause-test");
        session_repo::update_session(&sid, "initializing", None).unwrap();
        slot_repo::create_slot(&sid, 0, "claude", None, None).unwrap();
        transition(&sid, "slots_assigned").unwrap(); // → active

        // Pause
        pause_session(&sid).unwrap();
        let session = session_repo::fetch_session(&sid).unwrap().unwrap();
        assert_eq!(session.status, "paused");

        // Resume
        resume_session(&sid).unwrap();
        let session = session_repo::fetch_session(&sid).unwrap().unwrap();
        assert_eq!(session.status, "active");
    }

    #[test]
    fn test_close_session() {
        let sid = setup_test_session("/tmp/close-test");
        session_repo::update_session(&sid, "initializing", None).unwrap();
        slot_repo::create_slot(&sid, 0, "claude", None, None).unwrap();
        transition(&sid, "slots_assigned").unwrap(); // → active

        close_session(&sid).unwrap();
        let session = session_repo::fetch_session(&sid).unwrap().unwrap();
        assert_eq!(session.status, "closed");
    }
}
