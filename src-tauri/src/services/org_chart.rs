use serde::{Deserialize, Serialize};
use crate::db::{session_repo, slot_repo};
use crate::services::event_bus;

// ── Role Hierarchy Service ──

/// A role in the organization chart
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct OrgRole {
    pub role_id: String,
    pub title: String,
    pub parent_role_id: Option<String>,
    pub responsibilities: String,
    pub approval_authority: String, // "none", "low", "medium", "high", "critical"
}

/// Tree node for hierarchical display
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct OrgRoleNode {
    pub role: OrgRole,
    pub children: Vec<OrgRoleNode>,
}

// ── Templates ──

const SOLO_DEV: &[(&str, &str, Option<&str>, &str, &str)] = &[
    // (role_id, title, parent, responsibilities, approval_authority)
    ("ceo", "CEO / Solo Dev", None, "Full project ownership, all decisions", "critical"),
];

const DUO: &[(&str, &str, Option<&str>, &str, &str)] = &[
    ("ceo", "CEO / Lead", None, "Strategy, code review, approvals", "critical"),
    ("dev", "Developer", Some("ceo"), "Implementation, testing", "low"),
];

const SQUAD: &[(&str, &str, Option<&str>, &str, &str)] = &[
    ("ceo", "CEO / Tech Lead", None, "Architecture, final approvals", "critical"),
    ("backend", "Backend Lead", Some("ceo"), "API, database, services", "medium"),
    ("frontend", "Frontend Lead", Some("ceo"), "UI, state, components", "medium"),
    ("qa", "QA Engineer", Some("ceo"), "Testing, quality gates", "high"),
];

const FULL_TEAM: &[(&str, &str, Option<&str>, &str, &str)] = &[
    ("ceo", "CEO", None, "Vision, strategy, critical approvals", "critical"),
    ("cto", "CTO", Some("ceo"), "Architecture, technical decisions", "high"),
    ("backend-lead", "Backend Lead", Some("cto"), "Backend architecture, API design", "medium"),
    ("backend-dev", "Backend Dev", Some("backend-lead"), "Backend implementation", "low"),
    ("frontend-lead", "Frontend Lead", Some("cto"), "Frontend architecture, UX", "medium"),
    ("frontend-dev", "Frontend Dev", Some("frontend-lead"), "Frontend implementation", "low"),
    ("qa-lead", "QA Lead", Some("cto"), "Test strategy, quality gates", "high"),
    ("devops", "DevOps", Some("cto"), "CI/CD, deployment, infrastructure", "medium"),
];

fn template_data(name: &str) -> Option<&'static [(&'static str, &'static str, Option<&'static str>, &'static str, &'static str)]> {
    match name {
        "solo_dev" => Some(SOLO_DEV),
        "duo" => Some(DUO),
        "squad" => Some(SQUAD),
        "full_team" => Some(FULL_TEAM),
        _ => None,
    }
}

/// Create roles from a named template. Returns a flat list of OrgRole.
pub fn apply_template(session_id: &str, template_name: &str) -> Result<Vec<OrgRole>, String> {
    // Verify session exists
    session_repo::fetch_session(session_id)
        .map_err(|e| e.to_string())?
        .ok_or_else(|| format!("Session {} not found", session_id))?;

    let data = template_data(template_name)
        .ok_or_else(|| format!("Unknown template: {}. Valid: solo_dev, duo, squad, full_team", template_name))?;

    let roles: Vec<OrgRole> = data.iter().map(|(id, title, parent, resp, auth)| {
        OrgRole {
            role_id: id.to_string(),
            title: title.to_string(),
            parent_role_id: parent.map(|p| p.to_string()),
            responsibilities: resp.to_string(),
            approval_authority: auth.to_string(),
        }
    }).collect();

    event_bus::emit_log("info", "org_chart",
        &format!("Applied template '{}' for session {} ({} roles)", template_name, session_id, roles.len()),
        None,
    );

    Ok(roles)
}

/// Distribute a CEO-level goal down the role tree.
/// Each child receives a sub-goal derived from its responsibilities.
pub fn cascade_goals(session_id: &str, ceo_goal: &str) -> Result<Vec<(String, String)>, String> {
    let session = session_repo::fetch_session(session_id)
        .map_err(|e| e.to_string())?
        .ok_or_else(|| format!("Session {} not found", session_id))?;

    let slots = slot_repo::fetch_slots_for_session(session_id)
        .map_err(|e| e.to_string())?;

    let mut assignments = Vec::new();

    for slot in &slots {
        let sub_goal = format!(
            "{} — focus: {} (slot {})",
            ceo_goal,
            slot.current_task.as_deref().unwrap_or(&slot.agent_type),
            slot.slot_index,
        );
        assignments.push((slot.id.clone(), sub_goal.clone()));

        slot_repo::update_slot(&slot.id, &slot.status, Some(&sub_goal))
            .map_err(|e| e.to_string())?;
    }

    event_bus::emit_log("info", "org_chart",
        &format!("Cascaded goal to {} slots: {}", assignments.len(), ceo_goal),
        Some(session_id),
    );

    Ok(assignments)
}

