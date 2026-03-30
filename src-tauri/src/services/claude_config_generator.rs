use std::fs;
use std::path::Path;
use serde::{Deserialize, Serialize};

use crate::services::cockpit_brain::CockpitOrchestrationPlan;

// ── Claude Code Configuration Generator ──
//
// US-003: Hooks configuration generator
// US-004: CLAUDE.md and rules generation
//
// Generates .claude/ configuration files that drive Claude Code behavior:
// - settings.local.json (hooks for safety, testing, stop guard)
// - CLAUDE.md (project context, orchestration rules)
// - rules/{backend,frontend,testing}.md (scoped instructions)

// ── US-003: Hooks Configuration ──

/// Dangerous operation patterns for the safety hook
const DANGEROUS_PATTERNS: &[&str] = &[
    "rm -rf",
    "DROP TABLE",
    "DROP DATABASE",
    "git push --force",
    "git push -f",
    "git reset --hard",
    "chmod 777",
    "sudo rm",
    "> /dev/sda",
    "mkfs",
    "dd if=",
    ":(){ :|:& };:",
    "curl | bash",
];

/// Generate .claude/settings.local.json with safety hooks.
pub fn generate_hooks_config(project_path: &str) -> Result<String, String> {
    let claude_dir = Path::new(project_path).join(".claude");
    let hooks_dir = claude_dir.join("hooks");
    fs::create_dir_all(&hooks_dir)
        .map_err(|e| format!("Failed to create .claude/hooks/: {}", e))?;

    // 1. Generate the safe-executor hook script
    let patterns_check = DANGEROUS_PATTERNS.iter()
        .map(|p| format!(r#"  echo "$INPUT" | grep -qi '{}' && echo '{{"decision":"block","reason":"Dangerous operation detected: {}"}}'  && exit 0"#, p, p))
        .collect::<Vec<_>>()
        .join("\n");

    let safe_executor_script = format!(
r#"#!/bin/bash
# XRoads safe-executor hook — blocks dangerous operations
# Exit 0 + JSON = block/allow decision
# Called by Claude Code PreToolUse hook

INPUT="$CLAUDE_TOOL_INPUT"
TOOL="$CLAUDE_TOOL_NAME"

# Only check Bash tool calls
if [ "$TOOL" != "Bash" ]; then
  echo '{{"decision":"allow"}}'
  exit 0
fi

# Check for dangerous patterns
{patterns_check}

# Allow by default
echo '{{"decision":"allow"}}'
exit 0
"#,
        patterns_check = patterns_check,
    );

    let script_path = hooks_dir.join("safe-executor.sh");
    fs::write(&script_path, &safe_executor_script)
        .map_err(|e| format!("Failed to write safe-executor.sh: {}", e))?;

    // Make executable on Unix
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        let _ = fs::set_permissions(&script_path, fs::Permissions::from_mode(0o755));
    }

    // 2. Generate settings.local.json
    let settings = serde_json::json!({
        "hooks": {
            "PreToolUse": [
                {
                    "type": "command",
                    "matcher": "Bash",
                    "command": hooks_dir.join("safe-executor.sh").to_string_lossy().to_string(),
                }
            ],
            "PostToolUse": [
                {
                    "type": "command",
                    "matcher": "Edit|Write",
                    "command": "echo '{\"decision\":\"allow\"}'",
                }
            ],
            "Stop": [
                {
                    "type": "prompt",
                    "prompt": "Have all assigned stories been implemented with passing tests? If not, list what remains and continue working. Only stop when all tasks are complete.",
                }
            ]
        }
    });

    let settings_path = claude_dir.join("settings.local.json");
    let settings_str = serde_json::to_string_pretty(&settings)
        .map_err(|e| format!("Failed to serialize settings: {}", e))?;
    fs::write(&settings_path, &settings_str)
        .map_err(|e| format!("Failed to write settings.local.json: {}", e))?;

    Ok(settings_path.to_string_lossy().to_string())
}

// ── US-004: CLAUDE.md and Rules Generation ──

/// Detect tech stack from project files
fn detect_tech_stack(project_path: &Path) -> Vec<String> {
    let mut stack = Vec::new();

    if project_path.join("Cargo.toml").exists() { stack.push("Rust".into()); }
    if project_path.join("package.json").exists() { stack.push("Node.js/TypeScript".into()); }
    if project_path.join("pyproject.toml").exists() || project_path.join("requirements.txt").exists() { stack.push("Python".into()); }
    if project_path.join("go.mod").exists() { stack.push("Go".into()); }
    if project_path.join("pom.xml").exists() || project_path.join("build.gradle").exists() { stack.push("Java/Kotlin".into()); }
    if project_path.join("Gemfile").exists() { stack.push("Ruby".into()); }
    if project_path.join("tailwind.config.js").exists() || project_path.join("tailwind.config.ts").exists() { stack.push("Tailwind CSS".into()); }
    if project_path.join("next.config.js").exists() || project_path.join("next.config.ts").exists() { stack.push("Next.js".into()); }
    if project_path.join("vite.config.ts").exists() || project_path.join("vite.config.js").exists() { stack.push("Vite".into()); }
    if project_path.join("docker-compose.yml").exists() || project_path.join("Dockerfile").exists() { stack.push("Docker".into()); }

    if stack.is_empty() { stack.push("Unknown".into()); }
    stack
}

/// Detect test command from project files
fn detect_test_command(project_path: &Path) -> &'static str {
    if project_path.join("Cargo.toml").exists() { return "cargo test"; }
    if project_path.join("package.json").exists() { return "npm test"; }
    if project_path.join("pyproject.toml").exists() { return "pytest"; }
    if project_path.join("go.mod").exists() { return "go test ./..."; }
    "echo 'No test command detected'"
}

