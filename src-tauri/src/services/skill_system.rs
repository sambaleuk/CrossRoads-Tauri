use std::collections::HashMap;
use std::path::{Path, PathBuf};
use serde::{Deserialize, Serialize};
use crate::db::skill_repo;

// ── US-001: Skill Loader ──

/// A loaded skill with parsed frontmatter and content
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct LoadedSkill {
    pub name: String,
    pub family: String,
    pub description: Option<String>,
    pub required_mcps: Option<String>,
    pub content: String,
    pub source_path: String,
    pub is_user_override: bool,
}

/// Scan directories for SKILL.md files and load them
pub fn scan_skills(project_path: Option<&str>) -> Vec<LoadedSkill> {
    let mut skills = HashMap::new(); // name -> LoadedSkill (user overrides bundled)

    // 1. Bundled skills: {project}/skills/
    if let Some(proj) = project_path {
        let bundled = Path::new(proj).join("skills");
        load_skills_from_dir(&bundled, false, &mut skills);
    }

    // 2. Global user skills: ~/.xroads/skills/
    let home = std::env::var("HOME").unwrap_or_default();
    let global = PathBuf::from(format!("{}/.xroads/skills", home));
    load_skills_from_dir(&global, true, &mut skills);

    // 3. User project skills: {project}/.crossroads/skills/
    if let Some(proj) = project_path {
        let local = Path::new(proj).join(".crossroads/skills");
        load_skills_from_dir(&local, true, &mut skills);
    }

    let mut result: Vec<LoadedSkill> = skills.into_values().collect();
    result.sort_by(|a, b| a.name.cmp(&b.name));
    result
}

fn load_skills_from_dir(dir: &Path, is_user: bool, skills: &mut HashMap<String, LoadedSkill>) {
    if !dir.exists() {
        return;
    }

    let entries = match std::fs::read_dir(dir) {
        Ok(e) => e,
        Err(_) => return,
    };

    for entry in entries.filter_map(|e| e.ok()) {
        let path = entry.path();
        // Look for *.md files or directories containing SKILL.md
        if path.is_file() && path.extension().map(|e| e == "md").unwrap_or(false) {
            if let Some(skill) = load_skill_file(&path, is_user) {
                skills.insert(skill.name.clone(), skill);
            }
        } else if path.is_dir() {
            let skill_md = path.join("SKILL.md");
            if skill_md.exists() {
                if let Some(skill) = load_skill_file(&skill_md, is_user) {
                    skills.insert(skill.name.clone(), skill);
                }
            }
        }
    }
}

/// Load and parse a single skill file with frontmatter
fn load_skill_file(path: &Path, is_user: bool) -> Option<LoadedSkill> {
    let content = std::fs::read_to_string(path).ok()?;
    let (frontmatter, body) = parse_frontmatter(&content);

    let name = frontmatter.get("name")
        .cloned()
        .or_else(|| {
            path.file_stem().map(|s| s.to_string_lossy().to_string())
        })?;

    Some(LoadedSkill {
        name,
        family: frontmatter.get("family").cloned().unwrap_or_else(|| "general".into()),
        description: frontmatter.get("description").cloned(),
        required_mcps: frontmatter.get("required_mcps").cloned(),
        content: body,
        source_path: path.to_string_lossy().to_string(),
        is_user_override: is_user,
    })
}

/// Parse YAML-like frontmatter from a markdown file
fn parse_frontmatter(content: &str) -> (HashMap<String, String>, String) {
    let mut map = HashMap::new();

    if !content.starts_with("---") {
        return (map, content.to_string());
    }

    let after_first = &content[3..];
    if let Some(end_idx) = after_first.find("---") {
        let fm_text = &after_first[..end_idx];
        let body = &after_first[end_idx + 3..].trim_start();

        for line in fm_text.lines() {
            let line = line.trim();
            if let Some((key, value)) = line.split_once(':') {
                let key = key.trim().to_string();
                let value = value.trim().trim_matches('"').trim_matches('\'').to_string();
                if !key.is_empty() && !value.is_empty() {
                    map.insert(key, value);
                }
            }
        }

        return (map, body.to_string());
    }

    (map, content.to_string())
}

// ── US-002: Skill Registry ──

/// Register all discovered skills into the DB
pub fn register_skills(skills: &[LoadedSkill]) -> Result<Vec<String>, String> {
    let mut ids = Vec::new();
    for skill in skills {
        let existing = skill_repo::find_by_name(&skill.name).map_err(|e| e.to_string())?;
        if let Some(s) = existing {
            ids.push(s.id);
        } else {
            let s = skill_repo::create_skill(
                &skill.name,
                &skill.family,
                &skill.source_path,
                skill.description.as_deref(),
            ).map_err(|e| e.to_string())?;
            ids.push(s.id);
        }
    }
    Ok(ids)
}

