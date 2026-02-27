use crate::error::CodehudError;
use crate::extractor::find_attr_start;
use crate::languages::{ts_language, Language};
use crate::parser;
use tree_sitter::{Node, Tree};
use tree_sitter::StreamingIterator;

/// Replace an entire symbol (including attributes) with new content.
/// Returns the modified source code.
pub fn replace(
    source: &str,
    symbol_name: &str,
    new_content: &str,
    language: Language,
) -> Result<String, CodehudError> {
    let tree = parser::parse(source, language)?;
    let (start_byte, end_byte) = find_symbol_range(source, &tree, symbol_name, language)?;
    
    // Build the new source
    let mut result = String::new();
    result.push_str(&source[..start_byte]);
    result.push_str(new_content);
    result.push_str(&source[end_byte..]);
    
    // Validate by re-parsing
    validate_result(&result, language)?;
    
    Ok(result)
}

/// Delete a symbol (including attributes).
/// Returns the modified source code.
pub fn delete(
    source: &str,
    symbol_name: &str,
    language: Language,
) -> Result<String, CodehudError> {
    let tree = parser::parse(source, language)?;
    let (start_byte, end_byte) = find_symbol_range(source, &tree, symbol_name, language)?;
    
    // Find if there's a trailing newline to remove
    let mut effective_end = end_byte;
    if end_byte < source.len() && source.as_bytes()[end_byte] == b'\n' {
        effective_end = end_byte + 1;
    }
    
    // Build the new source
    let mut result = String::new();
    result.push_str(&source[..start_byte]);
    result.push_str(&source[effective_end..]);
    
    // Validate by re-parsing
    validate_result(&result, language)?;
    
    Ok(result)
}

/// Replace only the body block (`{ ... }`) of a symbol, preserving signature/attributes.
/// `new_body` should be the inner content (without outer braces), e.g. `    println!("hi");\n`.
/// Indentation is auto-adjusted to match the original block's indent level.
pub fn replace_body(
    source: &str,
    symbol_name: &str,
    new_body: &str,
    language: Language,
) -> Result<String, CodehudError> {
    let tree = parser::parse(source, language)?;
    let item_node = find_symbol_node(source, &tree, symbol_name, language)?;
    
    let body_node = find_body_node(item_node, language)?;
    let body_start = body_node.start_byte();
    let body_end = body_node.end_byte();
    
    // Detect indent level of the body's opening brace line
    let line_start = source[..body_start].rfind('\n').map(|i| i + 1).unwrap_or(0);
    let original_indent = &source[line_start..body_start]
        .chars()
        .take_while(|c| c.is_whitespace())
        .collect::<String>();
    
    // Build the new body block with proper indentation
    let reindented = reindent_body(new_body, original_indent);
    let new_block = if language.uses_braces_for_blocks() {
        format!("{{\n{}\n{}}}", reindented, original_indent)
    } else {
        format!("\n{}", reindented)
    };
    
    let mut result = String::new();
    result.push_str(&source[..body_start]);
    result.push_str(&new_block);
    result.push_str(&source[body_end..]);
    
    validate_result(&result, language)?;
    Ok(result)
}

