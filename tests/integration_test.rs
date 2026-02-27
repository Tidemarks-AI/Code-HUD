use codehud::{process_path, ProcessOptions, OutputFormat};

const FIXTURE_PATH: &str = "tests/fixtures/sample.rs";
const FIXTURE_DIR: &str = "tests/fixtures";

#[test]
fn test_interface_mode_basic() {
    let options = ProcessOptions {
        symbols: vec![],
        pub_only: false,
        fns_only: false,
        types_only: false, no_tests: false,
        depth: None,
        format: OutputFormat::Plain, stats: false, stats_detailed: true,
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
    
};
    
    let result = process_path(FIXTURE_PATH, options);
    assert!(result.is_ok(), "process_path failed: {:?}", result.err());
    let output = result.unwrap();
    
    // Should contain struct signature
    assert!(output.contains("pub struct User"), "Missing User struct");
    // Should contain enum
    assert!(output.contains("pub enum Role"), "Missing Role enum");
    // Should contain trait
    assert!(output.contains("pub trait Authenticatable"), "Missing Authenticatable trait");
    // Should have collapsed bodies (functions shown as { ... } in interface mode)
    assert!(output.contains("{ ... }"), "Missing collapsed function bodies");
}

#[test]
fn test_expand_mode() {
    let options = ProcessOptions {
        symbols: vec!["User".to_string()],
        pub_only: false,
        fns_only: false,
        types_only: false, no_tests: false,
        depth: None,
        format: OutputFormat::Plain, stats: false, stats_detailed: true,
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
    
};
    
    let result = process_path(FIXTURE_PATH, options);
    assert!(result.is_ok(), "process_path failed: {:?}", result.err());
    let output = result.unwrap();
    
    // Should contain full User struct definition
    assert!(output.contains("pub struct User"), "Missing User struct");
    assert!(output.contains("pub name: String"), "Missing name field");
    assert!(output.contains("pub age: u32"), "Missing age field");
    assert!(output.contains("email: String"), "Missing email field");
}

#[test]
fn test_expand_function() {
    let options = ProcessOptions {
        symbols: vec!["public_utility".to_string()],
        pub_only: false,
        fns_only: false,
        types_only: false, no_tests: false,
        depth: None,
        format: OutputFormat::Plain, stats: false, stats_detailed: true,
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
    
};
    
    let result = process_path(FIXTURE_PATH, options);
    assert!(result.is_ok(), "process_path failed: {:?}", result.err());
    let output = result.unwrap();
    
    // Should contain full function body
    assert!(output.contains("pub fn public_utility"), "Missing function signature");
    assert!(output.contains("to_uppercase()"), "Missing function body");
}

#[test]
fn test_pub_filter() {
    let options = ProcessOptions {
        symbols: vec![],
        pub_only: true,
        fns_only: false,
        types_only: false, no_tests: false,
        depth: None,
        format: OutputFormat::Plain, stats: false, stats_detailed: true,
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
    
};
    
    let result = process_path(FIXTURE_PATH, options);
    assert!(result.is_ok(), "process_path failed: {:?}", result.err());
    let output = result.unwrap();
    
    // Should contain public items
    assert!(output.contains("pub struct User"), "Missing public struct");
    
    // Should NOT contain private items
    assert!(!output.contains("private_helper"), "Should not contain private_helper");
    assert!(!output.contains("validate_email"), "Should not contain private validate_email method");
}

#[test]
fn test_fns_filter() {
    let options = ProcessOptions {
        symbols: vec![],
        pub_only: false,
        fns_only: true,
        types_only: false, no_tests: false,
        depth: None,
        format: OutputFormat::Plain, stats: false, stats_detailed: true,
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
    
};
    
    let result = process_path(FIXTURE_PATH, options);
    assert!(result.is_ok(), "process_path failed: {:?}", result.err());
    let output = result.unwrap();
    
    // Should contain collapsed function bodies
    assert!(output.contains("{ ... }"), "Missing collapsed function bodies");
    
    // Should NOT contain struct/enum definitions
    assert!(!output.contains("pub struct User {"), "Should not contain struct definition");
    assert!(!output.contains("pub enum Role {"), "Should not contain enum definition");
    assert!(!output.contains("pub trait Authenticatable {"), "Should not contain trait definition");
}

