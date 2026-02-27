use codehud::{process_path, extract_lines, ProcessOptions, OutputFormat};

const TOML_FIXTURE: &str = "tests/fixtures/config.toml";
const MD_FIXTURE: &str = "tests/fixtures/readme.md";
const JSON_FIXTURE: &str = "tests/fixtures/settings.json";
const ENV_FIXTURE: &str = "tests/fixtures/sample.env";
const FIXTURE_DIR: &str = "tests/fixtures";

fn default_options() -> ProcessOptions {
    ProcessOptions {
        symbols: vec![],
        pub_only: false,
        fns_only: false,
        types_only: false,
        no_tests: false,
        depth: None,
        format: OutputFormat::Plain,
        stats: false, stats_detailed: true,
        ext: vec![],
        signatures: false,
        max_lines: None,
        list_symbols: false,
        no_imports: false,
        smart_depth: false,
        symbol_depth: None,
        exclude: vec![],
        outline: false,
        compact: false,
        minimal: false,
        expand_symbols: vec![],
        yes: false,
        warn_threshold: 10_000,
    }
}

#[test]
fn passthrough_view_toml() {
    let result = process_path(TOML_FIXTURE, default_options()).unwrap();
    assert!(result.contains("[package]"));
    assert!(result.contains("name = \"example\""));
    // Should have line numbers
    assert!(result.contains("1:"));
}

#[test]
fn passthrough_view_markdown() {
    let result = process_path(MD_FIXTURE, default_options()).unwrap();
    assert!(result.contains("# Example Project"));
    assert!(result.contains("Feature one"));
}

#[test]
fn passthrough_view_json() {
    let result = process_path(JSON_FIXTURE, default_options()).unwrap();
    assert!(result.contains("\"debug\": true"));
}

#[test]
fn passthrough_view_env() {
    let result = process_path(ENV_FIXTURE, default_options()).unwrap();
    assert!(result.contains("DATABASE_URL"));
    assert!(result.contains("SECRET_KEY"));
}

#[test]
fn passthrough_lines_range() {
    let result = extract_lines(TOML_FIXTURE, "2-4", false).unwrap();
    assert!(result.contains("name = \"example\""));
    assert!(result.contains("version = \"1.0.0\""));
    // Should not contain line 1
    assert!(!result.contains("[package]"));
}

#[test]
fn passthrough_stats_mode() {
    let mut opts = default_options();
    opts.stats = true;
    let result = process_path(TOML_FIXTURE, opts).unwrap();
    assert!(result.contains("Lines:"));
    assert!(result.contains("Bytes:"));
}

#[test]
fn passthrough_json_output() {
    let mut opts = default_options();
    opts.format = OutputFormat::Json;
    let result = process_path(TOML_FIXTURE, opts).unwrap();
    // Should be valid JSON
    let parsed: serde_json::Value = serde_json::from_str(&result).unwrap();
    assert!(parsed["files"][0]["path"].as_str().unwrap().contains("config.toml"));
}

#[test]
fn passthrough_symbol_expand_errors() {
    let mut opts = default_options();
    opts.symbols = vec!["some_symbol".to_string()];
    let result = process_path(TOML_FIXTURE, opts);
    assert!(result.is_err());
    let err = result.unwrap_err().to_string();
    assert!(err.contains("Symbol expansion not available"));
}

#[test]
fn passthrough_directory_includes_unsupported() {
    let mut opts = default_options();
    opts.stats = true;
    let result = process_path(FIXTURE_DIR, opts).unwrap();
    // Should include both .rs and .toml/.md/.json/.env files
    assert!(result.contains("config.toml") || result.contains("Files:"));
    // The file count should be more than just the .rs files (3 rs + 4 unsupported = 7)
}

#[test]
fn passthrough_directory_lists_unsupported_files() {
    let result = process_path(FIXTURE_DIR, default_options()).unwrap();
    // Should include unsupported files in output
    assert!(result.contains("config.toml"));
    assert!(result.contains("readme.md"));
    assert!(result.contains("settings.json"));
    assert!(result.contains("sample.env"));
}
