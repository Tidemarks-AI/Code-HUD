use codehud::{process_path, ProcessOptions, OutputFormat};
use std::process::Command;

const FIXTURE_PATH: &str = "tests/fixtures/sample.rs";
const FIXTURE_DIR: &str = "tests/fixtures";

fn run_codehud(args: &[&str]) -> String {
    let bin = env!("CARGO_BIN_EXE_codehud");
    let output = Command::new(bin)
        .args(args)
        .output()
        .expect("failed to run codehud");
    assert!(output.status.success(), "codehud failed: {}", String::from_utf8_lossy(&output.stderr));
    String::from_utf8(output.stdout).unwrap()
}

fn default_options() -> ProcessOptions {
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
        list_symbols: true,
        no_imports: false,
        smart_depth: false,
        symbol_depth: None,
        exclude: vec![],
        outline: false,
        compact: false,
    }
}

#[test]
fn test_list_symbols_single_file() {
    let output = process_path(FIXTURE_PATH, default_options()).unwrap();
    // Should contain file header
    assert!(output.contains("sample.rs"));
    // Should list symbols with kind and line number
    assert!(output.contains("struct") && output.contains("User"));
    assert!(output.contains("enum") && output.contains("Role"));
    assert!(output.contains("trait") && output.contains("Authenticatable"));
    // Should NOT contain bodies or full signatures
    assert!(!output.contains("{ ... }"));
    assert!(!output.contains("->"));
}

#[test]
fn test_list_symbols_compact_one_line_per_symbol() {
    let output = process_path(FIXTURE_PATH, default_options()).unwrap();
    // Each non-header line should start with "  " (indented symbol)
    for line in output.lines().skip(1) {
        // Each symbol line is indented
        assert!(line.starts_with("  "), "Expected indented line: {}", line);
        // Each symbol line should contain L followed by a number
        assert!(line.contains(" L"), "Expected line number: {}", line);
    }
}

#[test]
fn test_list_symbols_smaller_than_interface() {
    let interface_opts = ProcessOptions {
        list_symbols: false,
        no_imports: false,
        smart_depth: false,
        symbol_depth: None,
        exclude: vec![],
        outline: false,
        compact: false,
        ..default_options()
    };
    let interface_output = process_path(FIXTURE_PATH, interface_opts).unwrap();
    let list_output = process_path(FIXTURE_PATH, default_options()).unwrap();
    assert!(
        list_output.len() < interface_output.len(),
        "list-symbols output ({}) should be smaller than interface output ({})",
        list_output.len(),
        interface_output.len()
    );
}

#[test]
fn test_list_symbols_with_pub_filter() {
    let options = ProcessOptions {
        pub_only: true,
        ..default_options()
    };
    let output = process_path(FIXTURE_PATH, options).unwrap();
    // Should only have public items — private items filtered out
    assert!(output.contains("User"));
    // The output should not contain private functions
    // (depends on fixture, but pub filter should reduce items)
    let line_count = output.lines().count();
    let all_output = process_path(FIXTURE_PATH, default_options()).unwrap();
    let all_line_count = all_output.lines().count();
    assert!(line_count <= all_line_count);
}

#[test]
fn test_list_symbols_with_fns_filter() {
    let options = ProcessOptions {
        fns_only: true,
        ..default_options()
    };
    let output = process_path(FIXTURE_PATH, options).unwrap();
    // Should only contain fn symbols
    for line in output.lines().skip(1) {
        if line.trim().is_empty() { continue; }
        assert!(line.contains("fn "), "Expected fn in: {}", line);
    }
}

#[test]
fn test_list_symbols_with_types_filter() {
    let options = ProcessOptions {
        types_only: true,
        ..default_options()
    };
    let output = process_path(FIXTURE_PATH, options).unwrap();
    // Should only contain type symbols (struct/enum/trait/type)
    for line in output.lines().skip(1) {
        if line.trim().is_empty() { continue; }
        let has_type = line.contains("struct ") || line.contains("enum ") 
            || line.contains("trait ") || line.contains("type ")
            || line.contains("class ");
        assert!(has_type, "Expected type symbol in: {}", line);
    }
}

#[test]
fn test_list_symbols_with_no_tests() {
    let options = ProcessOptions {
        no_tests: true,
        ..default_options()
    };
    let output = process_path(FIXTURE_PATH, options).unwrap();
    // Should not contain test module
    assert!(!output.contains("mod tests"));
}

