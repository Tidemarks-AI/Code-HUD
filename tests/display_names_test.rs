use codehud::{process_path, ProcessOptions, OutputFormat};

fn list_options() -> ProcessOptions {
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
        list_symbols: true,
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
fn ts_list_symbols_shows_interface_not_trait() {
    let output = process_path("tests/fixtures/sample.ts", list_options()).unwrap();
    // TS interfaces should display as "interface", not "trait"
    assert!(output.contains("interface"), "Expected 'interface' in TS output, got:\n{}", output);
    assert!(!output.contains(" trait "), "Should not contain 'trait' for TS files, got:\n{}", output);
}

#[test]
fn ts_list_symbols_shows_import_not_use() {
    let output = process_path("tests/fixtures/sample.ts", list_options()).unwrap();
    assert!(output.contains("import"), "Expected 'import' in TS output, got:\n{}", output);
    // Should not have bare "use " as a kind label
    let has_use_label = output.lines().any(|l| l.trim_start().starts_with("use "));
    assert!(!has_use_label, "Should not show 'use' kind for TS files, got:\n{}", output);
}

#[test]
fn ts_list_symbols_shows_type_alias() {
    let output = process_path("tests/fixtures/sample.ts", list_options()).unwrap();
    assert!(output.contains("type alias"), "Expected 'type alias' in TS output, got:\n{}", output);
}

#[test]
fn ts_json_output_uses_ts_display_names() {
    let options = ProcessOptions {
        format: OutputFormat::Json,
        list_symbols: false,
        ..list_options()
    };
    let output = process_path("tests/fixtures/sample.ts", options).unwrap();
    assert!(output.contains("\"interface\""), "Expected 'interface' kind in JSON, got:\n{}", output);
    assert!(output.contains("\"import\""), "Expected 'import' kind in JSON, got:\n{}", output);
    assert!(output.contains("\"type alias\""), "Expected 'type alias' kind in JSON, got:\n{}", output);
}

#[test]
fn rust_list_symbols_still_shows_trait() {
    let output = process_path("tests/fixtures/sample.rs", list_options()).unwrap();
    // Rust files should still use Rust terminology
    assert!(output.contains("trait "), "Rust files should still show 'trait', got:\n{}", output);
}
