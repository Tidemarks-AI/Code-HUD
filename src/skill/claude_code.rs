use super::PlatformAdapter;
use crate::CodehudError;

pub struct ClaudeCodeAdapter;

impl PlatformAdapter for ClaudeCodeAdapter {
    fn install(&self) -> Result<(), CodehudError> {
        Err(CodehudError::NotImplemented("Claude Code".to_string()))
    }

    fn uninstall(&self) -> Result<(), CodehudError> {
        Err(CodehudError::NotImplemented("Claude Code".to_string()))
    }

    fn name(&self) -> &'static str {
        "Claude Code"
    }
}