#[test]
fn test_types_filter() {
    let options = ProcessOptions {
        symbols: vec![],
        pub_only: false,
        fns_only: false,
        types_only: true, no_tests: false,
        depth: None,
        format: OutputFormat::Plain, stats: false, stats_detailed: true,
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
    
};
    
    let result = process_path(FIXTURE_PATH, options);
    assert!(result.is_ok(), "process_path failed: {:?}", result.err());
    let output = result.unwrap();
    
    // Should contain type definitions
    assert!(output.contains("pub struct User"), "Missing User struct");
    assert!(output.contains("pub enum Role"), "Missing Role enum");
    assert!(output.contains("pub trait Authenticatable"), "Missing Authenticatable trait");
    assert!(output.contains("pub type UserMap"), "Missing UserMap type alias");
    
    // Should NOT contain standalone functions
    assert!(!output.contains("fn private_helper"), "Should not contain private_helper");
}

#[test]
fn test_combined_pub_fns() {
    let options = ProcessOptions {
        symbols: vec![],
        pub_only: true,
        fns_only: true,
        types_only: false, no_tests: false,
        depth: None,
        format: OutputFormat::Plain, stats: false, stats_detailed: true,
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
    
};
    
    let result = process_path(FIXTURE_PATH, options);
    assert!(result.is_ok(), "process_path failed: {:?}", result.err());
    let output = result.unwrap();
    
    // Should contain collapsed function bodies (public methods/functions)
    assert!(output.contains("{ ... }"), "Missing collapsed function bodies");
    
    // Should NOT contain private functions
    assert!(!output.contains("private_helper"), "Should not contain private_helper");
    assert!(!output.contains("validate_email"), "Should not contain private validate_email");
    
    // Should NOT contain type definitions
    assert!(!output.contains("pub struct User {"), "Should not contain struct definition");
    assert!(!output.contains("pub enum Role {"), "Should not contain enum definition");
}

#[test]
fn test_json_output() {
    let options = ProcessOptions {
        symbols: vec![],
        pub_only: false,
        fns_only: false,
        types_only: false, no_tests: false,
        depth: None,
        format: OutputFormat::Json, stats: false, stats_detailed: true,
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
    
};
    
    let result = process_path(FIXTURE_PATH, options);
    assert!(result.is_ok(), "process_path failed: {:?}", result.err());
    let output = result.unwrap();
    
    // Should be valid JSON
    let parsed: serde_json::Value = serde_json::from_str(&output)
        .expect("Output should be valid JSON");
    
    // Should have files array
    assert!(parsed.get("files").is_some(), "Missing files array");
    let files = parsed["files"].as_array().expect("files should be an array");
    assert!(!files.is_empty(), "files array should not be empty");
    
    // First file should have items
    assert!(files[0].get("items").is_some(), "Missing items in first file");
}

#[test]
fn test_nonexistent_path() {
    let options = ProcessOptions {
        symbols: vec![],
        pub_only: false,
        fns_only: false,
        types_only: false, no_tests: false,
        depth: None,
        format: OutputFormat::Plain, stats: false, stats_detailed: true,
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
    
};
    
    let result = process_path("nonexistent/path/file.rs", options);
    
    // Should return an error
    assert!(result.is_err(), "Should return error for nonexistent path");
}

#[test]
fn test_directory_mode() {
    let options = ProcessOptions {
        symbols: vec![],
        pub_only: false,
        fns_only: false,
        types_only: false, no_tests: false,
        depth: Some(1),
        format: OutputFormat::Plain, stats: false, stats_detailed: true,
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
    
};
    
    let result = process_path(FIXTURE_DIR, options);
    assert!(result.is_ok(), "process_path failed: {:?}", result.err());
    let output = result.unwrap();
    
    // Should find and process sample.rs
    assert!(output.contains("sample.rs") || output.contains("User"), 
            "Should process sample.rs from directory");
}

