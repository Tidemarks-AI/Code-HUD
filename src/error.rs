use thiserror::Error;

#[derive(Error, Debug)]
pub enum CodehudError {
    #[error("Path not found: {0}")]
    PathNotFound(String),
    
    #[error("Invalid path: {0}")]
    InvalidPath(String),
    
    #[error("Unsupported file extension: {0}")]
    UnsupportedExtension(String),
    
    #[error("No file extension found for path: {0}")]
    NoExtension(String),
    
    #[error("Failed to read {path}: {source}")]
    ReadError {
        path: String,
        #[source]
        source: std::io::Error,
    },
    
    #[error("Parse error: {0}")]
    ParseError(String),
    
    #[error("Serialization error")]
    SerializationError(#[from] serde_json::Error),

    #[error("symbol '{symbols}' not found in {path}")]
    SymbolNotFound {
        symbols: String,
        path: String,
    },

    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),

    #[error("Git error: {0}")]
    Git(String),

    #[error("Config error: {0}")]
    Config(String),

    #[error("Could not determine home directory")]
    HomeDir,

    #[error("Unknown platform '{platform}'. Available platforms: {available}")]
    UnknownPlatform {
        platform: String,
        available: String,
    },

    #[error("{0} adapter not yet implemented")]
    NotImplemented(String),
}