/// Filter skills by family
pub fn filter_by_family(skills: &[LoadedSkill], family: &str) -> Vec<LoadedSkill> {
    skills.iter()
        .filter(|s| s.family == family)
        .cloned()
        .collect()
}

/// Filter skills by CLI compatibility (checks required_mcps)
pub fn filter_by_cli(skills: &[LoadedSkill], cli_type: &str) -> Vec<LoadedSkill> {
    skills.iter()
        .filter(|s| {
            // If no required_mcps, compatible with all
            match &s.required_mcps {
                None => true,
                Some(mcps) => !mcps.contains(&format!("!{}", cli_type)), // exclude if explicitly blocked
            }
        })
        .cloned()
        .collect()
}

// ── US-003: CLI Skill Adapters ──

/// Adapt skill content for a specific CLI type
pub fn adapt_for_cli(skill: &LoadedSkill, cli_type: &str) -> String {
    match cli_type {
        "claude" => adapt_claude(&skill.content),
        "gemini" => adapt_gemini(&skill.content, &skill.name),
        "codex" => adapt_codex(&skill.content, &skill.name),
        _ => skill.content.clone(),
    }
}

/// Claude adapter: native markdown (minimal transformation)
fn adapt_claude(content: &str) -> String {
    format!("## Assigned Skill\n\n{}", content)
}

/// Gemini adapter: restructure for Gemini's context format
fn adapt_gemini(content: &str, name: &str) -> String {
    let mut output = String::new();
    output.push_str(&format!("<skill name=\"{}\">\n", name));
    output.push_str("<instructions>\n");
    output.push_str(content);
    output.push_str("\n</instructions>\n");
    output.push_str("</skill>\n");
    output
}

/// Codex adapter: OpenAI-specific system prompt format
fn adapt_codex(content: &str, name: &str) -> String {
    let mut output = String::new();
    output.push_str(&format!("[Skill: {}]\n", name));
    output.push_str("Follow these instructions precisely:\n\n");
    output.push_str(content);
    output.push_str("\n[End Skill]\n");
    output
}

// ── US-004: Skill Injection + Variable Substitution ──

/// Context variables for skill template substitution
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SkillContext {
    pub project_name: String,
    pub branch: String,
    pub story_title: Option<String>,
    pub story_id: Option<String>,
    pub slot_index: u32,
    pub agent_type: String,
}

/// Substitute template variables in skill content
pub fn substitute_variables(content: &str, ctx: &SkillContext) -> String {
    content
        .replace("{{PROJECT_NAME}}", &ctx.project_name)
        .replace("{{BRANCH}}", &ctx.branch)
        .replace("{{STORY_TITLE}}", ctx.story_title.as_deref().unwrap_or(""))
        .replace("{{STORY_ID}}", ctx.story_id.as_deref().unwrap_or(""))
        .replace("{{SLOT_INDEX}}", &ctx.slot_index.to_string())
        .replace("{{AGENT_TYPE}}", &ctx.agent_type)
}

