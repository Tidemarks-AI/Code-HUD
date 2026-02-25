mod openclaw;

use crate::CodehudError;

/// Trait that each platform adapter implements for agent registration.
pub trait AgentAdapter {
    /// Install/register the codehud agent for this platform.
    fn install(&self) -> Result<(), CodehudError>;
    /// Uninstall/remove the codehud agent from this platform.
    fn uninstall(&self, force: bool) -> Result<(), CodehudError>;
    /// Human-readable platform name.
    fn name(&self) -> &'static str;
}

/// All supported platform names for agent installation.
pub const PLATFORMS: &[&str] = &["openclaw"];

/// List all available platforms to stdout.
pub fn list_platforms() {
    println!("Available platforms:");
    for p in PLATFORMS {
        println!("  {}", p);
    }
}

/// Get the adapter for a given platform name.
fn get_adapter(platform: &str) -> Result<Box<dyn AgentAdapter>, CodehudError> {
    match platform {
        "openclaw" => Ok(Box::new(openclaw::OpenClawAdapter)),
        _ => Err(CodehudError::UnknownPlatform {
            platform: platform.to_string(),
            available: PLATFORMS.join(", "),
        }),
    }
}

/// Install agent for the given platform.
pub fn install(platform: &str) -> Result<(), CodehudError> {
    let adapter = get_adapter(platform)?;
    adapter.install()
}

/// Uninstall agent for the given platform.
pub fn uninstall(platform: &str, force: bool) -> Result<(), CodehudError> {
    let adapter = get_adapter(platform)?;
    adapter.uninstall(force)
}
