use codehud::{OutputFormat, ProcessOptions, process_path};

fn outline_expand_options(expand: Vec<&str>) -> ProcessOptions {
    ProcessOptions {
        symbols: vec![],
        pub_only: false,
        fns_only: false,
        types_only: false,
        no_tests: false,
        depth: None,
        format: OutputFormat::Plain,
        stats: false,
        stats_detailed: false,
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
        minimal: false,
        yes: false,
        warn_threshold: 10_000,
        expand_symbols: expand.into_iter().map(String::from).collect(),
        token_budget: None,
        with_comments: false,
    }
}

#[test]
fn outline_expand_shows_full_body_for_named_symbol() {
    let output = process_path(
        "tests/fixtures/sample.rs",
        outline_expand_options(vec!["greeting"]),
    )
    .unwrap();
    // The expanded symbol should contain its body (implementation)
    assert!(
        output.contains("format!"),
        "expanded symbol 'greet' should show its body with format!: {}",
        output
    );
}

#[test]
fn outline_expand_keeps_other_symbols_as_signatures() {
    let output = process_path(
        "tests/fixtures/sample.rs",
        outline_expand_options(vec!["greeting"]),
    )
    .unwrap();
    // 'new' should still be in outline (signature-only) form — no body
    // The outline for 'new' in an impl block shows signature only
    assert!(
        output.contains("fn new"),
        "outline should still contain 'new' signature: {}",
        output
    );
}

#[test]
fn outline_expand_no_expand_is_normal_outline() {
    let normal = process_path("tests/fixtures/sample.rs", outline_expand_options(vec![])).unwrap();
    let outline_only = process_path(
        "tests/fixtures/sample.rs",
        ProcessOptions {
            outline: true,
            compact: false,
            minimal: false,
            yes: false,
            warn_threshold: 10_000,
            expand_symbols: vec![],
            token_budget: None,
            with_comments: false,
            symbols: vec![],
            pub_only: false,
            fns_only: false,
            types_only: false,
            no_tests: false,
            depth: None,
            format: OutputFormat::Plain,
            stats: false,
            stats_detailed: false,
            ext: vec![],
            signatures: false,
            max_lines: None,
            list_symbols: false,
            no_imports: false,
            smart_depth: false,
            symbol_depth: None,
            exclude: vec![],
        },
    )
    .unwrap();
    assert_eq!(
        normal, outline_only,
        "empty expand should produce same output as plain outline"
    );
}

#[test]
fn outline_expand_top_level_function() {
    let output = process_path(
        "tests/fixtures/sample.rs",
        outline_expand_options(vec!["public_utility"]),
    )
    .unwrap();
    // Top-level function should be fully expanded
    assert!(
        output.contains("to_uppercase"),
        "expanded top-level fn should show body: {}",
        output
    );
    // Other functions should remain as signatures
    assert!(
        !output.contains("true"),
        "private_helper should stay as signature (no body 'true'): {}",
        output
    );
}

#[test]
fn outline_expand_multiple_symbols() {
    let output = process_path(
        "tests/fixtures/sample.rs",
        outline_expand_options(vec!["greeting", "new"]),
    )
    .unwrap();
    // Both should be expanded with their bodies
    assert!(
        output.contains("format!"),
        "greet should be expanded: {}",
        output
    );
}
