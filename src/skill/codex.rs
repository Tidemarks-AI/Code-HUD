use super::PlatformAdapter;
use super::content::SKILL_CONTENT;
use crate::CodehudError;
use std::fs;
use std::path::Path;
use tracing::info;

pub struct CodexAdapter;

const START_DELIMITER: &str = "<!-- codehud:start -->";
const END_DELIMITER: &str = "<!-- codehud:end -->";

/// Find the target file: prefer codex.md, fall back to AGENTS.md.
/// If neither exists, returns "AGENTS.md" (will be created).
fn target_file() -> &'static str {
    if Path::new("codex.md").exists() {
        "codex.md"
    } else {
        "AGENTS.md"
    }
}

/// Build the delimited block to insert.
fn build_block() -> String {
    format!(
        "{}\n{}\n{}",
        START_DELIMITER,
        SKILL_CONTENT.trim(),
        END_DELIMITER,
    )
}

/// Remove the codehud block from content, if present.
/// Returns the content without the block (and any surrounding blank lines cleaned up).
fn remove_block(content: &str) -> String {
    let Some(start) = content.find(START_DELIMITER) else {
        return content.to_string();
    };
    let Some(end) = content.find(END_DELIMITER) else {
        return content.to_string();
    };
    let end = end + END_DELIMITER.len();

    let before = content[..start].trim_end_matches('\n');
    let after = content[end..].trim_start_matches('\n');

    if before.is_empty() {
        after.to_string()
    } else if after.is_empty() {
        format!("{}\n", before)
    } else {
        format!("{}\n\n{}", before, after)
    }
}

impl PlatformAdapter for CodexAdapter {
    fn install(&self, _global: bool) -> Result<(), CodehudError> {
        let path = target_file();
        let block = build_block();

        let content = if Path::new(path).exists() {
            let existing = fs::read_to_string(path)?;
            // Remove old block first (idempotent)
            let cleaned = remove_block(&existing);
            if cleaned.is_empty() {
                format!("{}\n", block)
            } else {
                format!("{}\n\n{}\n", cleaned.trim_end(), block)
            }
        } else {
            format!("{}\n", block)
        };

        fs::write(path, &content)?;
        info!(path = %path, "Installed codehud skill");
        println!("Installed codehud skill to {}", path);
        Ok(())
    }

    fn uninstall(&self, _global: bool) -> Result<(), CodehudError> {
        // Check both files
        for path in &["codex.md", "AGENTS.md"] {
            let p = Path::new(path);
            if !p.exists() {
                continue;
            }
            let content = fs::read_to_string(p)?;
            if !content.contains(START_DELIMITER) {
                continue;
            }
            let cleaned = remove_block(&content);
            if cleaned.trim().is_empty() {
                fs::remove_file(p)?;
                info!(path = %path, "Removed codehud skill (deleted empty file)");
                println!("Removed {} (file was empty)", path);
            } else {
                fs::write(p, &cleaned)?;
                info!(path = %path, "Removed codehud skill block");
                println!("Removed codehud block from {}", path);
            }
            return Ok(());
        }

        println!("No codehud skill block found to remove");
        Ok(())
    }

    fn name(&self) -> &'static str {
        "Codex"
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn build_block_has_delimiters() {
        let block = build_block();
        assert!(block.starts_with(START_DELIMITER));
        assert!(block.ends_with(END_DELIMITER));
        assert!(block.contains("codehud"));
    }

    #[test]
    fn remove_block_strips_delimited_section() {
        let content = format!("# My File\n\n{}\nsome content\n{}\n\n# End\n",
            START_DELIMITER, END_DELIMITER);
        let result = remove_block(&content);
        assert!(!result.contains(START_DELIMITER));
        assert!(!result.contains(END_DELIMITER));
        assert!(result.contains("# My File"));
        assert!(result.contains("# End"));
    }

    #[test]
    fn remove_block_no_delimiters() {
        let content = "# Just a file\n";
        assert_eq!(remove_block(content), content);
    }

    #[test]
    fn remove_block_only_block() {
        let content = format!("{}\nstuff\n{}\n", START_DELIMITER, END_DELIMITER);
        let result = remove_block(&content);
        assert!(result.trim().is_empty() || result.is_empty());
    }
}
