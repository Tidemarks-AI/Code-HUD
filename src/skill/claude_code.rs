use super::{PlatformAdapter, SkillError};

pub struct ClaudeCodeAdapter;

impl PlatformAdapter for ClaudeCodeAdapter {
    fn install(&self) -> Result<(), SkillError> {
        Err(SkillError::NotImplemented("Claude Code".to_string()))
    }

    fn uninstall(&self) -> Result<(), SkillError> {
        Err(SkillError::NotImplemented("Claude Code".to_string()))
    }

    fn name(&self) -> &'static str {
        "Claude Code"
    }
}
