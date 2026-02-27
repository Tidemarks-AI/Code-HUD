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

fn write_py(content: &str) -> NamedTempFile {
    let mut f = tempfile::Builder::new().suffix(".py").tempfile().unwrap();
    f.write_all(content.as_bytes()).unwrap();
    f.flush().unwrap();
    f
}

const SAMPLE_PY: &str = r#"import os
from pathlib import Path

MY_CONST = 42
BASE_URL = "https://example.com"

def helper(x: int) -> int:
    return x * 2

def _private_helper(y: int) -> int:
    return y + 1

async def fetch_data(url: str) -> dict:
    response = await client.get(url)
    return response.json()

class UserService:
    def __init__(self, db):
        self.db = db

    def get_user(self, user_id: str):
        return self.db.get(user_id)

    def _validate(self, user):
        return user is not None

    @property
    def count(self):
        return len(self.db)

    async def async_method(self):
        await self.refresh()

class Config:
    host: str
    port: int

    def connection_string(self) -> str:
        return f"{self.host}:{self.port}"
"#;

// --- Interface mode ---

#[test]
fn python_interface_mode_basic() {
    let f = write_py(SAMPLE_PY);
    let output = process_path(f.path().to_str().unwrap(), opts()).unwrap();
    assert!(output.contains("import os"), "Missing import os");
    assert!(output.contains("from pathlib import Path"), "Missing from import");
    assert!(output.contains("MY_CONST"), "Missing MY_CONST");
    assert!(output.contains("BASE_URL"), "Missing BASE_URL");
    assert!(output.contains("def helper"), "Missing helper function");
    assert!(output.contains("def _private_helper"), "Missing _private_helper");
    assert!(output.contains("class UserService"), "Missing class UserService");
    assert!(output.contains("class Config"), "Missing class Config");
    assert!(output.contains("{ ... }"), "Missing collapsed bodies");
}

// --- Expand mode ---

#[test]
fn python_expand_function() {
    let f = write_py(SAMPLE_PY);
    let mut o = opts();
    o.symbols = vec!["helper".to_string()];
    let output = process_path(f.path().to_str().unwrap(), o).unwrap();
    assert!(output.contains("def helper"), "Missing helper");
    assert!(output.contains("x * 2"), "Missing function body");
}

#[test]
fn python_expand_class() {
    let f = write_py(SAMPLE_PY);
    let mut o = opts();
    o.symbols = vec!["UserService".to_string()];
    let output = process_path(f.path().to_str().unwrap(), o).unwrap();
    assert!(output.contains("class UserService"), "Missing class");
    assert!(output.contains("self.db"), "Missing class body");
}

// --- Class methods (visible when expanding class) ---

#[test]
fn python_class_methods_in_expand() {
    let f = write_py(SAMPLE_PY);
    let mut o = opts();
    o.symbols = vec!["UserService".to_string()];
    let output = process_path(f.path().to_str().unwrap(), o).unwrap();
    assert!(output.contains("def get_user"), "Missing public method get_user");
    assert!(output.contains("def _validate"), "Missing private method _validate");
    assert!(output.contains("def __init__"), "Missing __init__");
}

// --- Decorators on methods (visible in expand mode) ---

#[test]
fn python_decorator_on_method_in_expand() {
    let f = write_py(SAMPLE_PY);
    let mut o = opts();
    o.symbols = vec!["UserService".to_string()];
    let output = process_path(f.path().to_str().unwrap(), o).unwrap();
    assert!(output.contains("@property"), "Missing @property decorator on method");
}

// --- Import statements ---

#[test]
fn python_imports() {
    let f = write_py(SAMPLE_PY);
    let output = process_path(f.path().to_str().unwrap(), opts()).unwrap();
    assert!(output.contains("import os"), "Missing import os");
    assert!(output.contains("from pathlib import Path"), "Missing from import");
}

// --- Module-level assignments ---

#[test]
fn python_module_assignments() {
    let f = write_py(SAMPLE_PY);
    let output = process_path(f.path().to_str().unwrap(), opts()).unwrap();
    assert!(output.contains("MY_CONST"), "Missing MY_CONST");
    assert!(output.contains("42"), "Missing constant value");
    assert!(output.contains("BASE_URL"), "Missing BASE_URL");
}

// --- Filters ---
// Python visibility is based on underscore convention:
// Names starting with _ are private, everything else is public.

