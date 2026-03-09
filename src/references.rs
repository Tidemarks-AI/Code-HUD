use crate::error::CodehudError;
use crate::handler::{self, LanguageHandler};
use crate::languages::{self, Language};
use crate::parser;
use crate::walk;
use serde::Serialize;
use std::fmt::Write;
use std::fs;
use std::path::Path;
use tree_sitter::Node;

/// A single reference match.
#[derive(Debug, Clone, Serialize)]
pub struct Reference {
    pub file: String,
    pub line: usize,
    pub column: usize,
    pub kind: RefKind,
    pub line_content: String,
    pub context: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Serialize)]
#[serde(rename_all = "lowercase")]
pub enum RefKind {
    Definition,
    Reference,
    /// Text-only match (unsupported file type)
    Text,
}

/// Options for reference search.
pub struct ReferenceOptions {
    pub symbol: String,
    pub depth: Option<usize>,
    pub ext: Vec<String>,
    pub context_lines: usize,
    pub defs_only: bool,
    pub refs_only: bool,
    pub json: bool,
    pub exclude: Vec<String>,
}

/// Find all references to a symbol in a path (file or directory).
pub fn find_references(
    path: &str,
    options: &ReferenceOptions,
) -> Result<Vec<Reference>, CodehudError> {
    let path = Path::new(path);
    if !path.exists() {
        return Err(CodehudError::PathNotFound(path.display().to_string()));
    }

    let files = if path.is_file() {
        vec![path.to_path_buf()]
    } else if path.is_dir() {
        walk::filter_excludes(
            walk::walk_directory(path, options.depth, &options.ext)?,
            path,
            &options.exclude,
        )
    } else {
        return Err(CodehudError::InvalidPath(path.display().to_string()));
    };

    let mut all_refs = Vec::new();
    for file_path in &files {
        let source = match fs::read_to_string(file_path) {
            Ok(s) => s,
            Err(_) => continue,
        };
        let lines: Vec<&str> = source.lines().collect();

        if let Some(sfc_kind) = crate::sfc::detect_sfc(file_path) {
            // SFC files: extract script blocks and find refs in each
            let blocks = crate::sfc::extract_scripts(&source, sfc_kind);
            for block in &blocks {
                let tree = match parser::parse(&block.content, block.language) {
                    Ok(t) => t,
                    Err(_) => continue,
                };
                let block_lines: Vec<&str> = block.content.lines().collect();
                let mut block_refs = Vec::new();
                find_refs_in_tree(
                    &tree.root_node(),
                    &block.content,
                    &block_lines,
                    &options.symbol,
                    block.language,
                    file_path.to_string_lossy().as_ref(),
                    options.context_lines,
                    &mut block_refs,
                );
                // Remap line numbers to original file positions
                for mut r in block_refs {
                    r.line = r.line + block.start_line - 1;
                    // Replace context with original file lines
                    let orig_line_idx = r.line - 1;
                    r.line_content = if orig_line_idx < lines.len() {
                        lines[orig_line_idx].to_string()
                    } else {
                        r.line_content
                    };
                    r.context = get_context(&lines, orig_line_idx, options.context_lines);
                    all_refs.push(r);
                }
            }
        } else if languages::is_supported_file(file_path) {
            let language = match languages::detect_language(file_path) {
                Ok(l) => l,
                Err(_) => continue,
            };
            let tree = match parser::parse(&source, language) {
                Ok(t) => t,
                Err(_) => continue,
            };
            find_refs_in_tree(
                &tree.root_node(),
                &source,
                &lines,
                &options.symbol,
                language,
                file_path.to_string_lossy().as_ref(),
                options.context_lines,
                &mut all_refs,
            );
        } else {
            // Text fallback for unsupported files
            for (idx, line) in lines.iter().enumerate() {
                if contains_word(line, &options.symbol) {
                    all_refs.push(Reference {
                        file: file_path.to_string_lossy().to_string(),
                        line: idx + 1,
                        column: 0,
                        kind: RefKind::Text,
                        line_content: line.to_string(),
                        context: get_context(&lines, idx, options.context_lines),
                    });
                }
            }
        }
    }

    // Apply filters
    if options.defs_only {
        all_refs.retain(|r| r.kind == RefKind::Definition);
    } else if options.refs_only {
        all_refs.retain(|r| r.kind == RefKind::Reference);
    }

    Ok(all_refs)
}