/// Prepare skill content for injection into AGENT.md.
/// Supports multiple skills (concatenated), CLI adaptation, and variable substitution.
pub fn prepare_skill_injection(
    skills: &[LoadedSkill],
    cli_type: &str,
    ctx: &SkillContext,
) -> String {
    let mut parts = Vec::new();
    for skill in skills {
        let adapted = adapt_for_cli(skill, cli_type);
        let substituted = substitute_variables(&adapted, ctx);
        parts.push(substituted);
    }
    parts.join("\n\n---\n\n")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_frontmatter() {
        let content = "---\nname: backend\nfamily: engineering\ndescription: Backend dev skill\n---\n\n# Backend Skill\n\nDo backend things.";
        let (fm, body) = parse_frontmatter(content);
        assert_eq!(fm.get("name").unwrap(), "backend");
        assert_eq!(fm.get("family").unwrap(), "engineering");
        assert!(body.contains("Backend Skill"));
    }

    #[test]
    fn test_parse_frontmatter_no_frontmatter() {
        let content = "# Just markdown\n\nNo frontmatter here.";
        let (fm, body) = parse_frontmatter(content);
        assert!(fm.is_empty());
        assert_eq!(body, content);
    }

    #[test]
    fn test_scan_skills_from_dir() {
        let dir = format!("/tmp/xroads-skills-test-{}", uuid::Uuid::new_v4());
        let skills_dir = format!("{}/skills", dir);
        std::fs::create_dir_all(&skills_dir).unwrap();

        std::fs::write(
            format!("{}/backend.md", skills_dir),
            "---\nname: backend\nfamily: engineering\n---\n\nBackend skill content",
        ).unwrap();
        std::fs::write(
            format!("{}/frontend.md", skills_dir),
            "---\nname: frontend\nfamily: engineering\n---\n\nFrontend skill content",
        ).unwrap();

        let skills = scan_skills(Some(&dir));
        assert!(skills.len() >= 2);
        assert!(skills.iter().any(|s| s.name == "backend"));
        assert!(skills.iter().any(|s| s.name == "frontend"));

        std::fs::remove_dir_all(&dir).ok();
    }

    #[test]
    fn test_user_overrides_bundled() {
        let dir = format!("/tmp/xroads-override-test-{}", uuid::Uuid::new_v4());
        let bundled = format!("{}/skills", dir);
        let user = format!("{}/.crossroads/skills", dir);
        std::fs::create_dir_all(&bundled).unwrap();
        std::fs::create_dir_all(&user).unwrap();

        std::fs::write(
            format!("{}/test.md", bundled),
            "---\nname: test\nfamily: general\n---\nBundled version",
        ).unwrap();
        std::fs::write(
            format!("{}/test.md", user),
            "---\nname: test\nfamily: general\n---\nUser version",
        ).unwrap();

        let skills = scan_skills(Some(&dir));
        let test_skill = skills.iter().find(|s| s.name == "test").unwrap();
        assert!(test_skill.is_user_override);
        assert!(test_skill.content.contains("User version"));
    }

    #[test]
    fn test_filter_by_family() {
        let skills = vec![
            LoadedSkill { name: "a".into(), family: "eng".into(), description: None, required_mcps: None, content: "".into(), source_path: "".into(), is_user_override: false },
            LoadedSkill { name: "b".into(), family: "design".into(), description: None, required_mcps: None, content: "".into(), source_path: "".into(), is_user_override: false },
            LoadedSkill { name: "c".into(), family: "eng".into(), description: None, required_mcps: None, content: "".into(), source_path: "".into(), is_user_override: false },
        ];
        let filtered = filter_by_family(&skills, "eng");
        assert_eq!(filtered.len(), 2);
    }

    #[test]
    fn test_claude_adapter() {
        let result = adapt_claude("Do the thing.");
        assert!(result.starts_with("## Assigned Skill"));
        assert!(result.contains("Do the thing."));
    }

    #[test]
    fn test_gemini_adapter() {
        let result = adapt_gemini("Do the thing.", "backend");
        assert!(result.contains("<skill name=\"backend\">"));
        assert!(result.contains("<instructions>"));
        assert!(result.contains("Do the thing."));
    }

    #[test]
    fn test_codex_adapter() {
        let result = adapt_codex("Do the thing.", "backend");
        assert!(result.contains("[Skill: backend]"));
        assert!(result.contains("Follow these instructions"));
    }

    #[test]
    fn test_variable_substitution() {
        let content = "Project: {{PROJECT_NAME}}, Branch: {{BRANCH}}, Story: {{STORY_TITLE}}";
        let ctx = SkillContext {
            project_name: "MyApp".into(),
            branch: "feat/auth".into(),
            story_title: Some("Login form".into()),
            story_id: Some("US-001".into()),
            slot_index: 0,
            agent_type: "claude".into(),
        };
        let result = substitute_variables(content, &ctx);
        assert_eq!(result, "Project: MyApp, Branch: feat/auth, Story: Login form");
    }

    #[test]
    fn test_prepare_skill_injection_multi() {
        let skills = vec![
            LoadedSkill { name: "a".into(), family: "eng".into(), description: None, required_mcps: None, content: "Skill A for {{PROJECT_NAME}}".into(), source_path: "".into(), is_user_override: false },
            LoadedSkill { name: "b".into(), family: "eng".into(), description: None, required_mcps: None, content: "Skill B on {{BRANCH}}".into(), source_path: "".into(), is_user_override: false },
        ];
        let ctx = SkillContext {
            project_name: "TestProj".into(),
            branch: "main".into(),
            story_title: None,
            story_id: None,
            slot_index: 0,
            agent_type: "claude".into(),
        };
        let result = prepare_skill_injection(&skills, "claude", &ctx);
        assert!(result.contains("Skill A for TestProj"));
        assert!(result.contains("Skill B on main"));
        assert!(result.contains("---")); // separator between skills
    }

    #[test]
    fn test_register_skills_to_db() {
        crate::db::manager::initialize_memory().unwrap();
        let skills = vec![
            LoadedSkill { name: "db-test-skill".into(), family: "test".into(), description: Some("Test".into()), required_mcps: None, content: "content".into(), source_path: "/tmp/s.md".into(), is_user_override: false },
        ];
        let ids = register_skills(&skills).unwrap();
        assert_eq!(ids.len(), 1);

        // Idempotent
        let ids2 = register_skills(&skills).unwrap();
        assert_eq!(ids[0], ids2[0]);
    }
}
