use rusqlite::{params, Result};
use uuid::Uuid;
use crate::db::manager::with_db;
use crate::models::metier_skill::MetierSkill;

pub fn create_skill(name: &str, family: &str, skill_md_path: &str, description: Option<&str>) -> Result<MetierSkill> {
    with_db(|conn| {
        let id = Uuid::new_v4().to_string();
        let now = chrono::Utc::now().to_rfc3339();
        conn.execute(
            "INSERT INTO metier_skill (id,name,family,skillMdPath,description,createdAt) VALUES (?1,?2,?3,?4,?5,?6)",
            params![id, name, family, skill_md_path, description, now],
        )?;
        Ok(MetierSkill {
            id, name: name.to_string(), family: family.to_string(),
            skill_md_path: skill_md_path.to_string(), required_mcps: None,
            description: description.map(String::from), created_at: now,
        })
    })
}

pub fn find_by_name(name: &str) -> Result<Option<MetierSkill>> {
    with_db(|conn| {
        let mut stmt = conn.prepare(
            "SELECT id,name,family,skillMdPath,requiredMcps,description,createdAt FROM metier_skill WHERE name=?1"
        )?;
        let mut rows = stmt.query_map(params![name], |row| {
            Ok(MetierSkill {
                id: row.get(0)?, name: row.get(1)?, family: row.get(2)?,
                skill_md_path: row.get(3)?, required_mcps: row.get(4)?,
                description: row.get(5)?, created_at: row.get(6)?,
            })
        })?;
        match rows.next() {
            Some(Ok(s)) => Ok(Some(s)),
            _ => Ok(None),
        }
    })
}
