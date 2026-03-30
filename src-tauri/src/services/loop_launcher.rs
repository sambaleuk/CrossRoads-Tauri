use std::collections::HashMap;
use std::fs;
use std::path::{Path, PathBuf};
use uuid::Uuid;
use crate::services::cli_detector::find_loop_script;

/// Configuration for launching an agent loop
#[derive(Debug, Clone)]
pub struct LoopConfig {
    pub agent_type: String,
    pub worktree_path: String,
    pub prd_path: String,
    pub max_iterations: u32,
    pub sleep_seconds: u32,
    pub session_id: String,
    pub slot_index: u32,
    pub skill_content: Option<String>,
    pub handoff_context: Option<String>,
    pub chairman_brief: Option<String>,
    pub task_description: Option<String>,
}

/// Result of AGENT.md generation
#[derive(Debug)]
pub struct AgentContext {
    pub agent_md_path: PathBuf,
    pub log_dir: PathBuf,
    pub progress_path: PathBuf,
}

/// Locate the correct loop script for an agent type
pub fn resolve_loop_script(agent_type: &str) -> Result<String, String> {
    let script_name = match agent_type {
        "claude" => "nexus-loop",
        "gemini" => "gemini-loop",
        "codex" => "codex-loop",
        _ => return Err(format!("Unknown agent type: {}", agent_type)),
    };

    find_loop_script(script_name)
        .ok_or_else(|| format!("Loop script '{}' not found in PATH", script_name))
}

/// Build environment variables for loop execution
pub fn build_env(config: &LoopConfig, ctx: &AgentContext) -> HashMap<String, String> {
    let mut env = HashMap::new();
    env.insert("PRD_FILE".into(), config.prd_path.clone());
    env.insert("AGENTS_FILE".into(), ctx.agent_md_path.to_string_lossy().into());
    env.insert("PROGRESS_FILE".into(), ctx.progress_path.to_string_lossy().into());
    env.insert("LOG_DIR".into(), ctx.log_dir.to_string_lossy().into());
    env.insert("CROSSROADS_SESSION_ID".into(), config.session_id.clone());
    env.insert("CROSSROADS_SLOT_INDEX".into(), config.slot_index.to_string());
    env
}

/// Build CLI arguments for loop script
pub fn build_args(config: &LoopConfig) -> Vec<String> {
    vec![
        "--max-iterations".into(), config.max_iterations.to_string(),
        "--sleep".into(), config.sleep_seconds.to_string(),
    ]
}

/// Generate AGENT.md with full context injection (US-002)
pub fn generate_agent_md(config: &LoopConfig) -> Result<AgentContext, String> {
    let wt = Path::new(&config.worktree_path);
    let crossroads_dir = wt.join(".crossroads");
    let log_dir = crossroads_dir.join("logs");
    let agent_md_path = wt.join("AGENT.md");
    let progress_path = wt.join("progress.txt");

    // Create directories
    fs::create_dir_all(&log_dir).map_err(|e| format!("Failed to create log dir: {}", e))?;

    // Read PRD summary for context
    let prd_summary = read_prd_summary(&config.prd_path).unwrap_or_else(|_| "No PRD loaded.".into());

    // Read previous progress if exists
    let prev_progress = fs::read_to_string(&progress_path).unwrap_or_default();

    // Build AGENT.md content
    let mut sections = Vec::new();

    sections.push(format!("# Agent Context — Slot {}\n", config.slot_index));

    // Session info
    sections.push(format!(
        "## Session\n- Session ID: `{}`\n- Agent: {}\n- Slot: {}\n- Worktree: `{}`\n",
        config.session_id, config.agent_type, config.slot_index, config.worktree_path
    ));

    // Chairman brief (injected from cockpit deliberation)
    if let Some(ref brief) = config.chairman_brief {
        sections.push(format!("## Chairman Brief\n{}\n", brief));
    }

    // Task assignment
    if let Some(ref task) = config.task_description {
        sections.push(format!("## Your Assignment\n**Task**: {}\n\nFocus exclusively on this task. Do not modify files outside your scope.\n", task));
    } else {
        sections.push("## Mission\nImplement user stories from the PRD. Run unit tests after each story.\nUpdate story status to 'complete' in prd.json when done.\n".into());
    }

    // PRD summary
    sections.push(format!("## PRD Summary\n{}\n", prd_summary));

    // Skill injection
    if let Some(skill) = &config.skill_content {
        sections.push(format!("## Assigned Skill\n{}\n", skill));
    }

    // Previous learnings
    if !prev_progress.is_empty() {
        sections.push(format!("## Previous Progress\n```\n{}\n```\n", prev_progress.trim()));
    }

    // Handoff context
    if let Some(handoff) = &config.handoff_context {
        sections.push(format!("## Handoff Context\n{}\n", handoff));
    }

    // Coordination rules (multi-agent safety)
    sections.push("## Coordination Rules\n- You are working in an **isolated git worktree**. Other agents work in parallel on separate branches.\n- Do NOT modify files outside your assigned scope.\n- Do NOT merge or rebase — the orchestrator handles branch integration.\n- Write clean, atomic commits with descriptive messages.\n- If you encounter a conflict or dependency on another agent's work, write it to progress.txt and continue.\n".into());

    // Workflow guidance
    sections.push("## Workflow\n1. Read the assigned user story\n2. Implement the feature\n3. Write unit tests\n4. Run tests and fix failures\n5. Update prd.json story status to 'complete'\n6. Write learnings to progress.txt\n".into());

    let content = sections.join("\n");
    fs::write(&agent_md_path, &content)
        .map_err(|e| format!("Failed to write AGENT.md: {}", e))?;

    // Initialize progress.txt if not exists
    if !progress_path.exists() {
        fs::write(&progress_path, "# Progress Log\n")
            .map_err(|e| format!("Failed to create progress.txt: {}", e))?;
    }

    Ok(AgentContext {
        agent_md_path,
        log_dir,
        progress_path,
    })
}

