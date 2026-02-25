use super::{PlatformAdapter, SkillError};

pub struct AiderAdapter;

impl PlatformAdapter for AiderAdapter {
    fn install(&self) -> Result<(), SkillError> {
        Err(SkillError::NotImplemented("Aider".to_string()))
    }

    fn uninstall(&self) -> Result<(), SkillError> {
        Err(SkillError::NotImplemented("Aider".to_string()))
    }

    fn name(&self) -> &'static str {
        "Aider"
    }
}
