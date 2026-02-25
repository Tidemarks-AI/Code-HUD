use std::fs;
use tempfile::TempDir;

fn run_codehud(args: &[&str]) -> String {
    let bin = env!("CARGO_BIN_EXE_codehud");
    let output = std::process::Command::new(bin)
        .args(args)
        .output()
        .expect("failed to run codehud");
    String::from_utf8(output.stdout).unwrap()
}

fn write_ts_file(dir: &TempDir, name: &str, content: &str) -> String {
    let path = dir.path().join(name);
    fs::write(&path, content).unwrap();
    path.to_string_lossy().to_string()
}

const LONG_FUNCTION: &str = r#"
function processData(input: string): string {
  const a = 1;
  const b = 2;
  const c = 3;
  const d = 4;
  const e = 5;
  const f = 6;
  const g = 7;
  const h = 8;
  const i = 9;
  const j = 10;
  return input;
}

function helperFn(x: number): number {
  const a = 1;
  const b = 2;
  const c = 3;
  const d = 4;
  return x + a + b + c + d;
}
"#;

const CLASS_CODE: &str = r#"class Greeter {
  name: string;

  constructor(name: string) {
    this.name = name;
  }

  greet(): string {
    return `Hello, ${this.name}!`;
  }

  farewell(): string {
    return `Goodbye, ${this.name}!`;
  }
}
"#;

#[test]
fn max_lines_truncates_long_function() {
    let dir = TempDir::new().unwrap();
    let path = write_ts_file(&dir, "test.ts", LONG_FUNCTION);
    let output = run_codehud(&[&path, "processData", "--max-lines", "3"]);
    assert!(output.contains("[truncated:"), "expected truncation indicator, got:\n{}", output);
    // Should show exactly 3 lines of content before truncation
    let lines: Vec<&str> = output.lines().collect();
    let trunc_line = lines.iter().position(|l| l.contains("[truncated:")).unwrap();
    // Header is line 0, then 3 content lines, then truncation
    assert_eq!(trunc_line, 4, "truncation should be at line 4 (after header + 3 lines), got:\n{}", output);
}

#[test]
fn max_lines_no_truncation_when_under_limit() {
    let dir = TempDir::new().unwrap();
    let path = write_ts_file(&dir, "test.ts", LONG_FUNCTION);
    let output = run_codehud(&[&path, "processData", "--max-lines", "500"]);
    assert!(!output.contains("[truncated:"), "should not truncate when under limit, got:\n{}", output);
}

#[test]
fn max_lines_combined_with_signatures() {
    let dir = TempDir::new().unwrap();
    let path = write_ts_file(&dir, "test.ts", CLASS_CODE);
    let output = run_codehud(&[&path, "Greeter", "--signatures", "--max-lines", "2"]);
    assert!(output.contains("[truncated:"), "expected truncation with signatures, got:\n{}", output);
}

#[test]
fn max_lines_multiple_symbols() {
    let dir = TempDir::new().unwrap();
    let path = write_ts_file(&dir, "test.ts", LONG_FUNCTION);
    let output = run_codehud(&[&path, "processData", "helperFn", "--max-lines", "2"]);
    assert!(output.contains("processData"), "should contain processData");
    assert!(output.contains("helperFn"), "should contain helperFn");
    // Both should be truncated since they have more than 2 lines
    let truncation_count = output.matches("[truncated:").count();
    assert_eq!(truncation_count, 2, "both symbols should be truncated, got:\n{}", output);
}

#[test]
fn max_lines_exact_limit_no_truncation() {
    let dir = TempDir::new().unwrap();
    // helperFn has 7 lines (signature + 5 body + closing brace)
    let path = write_ts_file(&dir, "test.ts", LONG_FUNCTION);
    let output = run_codehud(&[&path, "helperFn", "--max-lines", "7"]);
    assert!(!output.contains("[truncated:"), "exact limit should not truncate, got:\n{}", output);
}