/// Apply multiple edits to a file in one pass.
/// Edits are applied bottom-to-top so byte offsets remain valid.
pub fn batch(
    source: &str,
    edits: &[BatchEdit],
    language: Language,
) -> Result<String, CodehudError> {
    // Resolve all byte ranges first, before any mutations
    let tree = parser::parse(source, language)?;
    let mut resolved: Vec<ResolvedEdit> = Vec::new();
    
    // Separate add-type operations (applied sequentially after byte-range edits)
    let mut deferred_adds: Vec<&BatchEdit> = Vec::new();
    
    for edit in edits {
        match edit.action {
            BatchAction::Replace => {
                let content = edit.content.as_deref().ok_or_else(|| {
                    CodehudError::ParseError(format!(
                        "Missing 'content' for replace action on '{}'", edit.symbol
                    ))
                })?;
                let (start, end) = find_symbol_range(source, &tree, &edit.symbol, language)?;
                resolved.push(ResolvedEdit { start, end, replacement: content.to_string() });
            }
            BatchAction::ReplaceBody => {
                let content = edit.content.as_deref().ok_or_else(|| {
                    CodehudError::ParseError(format!(
                        "Missing 'content' for replace-body action on '{}'", edit.symbol
                    ))
                })?;
                let item_node = find_symbol_node(source, &tree, &edit.symbol, language)?;
                let body_node = find_body_node(item_node, language)?;
                let body_start = body_node.start_byte();
                let body_end = body_node.end_byte();
                
                let line_start = source[..body_start].rfind('\n').map(|i| i + 1).unwrap_or(0);
                let original_indent = &source[line_start..body_start]
                    .chars()
                    .take_while(|c| c.is_whitespace())
                    .collect::<String>();
                let reindented = reindent_body(content, original_indent);
                let new_block = format!("{{\n{}\n{}}}", reindented, original_indent);
                
                resolved.push(ResolvedEdit { start: body_start, end: body_end, replacement: new_block });
            }
            BatchAction::Delete => {
                let (start, end) = find_symbol_range(source, &tree, &edit.symbol, language)?;
                let mut effective_end = end;
                if effective_end < source.len() && source.as_bytes()[effective_end] == b'\n' {
                    effective_end += 1;
                }
                resolved.push(ResolvedEdit { start, end: effective_end, replacement: String::new() });
            }
            BatchAction::AddAfter | BatchAction::AddBefore | BatchAction::Append | BatchAction::Prepend => {
                deferred_adds.push(edit);
            }
        }
    }
    
    // Sort by start byte descending (bottom-to-top) so earlier offsets stay valid
    resolved.sort_by(|a, b| b.start.cmp(&a.start));
    
    // Check for overlapping ranges
    for w in resolved.windows(2) {
        // w[0] has higher start than w[1] (sorted descending)
        if w[1].end > w[0].start {
            return Err(CodehudError::ParseError(
                "Overlapping edit ranges detected".to_string()
            ));
        }
    }
    
    let mut result = source.to_string();
    for edit in &resolved {
        result = format!("{}{}{}", &result[..edit.start], edit.replacement, &result[edit.end..]);
    }
    
    // Apply deferred add operations sequentially (each re-parses)
    for add_edit in &deferred_adds {
        let content = add_edit.content.as_deref().ok_or_else(|| {
            CodehudError::ParseError(format!(
                "Missing 'content' for {:?} action on '{}'", add_edit.action, add_edit.symbol
            ))
        })?;
        result = match add_edit.action {
            BatchAction::AddAfter => add_after(&result, &add_edit.symbol, content, language)?,
            BatchAction::AddBefore => add_before(&result, &add_edit.symbol, content, language)?,
            BatchAction::Append => append(&result, content, language)?,
            BatchAction::Prepend => prepend(&result, content, language)?,
            _ => unreachable!(),
        };
    }
    
    if deferred_adds.is_empty() {
        validate_result(&result, language)?;
    }
    Ok(result)
}

/// Insert new code after a named symbol.
/// Returns the modified source code.
pub fn add_after(
    source: &str,
    symbol_name: &str,
    new_content: &str,
    language: Language,
) -> Result<String, CodehudError> {
    let tree = parser::parse(source, language)?;
    let node = find_symbol_node(source, &tree, symbol_name, language)?;
    let end_byte = node.end_byte();
    
    // Find end of line after the symbol
    let insert_pos = source[end_byte..].find('\n')
        .map(|i| end_byte + i + 1)
        .unwrap_or(source.len());
    
    // Detect indentation of the reference symbol
    let (attr_start, _) = find_attr_start(node);
    let line_start = source[..attr_start].rfind('\n').map(|i| i + 1).unwrap_or(0);
    let indent: String = source[line_start..attr_start]
        .chars()
        .take_while(|c| c.is_whitespace())
        .collect();
    
    let reindented = reindent_to_level(new_content, &indent);
    
    let mut result = String::new();
    result.push_str(&source[..insert_pos]);
    result.push('\n');
    result.push_str(&reindented);
    if !reindented.ends_with('\n') {
        result.push('\n');
    }
    result.push_str(&source[insert_pos..]);
    
    validate_result(&result, language)?;
    Ok(result)
}

