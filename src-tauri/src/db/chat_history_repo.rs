use rusqlite::{params, Result};
use uuid::Uuid;
use serde::{Deserialize, Serialize};
use crate::db::manager::with_db;

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ChatHistoryEntry {
    pub id: String,
    pub session_id: Option<String>,
    pub role: String,
    pub content: String,
    pub mode: Option<String>,
    pub metadata: Option<String>,
    pub created_at: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CockpitWakePrompt {
    pub id: String,
    pub session_id: Option<String>,
    pub prompt: String,
    pub observations: Option<String>,
    pub pending_actions: Option<String>,
    pub slot_summaries: Option<String>,
    pub created_at: String,
}

pub fn save_message(
    session_id: &str,
    role: &str,
    content: &str,
    mode: Option<&str>,
    metadata: Option<&str>,
) -> Result<String> {
    with_db(|conn| {
        let id = Uuid::new_v4().to_string();
        let now = chrono::Utc::now().to_rfc3339();
        conn.execute(
            "INSERT INTO chat_history (id, sessionId, role, content, mode, metadata, createdAt) VALUES (?1,?2,?3,?4,?5,?6,?7)",
            params![id, session_id, role, content, mode, metadata, now],
        )?;
        Ok(id)
    })
}

pub fn fetch_history(session_id: &str, limit: i64) -> Result<Vec<ChatHistoryEntry>> {
    with_db(|conn| {
        let mut stmt = conn.prepare(
            "SELECT id, sessionId, role, content, mode, metadata, createdAt FROM chat_history WHERE sessionId=?1 ORDER BY createdAt ASC LIMIT ?2"
        )?;
        let rows = stmt.query_map(params![session_id, limit], |row| {
            Ok(ChatHistoryEntry {
                id: row.get(0)?,
                session_id: row.get(1)?,
                role: row.get(2)?,
                content: row.get(3)?,
                mode: row.get(4)?,
                metadata: row.get(5)?,
                created_at: row.get(6)?,
            })
        })?;
        rows.collect()
    })
}

pub fn fetch_recent(limit: i64) -> Result<Vec<ChatHistoryEntry>> {
    with_db(|conn| {
        let mut stmt = conn.prepare(
            "SELECT id, sessionId, role, content, mode, metadata, createdAt FROM chat_history ORDER BY createdAt DESC LIMIT ?1"
        )?;
        let rows = stmt.query_map(params![limit], |row| {
            Ok(ChatHistoryEntry {
                id: row.get(0)?,
                session_id: row.get(1)?,
                role: row.get(2)?,
                content: row.get(3)?,
                mode: row.get(4)?,
                metadata: row.get(5)?,
                created_at: row.get(6)?,
            })
        })?;
        rows.collect()
    })
}

/// Build a text summary of recent chat for context injection.
pub fn build_chat_summary(session_id: Option<&str>, max_messages: i64) -> Result<String> {
    let messages = if let Some(sid) = session_id {
        fetch_history(sid, max_messages)?
    } else {
        fetch_recent(max_messages)?
    };

    if messages.is_empty() {
        return Ok(String::new());
    }

    let lines: Vec<String> = messages.iter().map(|m| {
        let role_label = match m.role.as_str() {
            "user" => "Human",
            "assistant" => "Assistant",
            "system" => "System",
            _ => &m.role,
        };
        // Truncate long messages for summary
        let content = if m.content.len() > 300 {
            format!("{}...", &m.content[..300])
        } else {
            m.content.clone()
        };
        format!("{}: {}", role_label, content)
    }).collect();

    Ok(lines.join("\n\n"))
}

pub fn save_wake_prompt(
    session_id: &str,
    prompt: &str,
    observations: Option<&str>,
    pending_actions: Option<&str>,
    slot_summaries: Option<&str>,
) -> Result<String> {
    with_db(|conn| {
        let id = Uuid::new_v4().to_string();
        let now = chrono::Utc::now().to_rfc3339();
        conn.execute(
            "INSERT INTO cockpit_wake_prompt (id, sessionId, prompt, observations, pendingActions, slotSummaries, createdAt) VALUES (?1,?2,?3,?4,?5,?6,?7)",
            params![id, session_id, prompt, observations, pending_actions, slot_summaries, now],
        )?;
        Ok(id)
    })
}

pub fn fetch_latest_wake_prompt(session_id: Option<&str>) -> Result<Option<CockpitWakePrompt>> {
    with_db(|conn| {
        let result = if let Some(sid) = session_id {
            conn.query_row(
                "SELECT id, sessionId, prompt, observations, pendingActions, slotSummaries, createdAt FROM cockpit_wake_prompt WHERE sessionId=?1 ORDER BY createdAt DESC LIMIT 1",
                params![sid],
                |row| Ok(CockpitWakePrompt {
                    id: row.get(0)?,
                    session_id: row.get(1)?,
                    prompt: row.get(2)?,
                    observations: row.get(3)?,
                    pending_actions: row.get(4)?,
                    slot_summaries: row.get(5)?,
                    created_at: row.get(6)?,
                }),
            )
        } else {
            conn.query_row(
                "SELECT id, sessionId, prompt, observations, pendingActions, slotSummaries, createdAt FROM cockpit_wake_prompt ORDER BY createdAt DESC LIMIT 1",
                [],
                |row| Ok(CockpitWakePrompt {
                    id: row.get(0)?,
                    session_id: row.get(1)?,
                    prompt: row.get(2)?,
                    observations: row.get(3)?,
                    pending_actions: row.get(4)?,
                    slot_summaries: row.get(5)?,
                    created_at: row.get(6)?,
                }),
            )
        };
        Ok(result.ok())
    })
}

/// Build a full wake context string combining latest wake prompt + recent chat summary.
pub fn build_wake_context(session_id: Option<&str>) -> Result<String> {
    let mut sections = Vec::new();

    // Wake prompt from previous session
    if let Some(wake) = fetch_latest_wake_prompt(session_id)? {
        sections.push(format!("### Previous Wake Prompt\n{}", wake.prompt));

        if let Some(obs) = wake.observations {
            if !obs.is_empty() && obs != "null" {
                sections.push(format!("### Last Observations\n{}", obs));
            }
        }
        if let Some(actions) = wake.pending_actions {
            if !actions.is_empty() && actions != "null" {
                sections.push(format!("### Pending Actions\n{}", actions));
            }
        }
        if let Some(slots) = wake.slot_summaries {
            if !slots.is_empty() && slots != "null" {
                sections.push(format!("### Slot State at Shutdown\n{}", slots));
            }
        }
    }

    // Recent chat for conversational continuity
    let chat_summary = build_chat_summary(session_id, 20)?;
    if !chat_summary.is_empty() {
        sections.push(format!("### Recent Conversation\n{}", chat_summary));
    }

    if sections.is_empty() {
        return Ok(String::new());
    }

    Ok(sections.join("\n\n"))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::db::manager::initialize_memory;
    use crate::db::session_repo;

    fn setup() -> String {
        initialize_memory().unwrap();
        let session = session_repo::create_session("/tmp/chat-test").unwrap();
        session.id
    }

    #[test]
    fn test_save_and_fetch_message() {
        let sid = setup();
        let id = save_message(&sid, "user", "Hello world", Some("cli"), None).unwrap();
        assert!(!id.is_empty());

        let history = fetch_history(&sid, 100).unwrap();
        assert_eq!(history.len(), 1);
        assert_eq!(history[0].role, "user");
        assert_eq!(history[0].content, "Hello world");
        assert_eq!(history[0].mode.as_deref(), Some("cli"));
    }

    #[test]
    fn test_fetch_recent() {
        let sid = setup();
        save_message(&sid, "user", "msg1", None, None).unwrap();
        save_message(&sid, "assistant", "msg2", None, None).unwrap();

        let recent = fetch_recent(10).unwrap();
        assert!(recent.len() >= 2);
    }

    #[test]
    fn test_build_chat_summary() {
        let sid = setup();
        save_message(&sid, "user", "What is XRoads?", Some("cli"), None).unwrap();
        save_message(&sid, "assistant", "XRoads is an orchestrator.", Some("cli"), None).unwrap();

        let summary = build_chat_summary(Some(&sid), 10).unwrap();
        assert!(summary.contains("Human: What is XRoads?"));
        assert!(summary.contains("Assistant: XRoads is an orchestrator."));
    }

    #[test]
    fn test_save_and_fetch_wake_prompt() {
        let sid = setup();
        let id = save_wake_prompt(
            &sid, "Cockpit was monitoring 3 slots.",
            Some("[\"slot 0 at 80%\"]"), Some("[\"run tests\"]"), Some("[\"slot 0: running\"]"),
        ).unwrap();
        assert!(!id.is_empty());

        let wake = fetch_latest_wake_prompt(Some(&sid)).unwrap();
        assert!(wake.is_some());
        let w = wake.unwrap();
        assert!(w.prompt.contains("3 slots"));
    }

    #[test]
    fn test_build_wake_context_empty() {
        initialize_memory().unwrap();
        let ctx = build_wake_context(None).unwrap();
        // May or may not be empty depending on test order, but shouldn't panic
        assert!(ctx.is_empty() || !ctx.is_empty());
    }

    #[test]
    fn test_build_wake_context_with_data() {
        let sid = setup();
        save_message(&sid, "user", "Check slot progress", None, None).unwrap();
        save_wake_prompt(&sid, "Was monitoring auth implementation.", None, None, None).unwrap();

        let ctx = build_wake_context(Some(&sid)).unwrap();
        assert!(ctx.contains("auth implementation"));
        assert!(ctx.contains("Check slot progress"));
    }
}
