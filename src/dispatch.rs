//! Dispatch layer: generic functions that use `LanguageHandler` to answer
//! codehud operations (list symbols, expand, find members).

use tree_sitter::{Node, Query, QueryCursor, StreamingIterator, Tree};

use crate::extractor::{Item, ItemKind, Visibility};
use crate::handler::{ChildSymbol, LanguageHandler};
use crate::languages::Language;

// ---------------------------------------------------------------------------
// Helpers
// ---------------------------------------------------------------------------

fn line_of(source: &str, byte: usize) -> usize {
    source[..byte].matches('\n').count() + 1
}

pub fn node_to_item(
    node: Node,
    source: &str,
    handler: &dyn LanguageHandler,
    kind: ItemKind,
    name: Option<String>,
    visibility: Visibility,
) -> Item {
    let start = node.start_byte();
    let end = node.end_byte();
    let content = source[start..end].to_string();
    let signature = if matches!(kind, ItemKind::Method | ItemKind::Function) {
        Some(handler.signature(node, source))
    } else {
        None
    };
    Item {
        kind,
        name,
        visibility,
        line_start: line_of(source, start),
        line_end: line_of(source, end.saturating_sub(1).max(start)),
        content,
        signature,
        body: None,
        doc_comment: None,
        line_mappings: None,
    }
}

fn child_to_item(child: &ChildSymbol, source: &str, handler: &dyn LanguageHandler) -> Item {
    let vis = handler.member_visibility(child.node, source);
    let start = child.node.start_byte();
    let end = child.node.end_byte();
    let content = source[start..end].to_string();
    let signature = if matches!(child.kind, ItemKind::Method | ItemKind::Function) {
        Some(handler.signature(child.node, source))
    } else {
        None
    };
    Item {
        kind: child.kind.clone(),
        name: child.name.clone(),
        visibility: vis,
        line_start: line_of(source, start),
        line_end: line_of(source, end.saturating_sub(1).max(start)),
        content,
        signature,
        body: None,
        doc_comment: None,
        line_mappings: None,
    }
}

/// Unwrap an export_statement to its inner declaration node.
fn unwrap_export<'a>(node: Node<'a>, _source: &str) -> Node<'a> {
    if node.kind() == "export_statement" {
        let mut cursor = node.walk();
        for child in node.named_children(&mut cursor) {
            if child.kind() != "decorator" {
                return child;
            }
        }
    }
    node
}

// ---------------------------------------------------------------------------
// Core dispatch functions
// ---------------------------------------------------------------------------

/// List top-level symbols. If `depth >= 2`, includes child symbols of containers.
pub fn list_symbols(
    source: &str,
    tree: &Tree,
    handler: &dyn LanguageHandler,
    language: Language,
    depth: usize,
) -> Vec<Item> {
    let ts_lang = crate::languages::ts_language(language);
    let query = match Query::new(&ts_lang, handler.symbol_query()) {
        Ok(q) => q,
        Err(_) => return vec![],
    };

    let item_idx = match query.capture_index_for_name("item") {
        Some(i) => i,
        None => return vec![],
    };
    let name_idx = query.capture_index_for_name("name");

    let mut cursor = QueryCursor::new();
    cursor.set_max_start_depth(Some(3));

    let mut matches_iter = cursor.matches(&query, tree.root_node(), source.as_bytes());
    let mut items = Vec::new();
    // Deduplicate by @name byte range (exported symbols match twice: inner + export_statement)
    let mut seen_names = std::collections::HashSet::new();

    while let Some(m) = matches_iter.next() {
        let item_node = match m.captures.iter().find(|c| c.index == item_idx) {
            Some(c) => c.node,
            None => continue,
        };

        // Deduplicate by name node position
        if let Some(ni) = name_idx
            && let Some(nc) = m.captures.iter().find(|c| c.index == ni)
        {
            let name_range = (nc.node.start_byte(), nc.node.end_byte());
            if !seen_names.insert(name_range) {
                continue;
            }
        }

        let info = match handler.classify_node(item_node, source) {
            Some(i) => i,
            None => continue,
        };

        if info.kind == ItemKind::Use {
            continue;
        }

        let vis = handler.visibility(item_node, source);
        items.push(node_to_item(
            item_node,
            source,
            handler,
            info.kind.clone(),
            info.name.clone(),
            vis,
        ));

        if depth >= 2
            && matches!(
                info.kind,
                ItemKind::Class
                    | ItemKind::Trait
                    | ItemKind::Enum
                    | ItemKind::Impl
                    | ItemKind::Struct
            )
        {
            let inner = unwrap_export(item_node, source);
            for child in &handler.child_symbols(inner, source) {
                items.push(child_to_item(child, source, handler));
            }
        }
    }

    items
}

