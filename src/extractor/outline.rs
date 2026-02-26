//! Outline mode: signatures + docstrings + types, no implementation bodies.
//!
//! Middle ground between `--list-symbols` (one-liner per symbol) and full expand.
//! Shows the shape of code with enough detail to understand the API.

use super::{find_attr_start, Item, ItemKind, Visibility};
use crate::handler::{self, LanguageHandler};
use crate::languages::{ts_language, Language};
use tree_sitter::{Node, Query, QueryCursor, StreamingIterator, Tree};
use std::collections::BTreeMap;

/// Extract outline view: signatures + docstrings, no bodies.
pub fn extract_outline(source: &str, tree: &Tree, language: Language, pub_only: bool, compact: bool) -> Vec<Item> {
    let handler = handler::handler_for(language)
        .expect("all supported languages have handlers");
    let mut items = extract_outline_with_handler(source, tree, language, handler.as_ref(), pub_only);
    if compact {
        for item in &mut items {
            item.content = compactify_item(item, source);
        }
    }
    items
}

fn extract_outline_with_handler(
    source: &str,
    tree: &Tree,
    language: Language,
    handler: &dyn LanguageHandler,
    pub_only: bool,
) -> Vec<Item> {
    let ts_lang = ts_language(language);
    let query = Query::new(&ts_lang, handler.symbol_query())
        .expect("symbol_query should compile");

    let mut cursor = QueryCursor::new();
    cursor.set_max_start_depth(Some(2));
    let source_bytes = source.as_bytes();

    let item_idx = query.capture_index_for_name("item").unwrap();
    let name_idx = query.capture_index_for_name("name");

    let mut items_map: BTreeMap<usize, Item> = BTreeMap::new();
    let mut seen_names = std::collections::HashSet::new();

    let root = tree.root_node();
    let mut matches_iter = cursor.matches(&query, root, source_bytes);

    while let Some(m) = matches_iter.next() {
        let item_node = match m.captures.iter().find(|c| c.index == item_idx) {
            Some(c) => c.node,
            None => continue,
        };

        // Deduplicate by name node position
        if let Some(ni) = name_idx {
            if let Some(nc) = m.captures.iter().find(|c| c.index == ni) {
                let name_range = (nc.node.start_byte(), nc.node.end_byte());
                if !seen_names.insert(name_range) {
                    continue;
                }
            }
        }

        let info = match handler.classify_node(item_node, source) {
            Some(i) => i,
            None => continue,
        };

        let visibility = handler.visibility(item_node, source);
        if pub_only && !matches!(visibility, Visibility::Public) {
            continue;
        }

        let (_, line_start) = find_attr_start(item_node);
        let line_end = item_node.end_position().row + 1;

        let is_container = matches!(
            info.kind,
            ItemKind::Class | ItemKind::Impl | ItemKind::Trait
        );

        if is_container {
            // For containers: show header + member signatures
            let content = build_container_outline(source, item_node, handler, pub_only, language);
            items_map.entry(line_start).or_insert(Item {
                kind: info.kind,
                name: info.name,
                visibility,
                line_start,
                line_end,
                signature: None,
                body: None,
                content,
                line_mappings: None,
            });
        } else if matches!(info.kind, ItemKind::Function | ItemKind::Method) {
            // Functions: show signature with docstring
            let docstring = get_docstring(source, item_node);
            let sig = handler.signature(item_node, source);
            let content = if let Some(doc) = docstring {
                format!("{}\n{}", doc, sig)
            } else {
                sig.clone()
            };
            items_map.entry(line_start).or_insert(Item {
                kind: info.kind,
                name: info.name,
                visibility,
                line_start,
                line_end,
                signature: Some(sig),
                body: None,
                content,
                line_mappings: None,
            });
        } else if matches!(info.kind, ItemKind::Struct | ItemKind::Enum) {
            // Structs/enums: show full definition (fields are part of the type signature)
            let (effective_start_byte, _) = find_attr_start(item_node);
            let text = &source[effective_start_byte..item_node.end_byte()];
            let docstring = get_docstring(source, item_node);
            let content = if let Some(doc) = docstring {
                format!("{}\n{}", doc, text)
            } else {
                text.to_string()
            };
            items_map.entry(line_start).or_insert(Item {
                kind: info.kind,
                name: info.name,
                visibility,
                line_start,
                line_end,
                signature: None,
                body: None,
                content,
                line_mappings: None,
            });
        } else if matches!(info.kind, ItemKind::TypeAlias | ItemKind::Const | ItemKind::Static) {
            // Type aliases, constants, statics: show as-is
            let (effective_start_byte, _) = find_attr_start(item_node);
            let text = &source[effective_start_byte..item_node.end_byte()];
            let docstring = get_docstring(source, item_node);
            let content = if let Some(doc) = docstring {
                format!("{}\n{}", doc, text)
            } else {
                text.to_string()
            };
            items_map.entry(line_start).or_insert(Item {
                kind: info.kind,
                name: info.name,
                visibility,
                line_start,
                line_end,
                signature: None,
                body: None,
                content,
                line_mappings: None,
            });
        } else if matches!(info.kind, ItemKind::Use) {
            // Imports: include as-is
            let text = &source[item_node.start_byte()..item_node.end_byte()];
            items_map.entry(line_start).or_insert(Item {
                kind: info.kind,
                name: info.name,
                visibility,
                line_start,
                line_end,
                signature: None,
                body: None,
                content: text.to_string(),
                line_mappings: None,
            });
        }
        // Skip other kinds (Mod, MacroDef) — could add later
    }

    items_map.into_values().collect()
}

