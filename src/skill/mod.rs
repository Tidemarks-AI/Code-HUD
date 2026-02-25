pub mod content;
mod openclaw;
mod claude_code;
mod codex;
mod cursor;
mod aider;

use crate::CodehudError;

/// Trait that each platform adapter implements.
pub trait PlatformAdapter {
    /// Install the codehud skill for this platform.
    fn install(&self) -> Result<(), CodehudError>;
    /// Uninstall the codehud skill for this platform.
    fn uninstall(&self) -> Result<(), CodehudError>;
    /// Human-readable platform name.
    fn name(&self) -> &'static str;
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
fn get_adapter(platform: &str) -> Result<Box<dyn PlatformAdapter>, CodehudError> {
    match platform {
        "openclaw" => Ok(Box::new(openclaw::OpenClawAdapter)),
        "claude-code" => Ok(Box::new(claude_code::ClaudeCodeAdapter)),
        "codex" => Ok(Box::new(codex::CodexAdapter)),
        "cursor" => Ok(Box::new(cursor::CursorAdapter)),
        "aider" => Ok(Box::new(aider::AiderAdapter)),
        _ => Err(CodehudError::UnknownPlatform {
            platform: platform.to_string(),
            available: PLATFORMS.join(", "),
        }),
    }
}

/// Install skill for the given platform.
pub fn install(platform: &str) -> Result<(), CodehudError> {
    let adapter = get_adapter(platform)?;
    adapter.install()
}

/// Uninstall skill for the given platform.
pub fn uninstall(platform: &str) -> Result<(), CodehudError> {
    let adapter = get_adapter(platform)?;
    adapter.uninstall()
}