#[test]
fn test_expand_nonexistent_symbol() {
    let options = ProcessOptions {
        symbols: vec!["NonexistentSymbol".to_string()],
        pub_only: false,
        fns_only: false,
        types_only: false, no_tests: false,
        depth: None,
        format: OutputFormat::Plain, stats: false, stats_detailed: true,
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
    
};
    
    let result = process_path(FIXTURE_PATH, options);
    
    // Should fail with an error for nonexistent symbol
    assert!(result.is_err(), "process_path should fail for nonexistent symbol");
    let err = result.unwrap_err().to_string();
    assert!(err.contains("not found"), "Error should mention 'not found': {}", err);
    assert!(err.contains("NonexistentSymbol"), "Error should mention the symbol: {}", err);
}

#[test]
fn test_no_tests_filter() {
    let options = ProcessOptions {
        symbols: vec![],
        pub_only: false,
        fns_only: false,
        types_only: false,
        no_tests: true,
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
    
};

    let result = process_path(FIXTURE_PATH, options);
    assert!(result.is_ok(), "process_path failed: {:?}", result.err());
    let output = result.unwrap();

    // Should still contain normal items
    assert!(output.contains("pub struct User"), "Missing User struct");
    assert!(output.contains("pub fn public_utility"), "Missing public_utility");

    // Should NOT contain the test module
    assert!(!output.contains("mod tests"), "Should filter out mod tests");
    assert!(!output.contains("test_user_creation"), "Should filter out test functions");
}

#[test]
fn test_no_tests_filter_disabled() {
    // With no_tests: false, the test module should appear
    let options = ProcessOptions {
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
    
};

    let result = process_path(FIXTURE_PATH, options);
    assert!(result.is_ok(), "process_path failed: {:?}", result.err());
    let output = result.unwrap();

    // Should contain the test module
    assert!(output.contains("mod tests"), "Should include mod tests when no_tests is false");
}

#[test]
fn test_stats_output_plain() {
    let options = ProcessOptions {
        symbols: vec![],
        pub_only: false,
        fns_only: false,
        types_only: false,
        no_tests: false,
        depth: None,
        format: OutputFormat::Plain,
        stats: true, stats_detailed: true,
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
    
};

    let result = process_path(FIXTURE_PATH, options);
    assert!(result.is_ok(), "process_path failed: {:?}", result.err());
    let output = result.unwrap();

    // Stats output should contain file count, line/byte info, and token estimate
    assert!(output.contains("Files:") && output.contains("Lines:") && output.contains("Bytes:") && output.contains("Tokens:"),
            "Stats should contain summary counts including tokens. Got: {}", output);
}

#[test]
fn test_stats_output_json() {
    let options = ProcessOptions {
        symbols: vec![],
        pub_only: false,
        fns_only: false,
        types_only: false,
        no_tests: false,
        depth: None,
        format: OutputFormat::Json,
        stats: true, stats_detailed: true,
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
    
};

    let result = process_path(FIXTURE_PATH, options);
    assert!(result.is_ok(), "process_path failed: {:?}", result.err());
    let output = result.unwrap();

    // Should be valid JSON
    let parsed: serde_json::Value = serde_json::from_str(&output)
        .expect("Stats JSON output should be valid JSON");

    // Should have some structure with file info
    assert!(parsed.is_object() || parsed.is_array(),
            "Stats JSON should be an object or array");
}

