use std::process::Command;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CliStatus {
    pub name: String,
    pub available: bool,
    pub version: Option<String>,
    pub path: Option<String>,
}

/// Detect if a CLI tool is installed and get its version
fn detect_cli(name: &str, version_flag: &str) -> CliStatus {
    let result = Command::new("which")
        .arg(name)
        .output();

    let path = match result {
        Ok(output) if output.status.success() => {
            Some(String::from_utf8_lossy(&output.stdout).trim().to_string())
        }
        _ => None,
    };

    let version = if path.is_some() {
        Command::new(name)
            .arg(version_flag)
            .output()
            .ok()
            .filter(|o| o.status.success())
            .map(|o| String::from_utf8_lossy(&o.stdout).trim().to_string())
    } else {
        None
    };

    CliStatus {
        name: name.to_string(),
        available: path.is_some(),
        version,
        path,
    }
}

/// Detect all supported CLI tools
pub fn detect_all() -> Vec<CliStatus> {
    vec![
        detect_cli("claude", "--version"),
        detect_cli("gemini", "--version"),
        detect_cli("codex", "--version"),
        detect_cli("git", "--version"),
        detect_cli("node", "--version"),
    ]
}

/// Find loop scripts in PATH or common locations
pub fn find_loop_script(name: &str) -> Option<String> {
    let candidates = vec![
        format!("/usr/local/bin/{}", name),
        format!("{}/.local/bin/{}", std::env::var("HOME").unwrap_or_default(), name),
        format!("{}/.cargo/bin/{}", std::env::var("HOME").unwrap_or_default(), name),
    ];

    // Check which first
    if let Ok(output) = Command::new("which").arg(name).output() {
        if output.status.success() {
            return Some(String::from_utf8_lossy(&output.stdout).trim().to_string());
        }
    }

    candidates.into_iter().find(|p| std::path::Path::new(p).exists())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_detect_git() {
        let tools = detect_all();
        let git = tools.iter().find(|t| t.name == "git").unwrap();
        assert!(git.available, "git should be installed");
        assert!(git.version.is_some());
    }

    #[test]
    fn test_detect_nonexistent() {
        let status = detect_cli("nonexistent_tool_xyz", "--version");
        assert!(!status.available);
        assert!(status.path.is_none());
    }
}