/// Insert new code before a named symbol.
/// Returns the modified source code.
pub fn add_before(
    source: &str,
    symbol_name: &str,
    new_content: &str,
    language: Language,
) -> Result<String, CodehudError> {
    let tree = parser::parse(source, language)?;
    let node = find_symbol_node(source, &tree, symbol_name, language)?;
    let (attr_start, _) = find_attr_start(node);
    
    // Find start of line for the symbol (or its attribute)
    let line_start = source[..attr_start].rfind('\n').map(|i| i + 1).unwrap_or(0);
    let indent: String = source[line_start..attr_start]
        .chars()
        .take_while(|c| c.is_whitespace())
        .collect();
    
    let reindented = reindent_to_level(new_content, &indent);
    
    let mut result = String::new();
    result.push_str(&source[..line_start]);
    result.push_str(&reindented);
    if !reindented.ends_with('\n') {
        result.push('\n');
    }
    result.push('\n');
    result.push_str(&source[line_start..]);
    
    validate_result(&result, language)?;
    Ok(result)
}

/// Append new code to end of file.
/// Returns the modified source code.
pub fn append(
    source: &str,
    new_content: &str,
    language: Language,
) -> Result<String, CodehudError> {
    let mut result = source.to_string();
    if !result.ends_with('\n') && !result.is_empty() {
        result.push('\n');
    }
    if !result.is_empty() {
        result.push('\n');
    }
    result.push_str(new_content);
    if !result.ends_with('\n') {
        result.push('\n');
    }
    
    validate_result(&result, language)?;
    Ok(result)
}

/// Prepend new code at beginning of file (after any leading comments/attributes/shebangs).
/// Returns the modified source code.
pub fn prepend(
    source: &str,
    new_content: &str,
    language: Language,
) -> Result<String, CodehudError> {
    // Find insertion point: skip leading comments, shebangs, and blank lines
    let tree = parser::parse(source, language)?;
    let root = tree.root_node();
    
    let mut insert_byte = 0;
    let mut cursor = root.walk();
    for child in root.children(&mut cursor) {
        let kind = child.kind();
        if kind == "line_comment" || kind == "block_comment" || kind == "comment"
            || kind == "shebang" || kind == "hash_bang_line"
            || kind.starts_with("attribute")
        {
            insert_byte = child.end_byte();
            // Skip trailing newline
            if insert_byte < source.len() && source.as_bytes()[insert_byte] == b'\n' {
                insert_byte += 1;
            }
        } else {
            break;
        }
    }
    
    let mut result = String::new();
    result.push_str(&source[..insert_byte]);
    if insert_byte > 0 && !source[..insert_byte].ends_with('\n') {
        result.push('\n');
    }
    result.push_str(new_content);
    if !new_content.ends_with('\n') {
        result.push('\n');
    }
    if insert_byte < source.len() && !source[insert_byte..].starts_with('\n') {
        result.push('\n');
    }
    result.push_str(&source[insert_byte..]);
    
    validate_result(&result, language)?;
    Ok(result)
}

/// Result metadata for a single edit operation (used with --json output).
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct EditResult {
    pub symbol: String,
    pub action: String,
    pub line_start: usize,
    pub line_end: usize,
}

/// Get the 1-based line range of a symbol (including attributes).
pub fn symbol_line_range(
    source: &str,
    symbol_name: &str,
    language: Language,
) -> Result<(usize, usize), CodehudError> {
    let tree = parser::parse(source, language)?;
    let (start_byte, end_byte) = find_symbol_range(source, &tree, symbol_name, language)?;
    let line_start = source[..start_byte].matches('\n').count() + 1;
    let line_end = source[..end_byte].matches('\n').count() + 1;
    Ok((line_start, line_end))
}

/// A single edit in a batch operation.
#[derive(Debug, Clone, serde::Deserialize)]
pub struct BatchEdit {
    pub symbol: String,
    pub action: BatchAction,
    pub content: Option<String>,
}

#[derive(Debug, Clone, serde::Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum BatchAction {
    Replace,
    ReplaceBody,
    Delete,
    AddAfter,
    AddBefore,
    Append,
    Prepend,
}

struct ResolvedEdit {
    start: usize,
    end: usize,
    replacement: String,
}