/// Generate .claude/CLAUDE.md with project context and orchestration rules.
/// If a CLAUDE.md already exists, appends the orchestration section.
pub fn generate_claude_md(project_path: &str, cop: &CockpitOrchestrationPlan) -> Result<String, String> {
    let path = Path::new(project_path);
    let claude_dir = path.join(".claude");
    fs::create_dir_all(&claude_dir)
        .map_err(|e| format!("Failed to create .claude/: {}", e))?;

    let tech_stack = detect_tech_stack(path);
    let test_cmd = detect_test_command(path);

    let orchestration_section = format!(
r#"
# XRoads Orchestration Context

## Project
- **Name**: {project_name}
- **Type**: {project_type}
- **Domain**: {domain}
- **Tech Stack**: {tech_stack}

## Orchestration Rules
- You are an agent in a **multi-agent orchestration** managed by XRoads.
- You work in an **isolated git worktree** — other agents work in parallel on separate branches.
- **Commit prefix**: Use `[slot-N]` prefix on all commits (N = your slot number from AGENT.md).
- **Branch isolation**: Do NOT merge, rebase, or modify other branches.
- **Cross-worktree changes**: Do NOT modify files outside your worktree.
- **Tests**: Run `{test_cmd}` after each significant change.
- **PRD context**: Read @prd.json for user story details.

## Coding Conventions
- Write clean, tested code. No TODOs without implementation plans.
- Small, atomic commits with descriptive messages.
- Follow existing code patterns and style in the repository.
"#,
        project_name = cop.project_name,
        project_type = cop.project_type,
        domain = cop.domain,
        tech_stack = tech_stack.join(", "),
        test_cmd = test_cmd,
    );

    let claude_md_path = claude_dir.join("CLAUDE.md");

    // Preserve existing content if present
    let existing = fs::read_to_string(&claude_md_path).unwrap_or_default();
    let content = if existing.is_empty() {
        orchestration_section.trim().to_string()
    } else if existing.contains("# XRoads Orchestration Context") {
        // Replace existing orchestration section
        if let Some(idx) = existing.find("# XRoads Orchestration Context") {
            let before = existing[..idx].trim_end();
            format!("{}\n\n{}", before, orchestration_section.trim())
        } else {
            existing
        }
    } else {
        format!("{}\n\n{}", existing.trim(), orchestration_section.trim())
    };

    fs::write(&claude_md_path, &content)
        .map_err(|e| format!("Failed to write CLAUDE.md: {}", e))?;

    Ok(claude_md_path.to_string_lossy().to_string())
}

