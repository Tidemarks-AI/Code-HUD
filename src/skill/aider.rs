use super::PlatformAdapter;
use crate::CodehudError;

pub struct AiderAdapter;

impl PlatformAdapter for AiderAdapter {
    fn install(&self) -> Result<(), CodehudError> {
        Err(CodehudError::NotImplemented("Aider".to_string()))
    }

    fn uninstall(&self) -> Result<(), CodehudError> {
        Err(CodehudError::NotImplemented("Aider".to_string()))
    }

    fn name(&self) -> &'static str {
        "Aider"
    }
}