#[test]
fn test_list_symbols_directory() {
    let options = ProcessOptions {
        ext: vec!["rs".to_string()],
        ..default_options()
    };
    let output = process_path(FIXTURE_DIR, options).unwrap();
    // Should contain multiple file headers
    assert!(output.contains("sample.rs"));
    // Directory mode should work
    assert!(!output.is_empty());
}

#[test]
fn test_list_symbols_no_imports_excludes_use() {
    let options = ProcessOptions {
        no_imports: true,
        smart_depth: false,
        symbol_depth: None,
        exclude: vec![],
        outline: false,
        compact: false,
        ..default_options()
    };
    let output = process_path(FIXTURE_PATH, options).unwrap();
    // Should not contain any "use" entries
    assert!(!output.lines().any(|l| l.trim_start().starts_with("use ")),
        "Expected no 'use' entries with --no-imports, got:\n{}", output);
    // Should still contain other symbols
    assert!(output.contains("struct") && output.contains("User"));
    assert!(output.contains("enum") && output.contains("Role"));
}

#[test]
fn test_list_symbols_without_no_imports_includes_use() {
    let options = ProcessOptions {
        no_imports: false,
        smart_depth: false,
        symbol_depth: None,
        exclude: vec![],
        outline: false,
        compact: false,
        ..default_options()
    };
    let output = process_path(FIXTURE_PATH, options).unwrap();
    // Should contain "use" entries
    assert!(output.lines().any(|l| l.trim_start().starts_with("use ")),
        "Expected 'use' entries without --no-imports, got:\n{}", output);
}

#[test]
fn test_list_symbols_json_format() {
    let options = ProcessOptions {
        format: OutputFormat::Json,
        ..default_options()
    };
    let output = process_path(FIXTURE_PATH, options).unwrap();
    // Must be valid JSON
    let parsed: serde_json::Value = serde_json::from_str(&output)
        .expect("--json --list-symbols should produce valid JSON");
    // Should be an array of file entries
    let arr = parsed.as_array().expect("Expected JSON array");
    assert!(!arr.is_empty(), "Expected at least one file entry");
    // Each entry should have path and symbols
    let first = &arr[0];
    assert!(first.get("path").is_some(), "Expected 'path' field");
    let symbols = first.get("symbols").and_then(|s| s.as_array())
        .expect("Expected 'symbols' array");
    assert!(!symbols.is_empty(), "Expected at least one symbol");
    // Each symbol should have kind, name, line
    let sym = &symbols[0];
    assert!(sym.get("kind").is_some(), "Expected 'kind' field");
    assert!(sym.get("name").is_some(), "Expected 'name' field");
    assert!(sym.get("line").is_some(), "Expected 'line' field");
    assert!(sym.get("line_end").is_some(), "Expected 'line_end' field");
    assert!(sym.get("visibility").is_some(), "Expected 'visibility' field");
}

#[test]
fn test_list_symbols_json_contains_expected_symbols() {
    let options = ProcessOptions {
        format: OutputFormat::Json,
        no_imports: true,
        ..default_options()
    };
    let output = process_path(FIXTURE_PATH, options).unwrap();
    let parsed: serde_json::Value = serde_json::from_str(&output).unwrap();
    let symbols = parsed[0]["symbols"].as_array().unwrap();
    let names: Vec<&str> = symbols.iter()
        .filter_map(|s| s["name"].as_str())
        .collect();
    assert!(names.contains(&"User"), "Expected User struct in JSON symbols");
    assert!(names.contains(&"Role"), "Expected Role enum in JSON symbols");
}

#[test]
fn test_list_symbols_json_directory() {
    let options = ProcessOptions {
        format: OutputFormat::Json,
        ext: vec!["rs".to_string()],
        ..default_options()
    };
    let output = process_path(FIXTURE_DIR, options).unwrap();
    let parsed: serde_json::Value = serde_json::from_str(&output)
        .expect("--json --list-symbols on directory should produce valid JSON");
    let arr = parsed.as_array().unwrap();
    assert!(arr.len() >= 1, "Expected multiple file entries for directory");
}

