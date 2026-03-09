use codehud::{OutputFormat, ProcessOptions, process_path};
use std::io::Write;
use tempfile::NamedTempFile;

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
        minimal: false,
        expand_symbols: vec![],
        yes: false,
        warn_threshold: 10_000,
        token_budget: None,
        with_comments: false,
    }
}

fn write_py(content: &str) -> NamedTempFile {
    let mut f = tempfile::Builder::new().suffix(".py").tempfile().unwrap();
    f.write_all(content.as_bytes()).unwrap();
    f.flush().unwrap();
    f
}

fn write_rs(content: &str) -> NamedTempFile {
    let mut f = tempfile::Builder::new().suffix(".rs").tempfile().unwrap();
    f.write_all(content.as_bytes()).unwrap();
    f.flush().unwrap();
    f
}

fn write_ts(content: &str) -> NamedTempFile {
    let mut f = tempfile::Builder::new().suffix(".ts").tempfile().unwrap();
    f.write_all(content.as_bytes()).unwrap();
    f.flush().unwrap();
    f
}

// ============================================================
// Python outline mode
// ============================================================

const PYTHON_SAMPLE: &str = r#"import os
from pathlib import Path

MY_CONST = 42

def helper(x: int) -> int:
    """Helper function docstring."""
    return x * 2

def _private_fn(y: int) -> int:
    return y + 1

async def fetch_data(url: str) -> dict:
    response = await client.get(url)
    return response.json()

class UserService:
    """A service for managing users."""

    def __init__(self, db):
        self.db = db

    def get_user(self, user_id: str):
        return self.db.get(user_id)

    def _validate(self, user):
        return user is not None

class Config:
    host: str
    port: int

    def connection_string(self) -> str:
        return f"{self.host}:{self.port}"
"#;

#[test]
fn python_outline_shows_signatures() {
    let f = write_py(PYTHON_SAMPLE);
    let output = process_path(f.path().to_str().unwrap(), outline_opts()).unwrap();
    assert!(
        output.contains("def helper"),
        "Should show helper signature"
    );
    assert!(
        output.contains("def _private_fn"),
        "Should show private fn signature"
    );
    assert!(
        output.contains("async def fetch_data"),
        "Should show async fn"
    );
    assert!(output.contains("class UserService"), "Should show class");
    assert!(output.contains("class Config"), "Should show Config class");
}

#[test]
fn python_outline_omits_bodies() {
    let f = write_py(PYTHON_SAMPLE);
    let output = process_path(f.path().to_str().unwrap(), outline_opts()).unwrap();
    // Function bodies should not appear
    assert!(
        !output.contains("x * 2"),
        "Should omit function body: {}",
        output
    );
    assert!(
        !output.contains("y + 1"),
        "Should omit private fn body: {}",
        output
    );
    assert!(
        !output.contains("await client"),
        "Should omit async body: {}",
        output
    );
}

#[test]
fn python_outline_shows_imports() {
    let f = write_py(PYTHON_SAMPLE);
    let output = process_path(f.path().to_str().unwrap(), outline_opts()).unwrap();
    assert!(output.contains("import os"), "Should show import");
    assert!(
        output.contains("from pathlib import Path"),
        "Should show from import"
    );
}

#[test]
fn python_outline_shows_constants() {
    let f = write_py(PYTHON_SAMPLE);
    let output = process_path(f.path().to_str().unwrap(), outline_opts()).unwrap();
    assert!(
        output.contains("MY_CONST"),
        "Should show module-level constant"
    );
}

#[test]
fn python_outline_class_members() {
    let f = write_py(PYTHON_SAMPLE);
    let output = process_path(f.path().to_str().unwrap(), outline_opts()).unwrap();
    // Class should show member signatures
    assert!(
        output.contains("def get_user"),
        "Should show class method signature"
    );
    assert!(
        output.contains("def __init__"),
        "Should show __init__ signature"
    );
    assert!(
        output.contains("def connection_string"),
        "Should show Config method"
    );
}

// ============================================================
// Rust outline mode
// ============================================================

#[test]
fn rust_outline_shows_signatures() {
    let output = process_path("tests/fixtures/sample.rs", outline_opts()).unwrap();
    assert!(output.contains("pub struct User"), "Should show struct");
    assert!(
        output.contains("pub fn new"),
        "Should show fn signature in impl"
    );
    assert!(
        output.contains("pub fn greeting"),
        "Should show method signature"
    );
    assert!(output.contains("pub enum Role"), "Should show enum");
    assert!(
        output.contains("pub trait Authenticatable"),
        "Should show trait"
    );
    assert!(
        output.contains("pub fn public_utility"),
        "Should show free fn"
    );
}

