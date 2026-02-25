use super::{PlatformAdapter, SkillError};

pub struct OpenClawAdapter;

impl PlatformAdapter for OpenClawAdapter {
    fn install(&self) -> Result<(), SkillError> {
        Err(SkillError::NotImplemented("OpenClaw".to_string()))
    }

    fn uninstall(&self) -> Result<(), SkillError> {
        Err(SkillError::NotImplemented("OpenClaw".to_string()))
    }

    fn name(&self) -> &'static str {
        "OpenClaw"
    }
}