/// Format references as grep-like text output.
pub fn format_plain(refs: &[Reference]) -> String {
    let mut out = String::new();
    for r in refs {
        let kind_label = match r.kind {
            RefKind::Definition => " [def]",
            RefKind::Reference => " [ref]",
            RefKind::Text => " [text]",
        };
        writeln!(
            out,
            "{}:{}:{}{} {}",
            r.file,
            r.line,
            r.column,
            kind_label,
            r.line_content.trim()
        )
        .unwrap();
        for ctx in &r.context {
            writeln!(out, "  {}", ctx).unwrap();
        }
    }
    out
}

/// Format references as JSON.
pub fn format_json(refs: &[Reference]) -> String {
    serde_json::to_string_pretty(refs).unwrap_or_default()
}

/// Check if a word appears as a whole word in a line (simple text fallback).
fn contains_word(line: &str, word: &str) -> bool {
    let mut start = 0;
    while let Some(pos) = line[start..].find(word) {
        let abs = start + pos;
        let before_ok = abs == 0
            || !line.as_bytes()[abs - 1].is_ascii_alphanumeric()
                && line.as_bytes()[abs - 1] != b'_';
        let after = abs + word.len();
        let after_ok = after >= line.len()
            || !line.as_bytes()[after].is_ascii_alphanumeric() && line.as_bytes()[after] != b'_';
        if before_ok && after_ok {
            return true;
        }
        start = abs + 1;
    }
    false
}

fn get_context(lines: &[&str], idx: usize, context_lines: usize) -> Vec<String> {
    if context_lines == 0 {
        return vec![];
    }
    let mut ctx = Vec::new();
    let start = idx.saturating_sub(context_lines);
    let end = (idx + context_lines + 1).min(lines.len());
    for (i, line) in lines.iter().enumerate().skip(start).take(end - start) {
        if i != idx {
            ctx.push(format!("L{}: {}", i + 1, line));
        }
    }
    ctx
}

/// Node kinds that indicate a definition context (parent of an identifier = definition).
fn is_definition_parent(kind: &str, handler: &dyn LanguageHandler) -> bool {
    handler.definition_parent_kinds().contains(&kind)
}

/// Check if a node is an identifier that names a definition.
fn is_definition_identifier(node: Node, handler: &dyn LanguageHandler) -> bool {
    let parent = match node.parent() {
        Some(p) => p,
        None => return false,
    };

    // `new Foo()` — the identifier is a reference (constructor call), not a definition
    if parent.kind() == "new_expression" {
        return false;
    }

    // For definition parents, the identifier must be in a "name" field
    if is_definition_parent(parent.kind(), handler) {
        // Check if this node is the "name" field of the parent
        if let Some(name_node) = parent.child_by_field_name("name")
            && name_node.id() == node.id()
        {
            return true;
        }
        // For Rust let_declaration, the pattern field contains the name
        if parent.kind() == "let_declaration"
            && let Some(pat) = parent.child_by_field_name("pattern")
            && pat.id() == node.id()
        {
            return true;
        }
        // For variable_declarator (JS/TS), check name field
        if parent.kind() == "variable_declarator"
            && let Some(name_node) = parent.child_by_field_name("name")
            && name_node.id() == node.id()
        {
            return true;
        }
    }

    false
}

/// Identifier node kinds for each language.
fn is_identifier_node(kind: &str, handler: &dyn LanguageHandler) -> bool {
    handler.identifier_node_kinds().contains(&kind)
}

