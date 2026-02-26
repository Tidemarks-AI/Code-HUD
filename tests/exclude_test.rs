use codehud::{process_path, ProcessOptions, OutputFormat};

fn default_options() -> ProcessOptions {
    ProcessOptions {
        symbols: vec![],
        pub_only: false,
        fns_only: false,
        types_only: false,
        no_tests: false,
        depth: None,
        format: OutputFormat::Plain,
        stats: false,
        summary_only: false,
        ext: vec![],
        signatures: false,
        max_lines: None,
        list_symbols: false,
        no_imports: false,
        smart_depth: false,
        symbol_depth: None,
        exclude: vec![],
        outline: false,
    }
}

use std::fs;
use tempfile::TempDir;

fn setup_test_dir() -> TempDir {
    let dir = TempDir::new().unwrap();
    // Create .git so codehud treats it as a project root
    fs::create_dir(dir.path().join(".git")).unwrap();
    fs::create_dir_all(dir.path().join("src")).unwrap();
    fs::create_dir_all(dir.path().join("dist")).unwrap();
    fs::create_dir_all(dir.path().join("generated")).unwrap();

    fs::write(dir.path().join("src/main.rs"), "fn main() {}\nfn helper() {}\n").unwrap();
    fs::write(dir.path().join("dist/bundle.js"), "function main() {}\n").unwrap();
    fs::write(dir.path().join("generated/types.ts"), "export interface Foo { bar: string; }\n").unwrap();
    dir
}

#[test]
fn exclude_single_directory() {
    let dir = setup_test_dir();
    let options = ProcessOptions {
        exclude: vec!["dist".to_string()],
        outline: false,
        ..default_options()
    };
    let output = process_path(dir.path().to_str().unwrap(), options).unwrap();
    assert!(output.contains("main.rs"), "should include src/main.rs");
    assert!(!output.contains("bundle.js"), "should exclude dist/bundle.js");
    assert!(output.contains("types.ts"), "should include generated/types.ts");
}

#[test]
fn exclude_multiple_directories() {
    let dir = setup_test_dir();
    let options = ProcessOptions {
        exclude: vec!["dist".to_string(), "generated".to_string()],
        outline: false,
        ..default_options()
    };
    let output = process_path(dir.path().to_str().unwrap(), options).unwrap();
    assert!(output.contains("main.rs"), "should include src/main.rs");
    assert!(!output.contains("bundle.js"), "should exclude dist/bundle.js");
    assert!(!output.contains("types.ts"), "should exclude generated/types.ts");
}

#[test]
fn exclude_glob_pattern() {
    let dir = setup_test_dir();
    let options = ProcessOptions {
        exclude: vec!["*.js".to_string()],
        outline: false,
        ..default_options()
    };
    let output = process_path(dir.path().to_str().unwrap(), options).unwrap();
    assert!(output.contains("main.rs"), "should include .rs files");
    assert!(!output.contains("bundle.js"), "should exclude .js files via glob");
    assert!(output.contains("types.ts"), "should include .ts files");
}

#[test]
fn exclude_with_ext_filter() {
    let dir = setup_test_dir();
    // --ext rs --exclude src should result in nothing (only .rs files are in src/)
    let options = ProcessOptions {
        ext: vec!["rs".to_string()],
        exclude: vec!["src".to_string()],
        outline: false,
        ..default_options()
    };
    let output = process_path(dir.path().to_str().unwrap(), options).unwrap();
    assert!(!output.contains("main.rs"), "should exclude src/main.rs");
}

#[test]
fn exclude_with_search() {
    let dir = setup_test_dir();
    let search_opts = codehud::search::SearchOptions {
        pattern: "main".to_string(),
        regex: false,
        case_insensitive: false,
        depth: None,
        ext: vec![],
        max_results: None,
        no_tests: false,
        exclude: vec!["dist".to_string()],
        json: false,
    };
    let output = codehud::search::search_path(dir.path().to_str().unwrap(), &search_opts).unwrap();
    assert!(output.contains("main.rs"), "should find main in src/main.rs");
    assert!(!output.contains("bundle.js"), "should not search in dist/");
}

#[test]
fn exclude_with_list_symbols() {
    let dir = setup_test_dir();
    let options = ProcessOptions {
        list_symbols: true,
        exclude: vec!["dist".to_string(), "generated".to_string()],
        ..default_options()
    };
    let output = process_path(dir.path().to_str().unwrap(), options).unwrap();
    assert!(output.contains("main"), "should list symbols from src/main.rs");
    assert!(!output.contains("bundle"), "should not list symbols from dist/");
}

#[test]
fn exclude_with_tree_mode() {
    let dir = setup_test_dir();
    let tree_opts = codehud::tree::TreeOptions {
        depth: None,
        ext: vec![],
        stats: false,
        json: false,
        smart_depth: false,
        no_tests: false,
        exclude: vec!["dist".to_string()],
    };
    let output = codehud::tree::tree_view(dir.path().to_str().unwrap(), &tree_opts).unwrap();
    assert!(output.contains("src/"), "should show src/");
    assert!(!output.contains("dist/"), "should not show dist/");
    assert!(output.contains("generated/"), "should show generated/");
}

#[test]
fn exclude_wildcard_path_pattern() {
    let dir = TempDir::new().unwrap();
    fs::create_dir(dir.path().join(".git")).unwrap();
    fs::create_dir_all(dir.path().join("packages/foo/src")).unwrap();
    fs::create_dir_all(dir.path().join("packages/foo/migrations")).unwrap();
    fs::write(dir.path().join("packages/foo/src/lib.rs"), "fn foo() {}\n").unwrap();
    fs::write(dir.path().join("packages/foo/migrations/001.rs"), "fn migrate() {}\n").unwrap();

    let options = ProcessOptions {
        exclude: vec!["*/migrations/*".to_string()],
        outline: false,
        ..default_options()
    };
    let output = process_path(dir.path().to_str().unwrap(), options).unwrap();
    assert!(output.contains("lib.rs"), "should include src files");
    assert!(!output.contains("001.rs"), "should exclude migration files via glob");
}