/// Find the body block node of a symbol (Rust `block`, TS `statement_block`).
fn find_body_node<'a>(item_node: Node<'a>, language: Language) -> Result<Node<'a>, CodehudError> {
    let body_kinds: &[&str] = match language {
        Language::Rust | Language::Python | Language::Go => &["block"],
        Language::TypeScript | Language::Tsx | Language::JavaScript | Language::Jsx => &["statement_block"],
        Language::Java => &["block", "class_body", "interface_body", "enum_body", "constructor_body", "annotation_type_body"],
        Language::Cpp => &["compound_statement"],
    };
    
    // First try the `body` field (works for functions)
    if let Some(body) = item_node.child_by_field_name("body")
        && body_kinds.contains(&body.kind()) {
            return Ok(body);
        }
    
    // Fallback: search children for a matching block kind
    let mut cursor = item_node.walk();
    for child in item_node.children(&mut cursor) {
        if body_kinds.contains(&child.kind()) {
            return Ok(child);
        }
    }
    
    Err(CodehudError::ParseError(format!(
        "Symbol has no body block (kind: {})", item_node.kind()
    )))
}

/// Re-indent content to a target indent level (no extra nesting).
fn reindent_to_level(content: &str, target_indent: &str) -> String {
    let min_indent = content.lines()
        .filter(|l| !l.trim().is_empty())
        .map(|l| l.len() - l.trim_start().len())
        .min()
        .unwrap_or(0);
    
    content.lines()
        .map(|line| {
            if line.trim().is_empty() {
                String::new()
            } else {
                let stripped = if line.len() >= min_indent { &line[min_indent..] } else { line.trim_start() };
                format!("{}{}", target_indent, stripped)
            }
        })
        .collect::<Vec<_>>()
        .join("\n")
}

/// Re-indent body content to match the target indent level.
/// Each non-empty line gets `base_indent + one level (4 spaces)`.
fn reindent_body(body: &str, base_indent: &str) -> String {
    let inner_indent = format!("{}    ", base_indent);
    
    // Detect the minimum indent of the input to strip it
    let min_indent = body.lines()
        .filter(|l| !l.trim().is_empty())
        .map(|l| l.len() - l.trim_start().len())
        .min()
        .unwrap_or(0);
    
    body.lines()
        .map(|line| {
            if line.trim().is_empty() {
                String::new()
            } else {
                let stripped = if line.len() >= min_indent { &line[min_indent..] } else { line.trim_start() };
                format!("{}{}", inner_indent, stripped)
            }
        })
        .collect::<Vec<_>>()
        .join("\n")
}

/// Find the tree-sitter Node for a named symbol.
/// Supports dot-notation (e.g. "Class.method") via dispatch layer.
fn find_symbol_node<'a>(
    source: &str,
    tree: &'a Tree,
    symbol_name: &str,
    language: Language,
) -> Result<Node<'a>, CodehudError> {
    let is_qualified = symbol_name.contains('.') || symbol_name.contains("::");

    // For qualified names, use dispatch to find the member node directly
    if is_qualified {
        if let Some(handler) = crate::handler::handler_for(language) {
            let parts: Vec<&str> = symbol_name.split(['.', ':']).filter(|s| !s.is_empty()).collect();
            if parts.len() >= 2 {
                let class_name = parts[0];
                let member_name = parts[parts.len() - 1];
                if let Some(root) = crate::dispatch::find_symbol_node_by_query(source, tree, handler.as_ref(), language, class_name) {
                    // Unwrap export to get inner class
                    let mut walk = root.walk();
                    let inner = if root.kind() == "export_statement" {
                        
                        root.named_children(&mut walk).find(|c| c.kind() != "decorator").unwrap_or(root)
                    } else {
                        root
                    };
                    for child in &handler.child_symbols(inner, source) {
                        if child.name.as_deref() == Some(member_name) {
                            return Ok(child.node);
                        }
                    }
                }
            }
        }
        return Err(CodehudError::ParseError(format!("Symbol not found: {}", symbol_name)));
    }

    let handler_box = crate::handler::handler_for(language)
        .ok_or_else(|| CodehudError::ParseError("No handler for language".to_string()))?;
    let ts_lang = ts_language(language);
    let query = tree_sitter::Query::new(&ts_lang, handler_box.symbol_query())
        .map_err(|e| CodehudError::ParseError(format!("Query compilation failed: {}", e)))?;
    
    let mut cursor = tree_sitter::QueryCursor::new();
    let source_bytes = source.as_bytes();
    
    let item_idx = query.capture_index_for_name("item")
        .ok_or_else(|| CodehudError::ParseError("Query missing 'item' capture".to_string()))?;
    
    let mut matches_iter = cursor.matches(&query, tree.root_node(), source_bytes);
    
    while let Some(m) = matches_iter.next() {
        let item_node = match m.captures.iter().find(|c| c.index == item_idx) {
            Some(c) => c.node,
            None => continue,
        };
        
        let info = match handler_box.classify_node(item_node, source) {
            Some(i) => i,
            None => continue,
        };
        
        if let Some(ref n) = info.name
            && n == symbol_name {
                return Ok(item_node);
            }
    }

    // Fallback: try unqualified member search via dispatch
    if let Some(handler) = crate::handler::handler_for(language) {
        let items = crate::dispatch::find_unqualified_member(source, tree, handler.as_ref(), language, symbol_name);
        if !items.is_empty() {
            // We need the node, not an Item — re-search using dispatch
            let ts_lang_obj = ts_language(language);
            let q = tree_sitter::Query::new(&ts_lang_obj, handler.symbol_query()).ok();
            if let Some(q) = q {
                let item_idx2 = q.capture_index_for_name("item");
                if let Some(ii) = item_idx2 {
                    let mut cursor2 = tree_sitter::QueryCursor::new();
                    cursor2.set_max_start_depth(Some(3));
                    let mut mi = cursor2.matches(&q, tree.root_node(), source.as_bytes());
                    while let Some(m) = mi.next() {
                        let item_node = match m.captures.iter().find(|c| c.index == ii) {
                            Some(c) => c.node,
                            None => continue,
                        };
                        let info = match handler.classify_node(item_node, source) {
                            Some(i) => i,
                            None => continue,
                        };
                        if !matches!(info.kind, crate::extractor::ItemKind::Class | crate::extractor::ItemKind::Trait | crate::extractor::ItemKind::Enum) {
                            continue;
                        }
                        let mut wlk = item_node.walk();
                        let inner = if item_node.kind() == "export_statement" {
                            item_node.named_children(&mut wlk).find(|ch| ch.kind() != "decorator").unwrap_or(item_node)
                        } else {
                            item_node
                        };
                        for child in &handler.child_symbols(inner, source) {
                            if child.name.as_deref() == Some(symbol_name) {
                                return Ok(child.node);
                            }
                        }
                    }
                }
            }
        }
    }
    
    Err(CodehudError::ParseError(format!("Symbol not found: {}", symbol_name)))
}

