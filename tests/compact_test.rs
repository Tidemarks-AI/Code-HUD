use codehud::{process_path, ProcessOptions, OutputFormat};

fn outline_options(compact: bool) -> ProcessOptions {
    ProcessOptions {
        symbols: vec![],
        pub_only: false,
        fns_only: false,
        types_only: false,
        no_tests: false,
        depth: None,
        format: OutputFormat::Plain,
        stats: false,
        summary_only: false,
        ext: vec![],
        signatures: false,
        max_lines: None,
        list_symbols: false,
        no_imports: false,
        smart_depth: false,
        symbol_depth: None,
        exclude: vec![],
        outline: true,
        compact,
    }
}

#[test]
fn compact_outline_collapses_params_rust() {
    let output = process_path("tests/fixtures/sample.rs", outline_options(true)).unwrap();
    // Compact mode should show fn new(…) not fn new(name: String, age: u32, email: String)
    assert!(output.contains("(…)"), "compact should collapse params to …: {}", output);
    assert!(!output.contains("name: String"), "compact should not show param details: {}", output);
}

#[test]
fn normal_outline_shows_full_params_rust() {
    let output = process_path("tests/fixtures/sample.rs", outline_options(false)).unwrap();
    // Normal mode should show full params
    assert!(output.contains("name: String"), "normal outline should show param details: {}", output);
}

#[test]
fn compact_outline_strips_docstrings() {
    let output = process_path("tests/fixtures/sample.rs", outline_options(true)).unwrap();
    // Docstrings like "/// A sample struct" should be stripped
    assert!(!output.contains("/// A sample struct"), "compact should strip docstrings: {}", output);
}

#[test]
fn compact_outline_ts() {
    let output = process_path("tests/fixtures/sample.ts", outline_options(true)).unwrap();
    assert!(output.contains("(…)") || output.contains("()"), "compact should work for TS: {}", output);
}