#[test]
fn test_list_symbols_no_imports_typescript() {
    let options = ProcessOptions {
        no_imports: true,
        smart_depth: false,
        symbol_depth: None,
        exclude: vec![],
        outline: false,
        compact: false,
        ..default_options()
    };
    let output = process_path("tests/fixtures/imports_sample.ts", options).unwrap();
    // Should not contain any "use" entries (TS imports map to ItemKind::Use)
    assert!(!output.lines().any(|l| l.trim_start().starts_with("use ")),
        "Expected no import entries with --no-imports, got:\n{}", output);
    // Should still contain the actual symbols
    assert!(output.contains("UserService"));
    assert!(output.contains("UserController"));
    assert!(output.contains("createRouter"));
}

#[test]
fn test_list_symbols_empty_vue_script_shows_header() {
    let output = process_path("tests/fixtures/empty_script.vue", default_options()).unwrap();
    // File header should appear even when script block is empty
    assert!(output.contains("empty_script.vue"),
        "Expected file header for empty Vue script, got:\n{}", output);
}

#[test]
fn test_list_symbols_vue_sfc_depth_2_shows_class_members() {
    let options = ProcessOptions {
        symbol_depth: Some(2),
        exclude: vec![],
        outline: false,
        compact: false,
        ..default_options()
    };
    let output = process_path("tests/fixtures/component.vue", options).unwrap();
    // Should show class methods at depth 2
    assert!(output.contains("increment"), "Expected 'increment' method at depth 2, got:\n{}", output);
    assert!(output.contains("decrement"), "Expected 'decrement' method at depth 2, got:\n{}", output);
    // Should use TypeScript display names, not Rust fallback
    assert!(output.contains("interface"), "Expected 'interface' (not 'trait') for Vue SFC, got:\n{}", output);
    assert!(!output.contains("trait"), "Should not use Rust 'trait' display name for Vue SFC, got:\n{}", output);
}

#[test]
fn test_list_symbols_vue_sfc_depth_1_hides_class_members() {
    let options = ProcessOptions {
        symbol_depth: Some(1),
        exclude: vec![],
        outline: false,
        compact: false,
        ..default_options()
    };
    let output = process_path("tests/fixtures/component.vue", options).unwrap();
    // Class should appear but methods should not at depth 1
    assert!(output.contains("Counter"), "Expected 'Counter' class at depth 1");
    // Methods should be hidden at depth 1
    let lines: Vec<&str> = output.lines().filter(|l| l.contains("increment")).collect();
    assert!(lines.is_empty(), "Methods should be hidden at depth 1, got:\n{}", output);
}

#[test]
fn test_list_symbols_kind_column_alignment() {
    let output = process_path(FIXTURE_PATH, default_options()).unwrap();
    // All symbol lines should have the kind column padded to 10 chars
    for line in output.lines() {
        if line.starts_with("  ") {
            // After "  " there should be a 10-char kind field
            let after_indent = &line[2..];
            assert!(after_indent.len() >= 10,
                "Line too short for padded kind: {}", line);
            // The 10th character should be a space (padding)
            // because even "type alias" (10 chars) gets followed by a space from the format
        }
    }
}

#[test]
fn test_list_symbols_depth_2_shows_class_members() {
    let options = ProcessOptions {
        symbol_depth: Some(2),
        exclude: vec![],
        outline: false,
        compact: false,
        ..default_options()
    };
    let output = process_path("tests/fixtures/sample.ts", options).unwrap();
    // Should show methods indented under classes
    // At depth 2, Method items should appear
    assert!(output.contains("fn"), "Expected method entries with symbol-depth 2, got:\n{}", output);
}

#[test]
fn test_list_symbols_depth_2_json_includes_methods() {
    let options = ProcessOptions {
        symbol_depth: Some(2),
        exclude: vec![],
        outline: false,
        compact: false,
        format: OutputFormat::Json,
        ..default_options()
    };
    let output = process_path("tests/fixtures/sample.ts", options).unwrap();
    let parsed: serde_json::Value = serde_json::from_str(&output).unwrap();
    let symbols = parsed[0]["symbols"].as_array().unwrap();
    let kinds: Vec<&str> = symbols.iter().filter_map(|s| s["kind"].as_str()).collect();
    assert!(kinds.contains(&"fn"), "Expected 'fn' kind for methods in depth-2 JSON, got: {:?}", kinds);
}

// ============================================================
// CLI integration tests for --json --list-symbols (fixes #149)
// ============================================================