/// Determine which role should approve a gate based on risk level.
/// Higher risk requires higher authority in the hierarchy.
pub fn route_gate_approval(session_id: &str, _slot_id: &str, risk_level: &str) -> Result<String, String> {
    // Verify session exists
    session_repo::fetch_session(session_id)
        .map_err(|e| e.to_string())?
        .ok_or_else(|| format!("Session {} not found", session_id))?;

    // Route based on risk level to the appropriate authority
    let approver = match risk_level {
        "low" => "dev",        // Any developer can approve low-risk
        "medium" => "backend", // Lead-level approval for medium
        "high" => "cto",       // CTO for high risk
        "critical" => "ceo",   // CEO only for critical
        _ => "ceo",            // Default to highest authority
    };

    Ok(approver.to_string())
}

/// Build a tree structure from a flat list of roles.
pub fn get_role_tree(session_id: &str) -> Result<Vec<OrgRoleNode>, String> {
    // Default to squad template for the session
    let roles = apply_template(session_id, "squad")?;
    Ok(build_tree(&roles))
}

/// Build tree from flat role list
fn build_tree(roles: &[OrgRole]) -> Vec<OrgRoleNode> {
    // Find root nodes (no parent)
    let roots: Vec<&OrgRole> = roles.iter()
        .filter(|r| r.parent_role_id.is_none())
        .collect();

    roots.into_iter().map(|root| {
        build_node(root, roles)
    }).collect()
}

fn build_node(role: &OrgRole, all_roles: &[OrgRole]) -> OrgRoleNode {
    let children: Vec<OrgRoleNode> = all_roles.iter()
        .filter(|r| r.parent_role_id.as_deref() == Some(&role.role_id))
        .map(|child| build_node(child, all_roles))
        .collect();

    OrgRoleNode {
        role: role.clone(),
        children,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::db::manager::initialize_memory;

    fn setup() -> String {
        initialize_memory().unwrap();
        let session = session_repo::create_session("/tmp/org-test").unwrap();
        session.id
    }

    #[test]
    fn test_squad_template() {
        let sid = setup();
        let roles = apply_template(&sid, "squad").unwrap();
        assert_eq!(roles.len(), 4);
        assert_eq!(roles[0].role_id, "ceo");
        assert_eq!(roles[0].approval_authority, "critical");
        // All non-CEO roles should have a parent
        assert!(roles[1..].iter().all(|r| r.parent_role_id.is_some()));
    }

    #[test]
    fn test_goal_cascade() {
        let sid = setup();
        // Create some slots first
        slot_repo::create_slot(&sid, 0, "claude", None, None).unwrap();
        slot_repo::create_slot(&sid, 1, "gemini", None, None).unwrap();

        let assignments = cascade_goals(&sid, "Ship v1.0 by Friday").unwrap();
        assert_eq!(assignments.len(), 2);
        assert!(assignments[0].1.contains("Ship v1.0 by Friday"));
        assert!(assignments[1].1.contains("Ship v1.0 by Friday"));
    }

    #[test]
    fn test_gate_routing() {
        let sid = setup();
        let slot = slot_repo::create_slot(&sid, 0, "claude", None, None).unwrap();

        assert_eq!(route_gate_approval(&sid, &slot.id, "low").unwrap(), "dev");
        assert_eq!(route_gate_approval(&sid, &slot.id, "medium").unwrap(), "backend");
        assert_eq!(route_gate_approval(&sid, &slot.id, "high").unwrap(), "cto");
        assert_eq!(route_gate_approval(&sid, &slot.id, "critical").unwrap(), "ceo");
    }

    #[test]
    fn test_role_tree_structure() {
        let roles = vec![
            OrgRole { role_id: "ceo".into(), title: "CEO".into(), parent_role_id: None, responsibilities: "All".into(), approval_authority: "critical".into() },
            OrgRole { role_id: "dev1".into(), title: "Dev 1".into(), parent_role_id: Some("ceo".into()), responsibilities: "Code".into(), approval_authority: "low".into() },
            OrgRole { role_id: "dev2".into(), title: "Dev 2".into(), parent_role_id: Some("ceo".into()), responsibilities: "Code".into(), approval_authority: "low".into() },
        ];

        let tree = build_tree(&roles);
        assert_eq!(tree.len(), 1); // One root
        assert_eq!(tree[0].role.role_id, "ceo");
        assert_eq!(tree[0].children.len(), 2);
    }

    #[test]
    fn test_unknown_template_rejected() {
        let sid = setup();
        let result = apply_template(&sid, "nonexistent");
        assert!(result.is_err());
        assert!(result.unwrap_err().contains("Unknown template"));
    }

    #[test]
    fn test_full_team_template() {
        let sid = setup();
        let roles = apply_template(&sid, "full_team").unwrap();
        assert_eq!(roles.len(), 8);
        // CEO has no parent
        assert!(roles[0].parent_role_id.is_none());
        // CTO reports to CEO
        let cto = roles.iter().find(|r| r.role_id == "cto").unwrap();
        assert_eq!(cto.parent_role_id.as_deref(), Some("ceo"));
    }
}
