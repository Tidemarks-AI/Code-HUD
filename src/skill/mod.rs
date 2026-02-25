pub mod content;
mod openclaw;
mod claude_code;
mod codex;
mod cursor;
mod aider;

use std::fmt;

/// Trait that each platform adapter implements.
pub trait PlatformAdapter {
    /// Install the codehud skill for this platform.
    fn install(&self) -> Result<(), SkillError>;
    /// Uninstall the codehud skill for this platform.
    fn uninstall(&self) -> Result<(), SkillError>;
    /// Human-readable platform name.
    fn name(&self) -> &'static str;
}

#[derive(Debug)]
pub enum SkillError {
    NotImplemented(String),
    Io(std::io::Error),
    UnknownPlatform(String),
}

impl fmt::Display for SkillError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            SkillError::NotImplemented(p) => write!(f, "{} adapter not yet implemented (see https://github.com/AvoCloud/codeview/issues)", p),
            SkillError::Io(e) => write!(f, "IO error: {}", e),
            SkillError::UnknownPlatform(p) => {
                write!(f, "Unknown platform '{}'. Available platforms: {}", p, PLATFORMS.join(", "))
            }
        }
    }
}

impl From<std::io::Error> for SkillError {
    fn from(e: std::io::Error) -> Self {
        SkillError::Io(e)
    }
}

/// All supported platform names.
pub const PLATFORMS: &[&str] = &["openclaw", "claude-code", "codex", "cursor", "aider"];

/// List all available platforms to stdout.
pub fn list_platforms() {
    println!("Available platforms:");
    for p in PLATFORMS {
        println!("  {}", p);
    }
}

/// Get the adapter for a given platform name.
fn get_adapter(platform: &str) -> Result<Box<dyn PlatformAdapter>, SkillError> {
    match platform {
        "openclaw" => Ok(Box::new(openclaw::OpenClawAdapter)),
        "claude-code" => Ok(Box::new(claude_code::ClaudeCodeAdapter)),
        "codex" => Ok(Box::new(codex::CodexAdapter)),
        "cursor" => Ok(Box::new(cursor::CursorAdapter)),
        "aider" => Ok(Box::new(aider::AiderAdapter)),
        _ => Err(SkillError::UnknownPlatform(platform.to_string())),
    }
}

/// Install skill for the given platform.
pub fn install(platform: &str) -> Result<(), SkillError> {
    let adapter = get_adapter(platform)?;
    adapter.install()
}

/// Uninstall skill for the given platform.
pub fn uninstall(platform: &str) -> Result<(), SkillError> {
    let adapter = get_adapter(platform)?;
    adapter.uninstall()
}