/// Check if a node is inside a string literal or comment.
fn is_in_string_or_comment(node: Node) -> bool {
    let mut current = node.parent();
    while let Some(n) = current {
        match n.kind() {
            "string_literal"
            | "raw_string_literal"
            | "string"
            | "template_string"
            | "string_content"
            | "line_comment"
            | "block_comment"
            | "comment"
            | "string_fragment"
            | "concatenated_string" => return true,
            _ => {}
        }
        current = n.parent();
    }
    false
}

/// Recursively find all references in a tree.
#[allow(clippy::too_many_arguments)]
pub fn find_refs_in_tree(
    node: &Node,
    source: &str,
    lines: &[&str],
    symbol: &str,
    language: Language,
    file_path: &str,
    context_lines: usize,
    refs: &mut Vec<Reference>,
) {
    let handler = handler::handler_for(language).expect("all supported languages have handlers");
    find_refs_in_tree_with_handler(
        node,
        source,
        lines,
        symbol,
        handler.as_ref(),
        file_path,
        context_lines,
        refs,
    );
}

/// Recursively find all references in a tree using a provided handler.
#[allow(clippy::too_many_arguments)]
fn find_refs_in_tree_with_handler(
    node: &Node,
    source: &str,
    lines: &[&str],
    symbol: &str,
    handler: &dyn LanguageHandler,
    file_path: &str,
    context_lines: usize,
    refs: &mut Vec<Reference>,
) {
    // Check if this node is an identifier matching our symbol
    if is_identifier_node(node.kind(), handler) {
        let text = &source[node.start_byte()..node.end_byte()];
        if text == symbol && !is_in_string_or_comment(*node) {
            let line_idx = node.start_position().row;
            let kind = if is_definition_identifier(*node, handler) {
                RefKind::Definition
            } else {
                RefKind::Reference
            };
            refs.push(Reference {
                file: file_path.to_string(),
                line: line_idx + 1,
                column: node.start_position().column,
                kind,
                line_content: lines.get(line_idx).unwrap_or(&"").to_string(),
                context: get_context(lines, line_idx, context_lines),
            });
        }
    }

    // Recurse into children
    let mut cursor = node.walk();
    for child in node.children(&mut cursor) {
        find_refs_in_tree_with_handler(
            &child,
            source,
            lines,
            symbol,
            handler,
            file_path,
            context_lines,
            refs,
        );
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use tempfile::TempDir;

    fn write_file(dir: &TempDir, name: &str, content: &str) -> String {
        let path = dir.path().join(name);
        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent).unwrap();
        }
        fs::write(&path, content).unwrap();
        path.to_string_lossy().to_string()
    }

    #[test]
    fn test_find_function_def_and_refs() {
        let dir = TempDir::new().unwrap();
        let path = write_file(
            &dir,
            "test.rs",
            r#"fn hello() {
    println!("hi");
}

fn main() {
    hello();
}
"#,
        );
        let opts = ReferenceOptions {
            symbol: "hello".to_string(),
            depth: None,
            ext: vec![],
            context_lines: 0,
            defs_only: false,
            refs_only: false,
            json: false,
            exclude: vec![],
        };
        let refs = find_references(&path, &opts).unwrap();
        assert_eq!(refs.len(), 2);
        assert_eq!(refs[0].kind, RefKind::Definition);
        assert_eq!(refs[0].line, 1);
        assert_eq!(refs[1].kind, RefKind::Reference);
        assert_eq!(refs[1].line, 6);
    }

    #[test]
    fn test_excludes_string_literals() {
        let dir = TempDir::new().unwrap();
        let path = write_file(
            &dir,
            "test.rs",
            r#"fn foo() {
    let x = "foo";
    foo();
}
"#,
        );
        let opts = ReferenceOptions {
            symbol: "foo".to_string(),
            depth: None,
            ext: vec![],
            context_lines: 0,
            defs_only: false,
            refs_only: false,
            json: false,
            exclude: vec![],
        };
        let refs = find_references(&path, &opts).unwrap();
        // Should find def (line 1) and call (line 3), but NOT string "foo" (line 2)
        assert_eq!(refs.len(), 2);
        assert!(refs.iter().all(|r| r.line != 2));
    }

    #[test]
    fn test_excludes_comments() {
        let dir = TempDir::new().unwrap();
        let path = write_file(
            &dir,
            "test.rs",
            r#"// call foo here
fn foo() {}
fn bar() { foo(); }
"#,
        );
        let opts = ReferenceOptions {
            symbol: "foo".to_string(),
            depth: None,
            ext: vec![],
            context_lines: 0,
            defs_only: false,
            refs_only: false,
            json: false,
            exclude: vec![],
        };
        let refs = find_references(&path, &opts).unwrap();
        // line 1 is a comment, should be excluded
        assert!(refs.iter().all(|r| r.line != 1));
        assert_eq!(refs.len(), 2); // def + usage
    }

    #[test]
    fn test_defs_only_filter() {
        let dir = TempDir::new().unwrap();
        let path = write_file(
            &dir,
            "test.rs",
            r#"fn hello() {}
fn main() { hello(); }
"#,
        );
        let opts = ReferenceOptions {
            symbol: "hello".to_string(),
            depth: None,
            ext: vec![],
            context_lines: 0,
            defs_only: true,
            refs_only: false,
            json: false,
            exclude: vec![],
        };
        let refs = find_references(&path, &opts).unwrap();
        assert_eq!(refs.len(), 1);
        assert_eq!(refs[0].kind, RefKind::Definition);
    }

    #[test]
    fn test_refs_only_filter() {
        let dir = TempDir::new().unwrap();
        let path = write_file(
            &dir,
            "test.rs",
            r#"fn hello() {}
fn main() { hello(); }
"#,
        );
        let opts = ReferenceOptions {
            symbol: "hello".to_string(),
            depth: None,
            ext: vec![],
            context_lines: 0,
            defs_only: false,
            refs_only: true,
            json: false,
            exclude: vec![],
        };
        let refs = find_references(&path, &opts).unwrap();
        assert_eq!(refs.len(), 1);
        assert_eq!(refs[0].kind, RefKind::Reference);
    }

    #[test]
    fn test_cross_file_directory() {
        let dir = TempDir::new().unwrap();
        fs::create_dir(dir.path().join(".git")).unwrap();
        write_file(&dir, "a.rs", "fn greet() {}\n");
        write_file(&dir, "b.rs", "fn main() { greet(); }\n");
        let opts = ReferenceOptions {
            symbol: "greet".to_string(),
            depth: None,
            ext: vec![],
            context_lines: 0,
            defs_only: false,
            refs_only: false,
            json: false,
            exclude: vec![],
        };
        let refs = find_references(dir.path().to_str().unwrap(), &opts).unwrap();
        assert_eq!(refs.len(), 2);
        let files: Vec<&str> = refs.iter().map(|r| r.file.as_str()).collect();
        assert!(files.iter().any(|f| f.contains("a.rs")));
        assert!(files.iter().any(|f| f.contains("b.rs")));
    }

    #[test]
    fn test_json_output() {
        let dir = TempDir::new().unwrap();
        write_file(&dir, "test.rs", "fn foo() { foo(); }\n");
        let opts = ReferenceOptions {
            symbol: "foo".to_string(),
            depth: None,
            ext: vec![],
            context_lines: 0,
            defs_only: false,
            refs_only: false,
            json: true,
            exclude: vec![],
        };
        let refs = find_references(dir.path().join("test.rs").to_str().unwrap(), &opts).unwrap();
        let json = format_json(&refs);
        assert!(json.contains("\"definition\""));
        assert!(json.contains("\"reference\""));
    }

    #[test]
    fn test_self_method_call() {
        let dir = TempDir::new().unwrap();
        let path = write_file(
            &dir,
            "test.rs",
            r#"struct Foo;
impl Foo {
    fn bar(&self) {}
    fn baz(&self) { self.bar(); }
}
"#,
        );
        let opts = ReferenceOptions {
            symbol: "bar".to_string(),
            depth: None,
            ext: vec![],
            context_lines: 0,
            defs_only: false,
            refs_only: false,
            json: false,
            exclude: vec![],
        };
        let refs = find_references(&path, &opts).unwrap();
        assert_eq!(refs.len(), 2);
        assert_eq!(refs[0].kind, RefKind::Definition);
        assert_eq!(refs[1].kind, RefKind::Reference);
    }

    #[test]
    fn test_use_import_is_reference() {
        let dir = TempDir::new().unwrap();
        let path = write_file(
            &dir,
            "test.rs",
            r#"use std::collections::HashMap;
fn foo() {
    let m: HashMap<String, i32> = HashMap::new();
}
"#,
        );
        let opts = ReferenceOptions {
            symbol: "HashMap".to_string(),
            depth: None,
            ext: vec![],
            context_lines: 0,
            defs_only: false,
            refs_only: false,
            json: false,
            exclude: vec![],
        };
        let refs = find_references(&path, &opts).unwrap();
        // All should be references (use import + type usage + constructor)
        assert!(refs.len() >= 2);
        assert!(refs.iter().all(|r| r.kind == RefKind::Reference));
    }

    #[test]
    fn test_typescript_references() {
        let dir = TempDir::new().unwrap();
        let path = write_file(
            &dir,
            "test.ts",
            r#"function greet(name: string) {
    console.log(name);
}
greet("world");
"#,
        );
        let opts = ReferenceOptions {
            symbol: "greet".to_string(),
            depth: None,
            ext: vec![],
            context_lines: 0,
            defs_only: false,
            refs_only: false,
            json: false,
            exclude: vec![],
        };
        let refs = find_references(&path, &opts).unwrap();
        assert_eq!(refs.len(), 2);
        assert_eq!(refs[0].kind, RefKind::Definition);
        assert_eq!(refs[1].kind, RefKind::Reference);
    }

    #[test]
    fn test_python_references() {
        let dir = TempDir::new().unwrap();
        let path = write_file(
            &dir,
            "test.py",
            r#"def greet():
    pass

greet()
"#,
        );
        let opts = ReferenceOptions {
            symbol: "greet".to_string(),
            depth: None,
            ext: vec![],
            context_lines: 0,
            defs_only: false,
            refs_only: false,
            json: false,
            exclude: vec![],
        };
        let refs = find_references(&path, &opts).unwrap();
        assert_eq!(refs.len(), 2);
        assert_eq!(refs[0].kind, RefKind::Definition);
        assert_eq!(refs[1].kind, RefKind::Reference);
    }

    #[test]
    fn test_context_lines() {
        let dir = TempDir::new().unwrap();
        let path = write_file(&dir, "test.rs", "fn a() {}\nfn foo() {}\nfn b() {}\n");
        let opts = ReferenceOptions {
            symbol: "foo".to_string(),
            depth: None,
            ext: vec![],
            context_lines: 1,
            defs_only: false,
            refs_only: false,
            json: false,
            exclude: vec![],
        };
        let refs = find_references(&path, &opts).unwrap();
        assert_eq!(refs.len(), 1);
        assert_eq!(refs[0].context.len(), 2); // 1 before + 1 after
    }

    #[test]
    fn test_text_fallback_for_unsupported() {
        let dir = TempDir::new().unwrap();
        let path = write_file(&dir, "test.toml", "[package]\nname = \"foo\"\n");
        let opts = ReferenceOptions {
            symbol: "foo".to_string(),
            depth: None,
            ext: vec![],
            context_lines: 0,
            defs_only: false,
            refs_only: false,
            json: false,
            exclude: vec![],
        };
        let refs = find_references(&path, &opts).unwrap();
        assert_eq!(refs.len(), 1);
        assert_eq!(refs[0].kind, RefKind::Text);
    }

    #[test]
    fn test_ext_filter() {
        let dir = TempDir::new().unwrap();
        fs::create_dir(dir.path().join(".git")).unwrap();
        write_file(&dir, "a.rs", "fn foo() {}\n");
        write_file(&dir, "b.ts", "function foo() {}\n");
        let opts = ReferenceOptions {
            symbol: "foo".to_string(),
            depth: None,
            ext: vec!["rs".to_string()],
            context_lines: 0,
            defs_only: false,
            refs_only: false,
            json: false,
            exclude: vec![],
        };
        let refs = find_references(dir.path().to_str().unwrap(), &opts).unwrap();
        assert_eq!(refs.len(), 1);
        assert!(refs[0].file.contains("a.rs"));
    }

    #[test]
    fn test_javascript_references() {
        let dir = TempDir::new().unwrap();
        let path = write_file(
            &dir,
            "test.js",
            r#"function hello() {}
hello();
"#,
        );
        let opts = ReferenceOptions {
            symbol: "hello".to_string(),
            depth: None,
            ext: vec![],
            context_lines: 0,
            defs_only: false,
            refs_only: false,
            json: false,
            exclude: vec![],
        };
        let refs = find_references(&path, &opts).unwrap();
        assert_eq!(refs.len(), 2);
        assert_eq!(refs[0].kind, RefKind::Definition);
        assert_eq!(refs[1].kind, RefKind::Reference);
    }

    #[test]
    fn test_plain_output_format() {
        let refs = vec![
            Reference {
                file: "test.rs".to_string(),
                line: 1,
                column: 3,
                kind: RefKind::Definition,
                line_content: "fn foo() {}".to_string(),
                context: vec![],
            },
            Reference {
                file: "test.rs".to_string(),
                line: 5,
                column: 4,
                kind: RefKind::Reference,
                line_content: "    foo();".to_string(),
                context: vec![],
            },
        ];
        let out = format_plain(&refs);
        assert!(out.contains("[def]"));
        assert!(out.contains("[ref]"));
        assert!(out.contains("test.rs:1:3"));
        assert!(out.contains("test.rs:5:4"));
    }

    #[test]
    fn test_ts_class_type_annotation_ref() {
        let dir = TempDir::new().unwrap();
        let path = write_file(
            &dir,
            "test.ts",
            r#"class Workflow {
    run() {}
}
function start(w: Workflow) {
    return w;
}
"#,
        );
        let opts = ReferenceOptions {
            symbol: "Workflow".to_string(),
            depth: None,
            ext: vec![],
            context_lines: 0,
            defs_only: false,
            refs_only: false,
            json: false,
            exclude: vec![],
        };
        let refs = find_references(&path, &opts).unwrap();
        // Should find: definition on line 1, type annotation on line 4
        assert!(
            refs.len() >= 2,
            "Should find at least 2 refs, got {}",
            refs.len()
        );
        let type_ref = refs.iter().find(|r| r.line == 4);
        assert!(
            type_ref.is_some(),
            "Should find Workflow in type annotation on line 4"
        );
        assert_eq!(type_ref.unwrap().kind, RefKind::Reference);
    }

    #[test]
    fn test_ts_new_expression_is_reference() {
        let dir = TempDir::new().unwrap();
        let path = write_file(
            &dir,
            "test.ts",
            r#"class Workflow {}
const w = new Workflow();
"#,
        );
        let opts = ReferenceOptions {
            symbol: "Workflow".to_string(),
            depth: None,
            ext: vec![],
            context_lines: 0,
            defs_only: false,
            refs_only: false,
            json: false,
            exclude: vec![],
        };
        let refs = find_references(&path, &opts).unwrap();
        assert_eq!(refs.len(), 2);
        assert_eq!(refs[0].kind, RefKind::Definition);
        assert_eq!(
            refs[1].kind,
            RefKind::Reference,
            "new Workflow() should be Reference, not Definition"
        );
    }
}
