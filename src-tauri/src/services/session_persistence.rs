use std::path::{Path, PathBuf};
use serde::{Deserialize, Serialize};
use crate::db::{session_repo, slot_repo};
use crate::services::git_service;

// ── US-001: Session State Persistence ──

/// Persisted session snapshot (saved to .crossroads/sessions.json)
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PersistedSession {
    pub session_id: String,
    pub project_path: String,
    pub status: String,
    pub chairman_brief: Option<String>,
    pub slots: Vec<PersistedSlot>,
    pub saved_at: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PersistedSlot {
    pub slot_id: String,
    pub slot_index: i32,
    pub status: String,
    pub agent_type: String,
    pub worktree_path: Option<String>,
    pub branch_name: Option<String>,
    pub current_task: Option<String>,
}

/// Save current session state to .crossroads/sessions.json
pub fn save_session_state(project_path: &str) -> Result<(), String> {
    let session = session_repo::active_session_for_path(project_path)
        .map_err(|e| e.to_string())?
        .ok_or("No active session to save")?;

    let slots = slot_repo::fetch_slots_for_session(&session.id)
        .map_err(|e| e.to_string())?;

    let persisted = PersistedSession {
        session_id: session.id.clone(),
        project_path: session.project_path.clone(),
        status: session.status.clone(),
        chairman_brief: session.chairman_brief.clone(),
        slots: slots.iter().map(|s| PersistedSlot {
            slot_id: s.id.clone(),
            slot_index: s.slot_index,
            status: s.status.clone(),
            agent_type: s.agent_type.clone(),
            worktree_path: s.worktree_path.clone(),
            branch_name: s.branch_name.clone(),
            current_task: s.current_task.clone(),
        }).collect(),
        saved_at: chrono::Utc::now().to_rfc3339(),
    };

    let sessions_dir = Path::new(project_path).join(".crossroads");
    std::fs::create_dir_all(&sessions_dir).map_err(|e| e.to_string())?;
    let file_path = sessions_dir.join("sessions.json");

    // Load existing, update or append
    let mut all: Vec<PersistedSession> = if file_path.exists() {
        let content = std::fs::read_to_string(&file_path).map_err(|e| e.to_string())?;
        serde_json::from_str(&content).unwrap_or_default()
    } else {
        Vec::new()
    };

    if let Some(existing) = all.iter_mut().find(|s| s.session_id == persisted.session_id) {
        *existing = persisted;
    } else {
        all.push(persisted);
    }

    let json = serde_json::to_string_pretty(&all).map_err(|e| e.to_string())?;
    std::fs::write(&file_path, json).map_err(|e| e.to_string())?;
    Ok(())
}

/// Load persisted session from .crossroads/sessions.json
pub fn load_session_state(project_path: &str, session_id: &str) -> Result<Option<PersistedSession>, String> {
    let file_path = Path::new(project_path).join(".crossroads/sessions.json");
    if !file_path.exists() {
        return Ok(None);
    }
    let content = std::fs::read_to_string(&file_path).map_err(|e| e.to_string())?;
    let all: Vec<PersistedSession> = serde_json::from_str(&content).map_err(|e| e.to_string())?;
    Ok(all.into_iter().find(|s| s.session_id == session_id))
}

// ── US-002: Crash Recovery ──

/// Recovery info for a non-closed session
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct RecoveryInfo {
    pub session_id: String,
    pub project_path: String,
    pub status: String,
    pub chairman_brief: Option<String>,
    pub slot_count: usize,
    pub running_slots: usize,
    pub error_slots: usize,
    pub created_at: String,
}

/// Detect non-closed sessions that need recovery
pub fn detect_recoverable_sessions() -> Result<Vec<RecoveryInfo>, String> {
    let all = session_repo::fetch_all_sessions().map_err(|e| e.to_string())?;
    let mut recoverable = Vec::new();

    for session in &all {
        if session.status != "closed" && session.status != "idle" {
            let slots = slot_repo::fetch_slots_for_session(&session.id)
                .map_err(|e| e.to_string())?;
            recoverable.push(RecoveryInfo {
                session_id: session.id.clone(),
                project_path: session.project_path.clone(),
                status: session.status.clone(),
                chairman_brief: session.chairman_brief.clone(),
                slot_count: slots.len(),
                running_slots: slots.iter().filter(|s| s.status == "running").count(),
                error_slots: slots.iter().filter(|s| s.status == "error").count(),
                created_at: session.created_at.clone(),
            });
        }
    }

    Ok(recoverable)
}

/// Discard a recoverable session (close it)
pub fn discard_session(session_id: &str) -> Result<(), String> {
    session_repo::update_session(session_id, "closed", None)
        .map_err(|e| e.to_string())
}

// ── US-003: Orchestration History ──

/// Orchestration history entry (saved to ~/.crossroads/history/)
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct HistoryEntry {
    pub prd_path: String,
    pub prd_name: String,
    pub project_path: String,
    pub started_at: String,
    pub finished_at: String,
    pub total_stories: i32,
    pub completed_stories: i32,
    pub failed_stories: i32,
    pub total_cost_cents: i64,
    pub merged_branches: Vec<String>,
    pub conflicts: Vec<String>,
    pub duration_seconds: i64,
}

fn history_file_path() -> PathBuf {
    let home = std::env::var("HOME").unwrap_or_else(|_| "/tmp".into());
    PathBuf::from(format!("{}/.crossroads/history", home))
}

/// Save an orchestration to history
pub fn save_to_history(entry: &HistoryEntry) -> Result<(), String> {
    let dir = history_file_path();
    std::fs::create_dir_all(&dir).map_err(|e| e.to_string())?;
    let file = dir.join("orchestrations.json");

    let mut entries: Vec<HistoryEntry> = if file.exists() {
        let content = std::fs::read_to_string(&file).map_err(|e| e.to_string())?;
        serde_json::from_str(&content).unwrap_or_default()
    } else {
        Vec::new()
    };

    entries.push(entry.clone());

    // Keep last 100 entries
    if entries.len() > 100 {
        entries = entries.split_off(entries.len() - 100);
    }

    let json = serde_json::to_string_pretty(&entries).map_err(|e| e.to_string())?;
    std::fs::write(&file, json).map_err(|e| e.to_string())?;
    Ok(())
}

/// Load orchestration history
pub fn load_history() -> Result<Vec<HistoryEntry>, String> {
    let file = history_file_path().join("orchestrations.json");
    if !file.exists() {
        return Ok(Vec::new());
    }
    let content = std::fs::read_to_string(&file).map_err(|e| e.to_string())?;
    serde_json::from_str(&content).map_err(|e| e.to_string())
}

// ── US-004: Worktree Cleanup ──

/// An orphaned worktree with no matching session
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct OrphanedWorktree {
    pub path: String,
    pub branch: String,
    pub reason: String, // "no_session", "stale_session"
}

/// Detect orphaned worktrees in a git repo
pub fn detect_orphaned_worktrees(repo_path: &str) -> Result<Vec<OrphanedWorktree>, String> {
    let worktrees = git_service::list_worktrees(repo_path)?;
    let mut orphans = Vec::new();

    // Get all active session worktree paths
    let sessions = session_repo::fetch_all_sessions().map_err(|e| e.to_string())?;
    let active_session_ids: Vec<&str> = sessions.iter()
        .filter(|s| s.status != "closed")
        .map(|s| s.id.as_str())
        .collect();

    let mut active_worktree_paths = std::collections::HashSet::new();
    for sid in &active_session_ids {
        let slots = slot_repo::fetch_slots_for_session(sid).map_err(|e| e.to_string())?;
        for slot in &slots {
            if let Some(ref wt) = slot.worktree_path {
                active_worktree_paths.insert(wt.clone());
            }
        }
    }

    for wt_line in &worktrees {
        // Parse worktree output: "/path/to/worktree branch-name"
        let parts: Vec<&str> = wt_line.splitn(2, ' ').collect();
        let wt_path = parts.first().unwrap_or(&"").to_string();
        let branch = parts.get(1).unwrap_or(&"unknown").to_string();

        // Skip the main worktree
        if wt_path == repo_path {
            continue;
        }

        if !active_worktree_paths.contains(&wt_path) {
            orphans.push(OrphanedWorktree {
                path: wt_path,
                branch,
                reason: "no_session".into(),
            });
        }
    }

    Ok(orphans)
}

/// Clean up orphaned worktrees
pub fn cleanup_worktrees(repo_path: &str, worktree_paths: &[String]) -> Result<u32, String> {
    let mut cleaned = 0;
    for path in worktree_paths {
        match git_service::delete_worktree(repo_path, path) {
            Ok(()) => {
                cleaned += 1;
                log::info!("Cleaned up worktree: {}", path);
            }
            Err(e) => {
                log::warn!("Failed to cleanup worktree {}: {}", path, e);
            }
        }
    }
    Ok(cleaned)
}

/// Detect stale sessions (>24h with no activity)
pub fn detect_stale_sessions() -> Result<Vec<RecoveryInfo>, String> {
    let all = detect_recoverable_sessions()?;
    let cutoff = chrono::Utc::now() - chrono::Duration::hours(24);
    let cutoff_str = cutoff.to_rfc3339();

    Ok(all.into_iter()
        .filter(|s| s.created_at < cutoff_str)
        .collect())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::db::manager::initialize_memory;

    #[test]
    fn test_session_persistence() {
        initialize_memory().unwrap();
        let dir = format!("/tmp/xroads-persist-test-{}", uuid::Uuid::new_v4());
        std::fs::create_dir_all(&dir).unwrap();

        let session = session_repo::create_session(&dir).unwrap();
        session_repo::update_session(&session.id, "active", Some("Test brief")).unwrap();
        slot_repo::create_slot(&session.id, 0, "claude", None, Some("feat/test")).unwrap();

        save_session_state(&dir).unwrap();

        // Verify file exists
        let file = Path::new(&dir).join(".crossroads/sessions.json");
        assert!(file.exists());

        // Load it back
        let loaded = load_session_state(&dir, &session.id).unwrap().unwrap();
        assert_eq!(loaded.session_id, session.id);
        assert_eq!(loaded.status, "active");
        assert_eq!(loaded.slots.len(), 1);
        assert_eq!(loaded.slots[0].agent_type, "claude");

        std::fs::remove_dir_all(&dir).ok();
    }

    #[test]
    fn test_detect_recoverable_sessions() {
        initialize_memory().unwrap();
        // Create an active session
        let session = session_repo::create_session("/tmp/recovery-test").unwrap();
        session_repo::update_session(&session.id, "active", None).unwrap();
        slot_repo::create_slot(&session.id, 0, "claude", None, None).unwrap();

        let recoverable = detect_recoverable_sessions().unwrap();
        assert!(recoverable.iter().any(|r| r.session_id == session.id));

        // Discard it
        discard_session(&session.id).unwrap();
        let after = detect_recoverable_sessions().unwrap();
        assert!(!after.iter().any(|r| r.session_id == session.id));
    }

    #[test]
    fn test_history_save_and_load() {
        let entry = HistoryEntry {
            prd_path: "/tmp/prd.json".into(),
            prd_name: "Test Feature".into(),
            project_path: "/tmp/proj".into(),
            started_at: "2026-03-29T10:00:00Z".into(),
            finished_at: "2026-03-29T11:00:00Z".into(),
            total_stories: 5,
            completed_stories: 4,
            failed_stories: 1,
            total_cost_cents: 1500,
            merged_branches: vec!["feat/a".into()],
            conflicts: vec![],
            duration_seconds: 3600,
        };

        save_to_history(&entry).unwrap();
        let history = load_history().unwrap();
        assert!(history.iter().any(|h| h.prd_name == "Test Feature"));
    }

    #[test]
    fn test_persisted_session_serialization() {
        let ps = PersistedSession {
            session_id: "s1".into(),
            project_path: "/tmp/proj".into(),
            status: "active".into(),
            chairman_brief: Some("Brief".into()),
            slots: vec![PersistedSlot {
                slot_id: "sl1".into(),
                slot_index: 0,
                status: "running".into(),
                agent_type: "claude".into(),
                worktree_path: Some("/tmp/wt".into()),
                branch_name: Some("feat/test".into()),
                current_task: Some("US-001".into()),
            }],
            saved_at: "2026-03-29T10:00:00Z".into(),
        };
        let json = serde_json::to_string(&ps).unwrap();
        assert!(json.contains("\"sessionId\":\"s1\""));
        assert!(json.contains("\"slotIndex\":0"));
    }

    #[test]
    fn test_recovery_info_serialization() {
        let ri = RecoveryInfo {
            session_id: "s1".into(),
            project_path: "/tmp/proj".into(),
            status: "active".into(),
            chairman_brief: None,
            slot_count: 3,
            running_slots: 2,
            error_slots: 1,
            created_at: "2026-03-29T10:00:00Z".into(),
        };
        let json = serde_json::to_string(&ri).unwrap();
        assert!(json.contains("\"runningSlots\":2"));
    }
}
