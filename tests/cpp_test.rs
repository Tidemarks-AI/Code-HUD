use codehud::{process_path, ProcessOptions, OutputFormat};
use std::io::Write;
use tempfile::NamedTempFile;

fn opts() -> ProcessOptions {
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
        yes: false,
        warn_threshold: 10_000,
        expand_symbols: vec![],
        token_budget: None,
    }
}

fn write_cpp(content: &str) -> NamedTempFile {
    let mut f = tempfile::Builder::new().suffix(".cpp").tempfile().unwrap();
    f.write_all(content.as_bytes()).unwrap();
    f.flush().unwrap();
    f
}

fn write_hpp(content: &str) -> NamedTempFile {
    let mut f = tempfile::Builder::new().suffix(".hpp").tempfile().unwrap();
    f.write_all(content.as_bytes()).unwrap();
    f.flush().unwrap();
    f
}

fn run(f: &NamedTempFile, opts: ProcessOptions) -> String {
    let path_str = f.path().to_str().unwrap();
    process_path(path_str, opts).unwrap()
}

const SAMPLE_CPP: &str = r#"#include <iostream>
#include "myheader.h"

class MyClass {
public:
    MyClass() {}
    ~MyClass() {}
    void method() const {}
    int value;
private:
    int secret;
};

struct MyStruct {
    int x;
    double y;
};

enum Color { Red, Green, Blue };

namespace myns {
    using MyAlias = int;
} // namespace myns

template<typename T>
T add(T a, T b) {
    return a + b;
}

void free_function(int x) {
    std::cout << x;
}
"#;

#[test]
fn cpp_list_symbols() {
    let f = write_cpp(SAMPLE_CPP);
    let mut opts = opts();
    opts.list_symbols = true;
    let text = run(&f, opts);
    assert!(text.contains("MyClass"), "should find MyClass: {text}");
    assert!(text.contains("MyStruct"), "should find MyStruct: {text}");
    assert!(text.contains("Color"), "should find Color: {text}");
    assert!(text.contains("free_function"), "should find free_function: {text}");
    assert!(text.contains("myns"), "should find namespace myns: {text}");
}

#[test]
fn cpp_list_symbols_no_imports() {
    let f = write_cpp(SAMPLE_CPP);
    let mut opts = opts();
    opts.list_symbols = true;
    opts.no_imports = true;
    let text = run(&f, opts);
    assert!(!text.contains("#include"), "should hide includes: {text}");
    assert!(text.contains("MyClass"), "should still show MyClass: {text}");
}

#[test]
fn cpp_expand_class() {
    let f = write_cpp(SAMPLE_CPP);
    let mut opts = opts();
    opts.expand_symbols = vec!["MyClass".to_string()];
    let text = run(&f, opts);
    assert!(text.contains("MyClass"), "should expand MyClass: {text}");
    assert!(text.contains("method"), "should contain method: {text}");
}

#[test]
fn cpp_fns_filter() {
    let f = write_cpp(SAMPLE_CPP);
    let mut opts = opts();
    opts.list_symbols = true;
    opts.fns_only = true;
    let text = run(&f, opts);
    assert!(text.contains("free_function") || text.contains("add"), "should show functions: {text}");
    assert!(!text.contains("MyStruct"), "should hide structs: {text}");
}

#[test]
fn cpp_types_filter() {
    let f = write_cpp(SAMPLE_CPP);
    let mut opts = opts();
    opts.list_symbols = true;
    opts.types_only = true;
    let text = run(&f, opts);
    assert!(text.contains("MyClass") || text.contains("MyStruct") || text.contains("Color"),
        "should show types: {text}");
}

#[test]
fn cpp_hpp_detection() {
    let f = write_hpp("class Foo { public:\n    void bar() {} };");
    let mut opts = opts();
    opts.list_symbols = true;
    let text = run(&f, opts);
    assert!(text.contains("Foo"), "should detect .hpp as C++: {text}");
}

#[test]
fn cpp_json_output() {
    let f = write_cpp(SAMPLE_CPP);
    let mut opts = opts();
    opts.list_symbols = true;
    opts.format = OutputFormat::Json;
    let text = run(&f, opts);
    let parsed: serde_json::Value = serde_json::from_str(&text).unwrap();
    assert!(parsed.is_array() || parsed.is_object(), "should produce valid JSON: {text}");
}

#[test]
fn cpp_depth2_shows_members() {
    let f = write_cpp(SAMPLE_CPP);
    let mut opts = opts();
    opts.list_symbols = true;
    opts.symbol_depth = Some(2);
    let text = run(&f, opts);
    assert!(text.contains("method"), "depth 2 should show class methods: {text}");
}

#[test]
fn cpp_template_function() {
    let source = r#"
template<typename T>
T maximum(T a, T b) {
    return a > b ? a : b;
}
"#;
    let f = write_cpp(source);
    let mut opts = opts();
    opts.list_symbols = true;
    let text = run(&f, opts);
    assert!(text.contains("maximum"), "should detect template function: {text}");
}

#[test]
fn cpp_enum_class() {
    let source = r#"
enum class Direction { North, South, East, West };
"#;
    let f = write_cpp(source);
    let mut opts = opts();
    opts.list_symbols = true;
    let text = run(&f, opts);
    assert!(text.contains("Direction"), "should detect enum class: {text}");
}

#[test]
fn cpp_stats_mode() {
    let f = write_cpp(SAMPLE_CPP);
    let mut opts = opts();
    opts.stats = true;
    let text = run(&f, opts);
    assert!(text.contains("C++"), "stats should show C++ language: {text}");
}