#[test]
fn python_pub_filter_shows_public() {
    let f = write_py(SAMPLE_PY);
    let mut o = opts();
    o.pub_only = true;
    let output = process_path(f.path().to_str().unwrap(), o).unwrap();
    assert!(output.contains("helper"), "Public function should appear with --pub");
    assert!(output.contains("UserService"), "Public class should appear with --pub");
    assert!(!output.contains("_private_helper"), "Private function should be hidden with --pub");
}

#[test]
fn python_fns_filter() {
    let f = write_py(SAMPLE_PY);
    let mut o = opts();
    o.fns_only = true;
    let output = process_path(f.path().to_str().unwrap(), o).unwrap();
    assert!(output.contains("def helper"), "Missing function");
    assert!(!output.contains("class UserService"), "Should not contain class");
    assert!(!output.contains("import os"), "Should not contain import");
    assert!(!output.contains("MY_CONST"), "Should not contain const");
}

#[test]
fn python_types_filter() {
    let f = write_py(SAMPLE_PY);
    let mut o = opts();
    o.types_only = true;
    let output = process_path(f.path().to_str().unwrap(), o).unwrap();
    assert!(output.contains("class UserService"), "Missing class");
    assert!(output.contains("Config"), "Missing Config class");
    assert!(!output.contains("def helper"), "Should not contain standalone function");
}

// --- Async functions ---

#[test]
fn python_async_function() {
    let f = write_py(SAMPLE_PY);
    let output = process_path(f.path().to_str().unwrap(), opts()).unwrap();
    assert!(output.contains("fetch_data"), "Missing async function fetch_data");
}

#[test]
fn python_async_expand() {
    let f = write_py(SAMPLE_PY);
    let mut o = opts();
    o.symbols = vec!["fetch_data".to_string()];
    let output = process_path(f.path().to_str().unwrap(), o).unwrap();
    assert!(output.contains("fetch_data"), "Missing fetch_data");
    assert!(output.contains("await"), "Missing async body");
}

// --- Nested classes ---

#[test]
fn python_nested_class_hidden() {
    let src = "class Outer:\n    class Inner:\n        def inner_method(self):\n            pass\n\n    def outer_method(self):\n        pass\n";
    let f = write_py(src);
    let output = process_path(f.path().to_str().unwrap(), opts()).unwrap();
    assert!(output.contains("class Outer"), "Missing Outer class");
    assert!(!output.contains("class Inner"), "Nested class should not appear at top level");
}

// --- Stats ---

#[test]
fn python_stats_mode() {
    let f = write_py(SAMPLE_PY);
    let mut o = opts();
    o.stats = true;
    let output = process_path(f.path().to_str().unwrap(), o).unwrap();
    assert!(output.contains("Files:"), "Missing files count");
    assert!(output.contains("Lines:"), "Missing lines count");
    assert!(output.contains("Bytes:"), "Missing bytes count");
}

// --- Combined filters ---

#[test]
fn python_fns_excludes_types_and_imports() {
    let f = write_py(SAMPLE_PY);
    let mut o = opts();
    o.fns_only = true;
    let output = process_path(f.path().to_str().unwrap(), o).unwrap();
    assert!(output.contains("def "), "Should contain functions");
    assert!(!output.contains("class "), "Should not contain classes");
    assert!(!output.contains("import"), "Should not contain imports");
}

// --- Expand multiple symbols ---

#[test]
fn python_expand_multiple_symbols() {
    let f = write_py(SAMPLE_PY);
    let mut o = opts();
    o.symbols = vec!["helper".to_string(), "Config".to_string()];
    let output = process_path(f.path().to_str().unwrap(), o).unwrap();
    assert!(output.contains("def helper"), "Missing helper");
    assert!(output.contains("x * 2"), "Missing helper body");
    assert!(output.contains("class Config"), "Missing Config");
    assert!(output.contains("connection_string"), "Missing Config body");
}

// --- Expand nonexistent symbol ---

#[test]
fn python_expand_nonexistent() {
    let f = write_py(SAMPLE_PY);
    let mut o = opts();
    o.symbols = vec!["nonexistent_symbol".to_string()];
    let result = process_path(f.path().to_str().unwrap(), o);
    assert!(result.is_err(), "Should error for nonexistent symbol");
    let err = result.unwrap_err().to_string();
    assert!(err.contains("not found"), "Error should mention 'not found': {}", err);
}