/// Parse prd.json for a summary string
fn read_prd_summary(prd_path: &str) -> Result<String, String> {
    let content = fs::read_to_string(prd_path).map_err(|e| e.to_string())?;
    let prd: serde_json::Value = serde_json::from_str(&content).map_err(|e| e.to_string())?;

    let name = prd.get("feature_name").and_then(|v| v.as_str()).unwrap_or("Unknown");
    let status = prd.get("status").and_then(|v| v.as_str()).unwrap_or("unknown");
    let stories = prd.get("user_stories").and_then(|v| v.as_array()).map(|a| a.len()).unwrap_or(0);
    let done = prd.get("user_stories").and_then(|v| v.as_array())
        .map(|arr| arr.iter().filter(|s| s.get("status").and_then(|v| v.as_str()) == Some("done")).count())
        .unwrap_or(0);

    Ok(format!("**{}** — {} ({}/{} stories done)", name, status, done, stories))
}

/// Parse progress.txt for iteration results (US-003)
pub fn parse_progress(progress_path: &str) -> Vec<ProgressEntry> {
    let content = match fs::read_to_string(progress_path) {
        Ok(c) => c,
        Err(_) => return Vec::new(),
    };

    let mut entries = Vec::new();
    let mut current: Option<ProgressEntry> = None;

    for line in content.lines() {
        if line.starts_with("## Iteration") || line.starts_with("## Story") {
            if let Some(entry) = current.take() {
                entries.push(entry);
            }
            current = Some(ProgressEntry {
                title: line.trim_start_matches('#').trim().to_string(),
                delta: String::new(),
                checks: String::new(),
                learnings: String::new(),
            });
        } else if let Some(ref mut entry) = current {
            if line.starts_with("Delta:") || line.starts_with("- Delta:") {
                entry.delta = line.trim_start_matches("- ").trim_start_matches("Delta:").trim().to_string();
            } else if line.starts_with("Checks:") || line.starts_with("- Checks:") {
                entry.checks = line.trim_start_matches("- ").trim_start_matches("Checks:").trim().to_string();
            } else if line.starts_with("Learnings:") || line.starts_with("- Learnings:") {
                entry.learnings = line.trim_start_matches("- ").trim_start_matches("Learnings:").trim().to_string();
            }
        }
    }
    if let Some(entry) = current {
        entries.push(entry);
    }
    entries
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ProgressEntry {
    pub title: String,
    pub delta: String,
    pub checks: String,
    pub learnings: String,
}

/// Get log files for a worktree, sorted by modification time (US-004)
pub fn list_iteration_logs(log_dir: &str) -> Result<Vec<LogFileInfo>, String> {
    let dir = Path::new(log_dir);
    if !dir.exists() {
        return Ok(Vec::new());
    }

    let mut logs: Vec<LogFileInfo> = fs::read_dir(dir)
        .map_err(|e| e.to_string())?
        .filter_map(|entry| entry.ok())
        .filter(|entry| {
            entry.path().extension().map(|e| e == "log").unwrap_or(false)
        })
        .filter_map(|entry| {
            let path = entry.path();
            let name = path.file_name()?.to_string_lossy().to_string();
            let size = entry.metadata().ok()?.len();
            let modified = entry.metadata().ok()?.modified().ok()?;
            Some(LogFileInfo {
                name,
                path: path.to_string_lossy().to_string(),
                size_bytes: size,
                modified_at: chrono::DateTime::<chrono::Utc>::from(modified).to_rfc3339(),
            })
        })
        .collect();

    logs.sort_by(|a, b| b.modified_at.cmp(&a.modified_at));
    Ok(logs)
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct LogFileInfo {
    pub name: String,
    pub path: String,
    pub size_bytes: u64,
    pub modified_at: String,
}

/// Read a log file content
pub fn read_log_file(path: &str) -> Result<String, String> {
    fs::read_to_string(path).map_err(|e| format!("Failed to read log: {}", e))
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;

    #[test]
    fn test_generate_agent_md() {
        let dir = format!("/tmp/xroads-loop-test-{}", Uuid::new_v4());
        fs::create_dir_all(&dir).unwrap();

        // Create a minimal prd.json
        let prd_path = format!("{}/prd.json", dir);
        fs::write(&prd_path, r#"{"feature_name":"Test Feature","status":"pending","user_stories":[{"id":"US-001","title":"Test","status":"pending"}]}"#).unwrap();

        let config = LoopConfig {
            agent_type: "claude".into(),
            worktree_path: dir.clone(),
            prd_path,
            max_iterations: 5,
            sleep_seconds: 10,
            session_id: "test-session".into(),
            slot_index: 0,
            skill_content: Some("## Test Skill\nDo testing.".into()),
            handoff_context: None,
            chairman_brief: Some("## Chairman Brief\nTest brief content".into()),
            task_description: Some("Implement auth core".into()),
        };

        let ctx = generate_agent_md(&config).unwrap();
        assert!(ctx.agent_md_path.exists());
        assert!(ctx.log_dir.exists());
        assert!(ctx.progress_path.exists());

        let content = fs::read_to_string(&ctx.agent_md_path).unwrap();
        assert!(content.contains("Session ID: `test-session`"));
        assert!(content.contains("Test Feature"));
        assert!(content.contains("Test Skill"));

        fs::remove_dir_all(&dir).ok();
    }

    #[test]
    fn test_parse_progress() {
        let dir = format!("/tmp/xroads-progress-test-{}", Uuid::new_v4());
        fs::create_dir_all(&dir).unwrap();
        let path = format!("{}/progress.txt", dir);

        fs::write(&path, "# Progress Log\n\n## Iteration 1\n- Delta: Added login form\n- Checks: All tests pass\n- Learnings: Use controlled components\n\n## Iteration 2\n- Delta: Added validation\n- Checks: 5/5 pass\n- Learnings: Zod works well\n").unwrap();

        let entries = parse_progress(&path);
        assert_eq!(entries.len(), 2);
        assert_eq!(entries[0].delta, "Added login form");
        assert_eq!(entries[1].checks, "5/5 pass");

        fs::remove_dir_all(&dir).ok();
    }

    #[test]
    fn test_list_iteration_logs() {
        let dir = format!("/tmp/xroads-logs-test-{}", Uuid::new_v4());
        fs::create_dir_all(&dir).unwrap();

        fs::write(format!("{}/claude_loop_iter_1.log", dir), "log 1").unwrap();
        fs::write(format!("{}/claude_loop_iter_2.log", dir), "log 2 content").unwrap();
        fs::write(format!("{}/not_a_log.txt", dir), "ignore").unwrap();

        let logs = list_iteration_logs(&dir).unwrap();
        assert_eq!(logs.len(), 2);
        assert!(logs[0].name.ends_with(".log"));

        fs::remove_dir_all(&dir).ok();
    }

    #[test]
    fn test_build_env() {
        let config = LoopConfig {
            agent_type: "claude".into(),
            worktree_path: "/tmp/wt".into(),
            prd_path: "/tmp/prd.json".into(),
            max_iterations: 3,
            sleep_seconds: 5,
            session_id: "s1".into(),
            slot_index: 2,
            skill_content: None,
            handoff_context: None,
            chairman_brief: None,
            task_description: None,
        };
        let ctx = AgentContext {
            agent_md_path: PathBuf::from("/tmp/wt/AGENT.md"),
            log_dir: PathBuf::from("/tmp/wt/.crossroads/logs"),
            progress_path: PathBuf::from("/tmp/wt/progress.txt"),
        };

        let env = build_env(&config, &ctx);
        assert_eq!(env.get("PRD_FILE").unwrap(), "/tmp/prd.json");
        assert_eq!(env.get("CROSSROADS_SESSION_ID").unwrap(), "s1");
        assert_eq!(env.get("CROSSROADS_SLOT_INDEX").unwrap(), "2");
    }
}
