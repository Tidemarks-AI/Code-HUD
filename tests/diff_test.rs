//! Integration tests for `--diff` and `--staged` (issues #8 and #11).

use std::fs;
use std::path::Path;
use std::process::Command;

fn git(repo: &Path, args: &[&str]) -> String {
    let out = Command::new("git")
        .args(["-C", &repo.display().to_string()])
        .args(args)
        .output()
        .expect("git failed to run");
    if !out.status.success() {
        panic!(
            "git {:?} failed: {}",
            args,
            String::from_utf8_lossy(&out.stderr)
        );
    }
    String::from_utf8_lossy(&out.stdout).to_string()
}

fn codehud_in(dir: &Path, args: &[&str]) -> std::process::Output {
    Command::new(env!("CARGO_BIN_EXE_codehud"))
        .current_dir(dir)
        .args(args)
        .output()
        .expect("codehud failed to run")
}

/// Create a temp git repo with an initial Rust file committed.
fn setup_repo() -> tempfile::TempDir {
    let dir = tempfile::TempDir::new().unwrap();
    let p = dir.path();
    git(p, &["init"]);
    git(p, &["config", "user.email", "test@test.com"]);
    git(p, &["config", "user.name", "Test"]);

    fs::write(
        p.join("lib.rs"),
        "fn hello() { println!(\"hello\"); }\n\nfn world() { println!(\"world\"); }\n",
    )
    .unwrap();
    git(p, &["add", "."]);
    git(p, &["commit", "-m", "initial"]);
    dir
}

// -------------------------------------------------------------------------
// Basic diff tests
// -------------------------------------------------------------------------

#[test]
fn test_diff_added_function() {
    let dir = setup_repo();
    let p = dir.path();

    // Add a new function in working tree
    fs::write(
        p.join("lib.rs"),
        "fn hello() { println!(\"hello\"); }\n\nfn world() { println!(\"world\"); }\n\nfn new_func() { 42 }\n",
    )
    .unwrap();

    let out = codehud_in(p, &["--diff", "HEAD", "."]);
    let stdout = String::from_utf8_lossy(&out.stdout);
    assert!(stdout.contains("new_func"), "expected new_func in output: {stdout}");
    assert!(stdout.contains("+"), "expected + marker for added symbol");
}

#[test]
fn test_diff_deleted_function() {
    let dir = setup_repo();
    let p = dir.path();

    // Remove `world` function
    fs::write(p.join("lib.rs"), "fn hello() { println!(\"hello\"); }\n").unwrap();

    let out = codehud_in(p, &["--diff", "HEAD", "."]);
    let stdout = String::from_utf8_lossy(&out.stdout);
    assert!(stdout.contains("world"), "expected world in deleted output: {stdout}");
    assert!(stdout.contains("-"), "expected - marker for deleted symbol");
}

#[test]
fn test_diff_modified_function() {
    let dir = setup_repo();
    let p = dir.path();

    // Modify hello body
    fs::write(
        p.join("lib.rs"),
        "fn hello() { println!(\"CHANGED\"); }\n\nfn world() { println!(\"world\"); }\n",
    )
    .unwrap();

    let out = codehud_in(p, &["--diff", "HEAD", "."]);
    let stdout = String::from_utf8_lossy(&out.stdout);
    assert!(stdout.contains("hello"), "expected hello in modified output: {stdout}");
    assert!(stdout.contains("~"), "expected ~ marker for modified symbol");
}

#[test]
fn test_diff_new_file() {
    let dir = setup_repo();
    let p = dir.path();

    // Add a completely new file (unstaged, in working tree)
    fs::write(p.join("new.rs"), "fn brand_new() { 1 }\n").unwrap();

    // The new file must be tracked by git diff (untracked files won't show)
    // For working tree diff, untracked files don't appear in `git diff --name-status HEAD`
    // so this should produce no output for new.rs — that's correct behavior.
    let out = codehud_in(p, &["--diff", "HEAD", "."]);
    let stdout = String::from_utf8_lossy(&out.stdout);
    // Untracked files should NOT appear
    assert!(!stdout.contains("brand_new"), "untracked file should not appear in diff");
}

#[test]
fn test_diff_empty_no_changes() {
    let dir = setup_repo();
    let p = dir.path();

    let out = codehud_in(p, &["--diff", "HEAD", "."]);
    let stdout = String::from_utf8_lossy(&out.stdout);
    assert!(
        stdout.contains("No symbol changes"),
        "expected empty diff message: {stdout}"
    );
}

// -------------------------------------------------------------------------
// Staged changes (issue #8)
// -------------------------------------------------------------------------

#[test]
fn test_staged_added_function() {
    let dir = setup_repo();
    let p = dir.path();

    // Stage a change
    fs::write(
        p.join("lib.rs"),
        "fn hello() { println!(\"hello\"); }\n\nfn world() { println!(\"world\"); }\n\nfn staged_fn() { 99 }\n",
    )
    .unwrap();
    git(p, &["add", "lib.rs"]);

    let out = codehud_in(p, &["--diff", "--staged", "."]);
    let stdout = String::from_utf8_lossy(&out.stdout);
    assert!(stdout.contains("staged_fn"), "expected staged_fn: {stdout}");
    assert!(stdout.contains("+"), "expected + marker");
}

