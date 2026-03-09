use super::collapse::build_source_line_mappings;
use super::{Item, ItemKind, find_attr_start};
use crate::handler::{self, LanguageHandler};
use crate::languages::{Language, ts_language};
use tree_sitter::{Query, QueryCursor, StreamingIterator, Tree};

/// Extract full implementation for specified symbols using tree-sitter queries.
pub fn extract(source: &str, tree: &Tree, symbols: &[String], language: Language) -> Vec<Item> {
    let handler = handler::handler_for(language).expect("all supported languages have handlers");
    extract_with_handler(source, tree, symbols, language, handler.as_ref())
}

fn extract_with_handler(
    source: &str,
    tree: &Tree,
    symbols: &[String],
    language: Language,
    handler: &dyn LanguageHandler,
) -> Vec<Item> {
    let ts_lang = ts_language(language);
    let query = Query::new(&ts_lang, handler.symbol_query()).expect("symbol_query should compile");

    let mut cursor = QueryCursor::new();
    let source_bytes = source.as_bytes();

    let item_idx = query.capture_index_for_name("item").unwrap();
    let mut items = Vec::new();
    let mut matches_iter = cursor.matches(&query, tree.root_node(), source_bytes);

    while let Some(m) = matches_iter.next() {
        let item_node = match m.captures.iter().find(|c| c.index == item_idx) {
            Some(c) => c.node,
            None => continue,
        };

        let info = match handler.classify_node(item_node, source) {
            Some(i) => i,
            None => continue,
        };

        let name_str = match &info.name {
            Some(n) => n.as_str(),
            None => continue,
        };
        if !symbols.iter().any(|s| s == name_str) {
            continue;
        }

        let (effective_start_byte, line_start) = find_attr_start(item_node);
        let line_end = item_node.end_position().row + 1;

        let content = source[effective_start_byte..item_node.end_byte()].to_string();
        let visibility = handler.visibility(item_node, source);

        items.push(Item {
            kind: info.kind,
            name: info.name,
            visibility,
            line_start,
            line_end,
            signature: None,
            body: None,
            content,
            doc_comment: None,
            line_mappings: None,
        });
    }

    items.sort_by_key(|item| item.line_start);
    items
}

/// Extract a class with method signatures collapsed, optionally expanding specific methods.
pub fn extract_signatures(
    source: &str,
    tree: &Tree,
    class_name: &str,
    expand_methods: &[String],
    language: Language,
) -> Vec<Item> {
    let handler = handler::handler_for(language).expect("all supported languages have handlers");
    let ts_lang = ts_language(language);
    let query = Query::new(&ts_lang, handler.symbol_query()).expect("symbol_query should compile");

    let mut cursor = QueryCursor::new();
    let source_bytes = source.as_bytes();

    let item_idx = query.capture_index_for_name("item").unwrap();

    let mut matches_iter = cursor.matches(&query, tree.root_node(), source_bytes);

    while let Some(m) = matches_iter.next() {
        let item_node = match m.captures.iter().find(|c| c.index == item_idx) {
            Some(c) => c.node,
            None => continue,
        };

        let info = match handler.classify_node(item_node, source) {
            Some(i) => i,
            None => continue,
        };

        let name_str = match &info.name {
            Some(n) => n.as_str(),
            None => continue,
        };
        if name_str != class_name {
            continue;
        }

        // Only apply signatures mode to class-like items
        if !matches!(info.kind, ItemKind::Class) {
            // Not a class — just return as full expand
            let (effective_start_byte, line_start) = find_attr_start(item_node);
            let line_end = item_node.end_position().row + 1;
            let content = source[effective_start_byte..item_node.end_byte()].to_string();
            let visibility = handler.visibility(item_node, source);
            return vec![Item {
                kind: info.kind,
                name: info.name,
                visibility,
                line_start,
                line_end,
                signature: None,
                body: None,
                content,
                doc_comment: None,
                line_mappings: None,
            }];
        }

        let (effective_start_byte, line_start) = find_attr_start(item_node);
        let line_end = item_node.end_position().row + 1;
        let visibility = handler.visibility(item_node, source);

        // Use handler-based signatures
        // Unwrap export wrapper to get the actual class node
        let mut walk = item_node.walk();
        let inner = if item_node.kind() == "export_statement" {
            item_node
                .named_children(&mut walk)
                .find(|c| c.kind() != "decorator")
                .unwrap_or(item_node)
        } else {
            item_node
        };
        let children = handler.child_symbols(inner, source);
        // Collect body ranges to collapse (methods whose bodies should be replaced)
        let mut body_ranges: Vec<(usize, usize)> = Vec::new();
        for child in &children {
            // Skip children that are in the expand list
            if let Some(ref cname) = child.name
                && expand_methods.iter().any(|m| m == cname)
            {
                continue;
            }
            // Only collapse nodes that have a body
            let body_field = handler.body_field_name();
            if let Some(body) = child.node.child_by_field_name(body_field) {
                body_ranges.push((body.start_byte(), body.end_byte()));
            }
        }
        body_ranges.sort_by_key(|&(s, _)| s);

        // Build content by replacing body ranges with "{ ... }"
        let end_byte = item_node.end_byte();
        let mut result = String::new();
        let mut pos = effective_start_byte;
        for (body_start, body_end) in &body_ranges {
            result.push_str(&source[pos..*body_start]);
            result.push_str("{ ... }");
            pos = *body_end;
        }
        result.push_str(&source[pos..end_byte]);

        let mappings = super::collapse::build_collapsed_block_mappings_pub(
            source,
            end_byte,
            &body_ranges,
            line_start,
            &result,
        );

        let line_mappings = if mappings.is_empty() {
            Some(build_source_line_mappings(&result, line_start))
        } else {
            Some(mappings)
        };
        return vec![Item {
            kind: info.kind,
            name: info.name,
            visibility,
            line_start,
            line_end,
            signature: None,
            body: None,
            content: result,
            doc_comment: None,
            line_mappings,
        }];
    }

    Vec::new()
}
