//! Headless execution driver for CLI agents (Claude Code, Antigravity, Codex)
//! and harness persona integration from `spec-harness-kit`.

use std::path::{Path, PathBuf};
use std::process::Command;

/// Resolve the directory where `spec-harness-kit/agents` lives.
pub fn harness_agents_dir() -> Option<PathBuf> {
    if let Ok(dir) = std::env::var("TALOS_SPEC_HARNESS_AGENTS") {
        let p = PathBuf::from(dir);
        if p.is_dir() {
            return Some(p);
        }
    }
    // Submodule inside repository
    let sub = Path::new(env!("CARGO_MANIFEST_DIR")).join("spec-harness-kit/agents");
    if sub.is_dir() {
        return Some(sub);
    }
    // Sibling in ~/Code
    if let Some(home) = crate::paths::home_dir() {
        let p = home.join("Code/spec-harness-kit/agents");
        if p.is_dir() {
            return Some(p);
        }
        let staged = home.join(".local/share/talos/harness/agents");
        if staged.is_dir() {
            return Some(staged);
        }
    }
    None
}

/// Load an agent's specification (instructions and rules) from spec-harness-kit.
pub fn load_harness_agent(name: &str) -> Option<String> {
    let dir = harness_agents_dir()?;
    let clean_name = name.trim().trim_start_matches('@').trim_end_matches(".md");
    let path = dir.join(format!("{clean_name}.md"));
    if path.is_file() {
        std::fs::read_to_string(&path).ok()
    } else {
        None
    }
}

/// List all available personas from spec-harness-kit.
pub fn list_harness_agents() -> Vec<(String, String)> {
    let mut list = Vec::new();
    let Some(dir) = harness_agents_dir() else {
        return list;
    };
    let Ok(entries) = std::fs::read_dir(dir) else {
        return list;
    };
    for entry in entries.flatten() {
        let path = entry.path();
        if path.extension().and_then(|s| s.to_str()) == Some("md") {
            let stem = path.file_stem().and_then(|s| s.to_str()).unwrap_or_default().to_string();
            let desc = std::fs::read_to_string(&path)
                .ok()
                .and_then(|content| {
                    for line in content.lines().take(15) {
                        if line.starts_with("description:") {
                            return Some(line.trim_start_matches("description:").trim().trim_matches('\'').trim_matches('"').to_string());
                        }
                    }
                    None
                })
                .unwrap_or_else(|| format!("Especialista @{stem} do spec-harness-kit"));
            list.push((stem, desc));
        }
    }
    list.sort_by(|a, b| a.0.cmp(&b.0));
    list
}

/// Run a CLI agent in headless mode using its authenticated OAuth credentials.
pub fn run_headless_cli(
    agent_cli: &str,
    system_prompt: Option<&str>,
    user_prompt: &str,
    cwd: &Path,
) -> Result<String, String> {
    let clean_cli = agent_cli.to_lowercase();
    let mut cmd = if clean_cli.contains("claude") {
        let mut c = Command::new("claude");
        c.arg("-p").arg(user_prompt);
        if let Some(sys) = system_prompt {
            c.arg("--append-system-prompt").arg(sys);
        }
        c
    } else if clean_cli.contains("antigravity") || clean_cli == "agy" {
        let mut c = Command::new("agy");
        let combined = if let Some(sys) = system_prompt {
            format!("System Instructions:\n{}\n\nUser Request:\n{}", sys, user_prompt)
        } else {
            user_prompt.to_string()
        };
        c.arg("-p").arg(&combined);
        c
    } else if clean_cli.contains("codex") {
        let mut c = Command::new("codex");
        let combined = if let Some(sys) = system_prompt {
            format!("System Instructions:\n{}\n\nUser Request:\n{}", sys, user_prompt)
        } else {
            user_prompt.to_string()
        };
        c.arg("exec").arg(&combined);
        c
    } else {
        // Fallback to generic command invocation with -p
        let mut c = Command::new(&clean_cli);
        c.arg("-p").arg(user_prompt);
        c
    };

    cmd.current_dir(cwd);

    let output = cmd.output().map_err(|e| {
        format!("Falha ao iniciar agente CLI headless '{agent_cli}': {e}")
    })?;

    let stdout = String::from_utf8_lossy(&output.stdout).trim().to_string();
    let stderr = String::from_utf8_lossy(&output.stderr).trim().to_string();

    if !stdout.is_empty() {
        Ok(stdout)
    } else if !stderr.is_empty() {
        if output.status.success() {
            Ok(stderr)
        } else {
            Err(format!("Agente '{agent_cli}' retornou erro: {stderr}"))
        }
    } else {
        Ok(format!("Agente '{agent_cli}' concluiu sem saída textual."))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_harness_agents_dir_resolves() {
        let dir = harness_agents_dir();
        assert!(dir.is_some(), "harness agents directory should resolve");
        let dir = dir.unwrap();
        assert!(dir.is_dir(), "agents path must be a directory");
    }

    #[test]
    fn test_load_harness_agent_spec_master() {
        let content = load_harness_agent("spec-master");
        assert!(content.is_some(), "spec-master agent should be loadable");
        assert!(content.unwrap().contains("spec-master"), "should contain agent spec");
    }

    #[test]
    fn test_list_harness_agents() {
        let agents = list_harness_agents();
        assert!(!agents.is_empty(), "should list available harness agents");
        let names: Vec<String> = agents.into_iter().map(|(n, _)| n).collect();
        assert!(names.contains(&"spec-master".to_string()));
        assert!(names.contains(&"architect".to_string()));
        assert!(names.contains(&"dev".to_string()));
    }
}

