use super::PlatformAdapter;
use crate::CodehudError;

pub struct AiderAdapter;

impl PlatformAdapter for AiderAdapter {
    fn install(&self, _global: bool) -> Result<(), CodehudError> {
        Err(CodehudError::NotImplemented("Aider".to_string()))
    }

    fn uninstall(&self, _global: bool) -> Result<(), CodehudError> {
        Err(CodehudError::NotImplemented("Aider".to_string()))
    }

    fn name(&self) -> &'static str {
        "Aider"
    }
}
