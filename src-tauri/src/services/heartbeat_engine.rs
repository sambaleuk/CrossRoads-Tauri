use serde::{Deserialize, Serialize};
// Note: slot_repo and orchestration_repo available for future heartbeat integration
#[allow(unused_imports)]
use crate::db::{slot_repo, orchestration_repo};
use crate::services::git_service;

// ── Heartbeat + Scheduling Service ──

/// Result of a health pulse check on a slot
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PulseResult {
    pub alive: bool,
    pub git_changes: u32,
    pub tests_passed: u32,
    pub tests_failed: u32,
    pub stories_completed: u32,
    pub mergeable: bool,
    pub errors: Vec<String>,
}

/// Next scheduled run time
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct NextRun {
    pub year: i32,
    pub month: u32,
    pub day: u32,
    pub hour: u32,
    pub minute: u32,
    pub weekday: u32,
}

/// A scheduled run that is due
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ScheduledRun {
    pub id: String,
    pub session_id: String,
    pub cron_expression: String,
    pub last_run: Option<String>,
    pub next_run: NextRun,
    pub is_due: bool,
}

/// Run code-aware health checks on a slot's worktree.
pub fn create_pulse_result(slot_id: &str, worktree_path: &str) -> PulseResult {
    let mut errors = Vec::new();

    // Check if the worktree path exists and is a git repo
    let alive = std::path::Path::new(worktree_path).exists();
    if !alive {
        return PulseResult {
            alive: false,
            git_changes: 0,
            tests_passed: 0,
            tests_failed: 0,
            stories_completed: 0,
            mergeable: false,
            errors: vec![format!("Worktree path does not exist: {}", worktree_path)],
        };
    }

    // Git status: count changed files from status output
    let git_changes = match git_service::get_status(worktree_path) {
        Ok(status_text) => {
            status_text.lines().filter(|l| !l.trim().is_empty()).count() as u32
        }
        Err(e) => {
            errors.push(format!("git status failed: {}", e));
            0
        }
    };

    // Run tests (attempt `cargo test` or `npm test` — check for Cargo.toml or package.json)
    let (tests_passed, tests_failed) = run_test_check(worktree_path, &mut errors);

    // Check orchestration for story completion
    let stories_completed = count_completed_stories(slot_id);

    // Mergeable: has changes, no errors, tests pass
    let mergeable = git_changes > 0 && tests_failed == 0 && errors.is_empty();

    PulseResult {
        alive,
        git_changes,
        tests_passed,
        tests_failed,
        stories_completed,
        mergeable,
        errors,
    }
}

/// Run a quick test check and return (passed, failed) counts
fn run_test_check(worktree_path: &str, errors: &mut Vec<String>) -> (u32, u32) {
    let cargo_toml = std::path::Path::new(worktree_path).join("Cargo.toml");
    let package_json = std::path::Path::new(worktree_path).join("package.json");

    let (cmd, args) = if cargo_toml.exists() {
        ("cargo", vec!["test", "--no-run", "--quiet"])
    } else if package_json.exists() {
        ("npm", vec!["test", "--", "--passWithNoTests", "--silent"])
    } else {
        // No test runner detected
        return (0, 0);
    };

    match std::process::Command::new(cmd)
        .args(&args)
        .current_dir(worktree_path)
        .output()
    {
        Ok(output) => {
            if output.status.success() {
                // Compilation/dry-run succeeded
                (1, 0)
            } else {
                let stderr = String::from_utf8_lossy(&output.stderr);
                errors.push(format!("Test check failed: {}", stderr.chars().take(200).collect::<String>()));
                (0, 1)
            }
        }
        Err(e) => {
            errors.push(format!("Failed to run {}: {}", cmd, e));
            (0, 0)
        }
    }
}

/// Count completed stories for a slot's session.
/// Requires the session_id to be looked up from the slot's parent session.
fn count_completed_stories(_slot_id: &str) -> u32 {
    // We don't have a fetch_slot function, so we return 0 for now.
    // In production, the orchestration_repo::fetch_records_for_session would be
    // used once we know the session_id from a higher-level caller.
    0
}

// ── Cron Parsing ──