#[test]
fn rust_outline_omits_bodies() {
    let output = process_path("tests/fixtures/sample.rs", outline_opts()).unwrap();
    assert!(
        !output.contains("format!(\"Hello, {}!\""),
        "Should omit fn body: {}",
        output
    );
    assert!(
        !output.contains("self.email.contains"),
        "Should omit validate body: {}",
        output
    );
}

#[test]
fn rust_outline_shows_types_and_consts() {
    let output = process_path("tests/fixtures/sample.rs", outline_opts()).unwrap();
    assert!(output.contains("MAX_USERS"), "Should show constant");
    assert!(output.contains("UserMap"), "Should show type alias");
}

#[test]
fn rust_outline_shows_imports() {
    let output = process_path("tests/fixtures/sample.rs", outline_opts()).unwrap();
    assert!(
        output.contains("use std::collections::HashMap"),
        "Should show imports"
    );
}

#[test]
fn rust_outline_shows_docstrings() {
    let output = process_path("tests/fixtures/sample.rs", outline_opts()).unwrap();
    assert!(
        output.contains("/// A sample struct"),
        "Should show docstring"
    );
}

// ============================================================
// TypeScript outline mode
// ============================================================

const TS_SAMPLE: &str = r#"import { Request, Response } from 'express';

/** Maximum retry count */
export const MAX_RETRIES = 3;

/** Fetches user data from the API */
export async function fetchUser(id: string): Promise<User> {
    const res = await fetch(`/api/users/${id}`);
    return res.json();
}

function privateHelper(x: number): number {
    return x * 2;
}

export class ApiClient {
    private baseUrl: string;

    constructor(baseUrl: string) {
        this.baseUrl = baseUrl;
    }

    /** Makes a GET request */
    async get(path: string): Promise<Response> {
        return fetch(this.baseUrl + path);
    }

    private handleError(err: Error): void {
        console.error(err);
    }
}

export interface UserResponse {
    id: string;
    name: string;
    email: string;
}

export type UserId = string;
"#;

#[test]
fn ts_outline_shows_signatures() {
    let f = write_ts(TS_SAMPLE);
    let output = process_path(f.path().to_str().unwrap(), outline_opts()).unwrap();
    assert!(
        output.contains("class ApiClient"),
        "Should show class: {}",
        output
    );
    // TS export functions may appear as export statements
    assert!(output.contains("ApiClient"), "Should contain ApiClient");
}

#[test]
fn ts_outline_omits_bodies() {
    let f = write_ts(TS_SAMPLE);
    let output = process_path(f.path().to_str().unwrap(), outline_opts()).unwrap();
    assert!(
        !output.contains("res.json()"),
        "Should omit fn body: {}",
        output
    );
    assert!(
        !output.contains("console.error"),
        "Should omit method body: {}",
        output
    );
}

// ============================================================
// Composability: --outline --pub
// ============================================================

#[test]
fn outline_pub_only_rust() {
    let mut opts = outline_opts();
    opts.pub_only = true;
    let output = process_path("tests/fixtures/sample.rs", opts).unwrap();
    assert!(
        output.contains("pub struct User"),
        "Should show public struct"
    );
    assert!(
        output.contains("pub fn public_utility"),
        "Should show public fn"
    );
    assert!(output.contains("pub enum Role"), "Should show public enum");
    assert!(
        !output.contains("fn private_helper"),
        "Should omit private fn: {}",
        output
    );
}

#[test]
fn outline_pub_only_python() {
    let f = write_py(PYTHON_SAMPLE);
    let mut opts = outline_opts();
    opts.pub_only = true;
    let output = process_path(f.path().to_str().unwrap(), opts).unwrap();
    assert!(output.contains("def helper"), "Public fn should appear");
    assert!(
        !output.contains("_private_fn"),
        "Private fn should be hidden: {}",
        output
    );
}

#[test]
fn outline_pub_only_ts() {
    let f = write_ts(TS_SAMPLE);
    let mut opts = outline_opts();
    opts.pub_only = true;
    let output = process_path(f.path().to_str().unwrap(), opts).unwrap();
    // Exported symbols should appear with --pub
    assert!(
        output.contains("ApiClient") || output.contains("export"),
        "Exported class should appear: {}",
        output
    );
}

// ============================================================
// Composability: --outline --no-tests
// ============================================================

