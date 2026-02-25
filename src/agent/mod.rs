mod openclaw;

use std::fmt;

/// Trait that each platform adapter implements for agent registration.
pub trait AgentAdapter {
    /// Install/register the codehud agent for this platform.
    fn install(&self) -> Result<(), AgentError>;
    /// Uninstall/remove the codehud agent from this platform.
    fn uninstall(&self, force: bool) -> Result<(), AgentError>;
    /// Human-readable platform name.
    fn name(&self) -> &'static str;
}

#[derive(Debug)]
pub enum AgentError {
    NotImplemented(String),
    Io(std::io::Error),
    Json(serde_json::Error),
    UnknownPlatform(String),
    Config(String),
}

impl fmt::Display for AgentError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            AgentError::NotImplemented(p) => write!(f, "{} agent adapter not yet implemented", p),
            AgentError::Io(e) => write!(f, "IO error: {}", e),
            AgentError::Json(e) => write!(f, "JSON error: {}", e),
            AgentError::UnknownPlatform(p) => {
                write!(f, "Unknown platform '{}'. Available platforms: {}", p, PLATFORMS.join(", "))
            }
            AgentError::Config(msg) => write!(f, "Config error: {}", msg),
        }
    }
}

impl From<std::io::Error> for AgentError {
    fn from(e: std::io::Error) -> Self {
        AgentError::Io(e)
    }
}

impl From<serde_json::Error> for AgentError {
    fn from(e: serde_json::Error) -> Self {
        AgentError::Json(e)
    }
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
fn get_adapter(platform: &str) -> Result<Box<dyn AgentAdapter>, AgentError> {
    match platform {
        "openclaw" => Ok(Box::new(openclaw::OpenClawAdapter)),
        _ => Err(AgentError::UnknownPlatform(platform.to_string())),
    }
}

/// Install agent for the given platform.
pub fn install(platform: &str) -> Result<(), AgentError> {
    let adapter = get_adapter(platform)?;
    adapter.install()
}

/// Uninstall agent for the given platform.
pub fn uninstall(platform: &str, force: bool) -> Result<(), AgentError> {
    let adapter = get_adapter(platform)?;
    adapter.uninstall(force)
}
