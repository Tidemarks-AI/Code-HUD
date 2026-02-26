use crate::error::CodehudError;
use crate::handler::{self, LanguageHandler};
use crate::languages::{self, Language};
use crate::parser;
use crate::walk;
use regex::{Regex, RegexBuilder};
use serde::Serialize;
use std::collections::BTreeMap;
use std::fmt::Write;
use std::fs;
use std::path::Path;
use tree_sitter::{Node, Tree};

/// A single search match with its line number, content, and enclosing symbol path.
#[derive(Debug, Clone)]
pub struct SearchMatch {
    pub line_number: usize,
    pub line_content: String,
    pub symbol_path: Vec<String>,
}

/// Options for structural search.
pub struct SearchOptions {
    pub pattern: String,
    pub regex: bool,
    pub case_insensitive: bool,
    pub depth: Option<usize>,
    pub ext: Vec<String>,
    pub max_results: Option<usize>,
    pub no_tests: bool,
    pub exclude: Vec<String>,
    pub json: bool,
}

/// Perform structural search on a path (file or directory).
pub fn search_path(
    path: &str,
    options: &SearchOptions,
) -> Result<String, CodehudError> {
    let effective_pattern = if options.regex {
        // Regex mode: normalize grep/sed-style escaped alternation (`\|`) to regex alternation (`|`).
        // In POSIX BRE and many CLI tools, `\|` means alternation, but the `regex` crate
        // treats `\|` as a literal pipe. Convert so both syntaxes work.
        options.pattern.replace(r"\|", "|")
    } else {
        // Literal mode (default): escape all regex metacharacters
        regex::escape(&options.pattern)
    };

    let regex = RegexBuilder::new(&effective_pattern)
        .case_insensitive(options.case_insensitive)
        .build()
        .map_err(|e| CodehudError::ParseError(format!("Invalid regex pattern: {}", e)))?;

    let path = Path::new(path);
    if !path.exists() {
        return Err(CodehudError::PathNotFound(path.display().to_string()));
    }

    let file_results: Vec<(String, Vec<SearchMatch>)> = if path.is_file() {
        let matches = search_any_file(path, &regex)?;
        if matches.is_empty() {
            vec![]
        } else {
            vec![(path.to_string_lossy().to_string(), matches)]
        }
    } else if path.is_dir() {
        let files = walk::walk_directory(path, options.depth, &options.ext)?;
        let files = walk::filter_excludes(files, path, &options.exclude);
        let mut results = Vec::new();
        for file_path in files {
            if options.no_tests && crate::test_detect::is_test_file_any_language(&file_path) {
                continue;
            }
            match search_any_file(&file_path, &regex) {
                Ok(matches) if !matches.is_empty() => {
                    results.push((file_path.to_string_lossy().to_string(), matches));
                }
                _ => {}
            }
        }
        results
    } else {
        return Err(CodehudError::InvalidPath(path.display().to_string()));
    };

    // Apply max_results cap
    if let Some(max) = options.max_results {
        let total_matches: usize = file_results.iter().map(|(_, m)| m.len()).sum();
        if total_matches > max {
            let overflow = total_matches - max;
            let mut kept = 0;
            let mut capped_results: Vec<(String, Vec<SearchMatch>)> = Vec::new();
            let mut overflow_files = 0usize;

            for (file_path, matches) in file_results {
                if kept >= max {
                    overflow_files += 1;
                    continue;
                }
                let remaining = max - kept;
                if matches.len() <= remaining {
                    kept += matches.len();
                    capped_results.push((file_path, matches));
                } else {
                    let taken: Vec<SearchMatch> = matches.into_iter().take(remaining).collect();
                    kept += taken.len();
                    capped_results.push((file_path, taken));
                }
            }

            // Count how many files had matches that were completely excluded
            let total_files_with_matches = capped_results.len() + overflow_files;
            let shown_files = capped_results.len();
            let extra_files = total_files_with_matches - shown_files;

            if options.json {
                return Ok(format_search_json(&capped_results));
            }
            let mut output = format_search_results(&capped_results);
            writeln!(output, "\n... and {} more matches across {} files", overflow, extra_files).unwrap();
            return Ok(output);
        }
    }

    if options.json {
        Ok(format_search_json(&file_results))
    } else {
        Ok(format_search_results(&file_results))
    }
}