/// Find a top-level symbol node by name.
pub fn find_symbol_node_by_query<'a>(
    source: &str,
    tree: &'a Tree,
    handler: &dyn LanguageHandler,
    language: Language,
    name: &str,
) -> Option<Node<'a>> {
    let ts_lang = crate::languages::ts_language(language);
    let query = Query::new(&ts_lang, handler.symbol_query()).ok()?;
    let name_idx = query.capture_index_for_name("name")?;
    let item_idx = query.capture_index_for_name("item")?;

    let mut cursor = QueryCursor::new();
    cursor.set_max_start_depth(Some(3));

    let mut matches_iter = cursor.matches(&query, tree.root_node(), source.as_bytes());

    while let Some(m) = matches_iter.next() {
        let item_cap = match m.captures.iter().find(|c| c.index == item_idx) {
            Some(c) => c,
            None => continue,
        };
        if let Some(nc) = m.captures.iter().find(|c| c.index == name_idx)
            && &source[nc.node.byte_range()] == name
        {
            return Some(item_cap.node);
        }
    }
    None
}

/// Find ALL top-level symbol nodes matching a name.
fn find_all_symbol_nodes_by_query<'a>(
    source: &str,
    tree: &'a Tree,
    handler: &dyn LanguageHandler,
    language: Language,
    name: &str,
) -> Vec<Node<'a>> {
    let ts_lang = crate::languages::ts_language(language);
    let query = match Query::new(&ts_lang, handler.symbol_query()) {
        Ok(q) => q,
        Err(_) => return vec![],
    };
    let name_idx = match query.capture_index_for_name("name") {
        Some(i) => i,
        None => return vec![],
    };
    let item_idx = match query.capture_index_for_name("item") {
        Some(i) => i,
        None => return vec![],
    };

    let mut cursor = QueryCursor::new();
    cursor.set_max_start_depth(Some(3));

    let mut matches_iter = cursor.matches(&query, tree.root_node(), source.as_bytes());
    let mut results = Vec::new();

    while let Some(m) = matches_iter.next() {
        let item_cap = match m.captures.iter().find(|c| c.index == item_idx) {
            Some(c) => c,
            None => continue,
        };
        if let Some(nc) = m.captures.iter().find(|c| c.index == name_idx)
            && &source[nc.node.byte_range()] == name
        {
            results.push(item_cap.node);
        }
    }
    results
}

/// Expand a symbol by path (dot or `::` notation).
///
/// - `"ClassName"` → full class content
/// - `"ClassName.methodName"` → just the method
pub fn expand_symbol(
    source: &str,
    tree: &Tree,
    handler: &dyn LanguageHandler,
    language: Language,
    symbol_path: &str,
) -> Option<Vec<Item>> {
    let parts: Vec<&str> = symbol_path
        .split(['.', ':'])
        .filter(|s| !s.is_empty())
        .collect();

    if parts.is_empty() {
        return None;
    }

    let root_node = find_symbol_node_by_query(source, tree, handler, language, parts[0])?;

    if parts.len() == 1 {
        let info = handler.classify_node(root_node, source)?;
        let vis = handler.visibility(root_node, source);
        return Some(vec![node_to_item(
            root_node, source, handler, info.kind, info.name, vis,
        )]);
    }

    let member_name = parts[parts.len() - 1];

    // Search through ALL nodes matching the parent name (struct + impl blocks share the same name)
    let matching_nodes = find_all_symbol_nodes_by_query(source, tree, handler, language, parts[0]);
    for node in &matching_nodes {
        let inner = unwrap_export(*node, source);
        for child in &handler.child_symbols(inner, source) {
            if child.name.as_deref() == Some(member_name) {
                return Some(vec![child_to_item(child, source, handler)]);
            }
        }
    }

    None
}

/// Find an unqualified member name across all container symbols.
pub fn find_unqualified_member(
    source: &str,
    tree: &Tree,
    handler: &dyn LanguageHandler,
    language: Language,
    name: &str,
) -> Vec<Item> {
    let ts_lang = crate::languages::ts_language(language);
    let query = match Query::new(&ts_lang, handler.symbol_query()) {
        Ok(q) => q,
        Err(_) => return vec![],
    };

    let item_idx = match query.capture_index_for_name("item") {
        Some(i) => i,
        None => return vec![],
    };
    let name_q_idx = query.capture_index_for_name("name");

    let mut cursor = QueryCursor::new();
    cursor.set_max_start_depth(Some(3));

    let mut matches_iter = cursor.matches(&query, tree.root_node(), source.as_bytes());
    let mut results = Vec::new();
    let mut seen_names = std::collections::HashSet::new();

    while let Some(m) = matches_iter.next() {
        let item_node = match m.captures.iter().find(|c| c.index == item_idx) {
            Some(c) => c.node,
            None => continue,
        };

        // Deduplicate by name node position
        if let Some(ni) = name_q_idx
            && let Some(nc) = m.captures.iter().find(|c| c.index == ni)
        {
            let nr = (nc.node.start_byte(), nc.node.end_byte());
            if !seen_names.insert(nr) {
                continue;
            }
        }

        let info = match handler.classify_node(item_node, source) {
            Some(i) => i,
            None => continue,
        };

        if !matches!(
            info.kind,
            ItemKind::Class | ItemKind::Trait | ItemKind::Enum | ItemKind::Impl | ItemKind::Struct
        ) {
            continue;
        }

        let inner = unwrap_export(item_node, source);
        for child in &handler.child_symbols(inner, source) {
            if child.name.as_deref() == Some(name) {
                results.push(child_to_item(child, source, handler));
            }
        }
    }

    results
}
