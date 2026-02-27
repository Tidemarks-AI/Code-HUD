use super::PlatformAdapter;
use super::content::SKILL_CONTENT;
use crate::CodehudError;
use std::fs;
use std::path::PathBuf;
use tracing::info;

pub struct ClaudeCodeAdapter;

const MARKER_START: &str = "<!-- codehud:start -->";
const MARKER_END: &str = "<!-- codehud:end -->";

const SLASH_COMMAND: &str = r#"# codehud

Run codehud commands for structural code intelligence.

Usage: `codehud <args>`

Examples:
- `codehud --stats .` — language breakdown
- `codehud --outline src/main.rs` — function signatures
- `codehud --search "Config" .` — find symbols
- `codehud --xrefs "parse_expr" .` — cross-references
- `codehud --diff` — structural diff vs HEAD
"#;

fn claude_md_path(global: bool) -> Result<PathBuf, CodehudError> {
    if global {
        Ok(dirs::home_dir()
            .ok_or(CodehudError::HomeDir)?
            .join(".claude/CLAUDE.md"))
    } else {
        Ok(PathBuf::from("CLAUDE.md"))
    }
}

fn build_block() -> String {
    format!("{}\n{}\n{}", MARKER_START, SKILL_CONTENT.trim(), MARKER_END)
}

/// Insert or replace the codehud block in content. Returns the new content.
fn upsert_block(existing: &str, block: &str) -> String {
    if let (Some(start), Some(end)) = (existing.find(MARKER_START), existing.find(MARKER_END)) {
        let end = end + MARKER_END.len();
        let mut result = String::with_capacity(existing.len());
        result.push_str(&existing[..start]);
        result.push_str(block);
        result.push_str(&existing[end..]);
        result
    } else {
        // Append with blank line separator
        let mut result = existing.to_string();
        if !result.is_empty() && !result.ends_with('\n') {
            result.push('\n');
        }
        if !result.is_empty() {
            result.push('\n');
        }
        result.push_str(block);
        result.push('\n');
        result
    }
}

/// Remove the codehud block from content. Returns the new content.
fn remove_block(existing: &str) -> String {
    if let (Some(start), Some(end)) = (existing.find(MARKER_START), existing.find(MARKER_END)) {
        let end = end + MARKER_END.len();
        let mut result = String::with_capacity(existing.len());
        result.push_str(&existing[..start]);
        let rest = &existing[end..];
        // Trim leading newlines from the remainder to avoid double blanks
        let rest = rest.trim_start_matches('\n');
        result.push_str(rest);
        // Remove trailing whitespace if file is now empty-ish
        let trimmed = result.trim();
        if trimmed.is_empty() {
            String::new()
        } else {
            let mut s = trimmed.to_string();
            s.push('\n');
            s
        }
    } else {
        existing.to_string()
    }
}

fn install_slash_command() -> Result<(), CodehudError> {
    let dir = PathBuf::from(".claude/commands");
    fs::create_dir_all(&dir)?;
    let path = dir.join("codehud.md");
    fs::write(&path, SLASH_COMMAND)?;
    info!(path = %path.display(), "Installed slash command");
    println!("Installed slash command to {}", path.display());
    Ok(())
}

fn uninstall_slash_command() {
    let path = PathBuf::from(".claude/commands/codehud.md");
    if path.exists() {
        if let Ok(()) = fs::remove_file(&path) {
            println!("Removed {}", path.display());
        }
    }
}

impl PlatformAdapter for ClaudeCodeAdapter {
    fn install(&self, global: bool) -> Result<(), CodehudError> {
        let path = claude_md_path(global)?;
        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent)?;
        }
        let existing = fs::read_to_string(&path).unwrap_or_default();
        let block = build_block();
        let content = upsert_block(&existing, &block);
        fs::write(&path, &content)?;
        info!(path = %path.display(), "Installed codehud skill");
        println!("Installed codehud skill to {}", path.display());

        // Install slash command for project-local installs
        if !global {
            install_slash_command()?;
        }

        Ok(())
    }

    fn uninstall(&self, global: bool) -> Result<(), CodehudError> {
        let path = claude_md_path(global)?;
        if !path.exists() {
            return Ok(());
        }
        let existing = fs::read_to_string(&path)?;
        let content = remove_block(&existing);
        if content.is_empty() {
            fs::remove_file(&path)?;
            println!("Removed {}", path.display());
        } else {
            fs::write(&path, &content)?;
            println!("Removed codehud block from {}", path.display());
        }

        if !global {
            uninstall_slash_command();
        }

        Ok(())
    }

    fn name(&self) -> &'static str {
        "Claude Code"
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_upsert_into_empty() {
        let block = build_block();
        let result = upsert_block("", &block);
        assert!(result.contains(MARKER_START));
        assert!(result.contains(MARKER_END));
        assert!(result.contains("codehud"));
    }

    #[test]
    fn test_upsert_appends_to_existing() {
        let block = build_block();
        let existing = "# My Project\n\nSome existing content.\n";
        let result = upsert_block(existing, &block);
        assert!(result.starts_with("# My Project"));
        assert!(result.contains(MARKER_START));
        assert!(result.contains("codehud"));
    }

    #[test]
    fn test_upsert_replaces_existing_block() {
        let old_block = format!("{}\nold content\n{}", MARKER_START, MARKER_END);
        let existing = format!("# Header\n\n{}\n\n# Footer\n", old_block);
        let new_block = build_block();
        let result = upsert_block(&existing, &new_block);
        assert!(result.contains(MARKER_START));
        assert!(!result.contains("old content"));
        assert!(result.contains("codehud"));
        assert!(result.contains("# Header"));
        assert!(result.contains("# Footer"));
        // Only one start marker
        assert_eq!(result.matches(MARKER_START).count(), 1);
    }

    #[test]
    fn test_idempotent_install() {
        let block = build_block();
        let first = upsert_block("", &block);
        let second = upsert_block(&first, &block);
        assert_eq!(first, second);
    }

    #[test]
    fn test_remove_block() {
        let block = build_block();
        let content = upsert_block("# Header\n", &block);
        let result = remove_block(&content);
        assert!(!result.contains(MARKER_START));
        assert!(!result.contains(MARKER_END));
        assert!(result.contains("# Header"));
    }

    #[test]
    fn test_remove_block_empty_file() {
        let block = build_block();
        let content = upsert_block("", &block);
        let result = remove_block(&content);
        assert!(result.is_empty());
    }

    #[test]
    fn test_remove_no_block() {
        let content = "# Just a file\n";
        let result = remove_block(content);
        assert_eq!(result, content);
    }
}
