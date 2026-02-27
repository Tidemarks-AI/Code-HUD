use super::PlatformAdapter;
use super::content::SKILL_CONTENT;
use crate::CodehudError;
use std::fs;
use std::path::PathBuf;
use tracing::{debug, info};

pub struct CursorAdapter;

const MDC_FRONTMATTER: &str = r#"---
description: "Tree-sitter powered structural code intelligence. Use for code exploration, symbol lookup, cross-references, and structural diff."
alwaysApply: true
---
"#;

fn rules_dir() -> PathBuf {
    PathBuf::from(".cursor/rules")
}

impl PlatformAdapter for CursorAdapter {
    fn install(&self) -> Result<(), CodehudError> {
        let dir = rules_dir();
        fs::create_dir_all(&dir)?;
        let path = dir.join("codehud.mdc");
        let content = format!("{}\n{}\n", MDC_FRONTMATTER.trim(), SKILL_CONTENT.trim());
        fs::write(&path, &content)?;
        info!(path = %path.display(), "Installed codehud skill for Cursor");
        println!("Installed codehud skill to {}", path.display());
        Ok(())
    }

    fn uninstall(&self) -> Result<(), CodehudError> {
        let dir = rules_dir();
        let path = dir.join("codehud.mdc");
        if path.exists() {
            fs::remove_file(&path)?;
            info!(path = %path.display(), "Removed codehud skill for Cursor");
            println!("Removed {}", path.display());
        }
        // Remove directory if empty
        if dir.exists() && fs::read_dir(&dir)?.next().is_none() {
            fs::remove_dir(&dir)?;
            debug!(dir = %dir.display(), "Removed empty rules directory");
            println!("Removed empty directory {}", dir.display());
        }
        Ok(())
    }

    fn name(&self) -> &'static str {
        "Cursor"
    }
}
