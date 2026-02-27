use codehud::{process_path, ProcessOptions, OutputFormat};

fn outline_opts() -> ProcessOptions {
    ProcessOptions {
        symbols: vec![],
        pub_only: false,
        fns_only: false,
        types_only: false,
        no_tests: false,
        depth: None,
        format: OutputFormat::Plain,
        stats: false,
        stats_detailed: true,
        ext: vec![],
        signatures: false,
        max_lines: None,
        list_symbols: false,
        no_imports: false,
        smart_depth: false,
        symbol_depth: None,
        exclude: vec![],
        outline: true,
        compact: false,
    }
}

// ── TypeScript: Arrow function exports ──

#[test]
fn ts_arrow_export_shows_signature_not_body() {
    let output = process_path("tests/fixtures/edge_cases.ts", outline_opts()).unwrap();
    // Should show the arrow function signature, NOT the body
    assert!(
        !output.contains("await fetch"),
        "arrow function body should not appear in outline:\n{output}"
    );
    assert!(
        output.contains("fetchUser"),
        "arrow function name should appear in outline:\n{output}"
    );
}

#[test]
fn ts_arrow_export_shows_param_types() {
    let output = process_path("tests/fixtures/edge_cases.ts", outline_opts()).unwrap();
    // The signature should contain parameter info
    assert!(
        output.contains("id: string"),
        "arrow function params should be visible:\n{output}"
    );
}

#[test]
fn ts_simple_arrow_export_no_body() {
    let output = process_path("tests/fixtures/edge_cases.ts", outline_opts()).unwrap();
    assert!(
        output.contains("add"),
        "simple arrow export should appear:\n{output}"
    );
    assert!(
        !output.contains("a + b"),
        "expression body of arrow should not appear:\n{output}"
    );
}

// ── TypeScript: Overloaded functions ──

#[test]
fn ts_overloaded_function_signatures_shown() {
    let output = process_path("tests/fixtures/edge_cases.ts", outline_opts()).unwrap();
    // Overload signatures should appear
    assert!(
        output.contains("function process(input: string): string"),
        "first overload signature should appear:\n{output}"
    );
    assert!(
        output.contains("function process(input: number): number"),
        "second overload signature should appear:\n{output}"
    );
}

#[test]
fn ts_exported_overloaded_function_shown() {
    let output = process_path("tests/fixtures/edge_cases.ts", outline_opts()).unwrap();
    assert!(
        output.contains("function format(value: string): string"),
        "exported overload signature should appear:\n{output}"
    );
}

// ── Rust: Inline mod blocks ──

#[test]
fn rust_inline_mod_shown_in_outline() {
    let output = process_path("tests/fixtures/edge_cases.rs", outline_opts()).unwrap();
    assert!(
        output.contains("mod helpers"),
        "inline mod block should appear in outline:\n{output}"
    );
}

#[test]
fn rust_inline_mod_shows_member_signatures() {
    let output = process_path("tests/fixtures/edge_cases.rs", outline_opts()).unwrap();
    assert!(
        output.contains("trim_string"),
        "mod members should show signatures:\n{output}"
    );
}

#[test]
fn rust_external_mod_shown() {
    let output = process_path("tests/fixtures/edge_cases.rs", outline_opts()).unwrap();
    assert!(
        output.contains("mod external"),
        "external mod declaration should appear:\n{output}"
    );
}

#[test]
fn rust_private_inline_mod_shown() {
    let output = process_path("tests/fixtures/edge_cases.rs", outline_opts()).unwrap();
    assert!(
        output.contains("mod private_mod"),
        "private inline mod should appear:\n{output}"
    );
}

#[test]
fn rust_inline_mod_no_body_leak() {
    let output = process_path("tests/fixtures/edge_cases.rs", outline_opts()).unwrap();
    // Should not contain function bodies
    assert!(
        !output.contains("s.trim().to_string()"),
        "function bodies inside mod should not appear:\n{output}"
    );
}

// ── Go: Basic outline support ──

#[test]
fn go_struct_in_outline() {
    let output = process_path("tests/fixtures/edge_cases.go", outline_opts()).unwrap();
    assert!(
        output.contains("User"),
        "Go struct should appear in outline:\n{output}"
    );
}

#[test]
fn go_interface_in_outline() {
    let output = process_path("tests/fixtures/edge_cases.go", outline_opts()).unwrap();
    assert!(
        output.contains("Stringer"),
        "Go interface should appear in outline:\n{output}"
    );
}

#[test]
fn go_function_in_outline() {
    let output = process_path("tests/fixtures/edge_cases.go", outline_opts()).unwrap();
    assert!(
        output.contains("NewUser"),
        "Go function should appear in outline:\n{output}"
    );
}

#[test]
fn go_method_in_outline() {
    let output = process_path("tests/fixtures/edge_cases.go", outline_opts()).unwrap();
    assert!(
        output.contains("String"),
        "Go method should appear in outline:\n{output}"
    );
}

#[test]
fn go_function_no_body() {
    let output = process_path("tests/fixtures/edge_cases.go", outline_opts()).unwrap();
    assert!(
        !output.contains("Sprintf"),
        "Go function body should not appear in outline:\n{output}"
    );
}

#[test]
fn go_const_in_outline() {
    let output = process_path("tests/fixtures/edge_cases.go", outline_opts()).unwrap();
    assert!(
        output.contains("MaxRetries"),
        "Go const should appear in outline:\n{output}"
    );
}

#[test]
fn go_pub_filter_uppercase() {
    let mut opts = outline_opts();
    opts.pub_only = true;
    let output = process_path("tests/fixtures/edge_cases.go", opts).unwrap();
    assert!(
        output.contains("NewUser"),
        "Exported Go function should appear with pub_only:\n{output}"
    );
    assert!(
        !output.contains("helperFunc"),
        "Unexported Go function should not appear with pub_only:\n{output}"
    );
}

#[test]
fn go_import_in_outline() {
    let output = process_path("tests/fixtures/edge_cases.go", outline_opts()).unwrap();
    assert!(
        output.contains("import"),
        "Go import should appear in outline:\n{output}"
    );
}