#[test]
fn test_staged_excludes_unstaged() {
    let dir = setup_repo();
    let p = dir.path();

    // Stage one change, leave another unstaged
    fs::write(
        p.join("lib.rs"),
        "fn hello() { println!(\"hello\"); }\n\nfn world() { println!(\"world\"); }\n\nfn staged_fn() { 99 }\n",
    )
    .unwrap();
    git(p, &["add", "lib.rs"]);

    // Now modify working tree further (unstaged)
    fs::write(
        p.join("lib.rs"),
        "fn hello() { println!(\"hello\"); }\n\nfn world() { println!(\"world\"); }\n\nfn staged_fn() { 99 }\n\nfn unstaged_fn() { 0 }\n",
    )
    .unwrap();

    let out = codehud_in(p, &["--diff", "--staged", "."]);
    let stdout = String::from_utf8_lossy(&out.stdout);
    assert!(stdout.contains("staged_fn"), "expected staged_fn: {stdout}");
    assert!(
        !stdout.contains("unstaged_fn"),
        "unstaged_fn should NOT appear in --staged output: {stdout}"
    );
}

#[test]
fn test_staged_new_file() {
    let dir = setup_repo();
    let p = dir.path();

    fs::write(p.join("added.rs"), "fn brand_new() { 1 }\n").unwrap();
    git(p, &["add", "added.rs"]);

    let out = codehud_in(p, &["--diff", "--staged", "."]);
    let stdout = String::from_utf8_lossy(&out.stdout);
    assert!(stdout.contains("brand_new"), "expected brand_new in staged new file: {stdout}");
}

#[test]
fn test_staged_deleted_file() {
    let dir = setup_repo();
    let p = dir.path();

    git(p, &["rm", "lib.rs"]);

    let out = codehud_in(p, &["--diff", "--staged", "."]);
    let stdout = String::from_utf8_lossy(&out.stdout);
    assert!(stdout.contains("hello"), "expected deleted symbols: {stdout}");
    assert!(stdout.contains("-"), "expected - marker for deleted");
}

// -------------------------------------------------------------------------
// JSON output
// -------------------------------------------------------------------------

#[test]
fn test_diff_json_output() {
    let dir = setup_repo();
    let p = dir.path();

    fs::write(
        p.join("lib.rs"),
        "fn hello() { println!(\"hello\"); }\n\nfn world() { println!(\"world\"); }\n\nfn extra() { 1 }\n",
    )
    .unwrap();

    let out = codehud_in(p, &["--diff", "HEAD", "--json", "."]);
    let stdout = String::from_utf8_lossy(&out.stdout);
    let parsed: serde_json::Value = serde_json::from_str(&stdout)
        .unwrap_or_else(|e| panic!("invalid JSON: {e}\n{stdout}"));
    assert!(parsed.is_array(), "expected JSON array");
    let arr = parsed.as_array().unwrap();
    assert!(!arr.is_empty(), "expected at least one entry");
    // Check structure
    let first = &arr[0];
    assert!(first.get("file").is_some());
    assert!(first.get("symbol").is_some());
    assert!(first.get("change_type").is_some());
    assert!(first.get("kind").is_some());
}

#[test]
fn test_staged_json_output() {
    let dir = setup_repo();
    let p = dir.path();

    fs::write(
        p.join("lib.rs"),
        "fn hello() { println!(\"MODIFIED\"); }\n\nfn world() { println!(\"world\"); }\n",
    )
    .unwrap();
    git(p, &["add", "lib.rs"]);

    let out = codehud_in(p, &["--diff", "--staged", "--json", "."]);
    let stdout = String::from_utf8_lossy(&out.stdout);
    let parsed: serde_json::Value = serde_json::from_str(&stdout)
        .unwrap_or_else(|e| panic!("invalid JSON: {e}\n{stdout}"));
    assert!(parsed.is_array());
    let arr = parsed.as_array().unwrap();
    assert!(arr.iter().any(|e| e["change_type"] == "modified"), "expected modified entry");
}

// -------------------------------------------------------------------------
// Filter composition
// -------------------------------------------------------------------------

#[test]
fn test_diff_ext_filter() {
    let dir = setup_repo();
    let p = dir.path();

    // Add a .py file and a .rs change
    fs::write(p.join("script.py"), "def new_py():\n    pass\n").unwrap();
    git(p, &["add", "script.py"]);
    git(p, &["commit", "-m", "add py"]);

    // Now modify both
    fs::write(p.join("script.py"), "def new_py():\n    return 1\ndef another():\n    pass\n").unwrap();
    fs::write(
        p.join("lib.rs"),
        "fn hello() { println!(\"changed\"); }\n\nfn world() { println!(\"world\"); }\n",
    )
    .unwrap();

    // Filter to only .py
    let out = codehud_in(p, &["--diff", "HEAD", "--ext", "py", "."]);
    let stdout = String::from_utf8_lossy(&out.stdout);
    assert!(
        !stdout.contains("hello"),
        "rs file should be filtered out: {stdout}"
    );
}