/// Search a single file and return matches with structural context.
/// Search a file, dispatching to SFC, code (with AST), or plain text search.
fn search_any_file(
    path: &Path,
    regex: &Regex,
) -> Result<Vec<SearchMatch>, CodehudError> {
    // SFC files: extract script blocks, search within each
    if let Some(sfc_kind) = crate::sfc::detect_sfc(path) {
        return search_sfc_file(path, regex, sfc_kind);
    }
    // Try code files with AST-based structural context
    match languages::detect_language(path) {
        Ok(lang) => search_file(path, regex, lang),
        Err(_) => {
            // Non-code text file: plain text search without structural context
            if languages::is_text_file(path) {
                search_plain_text_file(path, regex)
            } else {
                // Not a recognised text file at all — skip silently
                Ok(vec![])
            }
        }
    }
}

/// Search a plain text file (YAML, JSON, Markdown, .env, etc.) line by line.
/// No AST parsing — matches have empty symbol paths.
fn search_plain_text_file(
    path: &Path,
    regex: &Regex,
) -> Result<Vec<SearchMatch>, CodehudError> {
    let source = fs::read_to_string(path).map_err(|e| CodehudError::ReadError {
        path: path.display().to_string(),
        source: e,
    })?;

    let mut matches = Vec::new();
    for (idx, line) in source.lines().enumerate() {
        if regex.is_match(line) {
            matches.push(SearchMatch {
                line_number: idx + 1,
                line_content: line.to_string(),
                symbol_path: vec![],
            });
        }
    }
    Ok(matches)
}

/// Search within SFC file script blocks (Vue/Svelte/Astro).
fn search_sfc_file(
    path: &Path,
    regex: &Regex,
    sfc_kind: crate::sfc::SfcKind,
) -> Result<Vec<SearchMatch>, CodehudError> {
    let source = fs::read_to_string(path).map_err(|e| CodehudError::ReadError {
        path: path.display().to_string(),
        source: e,
    })?;

    let all_lines: Vec<&str> = source.lines().collect();
    let blocks = crate::sfc::extract_scripts(&source, sfc_kind);
    let mut matches = Vec::new();

    for block in &blocks {
        let tree = parser::parse(&block.content, block.language)?;
        let block_lines: Vec<&str> = block.content.lines().collect();

        for (idx, line) in block_lines.iter().enumerate() {
            if regex.is_match(line) {
                let original_line = block.start_line + idx; // 1-indexed
                let symbol_path = find_enclosing_symbols(&tree, &block.content, idx, block.language);
                // Use the original file line content for display
                let display_line = if original_line > 0 && original_line <= all_lines.len() {
                    all_lines[original_line - 1].to_string()
                } else {
                    line.to_string()
                };
                matches.push(SearchMatch {
                    line_number: original_line,
                    line_content: display_line,
                    symbol_path,
                });
            }
        }
    }

    // Also search non-script lines (template, style) as plain text
    let script_lines: std::collections::HashSet<usize> = blocks.iter().flat_map(|b| {
        let end = b.start_line + b.content.lines().count();
        b.start_line..end
    }).collect();

    for (idx, line) in all_lines.iter().enumerate() {
        let line_num = idx + 1;
        if !script_lines.contains(&line_num) && regex.is_match(line) {
            matches.push(SearchMatch {
                line_number: line_num,
                line_content: line.to_string(),
                symbol_path: vec![],
            });
        }
    }

    matches.sort_by_key(|m| m.line_number);
    Ok(matches)
}

fn search_file(
    path: &Path,
    regex: &Regex,
    language: Language,
) -> Result<Vec<SearchMatch>, CodehudError> {
    let source = fs::read_to_string(path).map_err(|e| CodehudError::ReadError {
        path: path.display().to_string(),
        source: e,
    })?;

    let tree = parser::parse(&source, language)?;
    let lines: Vec<&str> = source.lines().collect();

    let mut matches = Vec::new();
    for (idx, line) in lines.iter().enumerate() {
        if regex.is_match(line) {
            let line_number = idx + 1; // 1-indexed
            let symbol_path = find_enclosing_symbols(&tree, &source, idx, language);
            matches.push(SearchMatch {
                line_number,
                line_content: line.to_string(),
                symbol_path,
            });
        }
    }

    Ok(matches)
}

