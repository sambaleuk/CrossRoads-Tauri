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

/// Find loop scripts in bundled resources, PATH, or common locations
pub fn find_loop_script(name: &str) -> Option<String> {
    let home = std::env::var("HOME").unwrap_or_default();

    // 1. Bundled scripts (relative to binary or project root)
    let bundled_candidates = vec![
        format!("../scripts/{}", name),           // Dev mode (next to src-tauri/)
        format!("scripts/{}", name),               // From project root
        format!("./scripts/{}", name),             // CWD
        format!("../Resources/scripts/{}", name),  // macOS app bundle
    ];

    // Try bundled first — these are our real loop scripts
    for candidate in &bundled_candidates {
        let path = std::path::Path::new(candidate);
        if path.exists() {
            if let Ok(abs) = std::fs::canonicalize(path) {
                return Some(abs.to_string_lossy().to_string());
            }
        }
    }

    // 2. User-installed locations
    let user_candidates = vec![
        format!("/usr/local/bin/{}", name),
        format!("{}/.local/bin/{}", home, name),
        format!("{}/.xroads/scripts/{}", home, name),
        format!("{}/.nexus/{}", home, name),
    ];

    for candidate in &user_candidates {
        if std::path::Path::new(candidate).exists() {
            return Some(candidate.clone());
        }
    }

    // 3. Fall back to PATH
    if let Ok(output) = Command::new("which").arg(name).output() {
        if output.status.success() {
            return Some(String::from_utf8_lossy(&output.stdout).trim().to_string());
        }
    }

    None
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
