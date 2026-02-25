use super::{PlatformAdapter, SkillError};

pub struct CursorAdapter;

impl PlatformAdapter for CursorAdapter {
    fn install(&self) -> Result<(), SkillError> {
        Err(SkillError::NotImplemented("Cursor".to_string()))
    }

    fn uninstall(&self) -> Result<(), SkillError> {
        Err(SkillError::NotImplemented("Cursor".to_string()))
    }

    fn name(&self) -> &'static str {
        "Cursor"
    }
}
