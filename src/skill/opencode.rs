use super::content::SKILL_CONTENT;
use super::PlatformAdapter;
use crate::CodehudError;
use std::fs;
use std::path::PathBuf;
use tracing::{debug, info};

pub struct OpenCodeAdapter;

const FRONTMATTER: &str = r#"---
name: codehud
description: "Tree-sitter powered structural code intelligence. Use for code exploration, symbol lookup, cross-references, and structural diff."
metadata:
  opencode:
    emoji: "🧠"
    requires:
      bins: ["codehud"]
    install:
      - id: cargo
        kind: shell
        command: "cargo install codehud"
        bins: ["codehud"]
        label: "Install codehud (cargo)"
      - id: script
        kind: shell
        command: "curl -fsSL https://raw.githubusercontent.com/Tidemarks-AI/Code-HUD/main/install.sh | sh"
        bins: ["codehud"]
        label: "Install codehud (install script)"
---
"#;

fn skill_dir() -> Result<PathBuf, CodehudError> {
    Ok(dirs::home_dir()
        .ok_or(CodehudError::HomeDir)?
        .join(".opencode/skills/codehud"))
}

impl PlatformAdapter for OpenCodeAdapter {
    fn install(&self, _global: bool) -> Result<(), CodehudError> {
        let dir = skill_dir()?;
        fs::create_dir_all(&dir)?;
        let path = dir.join("SKILL.md");
        let content = format!("{}\n{}\n", FRONTMATTER.trim(), SKILL_CONTENT.trim());
        fs::write(&path, content)?;
        info!(path = %path.display(), "Installed codehud skill");
        println!("Installed codehud skill to {}", path.display());
        Ok(())
    }

    fn uninstall(&self, _global: bool) -> Result<(), CodehudError> {
        let dir = skill_dir()?;
        let path = dir.join("SKILL.md");
        if path.exists() {
            fs::remove_file(&path)?;
            info!(path = %path.display(), "Removed codehud skill");
            println!("Removed {}", path.display());
        }
        if dir.exists() && fs::read_dir(&dir)?.next().is_none() {
            fs::remove_dir(&dir)?;
            debug!(dir = %dir.display(), "Removed empty skill directory");
            println!("Removed empty directory {}", dir.display());
        }
        Ok(())
    }

    fn name(&self) -> &'static str {
        "OpenCode"
    }
}