/// Generate scoped rules files in .claude/rules/
pub fn generate_rules(project_path: &str, cop: &CockpitOrchestrationPlan) -> Result<Vec<String>, String> {
    let rules_dir = Path::new(project_path).join(".claude").join("rules");
    fs::create_dir_all(&rules_dir)
        .map_err(|e| format!("Failed to create .claude/rules/: {}", e))?;

    let mut created = Vec::new();

    // Backend rules
    let backend_rules = format!(
r#"---
description: Backend development rules for {project_name}
globs: ["src/api/**", "src/models/**", "src/services/**", "src-tauri/**", "server/**", "backend/**"]
---

# Backend Rules

- Follow the existing service/repository pattern in the codebase.
- All database changes require migrations — never modify tables directly.
- Handle errors explicitly — no unwrap() in production code.
- Write unit tests for all new functions.
- Validate inputs at API boundaries.
"#,
        project_name = cop.project_name,
    );
    let backend_path = rules_dir.join("backend.md");
    fs::write(&backend_path, &backend_rules).map_err(|e| e.to_string())?;
    created.push("backend.md".into());

    // Frontend rules
    let frontend_rules = format!(
r#"---
description: Frontend development rules for {project_name}
globs: ["src/components/**", "src/views/**", "src/stores/**", "src/styles/**", "app/**", "pages/**"]
---

# Frontend Rules

- Use existing component patterns and styling conventions.
- State management via the project's chosen solution (Zustand, Redux, etc.).
- Ensure accessibility: proper ARIA labels, keyboard navigation.
- No inline styles — use the existing CSS framework (Tailwind, etc.).
- Write component tests for interactive elements.
"#,
        project_name = cop.project_name,
    );
    let frontend_path = rules_dir.join("frontend.md");
    fs::write(&frontend_path, &frontend_rules).map_err(|e| e.to_string())?;
    created.push("frontend.md".into());

    // Testing rules
    let test_cmd = detect_test_command(Path::new(project_path));
    let testing_rules = format!(
r#"---
description: Testing rules for {project_name}
globs: ["tests/**", "spec/**", "**/*.test.*", "**/*.spec.*", "test/**"]
---

# Testing Rules

- Run the full test suite with: `{test_cmd}`
- Write tests BEFORE or alongside implementation (TDD preferred).
- Test edge cases: empty inputs, large inputs, concurrent access.
- Integration tests should use real dependencies where possible.
- Never skip failing tests — fix the underlying issue.
"#,
        project_name = cop.project_name,
        test_cmd = test_cmd,
    );
    let testing_path = rules_dir.join("testing.md");
    fs::write(&testing_path, &testing_rules).map_err(|e| e.to_string())?;
    created.push("testing.md".into());

    Ok(created)
}

