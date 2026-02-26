use codehud::{process_path, ProcessOptions, OutputFormat};

const FIXTURE_PATH: &str = "tests/fixtures/sample.rs";

fn default_options() -> ProcessOptions {
    ProcessOptions {
        symbols: vec![],
        pub_only: false,
        fns_only: false,
        types_only: false,
        no_tests: false,
        depth: None,
        format: OutputFormat::Plain,
        stats: false,
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
        summary_only: false,
    }
}

#[test]
fn test_symbol_not_found_returns_error() {
    let options = ProcessOptions {
        symbols: vec!["nonExistentSymbol".to_string()],
        ..default_options()
    };
    let result = process_path(FIXTURE_PATH, options);
    assert!(result.is_err(), "Expected error for non-existent symbol");
    let err = result.unwrap_err().to_string();
    assert!(err.contains("nonExistentSymbol"), "Error should mention the symbol name: {}", err);
    assert!(err.contains("not found"), "Error should say 'not found': {}", err);
}

#[test]
fn test_symbol_not_found_json_mode() {
    let options = ProcessOptions {
        symbols: vec!["doesNotExist".to_string()],
        format: OutputFormat::Json,
        ..default_options()
    };
    let result = process_path(FIXTURE_PATH, options);
    assert!(result.is_err());
}

#[test]
fn test_existing_symbol_still_works() {
    let options = ProcessOptions {
        symbols: vec!["User".to_string()],
        ..default_options()
    };
    let result = process_path(FIXTURE_PATH, options);
    assert!(result.is_ok(), "Existing symbol should work: {:?}", result.err());
    let output = result.unwrap();
    assert!(output.contains("User"), "Output should contain the symbol");
}