#[test]
fn test_stats_with_directory() {
    let options = ProcessOptions {
        symbols: vec![],
        pub_only: false,
        fns_only: false,
        types_only: false,
        no_tests: false,
        depth: None,
        format: OutputFormat::Plain,
        stats: true, stats_detailed: true,
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
    
};

    let result = process_path(FIXTURE_DIR, options);
    assert!(result.is_ok(), "process_path failed: {:?}", result.err());
    let output = result.unwrap();

    // Directory stats should show totals for multiple files
    assert!(!output.is_empty(), "Stats for directory should not be empty");
}

#[test]
fn test_stats_summary_only_plain() {
    let options = ProcessOptions {
        symbols: vec![],
        pub_only: false,
        fns_only: false,
        types_only: false,
        no_tests: false,
        depth: None,
        format: OutputFormat::Plain,
        stats: true,
        stats_detailed: false,
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
    };

    let result = process_path(FIXTURE_DIR, options);
    assert!(result.is_ok(), "process_path failed: {:?}", result.err());
    let output = result.unwrap();

    // Should have the summary line
    assert!(output.contains("Files:"), "Should contain summary. Got: {}", output);
    // Should NOT have per-file breakdown (lines with " — ")
    assert!(!output.contains(" — "), "summary-only should skip per-file lines");
}

#[test]
fn test_stats_summary_only_json() {
    let options = ProcessOptions {
        symbols: vec![],
        pub_only: false,
        fns_only: false,
        types_only: false,
        no_tests: false,
        depth: None,
        format: OutputFormat::Json,
        stats: true,
        stats_detailed: false,
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
    };

    let result = process_path(FIXTURE_DIR, options);
    assert!(result.is_ok(), "process_path failed: {:?}", result.err());
    let output = result.unwrap();

    let parsed: serde_json::Value = serde_json::from_str(&output)
        .expect("Should be valid JSON");
    // per_file should be empty
    let per_file = parsed.get("per_file").expect("should have per_file key");
    assert!(per_file.as_array().unwrap().is_empty(), "summary-only JSON should have empty per_file");
    // but files count should be > 0
    let files = parsed.get("files").unwrap().as_u64().unwrap();
    assert!(files > 0, "should still report file count");
}

#[test]
fn test_stats_summary_shows_languages() {
    let options = ProcessOptions {
        symbols: vec![],
        pub_only: false,
        fns_only: false,
        types_only: false,
        no_tests: false,
        depth: None,
        format: OutputFormat::Plain,
        stats: true,
        stats_detailed: false,
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
    };

    let result = process_path(FIXTURE_DIR, options);
    assert!(result.is_ok(), "process_path failed: {:?}", result.err());
    let output = result.unwrap();

    // Summary should include Languages line
    assert!(output.contains("Languages:"), "Summary should show language breakdown. Got: {}", output);
    // Should NOT show per-file details
    assert!(!output.contains(" — "), "Summary mode should not list individual files");
}

#[test]
fn test_stats_detailed_shows_per_file() {
    let options = ProcessOptions {
        symbols: vec![],
        pub_only: false,
        fns_only: false,
        types_only: false,
        no_tests: false,
        depth: None,
        format: OutputFormat::Plain,
        stats: true,
        stats_detailed: true,
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
    };

    let result = process_path(FIXTURE_DIR, options);
    assert!(result.is_ok(), "process_path failed: {:?}", result.err());
    let output = result.unwrap();

    // Detailed mode SHOULD show per-file breakdown
    assert!(output.contains(" — "), "Detailed mode should list individual files. Got: {}", output);
}

#[test]
fn test_stats_summary_shows_dirs() {
    let options = ProcessOptions {
        symbols: vec![],
        pub_only: false,
        fns_only: false,
        types_only: false,
        no_tests: false,
        depth: None,
        format: OutputFormat::Plain,
        stats: true,
        stats_detailed: false,
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
    };

    let result = process_path(FIXTURE_DIR, options);
    assert!(result.is_ok(), "process_path failed: {:?}", result.err());
    let output = result.unwrap();

    // Summary should include Dirs count
    assert!(output.contains("Dirs:"), "Summary should show directory count. Got: {}", output);
}
