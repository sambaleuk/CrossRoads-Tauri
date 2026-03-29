use rusqlite::{params, Result};
use uuid::Uuid;
use serde::{Deserialize, Serialize};
use crate::db::manager::with_db;

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AgentRuntime {
    pub id: String,
    pub name: String,
    pub runtime_type: String,
    pub command: Option<String>,
    pub url: Option<String>,
    pub docker_image: Option<String>,
    pub health_check_command: Option<String>,
    pub config_schema: Option<String>,
    pub config_defaults: Option<String>,
    pub capabilities: String,
    pub icon: Option<String>,
    pub color: Option<String>,
    pub is_builtin: bool,
    pub is_enabled: bool,
    pub created_at: String,
}

const COLS: &str = "id,name,runtimeType,command,url,dockerImage,healthCheckCommand,configSchema,configDefaults,capabilities,icon,color,isBuiltin,isEnabled,createdAt";

fn row_to_runtime(row: &rusqlite::Row) -> rusqlite::Result<AgentRuntime> {
    Ok(AgentRuntime {
        id: row.get(0)?,
        name: row.get(1)?,
        runtime_type: row.get(2)?,
        command: row.get(3)?,
        url: row.get(4)?,
        docker_image: row.get(5)?,
        health_check_command: row.get(6)?,
        config_schema: row.get(7)?,
        config_defaults: row.get(8)?,
        capabilities: row.get(9)?,
        icon: row.get(10)?,
        color: row.get(11)?,
        is_builtin: row.get(12)?,
        is_enabled: row.get(13)?,
        created_at: row.get(14)?,
    })
}

pub fn register_runtime(
    name: &str,
    runtime_type: &str,
    command: Option<&str>,
    url: Option<&str>,
    capabilities_json: &str,
    is_builtin: bool,
) -> Result<AgentRuntime> {
    with_db(|conn| {
        let id = Uuid::new_v4().to_string();
        let now = chrono::Utc::now().to_rfc3339();
        conn.execute(
            "INSERT INTO agent_runtime (id,name,runtimeType,command,url,capabilities,isBuiltin,createdAt) VALUES (?1,?2,?3,?4,?5,?6,?7,?8)",
            params![id, name, runtime_type, command, url, capabilities_json, is_builtin, now],
        )?;
        Ok(AgentRuntime {
            id,
            name: name.to_string(),
            runtime_type: runtime_type.to_string(),
            command: command.map(String::from),
            url: url.map(String::from),
            docker_image: None,
            health_check_command: None,
            config_schema: None,
            config_defaults: None,
            capabilities: capabilities_json.to_string(),
            icon: None,
            color: None,
            is_builtin,
            is_enabled: true,
            created_at: now,
        })
    })
}

pub fn fetch_runtime(id: &str) -> Result<Option<AgentRuntime>> {
    with_db(|conn| {
        Ok(conn.query_row(
            &format!("SELECT {} FROM agent_runtime WHERE id=?1", COLS),
            params![id],
            row_to_runtime,
        ).ok())
    })
}

pub fn fetch_all_runtimes() -> Result<Vec<AgentRuntime>> {
    with_db(|conn| {
        let mut stmt = conn.prepare(
            &format!("SELECT {} FROM agent_runtime ORDER BY isBuiltin DESC, name", COLS),
        )?;
        let rows = stmt.query_map([], row_to_runtime)?;
        rows.collect()
    })
}

pub fn find_by_name(name: &str) -> Result<Option<AgentRuntime>> {
    with_db(|conn| {
        Ok(conn.query_row(
            &format!("SELECT {} FROM agent_runtime WHERE name=?1", COLS),
            params![name],
            row_to_runtime,
        ).ok())
    })
}

/// Register built-in runtimes (claude, gemini, codex) if they don't already exist.
pub fn register_builtins() -> Result<()> {
    let builtins = vec![
        ("claude", "cli", Some("claude"), "[\"code\",\"review\",\"test\",\"debug\"]"),
        ("gemini", "cli", Some("gemini"), "[\"code\",\"review\",\"test\"]"),
        ("codex", "cli", Some("codex"), "[\"code\",\"test\"]"),
    ];

    for (name, rt, cmd, caps) in builtins {
        if find_by_name(name)?.is_none() {
            register_runtime(name, rt, cmd, None, caps, true)?;
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::db::manager::initialize_memory;

    #[test]
    fn test_runtime_crud() {
        initialize_memory().unwrap();

        let rt = register_runtime("custom-agent", "docker", None, Some("http://localhost:8080"), "[\"code\"]", false).unwrap();
        assert_eq!(rt.name, "custom-agent");
        assert!(!rt.is_builtin);

        let fetched = fetch_runtime(&rt.id).unwrap().unwrap();
        assert_eq!(fetched.url.as_deref(), Some("http://localhost:8080"));

        let by_name = find_by_name("custom-agent").unwrap().unwrap();
        assert_eq!(by_name.id, rt.id);

        assert!(find_by_name("nonexistent").unwrap().is_none());

        let all = fetch_all_runtimes().unwrap();
        assert_eq!(all.len(), 1);
    }

    #[test]
    fn test_builtins() {
        initialize_memory().unwrap();

        register_builtins().unwrap();

        let all = fetch_all_runtimes().unwrap();
        assert_eq!(all.len(), 3);

        let claude = find_by_name("claude").unwrap().unwrap();
        assert!(claude.is_builtin);
        assert_eq!(claude.command.as_deref(), Some("claude"));

        let gemini = find_by_name("gemini").unwrap().unwrap();
        assert!(gemini.is_builtin);

        let codex = find_by_name("codex").unwrap().unwrap();
        assert!(codex.is_builtin);

        // Idempotent — running again should not create duplicates
        register_builtins().unwrap();
        let all2 = fetch_all_runtimes().unwrap();
        assert_eq!(all2.len(), 3);
    }
}