/// Find the enclosing symbol hierarchy for a given line (0-indexed).
pub fn find_enclosing_symbols(
    tree: &Tree,
    source: &str,
    line_idx: usize,
    language: Language,
) -> Vec<String> {
    let handler = handler::handler_for(language);
    let root = tree.root_node();
    let mut symbols = Vec::new();
    if let Some(ref h) = handler {
        find_symbols_at_line(root, source, line_idx, h.as_ref(), &mut symbols);
    }
    symbols
}

/// Recursively find named symbols that contain the given line.
fn find_symbols_at_line(
    node: Node,
    source: &str,
    line_idx: usize,
    handler: &dyn LanguageHandler,
    symbols: &mut Vec<String>,
) {
    let start_line = node.start_position().row;
    let end_line = node.end_position().row;

    if line_idx < start_line || line_idx > end_line {
        return;
    }

    // Check if this node is a named symbol
    if let Some(info) = handler.classify_node(node, source) {
        if let Some(name) = info.name {
            // Format impl blocks with "impl" prefix for readability
            if matches!(info.kind, crate::extractor::ItemKind::Impl) {
                symbols.push(format!("impl {}", name));
            } else if matches!(info.kind, crate::extractor::ItemKind::Function | crate::extractor::ItemKind::Method) {
                symbols.push(format!("{}()", name));
            } else {
                symbols.push(name);
            }
        }
    } else {
        // For nodes not classified at top level (e.g., method_definition inside a class),
        // check if this is a method-like node with a name field
        let kind = node.kind();
        match kind {
            "method_definition" | "function_item" | "function_definition" | "function_declaration" => {
                if let Some(name_node) = node.child_by_field_name("name") {
                    let name = source[name_node.byte_range()].to_string();
                    symbols.push(format!("{}()", name));
                }
            }
            _ => {}
        }
    }

    // Recurse into children
    let mut cursor = node.walk();
    for child in node.children(&mut cursor) {
        find_symbols_at_line(child, source, line_idx, handler, symbols);
    }
}

/// Format search results grouped by file and enclosing symbol.
fn format_search_json(file_results: &[(String, Vec<SearchMatch>)]) -> String {
    #[derive(Serialize)]
    struct JsonMatch {
        file: String,
        line: usize,
        content: String,
        #[serde(skip_serializing_if = "Vec::is_empty")]
        symbol_path: Vec<String>,
    }

    let mut lines = Vec::new();
    for (file_path, matches) in file_results {
        for m in matches {
            let entry = JsonMatch {
                file: file_path.clone(),
                line: m.line_number,
                content: m.line_content.clone(),
                symbol_path: m.symbol_path.clone(),
            };
            lines.push(serde_json::to_string(&entry).unwrap());
        }
    }
    lines.join("\n")
}

