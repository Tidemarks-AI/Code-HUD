use super::PlatformAdapter;
use crate::CodehudError;

pub struct CursorAdapter;

impl PlatformAdapter for CursorAdapter {
    fn install(&self) -> Result<(), CodehudError> {
        Err(CodehudError::NotImplemented("Cursor".to_string()))
    }

    fn uninstall(&self) -> Result<(), CodehudError> {
        Err(CodehudError::NotImplemented("Cursor".to_string()))
    }

    fn name(&self) -> &'static str {
        "Cursor"
    }
}
