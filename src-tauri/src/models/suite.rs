use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Suite {
    pub id: String,
    pub name: String,
    pub description: String,
    pub icon: String,
    pub accent_hex: String,
    pub glow_hex: String,
    pub roles: Vec<SuiteRole>,
    pub phases: Vec<SuitePhase>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SuiteRole {
    pub id: String,
    pub name: String,
    pub description: String,
    pub icon: String,
    pub skill_ids: Vec<String>,
    pub agent_preference: Option<String>,
    pub action_type: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SuitePhase {
    pub id: String,
    pub name: String,
    pub description: String,
    pub role_ids: Vec<String>,
    pub transition_condition: String,
}

/// Returns the 4 built-in suites matching the Swift Suite.builtIn definitions.
pub fn builtin_suites() -> Vec<Suite> {
    vec![
        // Developer suite — the original XRoads experience
        Suite {
            id: "developer".into(),
            name: "Developer".into(),
            description: "Multi-agent software development with PRD-driven orchestration".into(),
            icon: "chevron.left.forwardslash.chevron.right".into(),
            accent_hex: "#00E6FF".into(),
            glow_hex: "#0099CC".into(),
            roles: vec![
                SuiteRole {
                    id: "implementer".into(), name: "Implementer".into(),
                    description: "Feature development from PRD stories with unit tests".into(),
                    icon: "hammer.fill".into(),
                    skill_ids: vec!["prd".into(), "code-writer".into(), "commit".into()],
                    agent_preference: Some("claude".into()), action_type: "implement".into(),
                },
                SuiteRole {
                    id: "tester".into(), name: "Tester".into(),
                    description: "Integration, E2E, and performance testing".into(),
                    icon: "testtube.2".into(),
                    skill_ids: vec!["integration-test".into(), "e2e-test".into(), "perf-test".into()],
                    agent_preference: Some("gemini".into()), action_type: "integrationTest".into(),
                },
                SuiteRole {
                    id: "reviewer".into(), name: "Reviewer".into(),
                    description: "Deep code review — OWASP, SOLID, complexity".into(),
                    icon: "eye.fill".into(),
                    skill_ids: vec!["code-reviewer".into(), "lint".into()],
                    agent_preference: Some("claude".into()), action_type: "review".into(),
                },
                SuiteRole {
                    id: "writer".into(), name: "Doc Writer".into(),
                    description: "README, API docs, architecture guides".into(),
                    icon: "doc.text.fill".into(),
                    skill_ids: vec!["doc-generator".into()],
                    agent_preference: None, action_type: "write".into(),
                },
                SuiteRole {
                    id: "debugger".into(), name: "Debugger".into(),
                    description: "Bug reproduction, diagnosis, and fix".into(),
                    icon: "ladybug.fill".into(),
                    skill_ids: vec!["code-reviewer".into(), "commit".into()],
                    agent_preference: Some("claude".into()), action_type: "debug".into(),
                },
                SuiteRole {
                    id: "security".into(), name: "Security Auditor".into(),
                    description: "Vulnerability assessment and compliance".into(),
                    icon: "lock.shield.fill".into(),
                    skill_ids: vec!["code-reviewer".into()],
                    agent_preference: Some("claude".into()), action_type: "review".into(),
                },
                SuiteRole {
                    id: "devops".into(), name: "DevOps".into(),
                    description: "CI/CD, infrastructure, deployment".into(),
                    icon: "server.rack".into(),
                    skill_ids: vec![],
                    agent_preference: Some("claude".into()), action_type: "custom".into(),
                },
            ],
            phases: vec![
                SuitePhase { id: "understand".into(), name: "UNDERSTAND".into(), description: "Scan codebase, detect patterns, assess state".into(), role_ids: vec!["reviewer".into()], transition_condition: "manual".into() },
                SuitePhase { id: "build".into(), name: "BUILD".into(), description: "Implement stories in parallel".into(), role_ids: vec!["implementer".into(), "writer".into()], transition_condition: "all_stories_done".into() },
                SuitePhase { id: "verify".into(), name: "VERIFY".into(), description: "Test and review all implementations".into(), role_ids: vec!["tester".into(), "reviewer".into(), "security".into()], transition_condition: "all_tests_pass".into() },
                SuitePhase { id: "deliver".into(), name: "DELIVER".into(), description: "Finalize docs, merge prep, retro".into(), role_ids: vec!["writer".into(), "devops".into()], transition_condition: "manual".into() },
            ],
        },

        // Marketing suite — content creation, GTM, growth
        Suite {
            id: "marketer".into(),
            name: "Marketer".into(),
            description: "Content creation, copywriting, SEO, email campaigns, and go-to-market strategy. 16 specialized marketing skills.".into(),
            icon: "megaphone.fill".into(),
            accent_hex: "#FF6B2B".into(),
            glow_hex: "#FF3D00".into(),
            roles: vec![
                SuiteRole {
                    id: "copywriter".into(), name: "Copywriter".into(),
                    description: "Landing pages, value props, direct response copy, lead magnets".into(),
                    icon: "text.cursor".into(),
                    skill_ids: vec!["05-direct-response-copy".into(), "01-brand-voice".into(), "04-lead-magnet".into()],
                    agent_preference: Some("claude".into()), action_type: "custom".into(),
                },
                SuiteRole {
                    id: "seo".into(), name: "SEO Specialist".into(),
                    description: "Keyword research, SEO content, blog articles, FAQ sections".into(),
                    icon: "magnifyingglass".into(),
                    skill_ids: vec!["03-keyword-research".into(), "06-seo-content".into()],
                    agent_preference: Some("claude".into()), action_type: "custom".into(),
                },
                SuiteRole {
                    id: "email".into(), name: "Email Strategist".into(),
                    description: "Welcome sequences, nurture drips, newsletters, launch campaigns".into(),
                    icon: "envelope.fill".into(),
                    skill_ids: vec!["08-email-sequences".into(), "07-newsletter".into()],
                    agent_preference: Some("claude".into()), action_type: "custom".into(),
                },
                SuiteRole {
                    id: "social".into(), name: "Social Creator".into(),
                    description: "Content atomization, social graphics, carousel copy, video scripts".into(),
                    icon: "bubble.left.and.bubble.right.fill".into(),
                    skill_ids: vec!["09-content-atomizer".into(), "14-social-graphics".into(), "15-product-video".into()],
                    agent_preference: None, action_type: "custom".into(),
                },
                SuiteRole {
                    id: "strategist".into(), name: "Growth Strategist".into(),
                    description: "Market positioning, competitive analysis, GTM planning, orchestration".into(),
                    icon: "chart.line.uptrend.xyaxis".into(),
                    skill_ids: vec!["02-positioning-angles".into(), "10-orchestrator".into(), "11-creative-strategist".into()],
                    agent_preference: Some("claude".into()), action_type: "custom".into(),
                },
                SuiteRole {
                    id: "creative".into(), name: "Creative Director".into(),
                    description: "Visual DNA, image generation, product photography, brand assets".into(),
                    icon: "paintbrush.fill".into(),
                    skill_ids: vec!["11-creative-strategist".into(), "12-image-generation".into(), "13-product-photography".into(), "16-talking-head".into()],
                    agent_preference: Some("claude".into()), action_type: "custom".into(),
                },
            ],
            phases: vec![
                SuitePhase { id: "foundation".into(), name: "FOUNDATION".into(), description: "Brand voice + positioning angles + keyword map. Must happen first.".into(), role_ids: vec!["strategist".into(), "seo".into()], transition_condition: "manual".into() },
                SuitePhase { id: "create".into(), name: "CREATE".into(), description: "All content assets in parallel: copy, emails, social, visuals".into(), role_ids: vec!["copywriter".into(), "email".into(), "social".into(), "creative".into()], transition_condition: "deliverables_complete".into() },
                SuitePhase { id: "multiply".into(), name: "MULTIPLY".into(), description: "Atomize content: 1 blog post -> 15 pieces across channels".into(), role_ids: vec!["social".into(), "email".into()], transition_condition: "manual".into() },
                SuitePhase { id: "publish".into(), name: "PUBLISH".into(), description: "Final formatting, A/B variants, scheduling prep, distribution".into(), role_ids: vec!["strategist".into(), "social".into(), "email".into()], transition_condition: "manual".into() },
            ],
        },

        // Researcher/Analyst suite
        Suite {
            id: "researcher".into(),
            name: "Researcher".into(),
            description: "Competitive intelligence, market analysis, technical research, and report generation".into(),
            icon: "magnifyingglass.circle.fill".into(),
            accent_hex: "#A855F7".into(),
            glow_hex: "#7C3AED".into(),
            roles: vec![
                SuiteRole {
                    id: "analyst".into(), name: "Analyst".into(),
                    description: "Data analysis, pattern detection, insight extraction".into(),
                    icon: "chart.bar.fill".into(),
                    skill_ids: vec![],
                    agent_preference: Some("claude".into()), action_type: "custom".into(),
                },
                SuiteRole {
                    id: "researcher".into(), name: "Researcher".into(),
                    description: "Deep web research, paper analysis, trend mapping".into(),
                    icon: "book.fill".into(),
                    skill_ids: vec![],
                    agent_preference: Some("claude".into()), action_type: "custom".into(),
                },
                SuiteRole {
                    id: "competitor".into(), name: "Competitive Intel".into(),
                    description: "Competitor analysis, feature comparison, positioning gaps".into(),
                    icon: "person.2.fill".into(),
                    skill_ids: vec![],
                    agent_preference: Some("claude".into()), action_type: "custom".into(),
                },
                SuiteRole {
                    id: "reporter".into(), name: "Report Writer".into(),
                    description: "Executive summaries, briefing docs, presentations".into(),
                    icon: "doc.richtext.fill".into(),
                    skill_ids: vec!["doc-generator".into()],
                    agent_preference: None, action_type: "write".into(),
                },
            ],
            phases: vec![
                SuitePhase { id: "scope".into(), name: "SCOPE".into(), description: "Define research questions, identify sources".into(), role_ids: vec!["researcher".into()], transition_condition: "manual".into() },
                SuitePhase { id: "gather".into(), name: "GATHER".into(), description: "Collect data, read sources, extract facts".into(), role_ids: vec!["researcher".into(), "competitor".into(), "analyst".into()], transition_condition: "manual".into() },
                SuitePhase { id: "analyze".into(), name: "ANALYZE".into(), description: "Cross-reference, detect patterns, form insights".into(), role_ids: vec!["analyst".into(), "competitor".into()], transition_condition: "manual".into() },
                SuitePhase { id: "report".into(), name: "REPORT".into(), description: "Synthesize findings into deliverables".into(), role_ids: vec!["reporter".into(), "analyst".into()], transition_condition: "deliverables_complete".into() },
            ],
        },

        // Operations suite — compliance, finance, legal, HR
        Suite {
            id: "ops".into(),
            name: "Operations".into(),
            description: "Compliance audits, financial analysis, legal review, and operational reporting".into(),
            icon: "building.2.fill".into(),
            accent_hex: "#3FB950".into(),
            glow_hex: "#238636".into(),
            roles: vec![
                SuiteRole {
                    id: "finance".into(), name: "Finance Analyst".into(),
                    description: "Budget analysis, cost optimization, financial reporting".into(),
                    icon: "dollarsign.circle.fill".into(),
                    skill_ids: vec!["finance-analyst".into()],
                    agent_preference: Some("claude".into()), action_type: "custom".into(),
                },
                SuiteRole {
                    id: "legal".into(), name: "Legal Clerk".into(),
                    description: "Contract review, compliance checks, policy drafting".into(),
                    icon: "scale.3d".into(),
                    skill_ids: vec!["legal-clerk".into()],
                    agent_preference: Some("claude".into()), action_type: "custom".into(),
                },
                SuiteRole {
                    id: "hr".into(), name: "HR Manager".into(),
                    description: "Process documentation, handbook updates, policy review".into(),
                    icon: "person.crop.circle.fill".into(),
                    skill_ids: vec!["hr-wiki-manager".into()],
                    agent_preference: None, action_type: "custom".into(),
                },
                SuiteRole {
                    id: "auditor".into(), name: "Compliance Auditor".into(),
                    description: "Regulatory compliance, GDPR/SOC2 checks, risk assessment".into(),
                    icon: "checkmark.shield.fill".into(),
                    skill_ids: vec![],
                    agent_preference: Some("claude".into()), action_type: "custom".into(),
                },
            ],
            phases: vec![
                SuitePhase { id: "assess".into(), name: "ASSESS".into(), description: "Audit current state, identify gaps".into(), role_ids: vec!["auditor".into(), "finance".into()], transition_condition: "manual".into() },
                SuitePhase { id: "execute".into(), name: "EXECUTE".into(), description: "Process work items in parallel".into(), role_ids: vec!["finance".into(), "legal".into(), "hr".into()], transition_condition: "manual".into() },
                SuitePhase { id: "validate".into(), name: "VALIDATE".into(), description: "Cross-check, verify compliance".into(), role_ids: vec!["auditor".into()], transition_condition: "manual".into() },
                SuitePhase { id: "deliver".into(), name: "DELIVER".into(), description: "Finalize reports and deliverables".into(), role_ids: vec!["finance".into(), "legal".into(), "hr".into()], transition_condition: "deliverables_complete".into() },
            ],
        },
    ]
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_builtin_suites_count() {
        let suites = builtin_suites();
        assert_eq!(suites.len(), 4);
    }

    #[test]
    fn test_suite_ids() {
        let suites = builtin_suites();
        let ids: Vec<&str> = suites.iter().map(|s| s.id.as_str()).collect();
        assert_eq!(ids, vec!["developer", "marketer", "researcher", "ops"]);
    }

    #[test]
    fn test_developer_roles() {
        let suites = builtin_suites();
        let dev = &suites[0];
        assert_eq!(dev.roles.len(), 7);
        assert_eq!(dev.roles[0].id, "implementer");
    }

    #[test]
    fn test_suite_serialization() {
        let suites = builtin_suites();
        let json = serde_json::to_string(&suites[0]).unwrap();
        assert!(json.contains("accentHex"));
        assert!(json.contains("glowHex"));
        assert!(json.contains("skillIds"));
    }
}
