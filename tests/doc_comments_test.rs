use codehud::{OutputFormat, ProcessOptions, process_path};

fn opts_with_comments() -> ProcessOptions {
    ProcessOptions {
        symbols: vec![],
        pub_only: false,
        fns_only: false,
        types_only: false,
        no_tests: false,
        depth: None,
        format: OutputFormat::Json,
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
        outline: false,
        compact: false,
        minimal: false,
        yes: false,
        warn_threshold: 10_000,
        expand_symbols: vec![],
        token_budget: None,
        with_comments: true,
    }
}

#[test]
fn test_rust_doc_comments() {
    let result = process_path("tests/fixtures/doc_comments.rs", opts_with_comments()).unwrap();
    // /// line doc comments on struct
    assert!(
        result.contains("This is a documented struct"),
        "Missing Rust /// doc comment:\n{result}"
    );
    // /** block doc comment
    assert!(
        result.contains("Block doc comment"),
        "Missing Rust /** doc comment:\n{result}"
    );
    // Regular // comment should NOT appear as doc_comment
    assert!(
        !result.contains("Regular comment, not a doc comment"),
        "Regular comment leaked:\n{result}"
    );
}

#[test]
fn test_go_doc_comments() {
    let result = process_path("tests/fixtures/doc_comments.go", opts_with_comments()).unwrap();
    assert!(
        result.contains("Greet returns a greeting"),
        "Missing Go doc comment:\n{result}"
    );
}

#[test]
fn test_python_docstrings() {
    let result = process_path("tests/fixtures/doc_comments.py", opts_with_comments()).unwrap();
    assert!(
        result.contains("This function has a docstring"),
        "Missing Python docstring:\n{result}"
    );
}

#[test]
fn test_typescript_jsdoc() {
    let result = process_path("tests/fixtures/doc_comments.ts", opts_with_comments()).unwrap();
    assert!(
        result.contains("A documented interface"),
        "Missing TS JSDoc:\n{result}"
    );
    assert!(
        result.contains("JSDoc on a function"),
        "Missing TS function JSDoc:\n{result}"
    );
}

#[test]
fn test_javascript_jsdoc() {
    let result = process_path("tests/fixtures/doc_comments.js", opts_with_comments()).unwrap();
    assert!(
        result.contains("A documented function"),
        "Missing JS JSDoc:\n{result}"
    );
}

#[test]
fn test_java_javadoc() {
    let result = process_path("tests/fixtures/DocComments.java", opts_with_comments()).unwrap();
    assert!(
        result.contains("A documented class"),
        "Missing Java class JavaDoc:\n{result}"
    );
}

#[test]
fn test_csharp_xml_doc() {
    let result = process_path("tests/fixtures/DocComments.cs", opts_with_comments()).unwrap();
    assert!(
        result.contains("A documented class"),
        "Missing C# XML doc:\n{result}"
    );
}

#[test]
fn test_kotlin_kdoc() {
    let result = process_path("tests/fixtures/DocComments.kt", opts_with_comments()).unwrap();
    assert!(
        result.contains("A documented class"),
        "Missing Kotlin KDoc:\n{result}"
    );
}

#[test]
fn test_cpp_doxygen() {
    let result = process_path("tests/fixtures/doc_comments.cpp", opts_with_comments()).unwrap();
    assert!(
        result.contains("A documented function"),
        "Missing C++ Doxygen:\n{result}"
    );
}

#[test]
fn test_with_comments_false_omits_docs() {
    let mut opts = opts_with_comments();
    opts.with_comments = false;
    let result = process_path("tests/fixtures/doc_comments.rs", opts).unwrap();
    // doc_comment field should be null/absent
    assert!(
        !result.contains("This is a documented struct"),
        "Doc comment present when with_comments=false:\n{result}"
    );
}