/// Build outline content for a container (class/impl/trait).
/// Shows the container header, then each member's signature.
fn build_container_outline(
    source: &str,
    item_node: Node,
    handler: &dyn LanguageHandler,
    pub_only: bool,
    _language: Language,
) -> String {
    let mut lines = Vec::new();

    // Get the container header (everything before the body block)
    let inner_node = if item_node.kind() == "export_statement" {
        let mut walk = item_node.walk();
        item_node.named_children(&mut walk)
            .find(|c| c.kind() != "decorator" && c.kind() != "comment")
            .unwrap_or(item_node)
    } else {
        item_node
    };

    // Container docstring
    if let Some(doc) = get_docstring(source, item_node) {
        lines.push(doc);
    }

    // Build header: everything up to (but not including) the body
    let body_field = handler.body_field_name();
    let header = if let Some(body) = inner_node.child_by_field_name(body_field) {
        let header_text = source[inner_node.start_byte()..body.start_byte()].trim_end();
        format!("{} {{", header_text)
    } else {
        source[inner_node.start_byte()..inner_node.end_byte()].to_string()
    };

    // For Rust impl blocks that have attrs on the outer node
    let (effective_start_byte, _) = find_attr_start(item_node);
    if effective_start_byte < inner_node.start_byte() {
        let attrs = source[effective_start_byte..inner_node.start_byte()].trim_end();
        lines.push(attrs.to_string());
    }
    lines.push(header);

    // Collect child symbols
    let children = handler.child_symbols(inner_node, source);
    for child in &children {
        let vis = handler.member_visibility(child.node, source);
        if pub_only && !matches!(vis, Visibility::Public) {
            continue;
        }

        // Get docstring for child
        if let Some(doc) = get_docstring(source, child.node) {
            // Indent docstring
            for line in doc.lines() {
                lines.push(format!("    {}", line));
            }
        }

        if matches!(child.kind, ItemKind::Function | ItemKind::Method) {
            let sig = handler.signature(child.node, source);
            // Indent the signature
            for (i, line) in sig.lines().enumerate() {
                if i == 0 {
                    lines.push(format!("    {}", line));
                } else {
                    lines.push(format!("    {}", line));
                }
            }
        } else {
            // Fields, consts, type aliases inside containers: show as-is
            let text = &source[child.node.start_byte()..child.node.end_byte()];
            for line in text.lines() {
                lines.push(format!("    {}", line));
            }
        }
        lines.push(String::new()); // blank line between members
    }

    // Remove trailing blank line if present
    if lines.last().is_some_and(|l| l.is_empty()) {
        lines.pop();
    }

    lines.push("}".to_string());
    lines.join("\n")
}

/// Compact a single item's content: strip docstrings, replace param lists with `…`.
fn compactify_item(item: &Item, _source: &str) -> String {
    match item.kind {
        ItemKind::Class | ItemKind::Impl | ItemKind::Trait => {
            compactify_container(&item.content)
        }
        ItemKind::Function | ItemKind::Method => {
            compact_signature(&item.content)
        }
        ItemKind::Struct | ItemKind::Enum => {
            // Just the first line (type header)
            compact_type_header(&item.content)
        }
        _ => {
            // Use/const/type alias: keep as-is (already minimal)
            item.content.clone()
        }
    }
}