/// Find the byte range of a symbol (including attributes).
/// Delegates to `find_symbol_node` and returns (start_byte, end_byte).
fn find_symbol_range(
    source: &str,
    tree: &Tree,
    symbol_name: &str,
    language: Language,
) -> Result<(usize, usize), CodehudError> {
    let node = find_symbol_node(source, tree, symbol_name, language)?;
    let (start_byte, _line_start) = find_attr_start(node);
    let end_byte = node.end_byte();
    Ok((start_byte, end_byte))
}

/// Validate the result by re-parsing and checking for errors.
fn validate_result(source: &str, language: Language) -> Result<(), CodehudError> {
    let tree = parser::parse(source, language)?;
    if tree.root_node().has_error() {
        return Err(CodehudError::ParseError(
            "Edit resulted in invalid syntax".to_string()
        ));
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    fn test_replace_function() {
        let source = r#"fn foo() {
    println!("old");
}

fn bar() {
    println!("keep");
}
"#;
        let new_fn = r#"fn foo() {
    println!("new");
}"#;
        
        let result = replace(source, "foo", new_fn, Language::Rust).unwrap();
        assert!(result.contains(r#"println!("new")"#));
        assert!(result.contains(r#"println!("keep")"#));
        assert!(!result.contains(r#"println!("old")"#));
    }
    
    #[test]
    fn test_delete_function() {
        let source = r#"fn foo() {
    println!("delete me");
}

fn bar() {
    println!("keep me");
}
"#;
        
        let result = delete(source, "foo", Language::Rust).unwrap();
        assert!(!result.contains("delete me"));
        assert!(result.contains("keep me"));
        assert!(!result.contains("fn foo()"));
    }
    
    #[test]
    fn test_delete_struct() {
        let source = r#"struct Foo {
    x: i32,
}

struct Bar {
    y: i32,
}
"#;
        
        let result = delete(source, "Foo", Language::Rust).unwrap();
        assert!(!result.contains("struct Foo"));
        assert!(result.contains("struct Bar"));
    }
    
    #[test]
    fn test_validation_catches_bad_replacement() {
        let source = "fn foo() {}\n";
        let bad_replacement = "fn foo() { {{{{{ }";
        
        let result = replace(source, "foo", bad_replacement, Language::Rust);
        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("invalid syntax"));
    }
    
    #[test]
    fn test_symbol_not_found() {
        let source = "fn foo() {}\n";
        
        let result = replace(source, "nonexistent", "fn bar() {}", Language::Rust);
        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("Symbol not found"));
        
        let result = delete(source, "nonexistent", Language::Rust);
        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("Symbol not found"));
    }
    
    #[test]
    fn test_replace_with_attributes() {
        let source = r#"#[inline]
#[must_use]
fn foo() -> i32 {
    42
}
"#;
        let new_fn = r#"#[inline]
fn foo() -> i32 {
    43
}"#;
        
        let result = replace(source, "foo", new_fn, Language::Rust).unwrap();
        assert!(result.contains("43"));
        assert!(!result.contains("42"));
        // Should replace the entire thing including old attributes
        assert_eq!(result.lines().filter(|l| l.contains("#[must_use]")).count(), 0);
    }
    
    #[test]
    fn test_replace_body_function() {
        let source = r#"fn foo(x: i32) -> i32 {
    x + 1
}

fn bar() {}
"#;
        let result = replace_body(source, "foo", "x * 2", Language::Rust).unwrap();
        assert!(result.contains("fn foo(x: i32) -> i32"));
        assert!(result.contains("x * 2"));
        assert!(!result.contains("x + 1"));
        assert!(result.contains("fn bar()"));
    }
    
    #[test]
    fn test_replace_body_preserves_attributes() {
        let source = r#"#[inline]
pub fn foo() -> i32 {
    42
}
"#;
        let result = replace_body(source, "foo", "99", Language::Rust).unwrap();
        assert!(result.contains("#[inline]"));
        assert!(result.contains("pub fn foo() -> i32"));
        assert!(result.contains("99"));
        assert!(!result.contains("42"));
    }
    
    #[test]
    fn test_replace_body_reindents() {
        let source = "    fn foo() {\n        old_code();\n    }\n";
        // Providing body with no indent — should get auto-indented
        let result = replace_body(source, "foo", "new_code();\nmore_code();", Language::Rust).unwrap();
        assert!(result.contains("        new_code();"));
        assert!(result.contains("        more_code();"));
    }
    
    #[test]
    fn test_replace_body_no_body_errors() {
        let source = "struct Foo { x: i32 }\n";
        // struct doesn't have a "block" body in the function sense
        // This should still work since struct has a field_declaration_list, not a block
        let result = replace_body(source, "Foo", "y: i32", Language::Rust);
        assert!(result.is_err());
    }
    
    #[test]
    fn test_batch_multiple_edits() {
        let source = r#"fn foo() {
    old_foo();
}