#[test]
fn outline_no_tests_rust() {
    let mut opts = outline_opts();
    opts.no_tests = true;
    let output = process_path("tests/fixtures/sample.rs", opts).unwrap();
    assert!(output.contains("pub fn new"), "Should show normal fn");
    assert!(
        !output.contains("mod tests"),
        "Should omit test module: {}",
        output
    );
    assert!(
        !output.contains("test_user_creation"),
        "Should omit test fn: {}",
        output
    );
}

#[test]
fn outline_no_tests_python() {
    let src = r#"def helper(x: int) -> int:
    return x * 2

def test_helper():
    assert helper(2) == 4
"#;
    let f = write_py(src);
    let mut opts = outline_opts();
    opts.no_tests = true;
    let output = process_path(f.path().to_str().unwrap(), opts).unwrap();
    assert!(output.contains("def helper"), "Should show normal fn");
    assert!(
        !output.contains("test_helper"),
        "Should omit test fn: {}",
        output
    );
}

// ============================================================
// Composability: --outline --json
// ============================================================

#[test]
fn outline_json_rust() {
    let mut opts = outline_opts();
    opts.format = OutputFormat::Json;
    let output = process_path("tests/fixtures/sample.rs", opts).unwrap();
    let parsed: serde_json::Value = serde_json::from_str(&output).expect("Should be valid JSON");
    assert!(
        parsed.is_array() || parsed.is_object(),
        "JSON output should be structured"
    );
    let text = output.to_string();
    assert!(text.contains("User"), "JSON should contain User symbol");
}

#[test]
fn outline_json_python() {
    let f = write_py(PYTHON_SAMPLE);
    let mut opts = outline_opts();
    opts.format = OutputFormat::Json;
    let output = process_path(f.path().to_str().unwrap(), opts).unwrap();
    let _parsed: serde_json::Value = serde_json::from_str(&output).expect("Should be valid JSON");
    assert!(output.contains("helper"), "JSON should contain helper");
}

// ============================================================
// Directory mode: --outline on a directory
// ============================================================

#[test]
fn outline_directory_mode() {
    let opts = outline_opts();
    let output = process_path("tests/fixtures", opts).unwrap();
    // Should process multiple files
    assert!(
        output.contains("sample.rs") || output.contains("User"),
        "Should include Rust fixture content: {}",
        output
    );
}

#[test]
fn outline_directory_with_pub() {
    let mut opts = outline_opts();
    opts.pub_only = true;
    let output = process_path("tests/fixtures", opts).unwrap();
    // Public symbols should appear
    assert!(
        output.contains("User") || output.contains("pub"),
        "Directory outline with --pub should show public symbols"
    );
}

// ============================================================
// Snapshot-style tests: verify exact output structure
// ============================================================

#[test]
fn snapshot_rust_outline_structure() {
    let src = r#"/// Doc for Foo
pub struct Foo {
    pub x: i32,
    y: String,
}

impl Foo {
    /// Creates a new Foo
    pub fn new(x: i32, y: String) -> Self {
        Self { x, y }
    }

    fn private_method(&self) -> i32 {
        self.x
    }
}

pub fn top_level(a: i32, b: i32) -> i32 {
    a + b
}
"#;
    let f = write_rs(src);
    let output = process_path(f.path().to_str().unwrap(), outline_opts()).unwrap();

    // Verify structural properties of outline output
    assert!(
        output.contains("/// Doc for Foo"),
        "Should preserve docstring"
    );
    assert!(output.contains("pub struct Foo"), "Should show struct");
    assert!(
        output.contains("pub fn new"),
        "Should show public method sig"
    );
    assert!(
        output.contains("fn private_method"),
        "Should show private method sig"
    );
    assert!(output.contains("pub fn top_level"), "Should show free fn");
    // Bodies should not appear
    assert!(!output.contains("a + b"), "Should omit fn body: {}", output);
    assert!(
        !output.contains("self.x"),
        "Should omit method body: {}",
        output
    );
}

#[test]
fn snapshot_python_outline_structure() {
    let src = r#"import json

API_KEY = "secret"

def process(data: dict) -> list:
    """Process the data."""
    result = []
    for item in data:
        result.append(item)
    return result

class Handler:
    def handle(self, request):
        return self.dispatch(request)

    def dispatch(self, request):
        pass
"#;
    let f = write_py(src);
    let output = process_path(f.path().to_str().unwrap(), outline_opts()).unwrap();

    assert!(output.contains("import json"), "Should show import");
    assert!(output.contains("API_KEY"), "Should show constant");
    assert!(output.contains("def process"), "Should show fn signature");
    assert!(output.contains("class Handler"), "Should show class");
    assert!(output.contains("def handle"), "Should show method sig");
    assert!(output.contains("def dispatch"), "Should show method sig");
    // Bodies omitted
    assert!(
        !output.contains("result.append"),
        "Should omit body: {}",
        output
    );
    assert!(
        !output.contains("self.dispatch"),
        "Should omit method body: {}",
        output
    );
}

