use super::PlatformAdapter;
use crate::CodehudError;

pub struct CodexAdapter;

impl PlatformAdapter for CodexAdapter {
    fn install(&self) -> Result<(), CodehudError> {
        Err(CodehudError::NotImplemented("Codex".to_string()))
    }

    fn uninstall(&self) -> Result<(), CodehudError> {
        Err(CodehudError::NotImplemented("Codex".to_string()))
    }

    fn name(&self) -> &'static str {
        "Codex"
    }
}