fn bar() {
    old_bar();
}

fn baz() {
    old_baz();
}
"#;
        let edits = vec![
            BatchEdit { symbol: "foo".to_string(), action: BatchAction::ReplaceBody, content: Some("new_foo();".to_string()) },
            BatchEdit { symbol: "baz".to_string(), action: BatchAction::Delete, content: None },
        ];
        let result = batch(source, &edits, Language::Rust).unwrap();
        assert!(result.contains("new_foo()"));
        assert!(!result.contains("old_foo"));
        assert!(result.contains("old_bar")); // bar untouched
        assert!(!result.contains("baz"));
    }
    
    #[test]
    fn test_batch_replace_and_replace_body() {
        let source = r#"fn alpha() {
    1
}

fn beta() {
    2
}
"#;
        let edits = vec![
            BatchEdit { symbol: "alpha".to_string(), action: BatchAction::Replace, content: Some("fn alpha() {\n    100\n}".to_string()) },
            BatchEdit { symbol: "beta".to_string(), action: BatchAction::ReplaceBody, content: Some("200".to_string()) },
        ];
        let result = batch(source, &edits, Language::Rust).unwrap();
        assert!(result.contains("100"));
        assert!(result.contains("200"));
    }

    #[test]
    fn test_delete_with_attributes() {
        let source = r#"#[test]
fn test_foo() {
    assert!(true);
}

fn bar() {}
"#;
        
        let result = delete(source, "test_foo", Language::Rust).unwrap();
        assert!(!result.contains("test_foo"));
        assert!(!result.contains("#[test]"));
        assert!(result.contains("fn bar()"));
    }

    // ===== Add After tests =====

    #[test]
    fn test_add_after_rust() {
        let source = "fn foo() {\n    1\n}\n\nfn bar() {\n    2\n}\n";
        let new_fn = "fn baz() {\n    3\n}";
        let result = add_after(source, "foo", new_fn, Language::Rust).unwrap();
        assert!(result.contains("fn foo()"));
        assert!(result.contains("fn baz()"));
        assert!(result.contains("fn bar()"));
        // baz should appear between foo and bar
        let foo_pos = result.find("fn foo()").unwrap();
        let baz_pos = result.find("fn baz()").unwrap();
        let bar_pos = result.find("fn bar()").unwrap();
        assert!(foo_pos < baz_pos);
        assert!(baz_pos < bar_pos);
    }

    #[test]
    fn test_add_after_last_symbol() {
        let source = "fn foo() {\n    1\n}\n";
        let new_fn = "fn bar() {\n    2\n}";
        let result = add_after(source, "foo", new_fn, Language::Rust).unwrap();
        assert!(result.contains("fn foo()"));
        assert!(result.contains("fn bar()"));
    }

    #[test]
    fn test_add_after_typescript() {
        let source = "function foo() {\n    return 1;\n}\n\nfunction bar() {\n    return 2;\n}\n";
        let new_fn = "function baz() {\n    return 3;\n}";
        let result = add_after(source, "foo", new_fn, Language::TypeScript).unwrap();
        let foo_pos = result.find("function foo()").unwrap();
        let baz_pos = result.find("function baz()").unwrap();
        let bar_pos = result.find("function bar()").unwrap();
        assert!(foo_pos < baz_pos);
        assert!(baz_pos < bar_pos);
    }

    #[test]
    fn test_add_after_indented_in_impl() {
        let source = "impl Foo {\n    fn a(&self) {\n        1\n    }\n\n    fn b(&self) {\n        2\n    }\n}\n";
        let new_method = "fn c(&self) {\n    3\n}";
        let result = add_after(source, "a", new_method, Language::Rust).unwrap();
        assert!(result.contains("    fn c(&self)"));
        let a_pos = result.find("fn a(").unwrap();
        let c_pos = result.find("fn c(").unwrap();
        let b_pos = result.find("fn b(").unwrap();
        assert!(a_pos < c_pos);
        assert!(c_pos < b_pos);
    }

    // ===== Add Before tests =====

    #[test]
    fn test_add_before_rust() {
        let source = "fn foo() {\n    1\n}\n\nfn bar() {\n    2\n}\n";
        let new_fn = "fn baz() {\n    3\n}";
        let result = add_before(source, "bar", new_fn, Language::Rust).unwrap();
        let foo_pos = result.find("fn foo()").unwrap();
        let baz_pos = result.find("fn baz()").unwrap();
        let bar_pos = result.find("fn bar()").unwrap();
        assert!(foo_pos < baz_pos);
        assert!(baz_pos < bar_pos);
    }

    #[test]
    fn test_add_before_first_symbol() {
        let source = "fn foo() {\n    1\n}\n";
        let new_fn = "fn bar() {\n    2\n}";
        let result = add_before(source, "foo", new_fn, Language::Rust).unwrap();
        let bar_pos = result.find("fn bar()").unwrap();
        let foo_pos = result.find("fn foo()").unwrap();
        assert!(bar_pos < foo_pos);
    }

    #[test]
    fn test_add_before_typescript() {
        let source = "function foo() {\n    return 1;\n}\n";
        let new_fn = "function bar() {\n    return 2;\n}";
        let result = add_before(source, "foo", new_fn, Language::TypeScript).unwrap();
        let bar_pos = result.find("function bar()").unwrap();
        let foo_pos = result.find("function foo()").unwrap();
        assert!(bar_pos < foo_pos);
    }

    // ===== Append tests =====

    #[test]
    fn test_append_rust() {
        let source = "fn foo() {\n    1\n}\n";
        let new_fn = "fn bar() {\n    2\n}";
        let result = append(source, new_fn, Language::Rust).unwrap();
        assert!(result.contains("fn foo()"));
        assert!(result.contains("fn bar()"));
        let foo_pos = result.find("fn foo()").unwrap();
        let bar_pos = result.find("fn bar()").unwrap();
        assert!(foo_pos < bar_pos);
    }

    #[test]
    fn test_append_empty_file() {
        let source = "";
        let new_fn = "fn foo() {\n    1\n}";
        let result = append(source, new_fn, Language::Rust).unwrap();
        assert!(result.contains("fn foo()"));
    }

    #[test]
    fn test_append_typescript() {
        let source = "function foo() {\n    return 1;\n}\n";
        let new_fn = "function bar() {\n    return 2;\n}";
        let result = append(source, new_fn, Language::TypeScript).unwrap();
        assert!(result.contains("function foo()"));
        assert!(result.contains("function bar()"));
    }

    // ===== Prepend tests =====

    #[test]
    fn test_prepend_rust() {
        let source = "fn foo() {\n    1\n}\n";
        let new_fn = "use std::io;\n";
        let result = prepend(source, new_fn, Language::Rust).unwrap();
        let use_pos = result.find("use std::io").unwrap();
        let foo_pos = result.find("fn foo()").unwrap();
        assert!(use_pos < foo_pos);
    }

    #[test]
    fn test_prepend_after_comments() {
        let source = "// Module comment\nfn foo() {\n    1\n}\n";
        let new_use = "use std::io;";
        let result = prepend(source, new_use, Language::Rust).unwrap();
        let comment_pos = result.find("// Module comment").unwrap();
        let use_pos = result.find("use std::io").unwrap();
        let foo_pos = result.find("fn foo()").unwrap();
        assert!(comment_pos < use_pos);
        assert!(use_pos < foo_pos);
    }

    #[test]
    fn test_prepend_typescript() {
        let source = "function foo() {\n    return 1;\n}\n";
        let new_import = "import { bar } from './bar';";
        let result = prepend(source, new_import, Language::TypeScript).unwrap();
        let import_pos = result.find("import").unwrap();
        let foo_pos = result.find("function foo()").unwrap();
        assert!(import_pos < foo_pos);
    }

    // ===== Syntax validation tests =====

    #[test]
    fn test_add_after_validates_syntax() {
        let source = "fn foo() {\n    1\n}\n";
        let bad_code = "fn bar() { {{{{{ }";
        let result = add_after(source, "foo", bad_code, Language::Rust);
        assert!(result.is_err());
    }

    #[test]
    fn test_append_validates_syntax() {
        let source = "fn foo() {\n    1\n}\n";
        let bad_code = "fn bar( {";
        let result = append(source, bad_code, Language::Rust);
        assert!(result.is_err());
    }

    // ===== Batch add tests =====

    #[test]
    fn test_batch_with_add_after() {
        let source = "fn foo() {\n    1\n}\n\nfn bar() {\n    2\n}\n";
        let edits = vec![
            BatchEdit { symbol: "foo".to_string(), action: BatchAction::AddAfter, content: Some("fn baz() {\n    3\n}".to_string()) },
        ];
        let result = batch(source, &edits, Language::Rust).unwrap();
        let foo_pos = result.find("fn foo()").unwrap();
        let baz_pos = result.find("fn baz()").unwrap();
        let bar_pos = result.find("fn bar()").unwrap();
        assert!(foo_pos < baz_pos);
        assert!(baz_pos < bar_pos);
    }

    #[test]
    fn test_batch_with_append() {
        let source = "fn foo() {\n    1\n}\n";
        let edits = vec![
            BatchEdit { symbol: String::new(), action: BatchAction::Append, content: Some("fn bar() {\n    2\n}".to_string()) },
        ];
        let result = batch(source, &edits, Language::Rust).unwrap();
        assert!(result.contains("fn bar()"));
    }

    // ===== Python tests =====

    #[test]
    fn test_add_after_python() {
        let source = "def foo():\n    return 1\n\ndef bar():\n    return 2\n";
        let new_fn = "def baz():\n    return 3";
        let result = add_after(source, "foo", new_fn, Language::Python).unwrap();
        let foo_pos = result.find("def foo()").unwrap();
        let baz_pos = result.find("def baz()").unwrap();
        let bar_pos = result.find("def bar()").unwrap();
        assert!(foo_pos < baz_pos);
        assert!(baz_pos < bar_pos);
    }

    #[test]
    fn test_append_python() {
        let source = "def foo():\n    return 1\n";
        let new_fn = "def bar():\n    return 2";
        let result = append(source, new_fn, Language::Python).unwrap();
        assert!(result.contains("def foo()"));
        assert!(result.contains("def bar()"));
    }
}