#[test]
fn snapshot_ts_outline_structure() {
    let src = r#"export interface Config {
    host: string;
    port: number;
}

export class Server {
    private config: Config;

    constructor(config: Config) {
        this.config = config;
    }

    /** Start the server */
    async start(): Promise<void> {
        await listen(this.config.port);
    }

    stop(): void {
        process.exit(0);
    }
}

export function createServer(config: Config): Server {
    return new Server(config);
}
"#;
    let f = write_ts(src);
    let output = process_path(f.path().to_str().unwrap(), outline_opts()).unwrap();

    assert!(output.contains("interface Config"), "Should show interface");
    assert!(output.contains("class Server"), "Should show class");
    assert!(output.contains("start"), "Should show method sig");
    assert!(output.contains("stop"), "Should show method sig");
    assert!(output.contains("class Server"), "Should show class");
    assert!(output.contains("start"), "Should show method sig");
    // Bodies omitted
    assert!(
        !output.contains("process.exit"),
        "Should omit body: {}",
        output
    );
}

#[test]
fn snapshot_compact_outline_rust() {
    let src = r#"/// A point in 2D space
pub struct Point {
    pub x: f64,
    pub y: f64,
}

impl Point {
    /// Create a new point
    pub fn new(x: f64, y: f64) -> Self {
        Self { x, y }
    }

    pub fn distance(&self, other: &Point) -> f64 {
        ((self.x - other.x).powi(2) + (self.y - other.y).powi(2)).sqrt()
    }
}
"#;
    let f = write_rs(src);
    let mut opts = outline_opts();
    opts.compact = true;
    let output = process_path(f.path().to_str().unwrap(), opts).unwrap();

    // Compact: params collapsed, docstrings stripped
    assert!(
        output.contains("(…)"),
        "Compact should collapse params: {}",
        output
    );
    assert!(
        !output.contains("/// A point in 2D space"),
        "Compact should strip docstrings: {}",
        output
    );
    assert!(
        !output.contains("/// Create a new point"),
        "Compact should strip member docstrings: {}",
        output
    );
}

#[test]
fn snapshot_compact_outline_python() {
    let src = r#"def compute(x: int, y: int, z: int) -> int:
    return x + y + z

class Calculator:
    def add(self, a: int, b: int) -> int:
        return a + b
"#;
    let f = write_py(src);
    let mut opts = outline_opts();
    opts.compact = true;
    let output = process_path(f.path().to_str().unwrap(), opts).unwrap();

    assert!(
        output.contains("(…)"),
        "Compact should collapse params: {}",
        output
    );
}

// ============================================================
// Combined composability tests
// ============================================================

#[test]
fn outline_pub_no_tests_combined() {
    let mut opts = outline_opts();
    opts.pub_only = true;
    opts.no_tests = true;
    let output = process_path("tests/fixtures/sample.rs", opts).unwrap();
    assert!(
        output.contains("pub struct User"),
        "Should show public struct"
    );
    assert!(
        output.contains("pub fn public_utility"),
        "Should show public fn"
    );
    assert!(
        !output.contains("fn private_helper"),
        "Should omit private: {}",
        output
    );
    assert!(
        !output.contains("test_user_creation"),
        "Should omit tests: {}",
        output
    );
}

#[test]
fn outline_pub_json_combined() {
    let mut opts = outline_opts();
    opts.pub_only = true;
    opts.format = OutputFormat::Json;
    let output = process_path("tests/fixtures/sample.rs", opts).unwrap();
    let _parsed: serde_json::Value =
        serde_json::from_str(&output).expect("Should produce valid JSON");
    // Private symbols should not appear
    assert!(
        !output.contains("private_helper"),
        "JSON --pub should omit private: {}",
        output
    );
}

#[test]
fn outline_compact_json() {
    let mut opts = outline_opts();
    opts.compact = true;
    opts.format = OutputFormat::Json;
    let output = process_path("tests/fixtures/sample.rs", opts).unwrap();
    let _parsed: serde_json::Value =
        serde_json::from_str(&output).expect("Compact JSON should be valid");
}