#[test]
fn test_diff_no_tests_filter() {
    let dir = setup_repo();
    let p = dir.path();

    fs::write(
        p.join("lib.rs"),
        "fn hello() { println!(\"hello\"); }\n\nfn world() { println!(\"world\"); }\n\nfn test_something() { assert!(true); }\n",
    )
    .unwrap();

    let out = codehud_in(p, &["--diff", "HEAD", "--no-tests", "."]);
    let stdout = String::from_utf8_lossy(&out.stdout);
    assert!(
        !stdout.contains("test_something"),
        "test symbol should be filtered: {stdout}"
    );
}

// -------------------------------------------------------------------------
// Renamed files
// -------------------------------------------------------------------------

#[test]
fn test_diff_renamed_file() {
    let dir = setup_repo();
    let p = dir.path();

    // Rename via git mv
    git(p, &["mv", "lib.rs", "renamed.rs"]);
    git(p, &["commit", "-m", "rename"]);

    // Diff against previous commit
    let out = codehud_in(p, &["--diff", "HEAD~1", "."]);
    let stdout = String::from_utf8_lossy(&out.stdout);
    // Renamed file with same content → no symbol changes (or shows the rename)
    // Either "No symbol changes" or the file appears — both are fine
    assert!(out.status.success(), "command should succeed");
    let _ = stdout; // just assert it doesn't crash
}

// -------------------------------------------------------------------------
// Error cases
// -------------------------------------------------------------------------

#[test]
fn test_diff_not_a_git_repo() {
    let dir = tempfile::TempDir::new().unwrap();
    let p = dir.path();
    fs::write(p.join("file.rs"), "fn foo() {}").unwrap();

    let out = codehud_in(p, &["--diff", "HEAD", "."]);
    assert!(!out.status.success(), "should fail in non-git dir");
    let stderr = String::from_utf8_lossy(&out.stderr);
    assert!(
        stderr.contains("git error") || stderr.contains("Error"),
        "expected error message: {stderr}"
    );
}

#[test]
fn test_diff_invalid_ref() {
    let dir = setup_repo();
    let p = dir.path();

    let out = codehud_in(p, &["--diff", "nonexistent-ref-xyz", "."]);
    assert!(!out.status.success(), "should fail with invalid ref");
}

// -------------------------------------------------------------------------
// Binary files (should not crash)
// -------------------------------------------------------------------------

#[test]
fn test_diff_binary_file_no_crash() {
    let dir = setup_repo();
    let p = dir.path();

    // Add a binary file
    fs::write(p.join("image.png"), &[0x89, 0x50, 0x4E, 0x47, 0x0D, 0x0A, 0x1A, 0x0A]).unwrap();
    git(p, &["add", "image.png"]);
    git(p, &["commit", "-m", "add binary"]);

    // Modify it
    fs::write(p.join("image.png"), &[0x89, 0x50, 0x4E, 0x47, 0xFF, 0xFF]).unwrap();

    let out = codehud_in(p, &["--diff", "HEAD", "."]);
    assert!(out.status.success(), "should not crash on binary files");
}

// -------------------------------------------------------------------------
// TypeScript diff
// -------------------------------------------------------------------------

#[test]
fn test_diff_typescript() {
    let dir = setup_repo();
    let p = dir.path();

    fs::write(
        p.join("app.ts"),
        "export function greet(): string { return 'hi'; }\n",
    )
    .unwrap();
    git(p, &["add", "."]);
    git(p, &["commit", "-m", "add ts"]);

    fs::write(
        p.join("app.ts"),
        "export function greet(): string { return 'hello'; }\nexport function farewell(): string { return 'bye'; }\n",
    )
    .unwrap();

    let out = codehud_in(p, &["--diff", "HEAD", "."]);
    let stdout = String::from_utf8_lossy(&out.stdout);
    assert!(stdout.contains("farewell"), "expected farewell added: {stdout}");
}

// -------------------------------------------------------------------------
// Python diff
// -------------------------------------------------------------------------

#[test]
fn test_diff_python() {
    let dir = setup_repo();
    let p = dir.path();

    fs::write(p.join("app.py"), "def greet():\n    return 'hi'\n").unwrap();
    git(p, &["add", "."]);
    git(p, &["commit", "-m", "add py"]);

    fs::write(
        p.join("app.py"),
        "def greet():\n    return 'hello'\ndef farewell():\n    return 'bye'\n",
    )
    .unwrap();

    let out = codehud_in(p, &["--diff", "HEAD", "."]);
    let stdout = String::from_utf8_lossy(&out.stdout);
    assert!(stdout.contains("farewell"), "expected farewell: {stdout}");
}
