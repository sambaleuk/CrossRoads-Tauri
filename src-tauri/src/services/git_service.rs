use std::process::Command;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct GitCommit {
    pub sha: String,
    pub short_sha: String,
    pub message: String,
    pub author: String,
    pub date: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct MergeResult {
    pub success: bool,
    pub merged_branches: Vec<String>,
    pub conflicts: Vec<String>,
    pub rolled_back: bool,
}

fn run_git(args: &[&str], cwd: &str) -> Result<String, String> {
    let output = Command::new("git")
        .args(args)
        .current_dir(cwd)
        .output()
        .map_err(|e| format!("Failed to run git: {}", e))?;

    if output.status.success() {
        Ok(String::from_utf8_lossy(&output.stdout).trim().to_string())
    } else {
        Err(String::from_utf8_lossy(&output.stderr).trim().to_string())
    }
}

pub fn is_git_repo(path: &str) -> bool {
    run_git(&["rev-parse", "--is-inside-work-tree"], path).is_ok()
}

pub fn get_current_branch(path: &str) -> Result<String, String> {
    run_git(&["branch", "--show-current"], path)
}

pub fn get_recent_commits(path: &str, count: usize) -> Result<Vec<GitCommit>, String> {
    let format = "%H%n%h%n%s%n%an%n%aI";
    let output = run_git(&["log", &format!("-{}", count), &format!("--format={}", format)], path)?;

    let lines: Vec<&str> = output.lines().collect();
    let mut commits = Vec::new();

    for chunk in lines.chunks(5) {
        if chunk.len() == 5 {
            commits.push(GitCommit {
                sha: chunk[0].to_string(),
                short_sha: chunk[1].to_string(),
                message: chunk[2].to_string(),
                author: chunk[3].to_string(),
                date: chunk[4].to_string(),
            });
        }
    }
    Ok(commits)
}

pub fn get_branches(path: &str) -> Result<Vec<String>, String> {
    let output = run_git(&["branch", "--format=%(refname:short)"], path)?;
    Ok(output.lines().map(|l| l.trim().to_string()).filter(|l| !l.is_empty()).collect())
}

pub fn get_status(path: &str) -> Result<String, String> {
    run_git(&["status", "--porcelain"], path)
}

pub fn checkout(path: &str, branch: &str) -> Result<(), String> {
    run_git(&["checkout", branch], path)?;
    Ok(())
}

pub fn create_branch(path: &str, branch: &str) -> Result<(), String> {
    run_git(&["checkout", "-b", branch], path)?;
    Ok(())
}

// Worktree management
pub fn create_worktree(repo_path: &str, worktree_path: &str, branch: &str) -> Result<(), String> {
    run_git(&["worktree", "add", worktree_path, "-b", branch], repo_path)?;
    Ok(())
}

pub fn delete_worktree(repo_path: &str, worktree_path: &str) -> Result<(), String> {
    run_git(&["worktree", "remove", worktree_path, "--force"], repo_path)?;
    Ok(())
}

pub fn list_worktrees(repo_path: &str) -> Result<Vec<String>, String> {
    let output = run_git(&["worktree", "list", "--porcelain"], repo_path)?;
    Ok(output.lines()
        .filter(|l| l.starts_with("worktree "))
        .map(|l| l.trim_start_matches("worktree ").to_string())
        .collect())
}

pub fn prune_worktrees(repo_path: &str) -> Result<(), String> {
    run_git(&["worktree", "prune"], repo_path)?;
    Ok(())
}

pub fn delete_branch(repo_path: &str, branch: &str) -> Result<(), String> {
    run_git(&["branch", "-D", branch], repo_path)?;
    Ok(())
}

// Merge operations
pub fn merge(repo_path: &str, branch: &str, no_commit: bool) -> Result<(), String> {
    let mut args = vec!["merge", branch, "--no-ff"];
    if no_commit {
        args.push("--no-commit");
    }
    run_git(&args, repo_path)?;
    Ok(())
}

pub fn abort_merge(repo_path: &str) -> Result<(), String> {
    run_git(&["merge", "--abort"], repo_path)?;
    Ok(())
}

pub fn reset_hard(repo_path: &str, reference: Option<&str>) -> Result<(), String> {
    let mut args = vec!["reset", "--hard"];
    if let Some(r) = reference {
        args.push(r);
    }
    run_git(&args, repo_path)?;
    Ok(())
}

pub fn get_current_commit_hash(repo_path: &str) -> Result<String, String> {
    run_git(&["rev-parse", "HEAD"], repo_path)
}

pub fn list_conflicted_files(repo_path: &str) -> Result<Vec<String>, String> {
    let output = run_git(&["diff", "--name-only", "--diff-filter=U"], repo_path)?;
    Ok(output.lines().map(String::from).filter(|l| !l.is_empty()).collect())
}

/// Execute a full merge with rollback on partial failure
pub fn coordinate_merge(repo_path: &str, branches: &[&str]) -> Result<MergeResult, String> {
    let base_head = get_current_commit_hash(repo_path)?;
    let mut merged: Vec<String> = Vec::new();
    let mut conflicts: Vec<String> = Vec::new();
    let mut rolled_back = false;

    for branch in branches {
        match merge(repo_path, branch, false) {
            Ok(()) => merged.push(branch.to_string()),
            Err(_) => {
                let files = list_conflicted_files(repo_path).unwrap_or_default();
                conflicts.extend(files);
                let _ = abort_merge(repo_path);
                let _ = reset_hard(repo_path, Some(&base_head));
                merged.clear();
                rolled_back = true;
                break;
            }
        }
    }

    Ok(MergeResult {
        success: conflicts.is_empty(),
        merged_branches: merged,
        conflicts,
        rolled_back,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;

    fn setup_test_repo() -> String {
        let dir = format!("/tmp/xroads-git-test-{}", uuid::Uuid::new_v4());
        fs::create_dir_all(&dir).unwrap();
        run_git(&["init"], &dir).unwrap();
        run_git(&["config", "user.email", "test@test.com"], &dir).unwrap();
        run_git(&["config", "user.name", "Test"], &dir).unwrap();
        fs::write(format!("{}/README.md", dir), "# Test").unwrap();
        run_git(&["add", "."], &dir).unwrap();
        run_git(&["commit", "-m", "init"], &dir).unwrap();
        dir
    }

    #[test]
    fn test_is_git_repo() {
        let dir = setup_test_repo();
        assert!(is_git_repo(&dir));
        assert!(!is_git_repo("/tmp/nonexistent-xyz"));
        fs::remove_dir_all(&dir).ok();
    }

    #[test]
    fn test_get_branch_and_commits() {
        let dir = setup_test_repo();
        let branch = get_current_branch(&dir).unwrap();
        assert!(!branch.is_empty());

        let commits = get_recent_commits(&dir, 5).unwrap();
        assert_eq!(commits.len(), 1);
        assert_eq!(commits[0].message, "init");
        fs::remove_dir_all(&dir).ok();
    }

    #[test]
    fn test_merge_clean() {
        let dir = setup_test_repo();
        create_branch(&dir, "feat/test").unwrap();
        fs::write(format!("{}/new.txt", dir), "new file").unwrap();
        run_git(&["add", "."], &dir).unwrap();
        run_git(&["commit", "-m", "add new file"], &dir).unwrap();
        checkout(&dir, "main").unwrap_or_else(|_| {
            checkout(&dir, "master").unwrap();
        });

        let result = coordinate_merge(&dir, &["feat/test"]).unwrap();
        assert!(result.success);
        assert_eq!(result.merged_branches.len(), 1);
        assert!(!result.rolled_back);
        fs::remove_dir_all(&dir).ok();
    }

    #[test]
    fn test_merge_conflict_rollback() {
        let dir = setup_test_repo();
        let main = get_current_branch(&dir).unwrap();

        // Branch A modifies file
        create_branch(&dir, "branch-a").unwrap();
        fs::write(format!("{}/shared.txt", dir), "branch A content").unwrap();
        run_git(&["add", "."], &dir).unwrap();
        run_git(&["commit", "-m", "branch A"], &dir).unwrap();

        // Branch B modifies same file
        checkout(&dir, &main).unwrap();
        create_branch(&dir, "branch-b").unwrap();
        fs::write(format!("{}/shared.txt", dir), "branch B content").unwrap();
        run_git(&["add", "."], &dir).unwrap();
        run_git(&["commit", "-m", "branch B"], &dir).unwrap();

        // Merge both — should conflict on second
        checkout(&dir, &main).unwrap();
        let result = coordinate_merge(&dir, &["branch-a", "branch-b"]).unwrap();
        assert!(!result.success);
        assert!(result.rolled_back);
        assert_eq!(result.merged_branches.len(), 0);
        fs::remove_dir_all(&dir).ok();
    }
}
