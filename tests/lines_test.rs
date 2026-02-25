use std::fs;
use tempfile::TempDir;

fn write_file(dir: &TempDir, name: &str, content: &str) -> String {
    let path = dir.path().join(name);
    fs::write(&path, content).unwrap();
    path.to_string_lossy().to_string()
}

#[test]
fn lines_basic_extraction() {
    let dir = TempDir::new().unwrap();
    let path = write_file(&dir, "test.rs", "fn foo() {\n    let x = 1;\n    let y = 2;\n    x + y\n}\n");
    let result = codehud::extract_lines(&path, "2-4").unwrap();
    assert!(result.contains("// Inside: foo"));
    assert!(result.contains("L2:"));
    assert!(result.contains("L3:"));
    assert!(result.contains("L4:"));
    assert!(result.contains("let x = 1;"));
    assert!(!result.contains("L1:"));
    assert!(!result.contains("L5:"));
}

#[test]
fn lines_single_line() {
    let dir = TempDir::new().unwrap();
    let path = write_file(&dir, "test.rs", "fn foo() {\n    42\n}\n");
    let result = codehud::extract_lines(&path, "2-2").unwrap();
    assert!(result.contains("L2:"));
    assert!(result.contains("42"));
}

#[test]
fn lines_top_level_no_context() {
    let dir = TempDir::new().unwrap();
    let path = write_file(&dir, "test.rs", "use std::io;\n\nfn foo() {}\n");
    let result = codehud::extract_lines(&path, "1-1").unwrap();
    // use statement is a top-level item, not inside anything — but it may still show context
    assert!(result.contains("L1:"));
    assert!(result.contains("use std::io;"));
}

#[test]
fn lines_out_of_range_start() {
    let dir = TempDir::new().unwrap();
    let path = write_file(&dir, "test.rs", "fn foo() {}\n");
    let result = codehud::extract_lines(&path, "100-200");
    assert!(result.is_err());
    assert!(result.unwrap_err().to_string().contains("beyond end of file"));
}

#[test]
fn lines_end_beyond_file_clamps() {
    let dir = TempDir::new().unwrap();
    let path = write_file(&dir, "test.rs", "fn foo() {\n    42\n}\n");
    // End beyond file should be clamped
    let result = codehud::extract_lines(&path, "2-999").unwrap();
    assert!(result.contains("L2:"));
    assert!(result.contains("L3:"));
}

#[test]
fn lines_inverted_range_errors() {
    let dir = TempDir::new().unwrap();
    let path = write_file(&dir, "test.rs", "fn foo() {}\n");
    let result = codehud::extract_lines(&path, "5-3");
    assert!(result.is_err());
    assert!(result.unwrap_err().to_string().contains("Inverted range"));
}

#[test]
fn lines_directory_errors() {
    let dir = TempDir::new().unwrap();
    let result = codehud::extract_lines(&dir.path().to_string_lossy(), "1-5");
    assert!(result.is_err());
    assert!(result.unwrap_err().to_string().contains("not directories"));
}

#[test]
fn lines_nested_context_typescript() {
    let dir = TempDir::new().unwrap();
    let path = write_file(&dir, "test.ts", "class MyClass {\n    run() {\n        console.log('hello');\n    }\n}\n");
    let result = codehud::extract_lines(&path, "3-3").unwrap();
    assert!(result.contains("// Inside:"));
    assert!(result.contains("MyClass"));
    assert!(result.contains("run()"));
    assert!(result.contains("console.log"));
}

#[test]
fn lines_invalid_format() {
    let dir = TempDir::new().unwrap();
    let path = write_file(&dir, "test.rs", "fn foo() {}\n");
    let result = codehud::extract_lines(&path, "abc");
    assert!(result.is_err());
}

#[test]
fn lines_zero_start_errors() {
    let dir = TempDir::new().unwrap();
    let path = write_file(&dir, "test.rs", "fn foo() {}\n");
    let result = codehud::extract_lines(&path, "0-5");
    assert!(result.is_err());
    assert!(result.unwrap_err().to_string().contains("1-indexed"));
}
