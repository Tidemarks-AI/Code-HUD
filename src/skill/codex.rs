use super::{PlatformAdapter, SkillError};

pub struct CodexAdapter;

impl PlatformAdapter for CodexAdapter {
    fn install(&self) -> Result<(), SkillError> {
        Err(SkillError::NotImplemented("Codex".to_string()))
    }

    fn uninstall(&self) -> Result<(), SkillError> {
        Err(SkillError::NotImplemented("Codex".to_string()))
    }

    fn name(&self) -> &'static str {
        "Codex"
    }
}