/// Compact a function/method signature: strip doc comments, replace params with …
fn compact_signature(content: &str) -> String {
    let lines: Vec<&str> = content.lines().collect();
    // Skip doc comment lines
    let sig_lines: Vec<&str> = lines.iter().copied()
        .filter(|l| {
            let t = l.trim();
            !(t.starts_with("///") || t.starts_with("/**") || t.starts_with("* ") || t.starts_with("*/") || t.starts_with("#"))
        })
        .collect();
    let sig = sig_lines.join("\n");
    collapse_params(&sig)
}

/// Replace the contents of the first `(...)` with `…`
fn collapse_params(sig: &str) -> String {
    if let Some(open) = sig.find('(') {
        // Find matching close paren
        let mut depth = 0;
        let mut close = None;
        for (i, ch) in sig[open..].char_indices() {
            match ch {
                '(' => depth += 1,
                ')' => {
                    depth -= 1;
                    if depth == 0 {
                        close = Some(open + i);
                        break;
                    }
                }
                _ => {}
            }
        }
        if let Some(close_pos) = close {
            let inner = sig[open + 1..close_pos].trim();
            if inner.is_empty() {
                sig.to_string()
            } else {
                format!("{}(…){}", &sig[..open], &sig[close_pos + 1..])
            }
        } else {
            sig.to_string()
        }
    } else {
        sig.to_string()
    }
}

/// Compact a container: strip docstrings from members, collapse param lists
fn compactify_container(content: &str) -> String {
    let mut result = Vec::new();
    let mut in_doc = false;
    for line in content.lines() {
        let trimmed = line.trim();
        // Skip doc comment lines
        if trimmed.starts_with("///") || trimmed.starts_with("/**") || trimmed.starts_with("* ") || trimmed.starts_with("*/") {
            in_doc = true;
            continue;
        }
        if in_doc && trimmed.is_empty() {
            in_doc = false;
            continue;
        }
        in_doc = false;
        // Collapse params in member signatures
        let compacted = collapse_params(line);
        // Skip consecutive blank lines
        if compacted.trim().is_empty() && result.last().map_or(false, |l: &String| l.trim().is_empty()) {
            continue;
        }
        result.push(compacted);
    }
    // Remove trailing blank line before closing brace
    if result.len() >= 2 && result[result.len() - 2].trim().is_empty() {
        result.remove(result.len() - 2);
    }
    result.join("\n")
}

/// Compact a struct/enum: just show the header line
fn compact_type_header(content: &str) -> String {
    let lines: Vec<&str> = content.lines().collect();
    // Skip doc comments, find first non-doc line
    for line in &lines {
        let t = line.trim();
        if t.starts_with("///") || t.starts_with("/**") || t.starts_with("* ") || t.starts_with("*/") {
            continue;
        }
        // Return just this first real line + " { … }" if it has a body
        let l = t.to_string();
        if l.ends_with('{') {
            return format!("{} … }}", l.trim_end_matches('{').trim_end());
        }
        return l;
    }
    content.lines().next().unwrap_or("").to_string()
}

/// Extract docstring/comment immediately preceding a node.
fn get_docstring(source: &str, node: Node) -> Option<String> {
    // First check for attributes/decorators that precede
    let effective_node = {
        let mut current = node;
        loop {
            match current.prev_sibling() {
                Some(prev) if prev.kind() == "attribute_item" || prev.kind() == "decorator" => {
                    current = prev;
                }
                _ => break,
            }
        }
        current
    };

    // Now look for comment(s) immediately before
    let mut comments = Vec::new();
    let mut current = effective_node;
    loop {
        match current.prev_sibling() {
            Some(prev) => {
                let kind = prev.kind();
                if kind == "comment" || kind == "line_comment" || kind == "block_comment" {
                    let text = &source[prev.start_byte()..prev.end_byte()];
                    comments.push(text.to_string());
                    current = prev;
                } else {
                    break;
                }
            }
            None => break,
        }
    }

    if comments.is_empty() {
        return None;
    }

    // Comments were collected in reverse order
    comments.reverse();

    // Only include doc comments (/// or /** or #) not regular // comments
    let doc_comments: Vec<&String> = comments.iter().filter(|c| {
        let trimmed = c.trim();
        trimmed.starts_with("///")
            || trimmed.starts_with("/**")
            || trimmed.starts_with("* ")
            || trimmed.starts_with("#")  // Python docstrings via comment nodes
    }).collect();

    if doc_comments.is_empty() {
        // For TS/JS, also accept /** ... */ block comments
        let all_doc = comments.iter().any(|c| c.trim().starts_with("/**"));
        if all_doc {
            return Some(comments.join("\n"));
        }
        return None;
    }

    Some(doc_comments.into_iter().cloned().collect::<Vec<_>>().join("\n"))
}