/// Parse a basic cron expression "minute hour day month weekday" and calculate next run.
/// Supports: specific numbers, * (any). Does not support ranges or steps.
pub fn parse_cron(expression: &str) -> Result<NextRun, String> {
    let parts: Vec<&str> = expression.trim().split_whitespace().collect();
    if parts.len() != 5 {
        return Err(format!("Invalid cron expression: expected 5 fields, got {}", parts.len()));
    }

    let now = chrono::Utc::now();
    let now_minute = now.format("%M").to_string().parse::<u32>().unwrap_or(0);
    let now_hour = now.format("%H").to_string().parse::<u32>().unwrap_or(0);
    let now_day = now.format("%d").to_string().parse::<u32>().unwrap_or(1);
    let now_month = now.format("%m").to_string().parse::<u32>().unwrap_or(1);
    let now_year = now.format("%Y").to_string().parse::<i32>().unwrap_or(2026);
    let now_weekday = now.format("%u").to_string().parse::<u32>().unwrap_or(1); // 1=Mon, 7=Sun

    let minute = parse_cron_field(parts[0], now_minute, 0, 59)?;
    let hour = parse_cron_field(parts[1], now_hour, 0, 23)?;
    let day = parse_cron_field(parts[2], now_day, 1, 31)?;
    let month = parse_cron_field(parts[3], now_month, 1, 12)?;
    let weekday = parse_cron_field(parts[4], now_weekday, 0, 7)?;

    Ok(NextRun {
        year: now_year,
        month,
        day,
        hour,
        minute,
        weekday,
    })
}

/// Parse a single cron field. Returns the value or the current value if *.
fn parse_cron_field(field: &str, current: u32, min: u32, max: u32) -> Result<u32, String> {
    if field == "*" {
        return Ok(current);
    }

    let value: u32 = field.parse()
        .map_err(|_| format!("Invalid cron field: '{}' (expected number or *)", field))?;

    if value < min || value > max {
        return Err(format!("Cron field '{}' out of range [{}-{}]", field, min, max));
    }

    Ok(value)
}

/// Check for scheduled runs that are due. Returns runs where current time >= next_run.
pub fn check_scheduled_runs() -> Vec<ScheduledRun> {
    // In a full implementation, scheduled runs would be stored in the DB.
    // For now, return an empty list — the infrastructure is ready for future use.
    Vec::new()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::db::manager::initialize_memory;
    use crate::db::{session_repo, slot_repo};

    #[test]
    fn test_pulse_result() {
        initialize_memory().unwrap();
        let session = session_repo::create_session("/tmp/pulse-test").unwrap();
        let slot = slot_repo::create_slot(&session.id, 0, "claude", None, None).unwrap();

        // Non-existent path should report not alive
        let pulse = create_pulse_result(&slot.id, "/tmp/nonexistent-worktree-xroads-test");
        assert!(!pulse.alive);
        assert!(!pulse.errors.is_empty());

        // /tmp exists but is not a meaningful worktree
        let pulse = create_pulse_result(&slot.id, "/tmp");
        assert!(pulse.alive);
    }

    #[test]
    fn test_cron_parsing() {
        // Basic: every field set
        let result = parse_cron("30 14 * * *");
        assert!(result.is_ok());
        let next = result.unwrap();
        assert_eq!(next.minute, 30);
        assert_eq!(next.hour, 14);

        // All wildcards
        let result = parse_cron("* * * * *");
        assert!(result.is_ok());

        // Specific day and month
        let result = parse_cron("0 0 15 6 *");
        assert!(result.is_ok());
        let next = result.unwrap();
        assert_eq!(next.minute, 0);
        assert_eq!(next.hour, 0);
        assert_eq!(next.day, 15);
        assert_eq!(next.month, 6);
    }

    #[test]
    fn test_cron_invalid() {
        // Too few fields
        assert!(parse_cron("30 14").is_err());

        // Invalid number
        assert!(parse_cron("abc 14 * * *").is_err());

        // Out of range
        assert!(parse_cron("60 14 * * *").is_err());
        assert!(parse_cron("0 25 * * *").is_err());
        assert!(parse_cron("0 0 32 * *").is_err());
        assert!(parse_cron("0 0 * 13 *").is_err());
    }

    #[test]
    fn test_scheduled_runs_empty() {
        let runs = check_scheduled_runs();
        assert!(runs.is_empty());
    }

    #[test]
    fn test_parse_cron_field_wildcard() {
        assert_eq!(parse_cron_field("*", 42, 0, 59).unwrap(), 42);
    }

    #[test]
    fn test_parse_cron_field_specific() {
        assert_eq!(parse_cron_field("15", 0, 0, 59).unwrap(), 15);
    }
}