#[test]
fn test_cli_json_list_symbols_single_file() {
    let output = run_codehud(&["--json", "--list-symbols", FIXTURE_PATH]);
    let parsed: serde_json::Value = serde_json::from_str(&output)
        .expect("CLI --json --list-symbols should output valid JSON");
    let arr = parsed.as_array().expect("Expected JSON array");
    assert!(!arr.is_empty(), "Expected at least one file entry");
    assert!(arr[0].get("path").is_some(), "Expected 'path' field");
    assert!(arr[0].get("symbols").is_some(), "Expected 'symbols' field");
}

#[test]
fn test_cli_json_list_symbols_directory() {
    let output = run_codehud(&["--json", "--list-symbols", FIXTURE_DIR]);
    let parsed: serde_json::Value = serde_json::from_str(&output)
        .expect("CLI --json --list-symbols on directory should output valid JSON");
    let arr = parsed.as_array().expect("Expected JSON array");
    assert!(arr.len() > 1, "Expected multiple file entries for directory");
}

#[test]
fn test_cli_json_list_symbols_typescript() {
    let output = run_codehud(&["--json", "--list-symbols", "tests/fixtures/sample.ts"]);
    let parsed: serde_json::Value = serde_json::from_str(&output)
        .expect("CLI --json --list-symbols on .ts should output valid JSON");
    let arr = parsed.as_array().unwrap();
    assert!(!arr.is_empty());
    let symbols = arr[0]["symbols"].as_array().unwrap();
    assert!(!symbols.is_empty(), "Expected symbols in TypeScript file");
    // Verify symbol structure
    for sym in symbols {
        assert!(sym.get("kind").is_some(), "Missing 'kind'");
        assert!(sym.get("name").is_some(), "Missing 'name'");
        assert!(sym.get("line").is_some(), "Missing 'line'");
        assert!(sym.get("line_end").is_some(), "Missing 'line_end'");
        assert!(sym.get("visibility").is_some(), "Missing 'visibility'");
    }
}

#[test]
fn test_cli_json_list_symbols_with_no_imports() {
    let output = run_codehud(&["--json", "--list-symbols", "--no-imports", FIXTURE_PATH]);
    let parsed: serde_json::Value = serde_json::from_str(&output)
        .expect("CLI --json --list-symbols --no-imports should output valid JSON");
    let arr = parsed.as_array().unwrap();
    if !arr.is_empty() {
        let symbols = arr[0]["symbols"].as_array().unwrap();
        for sym in symbols {
            assert_ne!(sym["kind"].as_str().unwrap(), "use", "Should not contain 'use' with --no-imports");
        }
    }
}

#[test]
fn test_cli_json_list_symbols_with_pub() {
    let output = run_codehud(&["--json", "--list-symbols", "--pub", FIXTURE_PATH]);
    let parsed: serde_json::Value = serde_json::from_str(&output)
        .expect("CLI --json --list-symbols --pub should output valid JSON");
    let arr = parsed.as_array().unwrap();
    if !arr.is_empty() {
        let symbols = arr[0]["symbols"].as_array().unwrap();
        for sym in symbols {
            assert_eq!(sym["visibility"].as_str().unwrap(), "public",
                "All symbols should be public with --pub flag");
        }
    }
}

#[test]
fn test_cli_json_list_symbols_with_symbol_depth() {
    let output = run_codehud(&["--json", "--list-symbols", "--symbol-depth", "2", "tests/fixtures/sample.ts"]);
    let parsed: serde_json::Value = serde_json::from_str(&output)
        .expect("CLI --json --list-symbols --symbol-depth 2 should output valid JSON");
    let arr = parsed.as_array().unwrap();
    assert!(!arr.is_empty());
}

#[test]
fn test_cli_json_list_symbols_not_plain_text() {
    // Explicitly verify that --json flag changes the output format
    let plain_output = run_codehud(&["--list-symbols", FIXTURE_PATH]);
    let json_output = run_codehud(&["--json", "--list-symbols", FIXTURE_PATH]);
    
    // Plain output should NOT be valid JSON
    assert!(serde_json::from_str::<serde_json::Value>(&plain_output).is_err(),
        "Plain --list-symbols should NOT be valid JSON");
    
    // JSON output MUST be valid JSON  
    assert!(serde_json::from_str::<serde_json::Value>(&json_output).is_ok(),
        "--json --list-symbols MUST be valid JSON, got:\n{}", &json_output[..json_output.len().min(200)]);
}
