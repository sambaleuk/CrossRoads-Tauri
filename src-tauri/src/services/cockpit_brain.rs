use serde::{Deserialize, Serialize};
use std::path::Path;

use crate::db::{session_repo, slot_repo, cost_repo};

// ── Cockpit Brain: Autonomous Orchestration Layer ──
//
// Sits above coding agents. Generates a COP (Cockpit Orchestration Plan)
// that drives transverse productions (marketing, docs, strategy),
// specialist triggers (security, compliance, dx), and a META monitor agent.

// ── Core Structs ──

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CockpitOrchestrationPlan {
    pub project_name: String,
    pub project_type: String,
    pub domain: String,
    pub market_context: String,
    pub transverse_productions: Vec<TransverseCategory>,
    pub specialist_triggers: Vec<SpecialistTrigger>,
    pub meta_agent_config: MetaAgentConfig,
    pub deliverables_path: String,
    pub created_at: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct TransverseCategory {
    pub category: String,
    pub deliverables: Vec<String>,
    pub priority: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SpecialistTrigger {
    pub condition: String,
    pub specialist: String,
    pub reason: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct MetaAgentConfig {
    pub capabilities: Vec<String>,
    pub monitoring_interval_ms: u32,
    pub autonomy_level: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AdaptationAction {
    pub action: String,
    pub target_slot: Option<String>,
    pub reason: String,
}

// ── Project Detection Helpers ──

/// Detect project type from config files in the project directory.
fn detect_project_type(project_path: &Path) -> String {
    if project_path.join("package.json").exists() {
        // Distinguish SaaS/web from other Node projects
        let content = std::fs::read_to_string(project_path.join("package.json")).unwrap_or_default();
        let pkg: serde_json::Value = serde_json::from_str(&content).unwrap_or_default();

        let has_next = pkg.get("dependencies")
            .and_then(|d| d.get("next"))
            .is_some();
        let has_react = pkg.get("dependencies")
            .and_then(|d| d.get("react"))
            .is_some();
        let has_express = pkg.get("dependencies")
            .and_then(|d| d.get("express"))
            .is_some();
        let has_fastify = pkg.get("dependencies")
            .and_then(|d| d.get("fastify"))
            .is_some();

        if has_next || has_react {
            return "saas".into();
        }
        if has_express || has_fastify {
            return "api".into();
        }

        // Check for CLI indicators
        let has_bin = pkg.get("bin").is_some();
        if has_bin {
            return "cli".into();
        }

        return "web".into();
    }

    if project_path.join("Cargo.toml").exists() {
        let content = std::fs::read_to_string(project_path.join("Cargo.toml")).unwrap_or_default();
        let lower = content.to_lowercase();

        // Check for [[bin]] section or specific crate types
        if lower.contains("[[bin]]") || lower.contains("crate-type") && lower.contains("\"bin\"") {
            return "cli".into();
        }
        if lower.contains("actix") || lower.contains("axum") || lower.contains("rocket") || lower.contains("warp") {
            return "api".into();
        }
        if lower.contains("[lib]") && !lower.contains("[[bin]]") {
            return "library".into();
        }
        return "cli".into();
    }

    if project_path.join("pubspec.yaml").exists() {
        return "mobile".into();
    }

    if project_path.join("build.gradle").exists() || project_path.join("build.gradle.kts").exists() {
        let has_android = project_path.join("android").exists();
        if has_android {
            return "mobile".into();
        }
        return "api".into();
    }

    "web".into()
}

/// Detect domain from PRD content using keyword matching.
fn detect_domain_from_prd(prd_json: &serde_json::Value) -> String {
    let mut text_pool = String::new();

    if let Some(name) = prd_json.get("feature_name").and_then(|v| v.as_str()) {
        text_pool.push_str(name);
        text_pool.push(' ');
    }
    if let Some(desc) = prd_json.get("description").and_then(|v| v.as_str()) {
        text_pool.push_str(desc);
        text_pool.push(' ');
    }
    if let Some(stories) = prd_json.get("user_stories").and_then(|v| v.as_array()) {
        for story in stories {
            if let Some(title) = story.get("title").and_then(|v| v.as_str()) {
                text_pool.push_str(title);
                text_pool.push(' ');
            }
            if let Some(desc) = story.get("description").and_then(|v| v.as_str()) {
                text_pool.push_str(desc);
                text_pool.push(' ');
            }
        }
    }

    let lower = text_pool.to_lowercase();

    // Domain keyword rules (ordered by specificity)
    let domain_rules: &[(&str, &[&str])] = &[
        ("fintech", &["payment", "billing", "subscription", "invoice", "stripe", "checkout", "wallet", "transaction"]),
        ("health", &["patient", "medical", "health", "diagnosis", "appointment", "prescription", "clinical"]),
        ("security", &["auth", "login", "password", "session", "token", "oauth", "permission", "role", "rbac", "jwt"]),
        ("devtools", &["api", "endpoint", "rest", "graphql", "sdk", "cli", "developer", "plugin", "extension"]),
        ("ecommerce", &["cart", "product", "catalog", "order", "shipping", "inventory"]),
        ("analytics", &["dashboard", "metric", "report", "chart", "analytics", "tracking", "event"]),
        ("data", &["pipeline", "etl", "data", "transform", "migration", "batch", "stream"]),
        ("infra", &["deploy", "infrastructure", "ci", "cd", "kubernetes", "docker", "terraform"]),
        ("ai", &["model", "ml", "prediction", "training", "inference", "embedding", "llm"]),
        ("messaging", &["chat", "message", "notification", "email", "sms", "push"]),
    ];

    let mut best_domain = "general";
    let mut best_score = 0u32;

    for (domain, keywords) in domain_rules {
        let score: u32 = keywords.iter()
            .map(|kw| if lower.contains(kw) { 1 } else { 0 })
            .sum();
        if score > best_score {
            best_score = score;
            best_domain = domain;
        }
    }

    best_domain.to_string()
}

/// Detect market context from README.md content (heuristic).
fn detect_market_context(project_path: &Path) -> String {
    let readme_path = project_path.join("README.md");
    let content = std::fs::read_to_string(&readme_path).unwrap_or_default().to_lowercase();

    if content.contains("compliance") || content.contains("regulation") || content.contains("hipaa")
        || content.contains("gdpr") || content.contains("pci") || content.contains("sox")
    {
        return "regulated".into();
    }

    if content.contains("competitor") || content.contains("alternative to")
        || content.contains("vs ") || content.contains("compared to")
    {
        return "competitive".into();
    }

    "greenfield".into()
}

// ── Transverse Production Logic ──

/// Determine transverse production categories based on project type and domain.
fn build_transverse_productions(project_type: &str, domain: &str) -> Vec<TransverseCategory> {
    let mut categories = Vec::new();

    // All projects need documentation
    categories.push(TransverseCategory {
        category: "documentation".into(),
        deliverables: match project_type {
            "cli" => vec![
                "man-page".into(),
                "install-guide".into(),
                "usage-examples".into(),
                "changelog".into(),
            ],
            "api" => vec![
                "api-reference".into(),
                "sdk-guide".into(),
                "authentication-guide".into(),
                "deploy-guide".into(),
                "changelog".into(),
            ],
            "library" => vec![
                "api-reference".into(),
                "getting-started".into(),
                "migration-guide".into(),
                "changelog".into(),
            ],
            _ => vec![
                "README".into(),
                "deploy-guide".into(),
                "architecture-overview".into(),
                "changelog".into(),
            ],
        },
        priority: "high".into(),
    });

    // SaaS and web projects need marketing
    if project_type == "saas" || project_type == "web" {
        categories.push(TransverseCategory {
            category: "marketing".into(),
            deliverables: vec![
                "landing-page-copy".into(),
                "value-proposition".into(),
                "feature-comparison".into(),
                "onboarding-flow-copy".into(),
            ],
            priority: "high".into(),
        });

        categories.push(TransverseCategory {
            category: "strategy".into(),
            deliverables: vec![
                "pricing-model".into(),
                "go-to-market-plan".into(),
                "user-persona-profiles".into(),
            ],
            priority: "medium".into(),
        });
    }

    // API projects need developer-focused research
    if project_type == "api" {
        categories.push(TransverseCategory {
            category: "research".into(),
            deliverables: vec![
                "developer-personas".into(),
                "integration-patterns".into(),
                "rate-limit-strategy".into(),
            ],
            priority: "medium".into(),
        });
    }

    // SaaS projects need competitive research
    if project_type == "saas" {
        categories.push(TransverseCategory {
            category: "research".into(),
            deliverables: vec![
                "competitive-analysis".into(),
                "market-sizing".into(),
                "user-interview-template".into(),
            ],
            priority: "medium".into(),
        });
    }

    // Domain-specific additions
    match domain {
        "fintech" => {
            categories.push(TransverseCategory {
                category: "ops".into(),
                deliverables: vec![
                    "compliance-checklist".into(),
                    "audit-trail-spec".into(),
                    "incident-response-plan".into(),
                ],
                priority: "high".into(),
            });
        }
        "health" => {
            categories.push(TransverseCategory {
                category: "ops".into(),
                deliverables: vec![
                    "hipaa-compliance-checklist".into(),
                    "data-handling-policy".into(),
                    "consent-flow-spec".into(),
                ],
                priority: "high".into(),
            });
        }
        "security" => {
            categories.push(TransverseCategory {
                category: "ops".into(),
                deliverables: vec![
                    "security-policy".into(),
                    "threat-model".into(),
                    "penetration-test-plan".into(),
                ],
                priority: "high".into(),
            });
        }
        _ => {}
    }

    categories
}

// ── Specialist Detection ──

/// Specialist trigger rules: condition keyword -> specialist name + reason.
const SPECIALIST_RULES: &[(&[&str], &str, &str)] = &[
    (
        &["auth", "login", "password", "session", "oauth", "jwt", "permission"],
        "security-auditor",
        "Authentication and authorization code requires security review",
    ),
    (
        &["payment", "billing", "subscription", "stripe", "invoice", "checkout"],
        "fintech-compliance",
        "Payment processing code must comply with financial regulations",
    ),
    (
        &["api", "endpoint", "rest", "graphql", "sdk", "webhook"],
        "dx-specialist",
        "API surface needs developer experience review",
    ),
    (
        &["data", "pipeline", "etl", "migration", "batch", "transform"],
        "data-quality",
        "Data processing pipelines need quality and validation review",
    ),
    (
        &["deploy", "infrastructure", "ci", "cd", "kubernetes", "docker", "terraform"],
        "devops-specialist",
        "Infrastructure code needs operational review",
    ),
    (
        &["accessibility", "a11y", "aria", "screen-reader", "wcag"],
        "accessibility-specialist",
        "UI code needs accessibility compliance review",
    ),
    (
        &["performance", "cache", "latency", "throughput", "benchmark"],
        "performance-specialist",
        "Performance-sensitive code needs optimization review",
    ),
];

/// Detect specialist needs from PRD story titles and descriptions.
pub fn detect_specialist_needs(prd_json: &serde_json::Value) -> Vec<SpecialistTrigger> {
    let stories = match prd_json.get("user_stories").and_then(|v| v.as_array()) {
        Some(s) => s,
        None => return Vec::new(),
    };

    let mut text_pool = String::new();
    for story in stories {
        if let Some(title) = story.get("title").and_then(|v| v.as_str()) {
            text_pool.push_str(title);
            text_pool.push(' ');
        }
        if let Some(desc) = story.get("description").and_then(|v| v.as_str()) {
            text_pool.push_str(desc);
            text_pool.push(' ');
        }
    }

    let lower = text_pool.to_lowercase();
    let mut triggers = Vec::new();
    let mut seen_specialists: std::collections::HashSet<String> = std::collections::HashSet::new();

    for (keywords, specialist, reason) in SPECIALIST_RULES {
        let matched_keywords: Vec<&&str> = keywords.iter()
            .filter(|kw| lower.contains(**kw))
            .collect();

        if !matched_keywords.is_empty() && !seen_specialists.contains(*specialist) {
            let condition = matched_keywords.iter()
                .map(|kw| format!("domain contains \"{}\"", kw))
                .collect::<Vec<_>>()
                .join(" OR ");

            triggers.push(SpecialistTrigger {
                condition,
                specialist: specialist.to_string(),
                reason: reason.to_string(),
            });
            seen_specialists.insert(specialist.to_string());
        }
    }

    triggers
}

// ── Core Functions ──

/// Generate a Cockpit Orchestration Plan by analyzing project context and PRD.
pub fn generate_cop(project_path: &str, prd_json: &str) -> Result<CockpitOrchestrationPlan, String> {
    let path = Path::new(project_path);
    if !path.exists() {
        return Err(format!("Project path does not exist: {}", project_path));
    }

    let prd: serde_json::Value = serde_json::from_str(prd_json)
        .map_err(|e| format!("Invalid PRD JSON: {}", e))?;

    // Extract project name
    let project_name = path.file_name()
        .map(|n| n.to_string_lossy().to_string())
        .unwrap_or_else(|| "project".into());

    // Detect project characteristics
    let project_type = detect_project_type(path);
    let domain = detect_domain_from_prd(&prd);
    let market_context = detect_market_context(path);

    // Build transverse productions
    let transverse_productions = build_transverse_productions(&project_type, &domain);

    // Detect specialist triggers from PRD
    let specialist_triggers = detect_specialist_needs(&prd);

    // Configure META agent
    let mut capabilities = vec!["qa".to_string(), "doc_gen".to_string(), "git_master".to_string()];
    if domain == "security" || specialist_triggers.iter().any(|t| t.specialist == "security-auditor") {
        capabilities.push("security_scan".to_string());
    }

    let meta_agent_config = MetaAgentConfig {
        capabilities,
        monitoring_interval_ms: 30_000,
        autonomy_level: if market_context == "regulated" {
            "limited".into()
        } else {
            "full".into()
        },
    };

    let deliverables_path = format!("{}/.crossroads/deliverables/", project_path);

    let cop = CockpitOrchestrationPlan {
        project_name,
        project_type,
        domain,
        market_context,
        transverse_productions,
        specialist_triggers,
        meta_agent_config,
        deliverables_path: deliverables_path.clone(),
        created_at: chrono::Utc::now().to_rfc3339(),
    };

    // Persist COP to .crossroads/cockpit-plan.json
    let crossroads_dir = Path::new(project_path).join(".crossroads");
    std::fs::create_dir_all(&crossroads_dir)
        .map_err(|e| format!("Failed to create .crossroads dir: {}", e))?;

    let cop_json = serde_json::to_string_pretty(&cop)
        .map_err(|e| format!("Failed to serialize COP: {}", e))?;
    std::fs::write(crossroads_dir.join("cockpit-plan.json"), &cop_json)
        .map_err(|e| format!("Failed to write cockpit-plan.json: {}", e))?;

    Ok(cop)
}

/// Load a previously generated COP from disk.
pub fn get_cop(project_path: &str) -> Result<Option<CockpitOrchestrationPlan>, String> {
    let cop_path = Path::new(project_path).join(".crossroads").join("cockpit-plan.json");
    if !cop_path.exists() {
        return Ok(None);
    }

    let content = std::fs::read_to_string(&cop_path)
        .map_err(|e| format!("Failed to read cockpit-plan.json: {}", e))?;
    let cop: CockpitOrchestrationPlan = serde_json::from_str(&content)
        .map_err(|e| format!("Failed to parse cockpit-plan.json: {}", e))?;
    Ok(Some(cop))
}

// ── Skill Generation ──

/// Generate a SKILL.md for the META monitor agent.
pub fn generate_meta_skill(cop: &CockpitOrchestrationPlan) -> String {
    let risk_areas: Vec<&str> = cop.specialist_triggers.iter()
        .map(|t| t.specialist.as_str())
        .collect();

    let test_cmd = match cop.project_type.as_str() {
        "cli" | "library" => "cargo test",
        "api" if cop.deliverables_path.contains("Cargo") => "cargo test",
        _ => "npm test",
    };

    format!(
r#"# META Monitor Skill

You are the code quality monitor for the **{project_name}** project.

## Your responsibilities:
- Monitor all git worktrees for code quality issues
- Run `{test_cmd}` to verify test coverage after each agent iteration
- Check for security vulnerabilities in new code
- Auto-generate documentation updates when APIs change
- Flag merge conflicts early and report to the cockpit
- Report status every iteration

## Project context:
- **Type:** {project_type}
- **Domain:** {domain}
- **Market:** {market_context}
- **Risk areas:** {risk_areas}

## Capabilities:
{capabilities}

## You have access to:
- Git operations (diff, log, merge, worktree list)
- Test runners (`{test_cmd}`)
- File system (read/write .md files in .crossroads/)
- Conflict detection between worktrees

## Workflow:
1. Check each active worktree for uncommitted changes
2. Run tests in each worktree
3. Compare diffs between worktrees for potential conflicts
4. Update `.crossroads/meta-status.json` with findings
5. Create alerts for any issues found
"#,
        project_name = cop.project_name,
        test_cmd = test_cmd,
        project_type = cop.project_type,
        domain = cop.domain,
        market_context = cop.market_context,
        risk_areas = if risk_areas.is_empty() { "none identified".to_string() } else { risk_areas.join(", ") },
        capabilities = cop.meta_agent_config.capabilities.iter()
            .map(|c| format!("- {}", c))
            .collect::<Vec<_>>()
            .join("\n"),
    )
}

/// Generate a SKILL.md for a transverse production agent.
pub fn generate_transverse_skill(cop: &CockpitOrchestrationPlan, category: &str) -> Result<String, String> {
    let transverse = cop.transverse_productions.iter()
        .find(|t| t.category == category)
        .ok_or_else(|| format!("No transverse category '{}' found in COP", category))?;

    let deliverables_list = transverse.deliverables.iter()
        .map(|d| format!("- [ ] `{}.md`", d))
        .collect::<Vec<_>>()
        .join("\n");

    // Build PRD context summary from COP
    let specialist_context = if cop.specialist_triggers.is_empty() {
        "No specialist concerns identified.".to_string()
    } else {
        cop.specialist_triggers.iter()
            .map(|t| format!("- **{}**: {}", t.specialist, t.reason))
            .collect::<Vec<_>>()
            .join("\n")
    };

    Ok(format!(
r#"# {category_title} Production Skill

You are producing **{category}** deliverables for the **{project_name}** project.

## Deliverables to create:
{deliverables_list}

## Quality requirements:
- Each deliverable must be a standalone `.md` file
- Each deliverable must be actionable and specific (no filler content)
- Reference actual project context, features, and domain terminology
- Save all files to: `.crossroads/deliverables/{category}/`

## Project context:
- **Project:** {project_name}
- **Type:** {project_type}
- **Domain:** {domain}
- **Market:** {market_context}
- **Priority for this category:** {priority}

## Specialist considerations:
{specialist_context}

## Workflow:
1. Read the project README and PRD to understand features
2. Create each deliverable file in order of importance
3. Cross-reference deliverables for consistency
4. Write a `_index.md` summarizing all deliverables produced
"#,
        category_title = capitalize_first(category),
        category = category,
        project_name = cop.project_name,
        project_type = cop.project_type,
        domain = cop.domain,
        market_context = cop.market_context,
        priority = transverse.priority,
        deliverables_list = deliverables_list,
        specialist_context = specialist_context,
    ))
}

/// Generate a SKILL.md for a specialist agent.
pub fn generate_specialist_skill(cop: &CockpitOrchestrationPlan, specialist_name: &str) -> Result<String, String> {
    let trigger = cop.specialist_triggers.iter()
        .find(|t| t.specialist == specialist_name)
        .ok_or_else(|| format!("No specialist '{}' found in COP triggers", specialist_name))?;

    let focus_areas = match specialist_name {
        "security-auditor" => vec![
            "Authentication flow correctness",
            "Session management security",
            "Input validation and sanitization",
            "Secret handling (no hardcoded keys)",
            "OWASP Top 10 compliance",
        ],
        "fintech-compliance" => vec![
            "PCI DSS compliance for payment handling",
            "Idempotency in financial transactions",
            "Audit trail completeness",
            "Error handling for payment failures",
            "Data encryption at rest and in transit",
        ],
        "dx-specialist" => vec![
            "API consistency and naming conventions",
            "Error response format standardization",
            "Rate limiting documentation",
            "SDK usability and code examples",
            "Versioning strategy",
        ],
        "data-quality" => vec![
            "Data validation at pipeline boundaries",
            "Schema evolution handling",
            "Idempotent processing guarantees",
            "Error recovery and dead letter queues",
            "Data lineage tracking",
        ],
        "devops-specialist" => vec![
            "CI/CD pipeline reliability",
            "Infrastructure as code best practices",
            "Monitoring and alerting coverage",
            "Rollback procedures",
            "Secret management in deployment",
        ],
        "accessibility-specialist" => vec![
            "WCAG 2.1 AA compliance",
            "Screen reader compatibility",
            "Keyboard navigation support",
            "Color contrast ratios",
            "ARIA attribute correctness",
        ],
        "performance-specialist" => vec![
            "Query optimization and N+1 detection",
            "Cache invalidation strategy",
            "Memory usage profiling",
            "Response time budgets",
            "Load testing coverage",
        ],
        _ => vec![
            "Code quality review",
            "Best practices compliance",
            "Documentation accuracy",
        ],
    };

    let focus_list = focus_areas.iter()
        .map(|f| format!("- {}", f))
        .collect::<Vec<_>>()
        .join("\n");

    Ok(format!(
r#"# {specialist_title} Specialist Skill

You are the **{specialist_name}** for the **{project_name}** project.

## Trigger condition:
{condition}

## Reason for activation:
{reason}

## Focus areas:
{focus_list}

## Project context:
- **Type:** {project_type}
- **Domain:** {domain}
- **Market:** {market_context}

## Workflow:
1. Review all code changes in active worktrees related to your domain
2. Identify vulnerabilities, compliance gaps, or quality issues
3. Write findings to `.crossroads/deliverables/specialist/{specialist_name}-report.md`
4. Flag critical issues as blocking gates
5. Provide specific remediation steps for each finding

## Output format:
Each finding should include:
- **Severity**: critical / high / medium / low
- **Location**: file path and line range
- **Description**: what the issue is
- **Remediation**: how to fix it
"#,
        specialist_title = specialist_name.split('-').map(capitalize_first).collect::<Vec<_>>().join(" "),
        specialist_name = specialist_name,
        project_name = cop.project_name,
        condition = trigger.condition,
        reason = trigger.reason,
        focus_list = focus_list,
        project_type = cop.project_type,
        domain = cop.domain,
        market_context = cop.market_context,
    ))
}

// ── Deliverables Structure ──

/// Create the deliverables directory structure under .crossroads/.
pub fn create_deliverables_structure(project_path: &str, cop: &CockpitOrchestrationPlan) -> Result<(), String> {
    let base = Path::new(project_path).join(".crossroads").join("deliverables");

    // Standard directories
    let dirs = ["marketing", "research", "documentation", "strategy", "ops", "specialist"];

    for dir in &dirs {
        std::fs::create_dir_all(base.join(dir))
            .map_err(|e| format!("Failed to create {}: {}", dir, e))?;
    }

    // Write a manifest of expected deliverables
    let mut manifest = String::from("# Deliverables Manifest\n\n");
    manifest.push_str(&format!("Project: {}\n", cop.project_name));
    manifest.push_str(&format!("Generated: {}\n\n", cop.created_at));

    for cat in &cop.transverse_productions {
        manifest.push_str(&format!("## {} (priority: {})\n", capitalize_first(&cat.category), cat.priority));
        for d in &cat.deliverables {
            manifest.push_str(&format!("- [ ] {}.md\n", d));
        }
        manifest.push('\n');
    }

    if !cop.specialist_triggers.is_empty() {
        manifest.push_str("## Specialist Reports\n");
        for t in &cop.specialist_triggers {
            manifest.push_str(&format!("- [ ] {}-report.md\n", t.specialist));
        }
        manifest.push('\n');
    }

    std::fs::write(base.join("_manifest.md"), &manifest)
        .map_err(|e| format!("Failed to write manifest: {}", e))?;

    Ok(())
}

// ── Activation & Adaptation ──

/// Returns true when coding progress reaches 50%+, signaling
/// that transverse production agents should be activated.
pub fn should_activate_transverse(coding_progress_pct: f64) -> bool {
    coding_progress_pct >= 50.0
}

/// Analyze current session state and return adaptation actions.
pub fn get_adaptation_actions(session_id: &str) -> Result<Vec<AdaptationAction>, String> {
    let session = session_repo::fetch_session(session_id)
        .map_err(|e| e.to_string())?
        .ok_or_else(|| format!("Session {} not found", session_id))?;

    let slots = slot_repo::fetch_slots_for_session(session_id)
        .map_err(|e| e.to_string())?;

    let cost_summary = cost_repo::summary_for_session(session_id)
        .map_err(|e| e.to_string())?;

    let mut actions = Vec::new();

    // Count slot states
    let total_slots = slots.len();
    let done_slots = slots.iter().filter(|s| s.status == "done").count();
    let error_slots = slots.iter().filter(|s| s.status == "error").count();
    let running_slots = slots.iter().filter(|s| s.status == "running").count();

    // Rule: Too many test failures -> focus debugging
    // Error slots > 3 means repeated failures
    if error_slots > 3 {
        actions.push(AdaptationAction {
            action: "focus_debugging".into(),
            target_slot: None,
            reason: format!("{} slots in error state — prioritize debugging before proceeding", error_slots),
        });
    }

    // Rule: Specific error slots -> target them for debugging
    for slot in &slots {
        if slot.status == "error" {
            actions.push(AdaptationAction {
                action: "focus_debugging".into(),
                target_slot: Some(slot.id.clone()),
                reason: format!(
                    "Slot {} (index {}) failed: {}",
                    slot.id, slot.slot_index,
                    slot.current_task.as_deref().unwrap_or("unknown error")
                ),
            });
        }
    }

    // Rule: Budget > 80% -> switch to cheaper model
    // Estimate budget usage: if total cost > 80% of a reasonable default (2500 cents = $25)
    let budget_cents = 2500_i64;
    let percent_used = if budget_cents > 0 {
        (cost_summary.total_cost_cents as f64 / budget_cents as f64) * 100.0
    } else {
        0.0
    };

    if percent_used > 80.0 {
        actions.push(AdaptationAction {
            action: "switch_model".into(),
            target_slot: None,
            reason: format!(
                "Budget at {:.0}% ({}c / {}c) — switch to cheaper model for remaining work",
                percent_used, cost_summary.total_cost_cents, budget_cents
            ),
        });
    }

    // Rule: Coding complete -> activate transverse
    if total_slots > 0 && done_slots == total_slots {
        actions.push(AdaptationAction {
            action: "activate_transverse".into(),
            target_slot: None,
            reason: "All coding slots completed — activate transverse production agents".into(),
        });
    } else if total_slots > 0 {
        let progress = (done_slots as f64 / total_slots as f64) * 100.0;
        if should_activate_transverse(progress) {
            actions.push(AdaptationAction {
                action: "activate_transverse".into(),
                target_slot: None,
                reason: format!(
                    "Coding at {:.0}% ({}/{} done) — transverse agents can start in parallel",
                    progress, done_slots, total_slots
                ),
            });
        }
    }

    // Rule: Conflict detected (multiple slots on error with similar tasks)
    if error_slots >= 2 {
        let error_tasks: Vec<String> = slots.iter()
            .filter(|s| s.status == "error")
            .filter_map(|s| s.current_task.clone())
            .collect();

        // Simple overlap check: if multiple error slots mention similar words
        let has_conflict = error_tasks.windows(2).any(|pair| {
            let words_a: std::collections::HashSet<&str> = pair[0].split_whitespace().collect();
            let words_b: std::collections::HashSet<&str> = pair[1].split_whitespace().collect();
            let overlap: usize = words_a.intersection(&words_b).count();
            overlap >= 2
        });

        if has_conflict {
            actions.push(AdaptationAction {
                action: "pause_and_resolve".into(),
                target_slot: None,
                reason: "Potential conflict detected between error slots — pause and resolve before continuing".into(),
            });
        }
    }

    // Rule: No running slots but session is active -> need attention
    if session.status == "active" && running_slots == 0 && done_slots < total_slots {
        actions.push(AdaptationAction {
            action: "restart_stalled".into(),
            target_slot: None,
            reason: format!(
                "Session active but no running slots ({}/{} done) — restart stalled agents",
                done_slots, total_slots
            ),
        });
    }

    Ok(actions)
}

// ── Utility ──

fn capitalize_first(s: &str) -> String {
    let mut chars = s.chars();
    match chars.next() {
        None => String::new(),
        Some(c) => c.to_uppercase().collect::<String>() + chars.as_str(),
    }
}

// ── Tests ──

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;

    fn make_temp_dir(name: &str) -> String {
        let dir = format!("/tmp/xroads-brain-test-{}-{}", name, uuid::Uuid::new_v4());
        fs::create_dir_all(&dir).unwrap();
        dir
    }

    fn sample_prd_saas() -> String {
        serde_json::json!({
            "feature_name": "User Dashboard",
            "description": "Build a SaaS dashboard with billing integration",
            "status": "pending",
            "user_stories": [
                {
                    "id": "US-001",
                    "title": "Create payment checkout flow",
                    "description": "Integrate Stripe for subscription billing",
                    "status": "pending"
                },
                {
                    "id": "US-002",
                    "title": "Build user login with OAuth",
                    "description": "Implement authentication using Google OAuth and JWT tokens",
                    "status": "pending"
                },
                {
                    "id": "US-003",
                    "title": "Dashboard metrics page",
                    "description": "Show analytics and charts for user activity",
                    "status": "pending"
                }
            ]
        }).to_string()
    }

    fn sample_prd_cli() -> String {
        serde_json::json!({
            "feature_name": "CLI Data Pipeline Tool",
            "description": "A CLI for ETL data transformations",
            "status": "pending",
            "user_stories": [
                {
                    "id": "US-001",
                    "title": "Implement data pipeline command",
                    "description": "Transform CSV files through configurable pipeline stages",
                    "status": "pending"
                },
                {
                    "id": "US-002",
                    "title": "Add batch processing mode",
                    "description": "Process multiple files in batch with progress reporting",
                    "status": "pending"
                }
            ]
        }).to_string()
    }

    #[test]
    fn test_generate_cop_saas() {
        let dir = make_temp_dir("cop-saas");

        // Create package.json with React (SaaS indicator)
        fs::write(
            format!("{}/package.json", dir),
            r#"{"name":"my-saas","dependencies":{"react":"^19","next":"^15","stripe":"^14"}}"#,
        ).unwrap();

        let cop = generate_cop(&dir, &sample_prd_saas()).unwrap();

        assert_eq!(cop.project_type, "saas");
        assert!(cop.domain == "fintech" || cop.domain == "security",
            "Expected fintech or security, got {}", cop.domain);

        // SaaS must have marketing + strategy
        let categories: Vec<&str> = cop.transverse_productions.iter()
            .map(|t| t.category.as_str())
            .collect();
        assert!(categories.contains(&"marketing"), "SaaS should have marketing, got: {:?}", categories);
        assert!(categories.contains(&"strategy"), "SaaS should have strategy, got: {:?}", categories);
        assert!(categories.contains(&"documentation"), "Should always have documentation");

        // Payment stories should trigger fintech-compliance specialist
        let specialists: Vec<&str> = cop.specialist_triggers.iter()
            .map(|t| t.specialist.as_str())
            .collect();
        assert!(specialists.contains(&"fintech-compliance"),
            "Payment stories should trigger fintech-compliance, got: {:?}", specialists);

        // Verify COP was persisted
        let cop_path = Path::new(&dir).join(".crossroads").join("cockpit-plan.json");
        assert!(cop_path.exists(), "cockpit-plan.json should be written to disk");

        fs::remove_dir_all(&dir).ok();
    }

    #[test]
    fn test_generate_cop_cli() {
        let dir = make_temp_dir("cop-cli");

        // Create Cargo.toml with [[bin]]
        fs::write(
            format!("{}/Cargo.toml", dir),
            "[package]\nname = \"data-tool\"\n\n[[bin]]\nname = \"data-tool\"\n",
        ).unwrap();

        let cop = generate_cop(&dir, &sample_prd_cli()).unwrap();

        assert_eq!(cop.project_type, "cli");
        assert_eq!(cop.domain, "data");

        // CLI should have man-page in documentation
        let doc_cat = cop.transverse_productions.iter()
            .find(|t| t.category == "documentation")
            .expect("CLI should have documentation category");
        assert!(doc_cat.deliverables.contains(&"man-page".to_string()),
            "CLI should have man-page deliverable, got: {:?}", doc_cat.deliverables);
        assert!(doc_cat.deliverables.contains(&"install-guide".to_string()));

        // CLI should NOT have marketing or strategy
        let categories: Vec<&str> = cop.transverse_productions.iter()
            .map(|t| t.category.as_str())
            .collect();
        assert!(!categories.contains(&"marketing"), "CLI should not have marketing");
        assert!(!categories.contains(&"strategy"), "CLI should not have strategy");

        fs::remove_dir_all(&dir).ok();
    }

    #[test]
    fn test_detect_specialist_auth() {
        let prd: serde_json::Value = serde_json::from_str(&sample_prd_saas()).unwrap();
        let triggers = detect_specialist_needs(&prd);

        let specialist_names: Vec<&str> = triggers.iter()
            .map(|t| t.specialist.as_str())
            .collect();

        assert!(specialist_names.contains(&"security-auditor"),
            "Auth stories should trigger security-auditor, got: {:?}", specialist_names);
    }

    #[test]
    fn test_detect_specialist_payment() {
        let prd: serde_json::Value = serde_json::from_str(&sample_prd_saas()).unwrap();
        let triggers = detect_specialist_needs(&prd);

        let specialist_names: Vec<&str> = triggers.iter()
            .map(|t| t.specialist.as_str())
            .collect();

        assert!(specialist_names.contains(&"fintech-compliance"),
            "Payment stories should trigger fintech-compliance, got: {:?}", specialist_names);
    }

    #[test]
    fn test_detect_specialist_no_match() {
        let prd: serde_json::Value = serde_json::from_str(&serde_json::json!({
            "feature_name": "Simple Feature",
            "user_stories": [
                { "title": "Add a button", "description": "A simple button" }
            ]
        }).to_string()).unwrap();

        let triggers = detect_specialist_needs(&prd);
        assert!(triggers.is_empty(), "Simple stories should not trigger specialists");
    }

    #[test]
    fn test_deliverables_structure() {
        let dir = make_temp_dir("deliverables");

        let cop = CockpitOrchestrationPlan {
            project_name: "test-project".into(),
            project_type: "saas".into(),
            domain: "fintech".into(),
            market_context: "competitive".into(),
            transverse_productions: vec![
                TransverseCategory {
                    category: "marketing".into(),
                    deliverables: vec!["landing-page-copy".into()],
                    priority: "high".into(),
                },
                TransverseCategory {
                    category: "documentation".into(),
                    deliverables: vec!["README".into(), "deploy-guide".into()],
                    priority: "high".into(),
                },
            ],
            specialist_triggers: vec![
                SpecialistTrigger {
                    condition: "domain contains auth".into(),
                    specialist: "security-auditor".into(),
                    reason: "Auth review needed".into(),
                },
            ],
            meta_agent_config: MetaAgentConfig {
                capabilities: vec!["qa".into()],
                monitoring_interval_ms: 30_000,
                autonomy_level: "full".into(),
            },
            deliverables_path: format!("{}/.crossroads/deliverables/", dir),
            created_at: "2026-03-29T00:00:00Z".into(),
        };

        create_deliverables_structure(&dir, &cop).unwrap();

        let base = Path::new(&dir).join(".crossroads").join("deliverables");
        assert!(base.join("marketing").is_dir());
        assert!(base.join("research").is_dir());
        assert!(base.join("documentation").is_dir());
        assert!(base.join("strategy").is_dir());
        assert!(base.join("ops").is_dir());
        assert!(base.join("specialist").is_dir());
        assert!(base.join("_manifest.md").exists());

        let manifest = fs::read_to_string(base.join("_manifest.md")).unwrap();
        assert!(manifest.contains("landing-page-copy"));
        assert!(manifest.contains("security-auditor"));

        fs::remove_dir_all(&dir).ok();
    }

    #[test]
    fn test_meta_skill_generation() {
        let cop = CockpitOrchestrationPlan {
            project_name: "my-api".into(),
            project_type: "api".into(),
            domain: "devtools".into(),
            market_context: "greenfield".into(),
            transverse_productions: vec![],
            specialist_triggers: vec![
                SpecialistTrigger {
                    condition: "domain contains api".into(),
                    specialist: "dx-specialist".into(),
                    reason: "API DX review".into(),
                },
            ],
            meta_agent_config: MetaAgentConfig {
                capabilities: vec!["qa".into(), "doc_gen".into(), "git_master".into()],
                monitoring_interval_ms: 30_000,
                autonomy_level: "full".into(),
            },
            deliverables_path: "/tmp/d/".into(),
            created_at: "2026-03-29T00:00:00Z".into(),
        };

        let skill = generate_meta_skill(&cop);
        assert!(skill.contains("my-api"));
        assert!(skill.contains("npm test"));
        assert!(skill.contains("devtools"));
        assert!(skill.contains("dx-specialist"));
    }

    #[test]
    fn test_transverse_skill_generation() {
        let cop = CockpitOrchestrationPlan {
            project_name: "my-saas".into(),
            project_type: "saas".into(),
            domain: "fintech".into(),
            market_context: "competitive".into(),
            transverse_productions: vec![
                TransverseCategory {
                    category: "marketing".into(),
                    deliverables: vec!["landing-page-copy".into(), "value-proposition".into()],
                    priority: "high".into(),
                },
            ],
            specialist_triggers: vec![],
            meta_agent_config: MetaAgentConfig {
                capabilities: vec!["qa".into()],
                monitoring_interval_ms: 30_000,
                autonomy_level: "full".into(),
            },
            deliverables_path: "/tmp/d/".into(),
            created_at: "2026-03-29T00:00:00Z".into(),
        };

        let skill = generate_transverse_skill(&cop, "marketing").unwrap();
        assert!(skill.contains("Marketing"));
        assert!(skill.contains("landing-page-copy"));
        assert!(skill.contains("value-proposition"));
        assert!(skill.contains("my-saas"));
        assert!(skill.contains("fintech"));

        // Unknown category should error
        let err = generate_transverse_skill(&cop, "nonexistent");
        assert!(err.is_err());
    }

    #[test]
    fn test_specialist_skill_generation() {
        let cop = CockpitOrchestrationPlan {
            project_name: "banking-app".into(),
            project_type: "saas".into(),
            domain: "fintech".into(),
            market_context: "regulated".into(),
            transverse_productions: vec![],
            specialist_triggers: vec![
                SpecialistTrigger {
                    condition: "domain contains payment".into(),
                    specialist: "fintech-compliance".into(),
                    reason: "Payment handling needs compliance review".into(),
                },
            ],
            meta_agent_config: MetaAgentConfig {
                capabilities: vec!["qa".into(), "security_scan".into()],
                monitoring_interval_ms: 30_000,
                autonomy_level: "limited".into(),
            },
            deliverables_path: "/tmp/d/".into(),
            created_at: "2026-03-29T00:00:00Z".into(),
        };

        let skill = generate_specialist_skill(&cop, "fintech-compliance").unwrap();
        assert!(skill.contains("Fintech Compliance"));
        assert!(skill.contains("PCI DSS"));
        assert!(skill.contains("banking-app"));
        assert!(skill.contains("regulated"));

        // Unknown specialist should error
        let err = generate_specialist_skill(&cop, "nonexistent-specialist");
        assert!(err.is_err());
    }

    #[test]
    fn test_should_activate_transverse() {
        assert!(!should_activate_transverse(0.0));
        assert!(!should_activate_transverse(25.0));
        assert!(!should_activate_transverse(49.9));
        assert!(should_activate_transverse(50.0));
        assert!(should_activate_transverse(75.0));
        assert!(should_activate_transverse(100.0));
    }

    #[test]
    fn test_adaptation_rules_test_failures() {
        use crate::db::manager::initialize_memory;
        initialize_memory().unwrap();

        let session = session_repo::create_session("/tmp/adapt-test").unwrap();
        session_repo::update_session(&session.id, "initializing", None).unwrap();

        // Create 4 error slots
        for i in 0..4 {
            let slot = slot_repo::create_slot(&session.id, i, "claude", None, None).unwrap();
            slot_repo::update_slot(&slot.id, "error", Some("Test failure")).unwrap();
        }
        // Transition to active
        session_repo::update_session(&session.id, "active", None).unwrap();

        let actions = get_adaptation_actions(&session.id).unwrap();
        let action_types: Vec<&str> = actions.iter().map(|a| a.action.as_str()).collect();

        assert!(action_types.contains(&"focus_debugging"),
            "4 error slots should trigger focus_debugging, got: {:?}", action_types);
    }

    #[test]
    fn test_adaptation_rules_coding_complete() {
        use crate::db::manager::initialize_memory;
        initialize_memory().unwrap();

        let session = session_repo::create_session("/tmp/adapt-done").unwrap();
        session_repo::update_session(&session.id, "active", None).unwrap();

        // Create 3 done slots
        for i in 0..3 {
            let slot = slot_repo::create_slot(&session.id, i, "claude", None, None).unwrap();
            slot_repo::update_slot(&slot.id, "done", Some("Completed")).unwrap();
        }

        let actions = get_adaptation_actions(&session.id).unwrap();
        let action_types: Vec<&str> = actions.iter().map(|a| a.action.as_str()).collect();

        assert!(action_types.contains(&"activate_transverse"),
            "All done slots should trigger activate_transverse, got: {:?}", action_types);
    }

    #[test]
    fn test_get_cop_roundtrip() {
        let dir = make_temp_dir("cop-roundtrip");

        fs::write(
            format!("{}/package.json", dir),
            r#"{"name":"test","dependencies":{"react":"^19"}}"#,
        ).unwrap();

        let prd = serde_json::json!({
            "feature_name": "Test",
            "user_stories": [{"title": "Add login", "description": "Basic auth"}]
        }).to_string();

        let cop1 = generate_cop(&dir, &prd).unwrap();
        let cop2 = get_cop(&dir).unwrap().expect("COP should be loadable from disk");

        assert_eq!(cop1.project_name, cop2.project_name);
        assert_eq!(cop1.project_type, cop2.project_type);
        assert_eq!(cop1.domain, cop2.domain);

        fs::remove_dir_all(&dir).ok();
    }

    #[test]
    fn test_detect_project_type_api_rust() {
        let dir = make_temp_dir("detect-api");
        fs::write(
            format!("{}/Cargo.toml", dir),
            "[package]\nname = \"my-api\"\n\n[dependencies]\naxum = \"0.7\"\n",
        ).unwrap();

        assert_eq!(detect_project_type(Path::new(&dir)), "api");
        fs::remove_dir_all(&dir).ok();
    }

    #[test]
    fn test_detect_project_type_library() {
        let dir = make_temp_dir("detect-lib");
        fs::write(
            format!("{}/Cargo.toml", dir),
            "[package]\nname = \"my-lib\"\n\n[lib]\nname = \"my_lib\"\n",
        ).unwrap();

        assert_eq!(detect_project_type(Path::new(&dir)), "library");
        fs::remove_dir_all(&dir).ok();
    }

    #[test]
    fn test_detect_domain_fintech() {
        let prd: serde_json::Value = serde_json::from_str(&sample_prd_saas()).unwrap();
        let domain = detect_domain_from_prd(&prd);
        // The SaaS PRD mentions payment, billing, subscription -> fintech
        assert_eq!(domain, "fintech", "PRD with payment/billing should detect as fintech");
    }

    #[test]
    fn test_capitalize_first() {
        assert_eq!(capitalize_first("marketing"), "Marketing");
        assert_eq!(capitalize_first(""), "");
        assert_eq!(capitalize_first("a"), "A");
        assert_eq!(capitalize_first("API"), "API");
    }
}
