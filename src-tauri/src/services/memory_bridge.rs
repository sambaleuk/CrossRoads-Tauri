use std::fs;
use std::path::Path;

use crate::db::{memory_repo, learning_repo};
use crate::services::event_bus;

// ── US-006: Persistent Memory Bridge ──
//
// Bidirectional sync between XRoads learning data and Claude Code subagent MEMORY.md:
// 1. On session start: inject memories from LearningRecord + AgentMemory into MEMORY.md
// 2. After session: read MEMORY.md and sync learnings back to DB

/// Inject initial memories from DB into a subagent's MEMORY.md.
/// Writes to {worktree_path}/.claude/agents/{agent_name}/MEMORY.md
pub fn inject_memories(
    worktree_path: &str,
    agent_name: &str,
    agent_type: &str,
    session_id: &str,
) -> Result<String, String> {
    // Collect relevant memories from DB
    let mut memory_entries: Vec<String> = Vec::new();

    // 1. Agent-specific memories from memory_repo
    let memories = memory_repo::recall_memories(agent_type, "", 20)
        .unwrap_or_default();
    for mem in &memories {
        memory_entries.push(format!(
            "- [{}] {}: {} (confidence: {:.0}%)",
            mem.memory_type, mem.domain, mem.content,
            mem.confidence * 100.0,
        ));
    }

    // 2. Learning records from previous sessions
    let records = learning_repo::fetch_learning_records(session_id)
        .unwrap_or_default();
    for record in &records {
        if record.tests_failed > 0 {
            memory_entries.push(format!(
                "- [learning] Struggled with '{}': {} tests failed (agent: {})",
                record.story_title, record.tests_failed, record.agent_type,
            ));
        }
        if record.tests_passed > 5 {
            memory_entries.push(format!(
                "- [learning] Excelled at '{}': {}/{} tests passed (agent: {})",
                record.story_title, record.tests_passed,
                record.tests_passed + record.tests_failed, record.agent_type,
            ));
        }
    }

    // Build MEMORY.md content
    let content = if memory_entries.is_empty() {
        "# Agent Memory\n\nNo prior memories available. This is a fresh session.\n".to_string()
    } else {
        format!(
            "# Agent Memory\n\n## Prior Knowledge\n{}\n\n## Notes\nThese memories were injected from XRoads orchestration history.\nAdd new observations below as you work.\n",
            memory_entries.join("\n"),
        )
    };

    // Write to the subagent's memory directory
    let memory_dir = Path::new(worktree_path)
        .join(".claude")
        .join("agents")
        .join(agent_name);
    fs::create_dir_all(&memory_dir)
        .map_err(|e| format!("Failed to create memory dir: {}", e))?;

    let memory_path = memory_dir.join("MEMORY.md");
    fs::write(&memory_path, &content)
        .map_err(|e| format!("Failed to write MEMORY.md: {}", e))?;

    event_bus::emit_log("info", "memory-bridge",
        &format!("Injected {} memories for {}", memory_entries.len(), agent_name), None);

    Ok(memory_path.to_string_lossy().to_string())
}

/// Read a subagent's MEMORY.md and sync new learnings back to DB.
/// Call this after a session ends.
pub fn sync_memories_back(
    worktree_path: &str,
    agent_name: &str,
    agent_type: &str,
    session_id: &str,
) -> Result<u32, String> {
    let memory_path = Path::new(worktree_path)
        .join(".claude")
        .join("agents")
        .join(agent_name)
        .join("MEMORY.md");

    if !memory_path.exists() {
        return Ok(0);
    }

    let content = fs::read_to_string(&memory_path)
        .map_err(|e| format!("Failed to read MEMORY.md: {}", e))?;

    // Extract new observations (lines that weren't in the original injection)
    // Look for content after "## Notes" section
    let mut count = 0;
    let mut in_notes = false;
    let mut past_header = false;

    for line in content.lines() {
        if line.starts_with("## Notes") {
            in_notes = true;
            continue;
        }
        if in_notes && (line.starts_with("These memories were injected") || line.starts_with("Add new observations")) {
            past_header = true;
            continue;
        }
        if in_notes && past_header && !line.trim().is_empty() && !line.starts_with("#") {
            // This is a new observation added by the agent
            let observation = line.trim().trim_start_matches("- ");
            if !observation.is_empty() {
                let _ = memory_repo::store_memory(
                    agent_type,
                    "session-learning",
                    "observation",
                    observation,
                    0.6,
                    Some(session_id),
                    None,
                    "[]",
                );
                count += 1;
            }
        }
        // Also check for a new section the agent might have created
        if line.starts_with("## ") && line != "## Prior Knowledge" && line != "## Notes" {
            in_notes = false; // Reset if we hit a new section
        }
    }

    if count > 0 {
        event_bus::emit_log("info", "memory-bridge",
            &format!("Synced {} new memories from {}", count, agent_name), None);
    }

    Ok(count)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::db::manager::initialize_memory;

    fn make_temp_dir(name: &str) -> String {
        let dir = format!("/tmp/xroads-memory-test-{}-{}", name, uuid::Uuid::new_v4());
        fs::create_dir_all(&dir).unwrap();
        dir
    }

    #[test]
    fn test_inject_memories_empty() {
        initialize_memory().unwrap();
        let dir = make_temp_dir("inject-empty");

        let result = inject_memories(&dir, "slot-0-backend", "claude", "test-session");
        assert!(result.is_ok());

        let path = format!("{}/.claude/agents/slot-0-backend/MEMORY.md", dir);
        assert!(Path::new(&path).exists());

        let content = fs::read_to_string(&path).unwrap();
        assert!(content.contains("Agent Memory"));

        fs::remove_dir_all(&dir).ok();
    }

    #[test]
    fn test_sync_memories_back_no_file() {
        let dir = make_temp_dir("sync-nofile");
        let result = sync_memories_back(&dir, "slot-0-backend", "claude", "test-session");
        assert!(result.is_ok());
        assert_eq!(result.unwrap(), 0);

        fs::remove_dir_all(&dir).ok();
    }

    #[test]
    fn test_sync_memories_back_with_new_observations() {
        initialize_memory().unwrap();
        let dir = make_temp_dir("sync-obs");

        // Create a MEMORY.md with new observations
        let memory_dir = format!("{}/.claude/agents/slot-0-backend", dir);
        fs::create_dir_all(&memory_dir).unwrap();
        fs::write(
            format!("{}/MEMORY.md", memory_dir),
            "# Agent Memory\n\n## Notes\nThese memories were injected from XRoads orchestration history.\nAdd new observations below as you work.\n- Rust async requires tokio runtime\n- The auth module uses JWT with RS256\n",
        ).unwrap();

        let result = sync_memories_back(&dir, "slot-0-backend", "claude", "test-session");
        assert!(result.is_ok());
        assert_eq!(result.unwrap(), 2);

        fs::remove_dir_all(&dir).ok();
    }
}