/// Generate all Claude Code configuration for a project (US-003 + US-004 combined).
pub fn generate_all_config(project_path: &str, cop: &CockpitOrchestrationPlan) -> Result<ConfigGenerationResult, String> {
    let hooks_path = generate_hooks_config(project_path)?;
    let claude_md_path = generate_claude_md(project_path, cop)?;
    let rules = generate_rules(project_path, cop)?;

    Ok(ConfigGenerationResult {
        hooks_path,
        claude_md_path,
        rules_created: rules,
    })
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ConfigGenerationResult {
    pub hooks_path: String,
    pub claude_md_path: String,
    pub rules_created: Vec<String>,
}

#[cfg(test)]
mod tests {
    use super::*;

    fn make_temp_dir(name: &str) -> String {
        let dir = format!("/tmp/xroads-config-test-{}-{}", name, uuid::Uuid::new_v4());
        fs::create_dir_all(&dir).unwrap();
        dir
    }

    fn sample_cop() -> CockpitOrchestrationPlan {
        CockpitOrchestrationPlan {
            project_name: "TestProject".into(),
            project_type: "saas".into(),
            domain: "fintech".into(),
            market_context: "B2B payments".into(),
            transverse_productions: vec![],
            specialist_triggers: vec![],
            meta_agent_config: crate::services::cockpit_brain::MetaAgentConfig {
                capabilities: vec!["monitor".into()],
                monitoring_interval_ms: 30000,
                autonomy_level: "supervised".into(),
            },
            deliverables_path: ".crossroads/deliverables".into(),
            created_at: "2026-03-30".into(),
        }
    }

    #[test]
    fn test_generate_hooks_config() {
        let dir = make_temp_dir("hooks");
        let result = generate_hooks_config(&dir);
        assert!(result.is_ok());

        // Verify files created
        let settings_path = format!("{}/.claude/settings.local.json", dir);
        assert!(Path::new(&settings_path).exists());

        let content = fs::read_to_string(&settings_path).unwrap();
        assert!(content.contains("PreToolUse"));
        assert!(content.contains("Stop"));

        let script_path = format!("{}/.claude/hooks/safe-executor.sh", dir);
        assert!(Path::new(&script_path).exists());

        let script = fs::read_to_string(&script_path).unwrap();
        assert!(script.contains("rm -rf"));
        assert!(script.contains("DROP TABLE"));

        fs::remove_dir_all(&dir).ok();
    }

    #[test]
    fn test_generate_claude_md() {
        let dir = make_temp_dir("claude-md");
        let cop = sample_cop();
        let result = generate_claude_md(&dir, &cop);
        assert!(result.is_ok());

        let md_path = format!("{}/.claude/CLAUDE.md", dir);
        let content = fs::read_to_string(&md_path).unwrap();
        assert!(content.contains("TestProject"));
        assert!(content.contains("Orchestration Rules"));
        assert!(content.contains("slot-N"));

        fs::remove_dir_all(&dir).ok();
    }

    #[test]
    fn test_generate_claude_md_preserves_existing() {
        let dir = make_temp_dir("claude-md-existing");
        let claude_dir = format!("{}/.claude", dir);
        fs::create_dir_all(&claude_dir).unwrap();
        fs::write(format!("{}/CLAUDE.md", claude_dir), "# My Project\n\nExisting content.").unwrap();

        let cop = sample_cop();
        let _ = generate_claude_md(&dir, &cop);

        let content = fs::read_to_string(format!("{}/CLAUDE.md", claude_dir)).unwrap();
        assert!(content.contains("My Project"));
        assert!(content.contains("Existing content"));
        assert!(content.contains("XRoads Orchestration Context"));

        fs::remove_dir_all(&dir).ok();
    }

    #[test]
    fn test_generate_rules() {
        let dir = make_temp_dir("rules");
        let cop = sample_cop();
        let result = generate_rules(&dir, &cop);
        assert!(result.is_ok());

        let rules = result.unwrap();
        assert_eq!(rules.len(), 3);
        assert!(rules.contains(&"backend.md".to_string()));
        assert!(rules.contains(&"frontend.md".to_string()));
        assert!(rules.contains(&"testing.md".to_string()));

        let backend = fs::read_to_string(format!("{}/.claude/rules/backend.md", dir)).unwrap();
        assert!(backend.contains("src/api/**"));
        assert!(backend.contains("TestProject"));

        fs::remove_dir_all(&dir).ok();
    }

    #[test]
    fn test_detect_tech_stack() {
        let dir = make_temp_dir("tech-stack");
        fs::write(format!("{}/Cargo.toml", dir), "[package]").unwrap();
        fs::write(format!("{}/package.json", dir), "{}").unwrap();

        let stack = detect_tech_stack(Path::new(&dir));
        assert!(stack.contains(&"Rust".to_string()));
        assert!(stack.contains(&"Node.js/TypeScript".to_string()));

        fs::remove_dir_all(&dir).ok();
    }

    #[test]
    fn test_generate_all_config() {
        let dir = make_temp_dir("all-config");
        let cop = sample_cop();
        let result = generate_all_config(&dir, &cop);
        assert!(result.is_ok());

        let r = result.unwrap();
        assert!(!r.hooks_path.is_empty());
        assert!(!r.claude_md_path.is_empty());
        assert_eq!(r.rules_created.len(), 3);

        fs::remove_dir_all(&dir).ok();
    }
}