fn format_search_results(file_results: &[(String, Vec<SearchMatch>)]) -> String {
    let mut output = String::new();

    for (i, (file_path, matches)) in file_results.iter().enumerate() {
        if i > 0 {
            output.push('\n');
        }
        writeln!(output, "{}", file_path).unwrap();

        // Group matches by symbol path
        let mut groups: BTreeMap<String, Vec<&SearchMatch>> = BTreeMap::new();
        let mut order: Vec<String> = Vec::new();

        for m in matches {
            let key = if m.symbol_path.is_empty() {
                "(top-level)".to_string()
            } else {
                m.symbol_path.join(" > ")
            };
            if !groups.contains_key(&key) {
                order.push(key.clone());
            }
            groups.entry(key).or_default().push(m);
        }

        for key in &order {
            let group = &groups[key];
            writeln!(output).unwrap();
            writeln!(output, "  {}", key).unwrap();
            for m in group {
                writeln!(output, "    L{}:{}", m.line_number, m.line_content).unwrap();
            }
        }
    }

    output
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use tempfile::TempDir;

    fn write_ts_file(dir: &TempDir, name: &str, content: &str) -> String {
        let path = dir.path().join(name);
        fs::write(&path, content).unwrap();
        path.to_string_lossy().to_string()
    }

    fn write_rs_file(dir: &TempDir, name: &str, content: &str) -> String {
        let path = dir.path().join(name);
        fs::write(&path, content).unwrap();
        path.to_string_lossy().to_string()
    }

    #[test]
    fn test_basic_search_rust() {
        let dir = TempDir::new().unwrap();
        let path = write_rs_file(&dir, "test.rs", r#"fn hello() {
    println!("world");
}

fn goodbye() {
    println!("farewell");
}
"#);
        let opts = SearchOptions {
            pattern: "println".to_string(),
            regex: false,
            case_insensitive: false,
            depth: None,
            ext: vec![],
            max_results: None,
            no_tests: false,
            exclude: vec![],
            json: false,
        };
        let result = search_path(&path, &opts).unwrap();
        assert!(result.contains("hello"));
        assert!(result.contains("goodbye"));
        assert!(result.contains("L2:"));
        assert!(result.contains("L6:"));
    }

    #[test]
    fn test_case_insensitive() {
        let dir = TempDir::new().unwrap();
        let path = write_rs_file(&dir, "test.rs", r#"fn hello() {
    let Message = "hi";
}
"#);
        // Case-sensitive: should not match "Message" with pattern "message"
        let opts = SearchOptions {
            pattern: "message".to_string(),
            regex: false,
            case_insensitive: false,
            depth: None,
            ext: vec![],
            max_results: None,
            no_tests: false,
            exclude: vec![],
            json: false,
        };
        let result = search_path(&path, &opts).unwrap();
        assert!(!result.contains("Message"));

        // Case-insensitive: should match
        let opts = SearchOptions {
            pattern: "message".to_string(),
            regex: false,
            case_insensitive: true,
            depth: None,
            ext: vec![],
            max_results: None,
            no_tests: false,
            exclude: vec![],
            json: false,
        };
        let result = search_path(&path, &opts).unwrap();
        assert!(result.contains("Message"));
    }

    #[test]
    fn test_regex_pattern() {
        let dir = TempDir::new().unwrap();
        let path = write_rs_file(&dir, "test.rs", r#"fn process() {
    let x = 42;
    let y = 100;
    let z = "hello";
}
"#);
        let opts = SearchOptions {
            pattern: r"let \w+ = \d+".to_string(),
            regex: true,
            case_insensitive: false,
            depth: None,
            ext: vec![],
            max_results: None,
            no_tests: false,
            exclude: vec![],
            json: false,
        };
        let result = search_path(&path, &opts).unwrap();
        assert!(result.contains("L2:"));
        assert!(result.contains("L3:"));
        assert!(!result.contains("L4:")); // "hello" is not digits
    }

    #[test]
    fn test_directory_search() {
        let dir = TempDir::new().unwrap();
        fs::create_dir(dir.path().join(".git")).unwrap();
        write_rs_file(&dir, "a.rs", "fn foo() {\n    target_word();\n}\n");
        write_rs_file(&dir, "b.rs", "fn bar() {\n    other();\n}\n");
        let opts = SearchOptions {
            pattern: "target_word".to_string(),
            regex: false,
            case_insensitive: false,
            depth: None,
            ext: vec![],
            max_results: None,
            no_tests: false,
            exclude: vec![],
            json: false,
        };
        let result = search_path(&dir.path().to_string_lossy().as_ref(), &opts).unwrap();
        assert!(result.contains("a.rs"));
        assert!(!result.contains("b.rs"));
    }

    #[test]
    fn test_no_matches() {
        let dir = TempDir::new().unwrap();
        let path = write_rs_file(&dir, "test.rs", "fn hello() {}\n");
        let opts = SearchOptions {
            pattern: "nonexistent".to_string(),
            regex: false,
            case_insensitive: false,
            depth: None,
            ext: vec![],
            max_results: None,
            no_tests: false,
            exclude: vec![],
            json: false,
        };
        let result = search_path(&path, &opts).unwrap();
        assert!(result.is_empty());
    }

    #[test]
    fn test_top_level_matches() {
        let dir = TempDir::new().unwrap();
        let path = write_rs_file(&dir, "test.rs", "use std::io;\nfn hello() {}\n");
        let opts = SearchOptions {
            pattern: "std::io".to_string(),
            regex: false,
            case_insensitive: false,
            depth: None,
            ext: vec![],
            max_results: None,
            no_tests: false,
            exclude: vec![],
            json: false,
        };
        let result = search_path(&path, &opts).unwrap();
        assert!(result.contains("(top-level)"));
    }

    #[test]
    fn test_typescript_class_method() {
        let dir = TempDir::new().unwrap();
        let path = write_ts_file(&dir, "test.ts", r#"class MyClass {
    run() {
        this.enqueue("data");
    }

    enqueue(data: string) {
        console.log(data);
    }
}
"#);
        let opts = SearchOptions {
            pattern: "enqueue".to_string(),
            regex: false,
            case_insensitive: false,
            depth: None,
            ext: vec![],
            max_results: None,
            no_tests: false,
            exclude: vec![],
            json: false,
        };
        let result = search_path(&path, &opts).unwrap();
        assert!(result.contains("MyClass"));
        assert!(result.contains("run()"));
        assert!(result.contains("enqueue()"));
    }

    #[test]
    fn test_nested_rust_impl() {
        let dir = TempDir::new().unwrap();
        let path = write_rs_file(&dir, "test.rs", r#"struct Foo;

impl Foo {
    fn bar(&self) {
        self.do_thing();
    }
}
"#);
        let opts = SearchOptions {
            pattern: "do_thing".to_string(),
            regex: false,
            case_insensitive: false,
            depth: None,
            ext: vec![],
            max_results: None,
            no_tests: false,
            exclude: vec![],
            json: false,
        };
        let result = search_path(&path, &opts).unwrap();
        assert!(result.contains("impl Foo"));
        assert!(result.contains("bar"));
    }

    #[test]
    fn test_max_results_caps_directory_search() {
        let dir = TempDir::new().unwrap();
        fs::create_dir(dir.path().join(".git")).unwrap();
        // Create files with many matches
        write_rs_file(&dir, "a.rs", "fn f1() { target(); }\nfn f2() { target(); }\nfn f3() { target(); }\n");
        write_rs_file(&dir, "b.rs", "fn g1() { target(); }\nfn g2() { target(); }\nfn g3() { target(); }\n");
        let opts = SearchOptions {
            pattern: "target".to_string(),
            regex: false,
            case_insensitive: false,
            depth: None,
            ext: vec![],
            max_results: Some(3),
            no_tests: false,
            exclude: vec![],
            json: false,
        };
        let result = search_path(&dir.path().to_string_lossy().as_ref(), &opts).unwrap();
        // Should contain the summary line
        assert!(result.contains("... and 3 more matches across"));
    }

    #[test]
    fn test_max_results_no_cap_when_under_limit() {
        let dir = TempDir::new().unwrap();
        let path = write_rs_file(&dir, "test.rs", "fn foo() { target(); }\n");
        let opts = SearchOptions {
            pattern: "target".to_string(),
            regex: false,
            case_insensitive: false,
            depth: None,
            ext: vec![],
            max_results: Some(10),
            no_tests: false,
            exclude: vec![],
            json: false,
        };
        let result = search_path(&path, &opts).unwrap();
        assert!(!result.contains("... and"));
    }

    #[test]
    fn test_single_file_no_default_cap() {
        let dir = TempDir::new().unwrap();
        // 25 matches in a single file - should all show (no default cap for single file)
        let mut content = String::new();
        for i in 0..25 {
            content.push_str(&format!("fn f{}() {{ target(); }}\n", i));
        }
        let path = write_rs_file(&dir, "test.rs", &content);
        let opts = SearchOptions {
            pattern: "target".to_string(),
            regex: false,
            case_insensitive: false,
            depth: None,
            ext: vec![],
            max_results: None, // single-file default: no cap
            no_tests: false,
            exclude: vec![],
            json: false,
        };
        let result = search_path(&path, &opts).unwrap();
        assert!(!result.contains("... and"));
        // All 25 matches should be present
        assert!(result.contains("f24"));
    }

    #[test]
    fn test_regex_or_alternation() {
        let dir = TempDir::new().unwrap();
        let path = write_rs_file(&dir, "test.rs", r#"fn check() {
    // TODO: fix this
    // FIXME: and this
    // HACK: workaround
    println!("clean line");
}
"#);
        // Standard regex alternation with |
        let opts = SearchOptions {
            pattern: "TODO|FIXME|HACK".to_string(),
            regex: true,
            case_insensitive: false,
            depth: None,
            ext: vec![],
            max_results: None,
            no_tests: false,
            exclude: vec![],
            json: false,
        };
        let result = search_path(&path, &opts).unwrap();
        assert!(result.contains("L2:"), "should match TODO line");
        assert!(result.contains("L3:"), "should match FIXME line");
        assert!(result.contains("L4:"), "should match HACK line");
        assert!(!result.contains("L5:"), "should not match clean line");
    }

    #[test]
    fn test_regex_or_backslash_pipe_syntax() {
        let dir = TempDir::new().unwrap();
        let path = write_rs_file(&dir, "test.rs", r#"fn check() {
    // TODO: fix this
    // FIXME: and this
    println!("clean line");
}
"#);
        // grep/sed-style \| alternation
        let opts = SearchOptions {
            pattern: r"TODO\|FIXME".to_string(),
            regex: true,
            case_insensitive: false,
            depth: None,
            ext: vec![],
            max_results: None,
            no_tests: false,
            exclude: vec![],
            json: false,
        };
        let result = search_path(&path, &opts).unwrap();
        assert!(result.contains("L2:"), "should match TODO line with \\| syntax");
        assert!(result.contains("L3:"), "should match FIXME line with \\| syntax");
        assert!(!result.contains("L4:"), "should not match clean line");
    }

    #[test]
    fn test_regex_or_case_insensitive() {
        let dir = TempDir::new().unwrap();
        let path = write_rs_file(&dir, "test.rs", r#"fn check() {
    // todo: lowercase
    // FIXME: uppercase
}
"#);
        let opts = SearchOptions {
            pattern: "todo|fixme".to_string(),
            regex: true,
            case_insensitive: true,
            depth: None,
            ext: vec![],
            max_results: None,
            no_tests: false,
            exclude: vec![],
            json: false,
        };
        let result = search_path(&path, &opts).unwrap();
        assert!(result.contains("L2:"), "case-insensitive should match todo");
        assert!(result.contains("L3:"), "case-insensitive should match FIXME");
    }

    #[test]
    fn test_no_tests_excludes_test_files_from_search() {
        let dir = TempDir::new().unwrap();
        fs::create_dir(dir.path().join(".git")).unwrap();
        fs::create_dir_all(dir.path().join("src/__tests__")).unwrap();
        write_rs_file(&dir, "src/main.rs", "fn main() {\n    target_word();\n}\n");
        write_ts_file(&dir, "src/utils.test.ts", "function target_word() {}\n");
        write_ts_file(&dir, "src/__tests__/foo.ts", "function target_word() {}\n");

        let dir_str = dir.path().to_string_lossy().to_string();

        // With no_tests: test files should be excluded from search
        let opts = SearchOptions {
            pattern: "target_word".to_string(),
            regex: false,
            case_insensitive: false,
            depth: None,
            ext: vec![],
            max_results: None,
            no_tests: true,
            exclude: vec![],
            json: false,
        };
        let result = search_path(&dir_str, &opts).unwrap();
        assert!(result.contains("main.rs"), "non-test file should appear in search results");
        assert!(!result.contains("utils.test.ts"), "--no-tests should exclude .test.ts from search");
        assert!(!result.contains("__tests__"), "--no-tests should exclude __tests__ dir from search");
    }

    // --- Non-code file search tests ---

    fn write_file(dir: &TempDir, name: &str, content: &str) -> String {
        let path = dir.path().join(name);
        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent).unwrap();
        }
        fs::write(&path, content).unwrap();
        path.to_string_lossy().to_string()
    }

    #[test]
    fn test_search_yaml_file() {
        let dir = TempDir::new().unwrap();
        let path = write_file(&dir, "config.yml", "name: my-app\nwebhook: https://example.com\nport: 8080\n");
        let opts = SearchOptions {
            pattern: "webhook".to_string(),
            regex: false,
            case_insensitive: false,
            depth: None,
            ext: vec![],
            max_results: None,
            no_tests: false,
            exclude: vec![],
            json: false,
        };
        let result = search_path(&path, &opts).unwrap();
        assert!(result.contains("config.yml"));
        assert!(result.contains("L2:"));
        assert!(result.contains("webhook"));
        // Non-code files should show (top-level) since no AST
        assert!(result.contains("(top-level)"));
    }

    #[test]
    fn test_search_json_file() {
        let dir = TempDir::new().unwrap();
        let path = write_file(&dir, "settings.json", "{\n  \"apiKey\": \"secret\",\n  \"debug\": true\n}\n");
        let opts = SearchOptions {
            pattern: "apiKey".to_string(),
            regex: false,
            case_insensitive: false,
            depth: None,
            ext: vec![],
            max_results: None,
            no_tests: false,
            exclude: vec![],
            json: false,
        };
        let result = search_path(&path, &opts).unwrap();
        assert!(result.contains("settings.json"));
        assert!(result.contains("L2:"));
    }

    #[test]
    fn test_search_markdown_file() {
        let dir = TempDir::new().unwrap();
        let path = write_file(&dir, "README.md", "# My Project\n\nThis has setupHotReload instructions.\n");
        let opts = SearchOptions {
            pattern: "setupHotReload".to_string(),
            regex: false,
            case_insensitive: false,
            depth: None,
            ext: vec![],
            max_results: None,
            no_tests: false,
            exclude: vec![],
            json: false,
        };
        let result = search_path(&path, &opts).unwrap();
        assert!(result.contains("README.md"));
        assert!(result.contains("L3:"));
    }

    #[test]
    fn test_search_env_file() {
        let dir = TempDir::new().unwrap();
        let path = write_file(&dir, "sample.env", "DATABASE_URL=postgres://localhost\nEXECUTIONS_MODE=queue\n");
        let opts = SearchOptions {
            pattern: "EXECUTIONS_MODE".to_string(),
            regex: false,
            case_insensitive: false,
            depth: None,
            ext: vec![],
            max_results: None,
            no_tests: false,
            exclude: vec![],
            json: false,
        };
        let result = search_path(&path, &opts).unwrap();
        assert!(result.contains("sample.env"));
        assert!(result.contains("L2:"));
    }

    #[test]
    fn test_search_directory_mixed_code_and_noncode() {
        let dir = TempDir::new().unwrap();
        fs::create_dir(dir.path().join(".git")).unwrap();
        write_rs_file(&dir, "main.rs", "fn main() {\n    webhook();\n}\n");
        write_file(&dir, "config.yml", "webhook: https://example.com\n");
        write_file(&dir, "README.md", "# Docs\nNo match here.\n");
        let opts = SearchOptions {
            pattern: "webhook".to_string(),
            regex: false,
            case_insensitive: false,
            depth: None,
            ext: vec![],
            max_results: None,
            no_tests: false,
            exclude: vec![],
            json: false,
        };
        let result = search_path(&dir.path().to_string_lossy().as_ref(), &opts).unwrap();
        assert!(result.contains("main.rs"), "should find match in code file");
        assert!(result.contains("config.yml"), "should find match in YAML file");
        assert!(!result.contains("README.md"), "should not include file with no matches");
    }

    #[test]
    fn test_search_directory_ext_filter_yml() {
        let dir = TempDir::new().unwrap();
        fs::create_dir(dir.path().join(".git")).unwrap();
        write_rs_file(&dir, "main.rs", "fn webhook() {}\n");
        write_file(&dir, "config.yml", "webhook: true\n");
        let opts = SearchOptions {
            pattern: "webhook".to_string(),
            regex: false,
            case_insensitive: false,
            depth: None,
            ext: vec!["yml".to_string()],
            max_results: None,
            no_tests: false,
            exclude: vec![],
            json: false,
        };
        let result = search_path(&dir.path().to_string_lossy().as_ref(), &opts).unwrap();
        assert!(result.contains("config.yml"));
        assert!(!result.contains("main.rs"), "--ext yml should exclude .rs files");
    }

    #[test]
    fn test_search_noncode_no_matches() {
        let dir = TempDir::new().unwrap();
        let path = write_file(&dir, "config.yml", "name: my-app\nport: 8080\n");
        let opts = SearchOptions {
            pattern: "nonexistent".to_string(),
            regex: false,
            case_insensitive: false,
            depth: None,
            ext: vec![],
            max_results: None,
            no_tests: false,
            exclude: vec![],
            json: false,
        };
        let result = search_path(&path, &opts).unwrap();
        assert!(result.is_empty());
    }

    #[test]
    fn test_search_toml_file() {
        let dir = TempDir::new().unwrap();
        let path = write_file(&dir, "Cargo.toml", "[package]\nname = \"codehud\"\nversion = \"1.0.0\"\n");
        let opts = SearchOptions {
            pattern: "codehud".to_string(),
            regex: false,
            case_insensitive: false,
            depth: None,
            ext: vec![],
            max_results: None,
            no_tests: false,
            exclude: vec![],
            json: false,
        };
        let result = search_path(&path, &opts).unwrap();
        assert!(result.contains("Cargo.toml"));
        assert!(result.contains("L2:"));
    }
}
