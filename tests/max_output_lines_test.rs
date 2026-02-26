use std::process::Command;

fn codehud() -> Command {
    Command::new(env!("CARGO_BIN_EXE_codehud"))
}

#[test]
fn test_max_output_lines_truncates() {
    let output = codehud()
        .args(&["tests/fixtures/sample.rs", "--max-output-lines", "5"])
        .output()
        .expect("failed to run codehud");
    let stdout = String::from_utf8_lossy(&output.stdout);
    let lines: Vec<&str> = stdout.lines().collect();
    // 5 content lines + 1 footer line
    assert_eq!(lines.len(), 6, "expected 6 lines (5 + footer), got {}:\n{}", lines.len(), stdout);
    assert!(lines[5].starts_with("[Output truncated:"), "expected truncation footer, got: {}", lines[5]);
}

#[test]
fn test_max_output_lines_no_truncation_when_under_limit() {
    let output = codehud()
        .args(&["tests/fixtures/sample.rs", "--max-output-lines", "99999"])
        .output()
        .expect("failed to run codehud");
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(!stdout.contains("[Output truncated:"), "should not truncate when under limit");
}

#[test]
fn test_max_output_lines_with_stats() {
    let output = codehud()
        .args(&["tests/fixtures/sample.rs", "--stats", "--max-output-lines", "1"])
        .output()
        .expect("failed to run codehud");
    let stdout = String::from_utf8_lossy(&output.stdout);
    let lines: Vec<&str> = stdout.lines().collect();
    assert_eq!(lines.len(), 2, "expected 2 lines (1 + footer), got {}:\n{}", lines.len(), stdout);
    assert!(lines[1].starts_with("[Output truncated:"));
}

#[test]
fn test_max_output_lines_with_search() {
    let output = codehud()
        .args(&["tests/fixtures/sample.rs", "--search", "fn", "--max-output-lines", "3"])
        .output()
        .expect("failed to run codehud");
    let stdout = String::from_utf8_lossy(&output.stdout);
    let lines: Vec<&str> = stdout.lines().collect();
    // Should have 3 content + 1 footer = 4
    assert_eq!(lines.len(), 4, "expected 4 lines, got {}:\n{}", lines.len(), stdout);
    assert!(lines[3].starts_with("[Output truncated:"));
}

#[test]
fn test_max_output_lines_with_tree() {
    let output = codehud()
        .args(&["tests/fixtures", "--tree", "--max-output-lines", "2"])
        .output()
        .expect("failed to run codehud");
    let stdout = String::from_utf8_lossy(&output.stdout);
    let lines: Vec<&str> = stdout.lines().collect();
    assert_eq!(lines.len(), 3, "expected 3 lines (2 + footer), got {}:\n{}", lines.len(), stdout);
    assert!(lines[2].starts_with("[Output truncated:"));
}
