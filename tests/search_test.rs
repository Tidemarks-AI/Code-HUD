use std::fs;
use tempfile::TempDir;

fn run_codehud(args: &[&str]) -> (String, String, bool) {
    let bin = env!("CARGO_BIN_EXE_codehud");
    let output = std::process::Command::new(bin)
        .args(args)
        .output()
        .expect("failed to run codehud");
    let stdout = String::from_utf8(output.stdout).unwrap();
    let stderr = String::from_utf8(output.stderr).unwrap();
    (stdout, stderr, output.status.success())
}

fn run_ok(args: &[&str]) -> String {
    let (stdout, stderr, success) = run_codehud(args);
    assert!(success, "codehud failed: {}", stderr);
    stdout
}

fn write_file(dir: &TempDir, name: &str, content: &str) -> String {
    let path = dir.path().join(name);
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent).unwrap();
    }
    fs::write(&path, content).unwrap();
    path.to_string_lossy().to_string()
}

// ---------------------------------------------------------------------------
// 1. Basic search — find a known symbol by exact name
// ---------------------------------------------------------------------------

#[test]
fn search_basic_exact_name() {
    let dir = TempDir::new().unwrap();
    let path = write_file(&dir, "sample.rs", r#"
fn hello_world() {
    println!("hi");
}

fn other() {}
"#);
    let out = run_ok(&[&path, "--search", "hello_world"]);
    assert!(out.contains("hello_world"), "should find the symbol");
    assert!(!out.contains("other"), "should not include unmatched lines");
}

#[test]
fn search_basic_in_struct() {
    let out = run_ok(&["tests/fixtures/sample.rs", "--search", "greeting"]);
    assert!(out.contains("greeting"), "should find greeting method");
    assert!(out.contains("impl User"), "should show enclosing impl block");
}

// ---------------------------------------------------------------------------
// 2. Regex patterns
// ---------------------------------------------------------------------------

#[test]
fn search_regex_pattern() {
    let dir = TempDir::new().unwrap();
    let path = write_file(&dir, "test.rs", r#"
fn get_name() {}
fn get_age() {}
fn set_name() {}
"#);
    let out = run_ok(&[&path, "--search", "get.*", "--regex"]);
    assert!(out.contains("get_name"), "should match get_name");
    assert!(out.contains("get_age"), "should match get_age");
    assert!(!out.contains("set_name"), "should not match set_name");
}

#[test]
fn search_regex_let_digits() {
    let dir = TempDir::new().unwrap();
    let path = write_file(&dir, "test.rs", r#"
fn process() {
    let x = 42;
    let y = 100;
    let z = "hello";
}
"#);
    let out = run_ok(&[&path, "--search", r"let \w+ = \d+", "--regex"]);
    assert!(out.contains("L3:") || out.contains("let x"), "should match let x = 42");
    assert!(out.contains("L4:") || out.contains("let y"), "should match let y = 100");
    assert!(!out.contains("let z"), "should not match let z = \"hello\"");
}

// ---------------------------------------------------------------------------
// 3. Max results cap
// ---------------------------------------------------------------------------

#[test]
fn search_max_results_limits_output() {
    let dir = TempDir::new().unwrap();
    // Need a directory search (max-results default applies to dirs)
    fs::create_dir(dir.path().join(".git")).unwrap();
    write_file(&dir, "a.rs", "fn f1() { target(); }\nfn f2() { target(); }\nfn f3() { target(); }\n");
    write_file(&dir, "b.rs", "fn g1() { target(); }\nfn g2() { target(); }\nfn g3() { target(); }\n");
    let dir_str = dir.path().to_string_lossy().to_string();

    let out = run_ok(&[&dir_str, "--search", "target", "--max-results", "1"]);
    // Should show truncation message
    assert!(out.contains("... and"), "should indicate truncated results: {}", out);
}

#[test]
fn search_max_results_single_file() {
    let dir = TempDir::new().unwrap();
    let path = write_file(&dir, "test.rs", "fn f1() { target(); }\nfn f2() { target(); }\nfn f3() { target(); }\n");
    // --max-results on single file
    let out = run_ok(&[&path, "--search", "target", "--max-results", "1"]);
    // Count actual match lines (lines starting with spaces containing "L")
    let match_lines: Vec<&str> = out.lines()
        .filter(|l| l.contains("L") && l.contains("target"))
        .collect();
    assert_eq!(match_lines.len(), 1, "should show exactly 1 match, got: {:?}", match_lines);
}

// ---------------------------------------------------------------------------
// 4. No matches — clean empty output
// ---------------------------------------------------------------------------

#[test]
fn search_no_matches_empty_output() {
    let (stdout, stderr, success) = run_codehud(&["tests/fixtures/sample.rs", "--search", "zzz_nonexistent_zzz"]);
    assert!(!success, "should exit non-zero when no matches found");
    assert!(stdout.trim().is_empty(), "no matches should produce empty stdout, got: '{}'", stdout);
    assert!(stderr.contains("No matches found"), "stderr should contain message, got: '{}'", stderr);
}

// ---------------------------------------------------------------------------
// 5. JSON output — --search with --json (note: may not be wired up)
// ---------------------------------------------------------------------------

#[test]
fn search_with_json_flag() {
    // --json may not affect search output (search has its own formatter).
    // This test verifies the command doesn't error out with both flags.
    let (stdout, _stderr, success) = run_codehud(&[
        "tests/fixtures/sample.rs", "--search", "User", "--json",
    ]);
    assert!(success, "should not error with --search --json");
    assert!(stdout.contains("User"), "should still find User");
}

// ---------------------------------------------------------------------------
// 6. Multi-language: Rust, TypeScript, Python
// ---------------------------------------------------------------------------

#[test]
fn search_rust_fixture() {
    let out = run_ok(&["tests/fixtures/sample.rs", "--search", "impl"]);
    assert!(out.contains("impl User") || out.contains("impl"), "should find impl blocks");
}

#[test]
fn search_typescript() {
    let dir = TempDir::new().unwrap();
    let path = write_file(&dir, "app.ts", r#"
class Calculator {
    add(a: number, b: number): number {
        return a + b;
    }

    subtract(a: number, b: number): number {
        return a - b;
    }
}
"#);
    let out = run_ok(&[&path, "--search", "subtract"]);
    assert!(out.contains("Calculator"), "should show enclosing class");
    assert!(out.contains("subtract"), "should find subtract method");
}

#[test]
fn search_python() {
    let dir = TempDir::new().unwrap();
    let path = write_file(&dir, "app.py", r#"
class Parser:
    def parse(self, text):
        return text.split()

    def validate(self, text):
        return len(text) > 0

def helper():
    pass
"#);
    let out = run_ok(&[&path, "--search", "parse"]);
    assert!(out.contains("parse"), "should find parse method");
    assert!(out.contains("Parser"), "should show enclosing class");
}

// ---------------------------------------------------------------------------
// Additional: case-insensitive search via CLI
// ---------------------------------------------------------------------------

#[test]
fn search_case_insensitive() {
    let dir = TempDir::new().unwrap();
    let path = write_file(&dir, "test.rs", "fn Hello() {\n    let World = 1;\n}\n");

    // Case-sensitive: "hello" should NOT match "Hello" → exits non-zero (no matches)
    let (stdout, _stderr, success) = run_codehud(&[&path, "--search", "hello"]);
    assert!(!success, "case-sensitive search with no matches should exit non-zero");
    assert!(!stdout.contains("Hello"), "case-sensitive should not match: {}", stdout);

    // Case-insensitive
    let out = run_ok(&[&path, "--search", "hello", "-i"]);
    assert!(out.contains("Hello"), "case-insensitive should match: {}", out);
}

// ---------------------------------------------------------------------------
// Directory search with default cap
// ---------------------------------------------------------------------------

#[test]
fn search_directory_default_cap() {
    let dir = TempDir::new().unwrap();
    fs::create_dir(dir.path().join(".git")).unwrap();
    // Create 25 matches across files — default cap is 20
    for i in 0..25 {
        write_file(&dir, &format!("f{}.rs", i), &format!("fn f{}() {{ target(); }}\n", i));
    }
    let dir_str = dir.path().to_string_lossy().to_string();
    let out = run_ok(&[&dir_str, "--search", "target"]);
    assert!(out.contains("... and"), "directory search should cap at 20 by default: {}", out);
}

// ---------------------------------------------------------------------------
// Search shows symbol hierarchy (enclosing context)
// ---------------------------------------------------------------------------

#[test]
fn search_shows_symbol_hierarchy() {
    let dir = TempDir::new().unwrap();
    let path = write_file(&dir, "test.ts", r#"
class MyService {
    processRequest(req: any) {
        const result = transform(req);
        return result;
    }
}
"#);
    let out = run_ok(&[&path, "--search", "transform"]);
    assert!(out.contains("MyService"), "should show enclosing class");
    assert!(out.contains("processRequest"), "should show enclosing method");
    assert!(out.contains("transform"), "should show the match");
}

// ---------------------------------------------------------------------------
// Top-level matches show (top-level)
// ---------------------------------------------------------------------------

#[test]
fn search_top_level_annotation() {
    let dir = TempDir::new().unwrap();
    let path = write_file(&dir, "test.rs", "use std::io;\nfn main() {}\n");
    let out = run_ok(&[&path, "--search", "std::io"]);
    assert!(out.contains("(top-level)"), "top-level matches should be annotated");
}

// ---------------------------------------------------------------------------
// No matches → stderr message + exit code 1
// ---------------------------------------------------------------------------

#[test]
fn search_no_matches_exits_nonzero() {
    let dir = TempDir::new().unwrap();
    let path = write_file(&dir, "test.rs", "fn hello() {}\n");
    let (stdout, stderr, success) = run_codehud(&[&path, "--search", "zzz_nonexistent"]);
    assert!(!success, "should exit with non-zero code when no matches found");
    assert!(stdout.is_empty(), "stdout should be empty");
    assert!(stderr.contains("No matches found for 'zzz_nonexistent'"), "stderr: {}", stderr);
}

#[test]
fn search_no_matches_directory_exits_nonzero() {
    let dir = TempDir::new().unwrap();
    fs::create_dir(dir.path().join(".git")).unwrap();
    write_file(&dir, "a.rs", "fn foo() {}\n");
    let dir_path = dir.path().to_string_lossy().to_string();
    let (stdout, stderr, success) = run_codehud(&[&dir_path, "--search", "zzz_nonexistent"]);
    assert!(!success, "should exit with non-zero code when no matches found in directory");
    assert!(stdout.is_empty(), "stdout should be empty");
    assert!(stderr.contains("No matches found for 'zzz_nonexistent'"), "stderr: {}", stderr);
}

// ---------------------------------------------------------------------------
// 11. --regex / -E flag
// ---------------------------------------------------------------------------

#[test]
fn search_literal_mode_escapes_regex_metacharacters() {
    let dir = TempDir::new().unwrap();
    let path = write_file(&dir, "test.rs", "fn example() {\n    let pattern = \"get.*\";\n    let name = get_name();\n}\n");
    // Without --regex, "get.*" is literal — should only match the literal string
    let out = run_ok(&[&path, "--search", "get.*"]);
    assert!(out.contains("get.*"), "literal mode should match literal 'get.*'");
    let lines: Vec<&str> = out.lines().filter(|l| l.trim_start().starts_with("L")).collect();
    assert_eq!(lines.len(), 1, "literal mode should find exactly 1 match, got: {:?}", lines);
}

#[test]
fn search_regex_flag_enables_pattern_matching() {
    let dir = TempDir::new().unwrap();
    let path = write_file(&dir, "test.rs", "fn get_name() {}\nfn get_age() {}\nfn set_name() {}\n");
    let out = run_ok(&[&path, "--search", "get.*", "--regex"]);
    assert!(out.contains("get_name"), "regex mode should match get_name");
    assert!(out.contains("get_age"), "regex mode should match get_age");
}

#[test]
fn search_short_flag_e_works() {
    let dir = TempDir::new().unwrap();
    let path = write_file(&dir, "test.rs", "fn process() {\n    let x = 42;\n    let y = \"hello\";\n}\n");
    let out = run_ok(&[&path, "--search", r"let \w+ = \d+", "-E"]);
    assert!(out.contains("let x"), "-E should enable regex matching");
    assert!(!out.contains("let y"), "should not match string assignment");
}

#[test]
fn search_env_var_pattern_with_regex() {
    let dir = TempDir::new().unwrap();
    let path = write_file(&dir, "test.ts", "function config() {\n    const host = process.env.HOST;\n    const port = process.env.PORT;\n    const name = \"codeview\";\n}\n");
    let out = run_ok(&[&path, "--search", r"process\.env\.\w+", "-E"]);
    assert!(out.contains("process.env.HOST"), "should match HOST env var");
    assert!(out.contains("process.env.PORT"), "should match PORT env var");
    assert!(!out.contains("codeview"), "should not match plain string");
}
