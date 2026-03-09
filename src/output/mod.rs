pub mod json;
pub mod plain;
pub mod stats;

#[derive(Debug, Clone, Copy, PartialEq, Default)]
pub enum OutputFormat {
    #[default]
    Plain,
    Json,
}
