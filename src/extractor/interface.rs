use super::collapse::{
    build_source_line_mappings, collapse_block, collapse_block_filtered, collapse_body,
};
use super::{Item, find_attr_start};
use super::{ItemKind, Visibility};
use crate::handler::{self, LanguageHandler};
use crate::languages::{Language, ts_language};
use std::collections::BTreeMap;
use tree_sitter::{Query, QueryCursor, StreamingIterator, Tree};

/// Extract interface view, optionally filtering private members from class bodies.
pub fn extract_filtered(
    source: &str,
    tree: &Tree,
    language: Language,
    pub_only: bool,
) -> Vec<Item> {
    let handler = handler::handler_for(language).expect("all supported languages have handlers");
    extract_with_handler(source, tree, language, handler.as_ref(), pub_only)
}

fn extract_with_handler(
    source: &str,
    tree: &Tree,
    language: Language,
    handler: &dyn LanguageHandler,
    pub_only: bool,
) -> Vec<Item> {
    let ts_lang = ts_language(language);
    let query = Query::new(&ts_lang, handler.symbol_query()).expect("symbol_query should compile");

    let mut cursor = QueryCursor::new();
    // Limit to top-level (depth ≤ 3 handles export wrappers)
    cursor.set_max_start_depth(Some(handler.max_query_depth()));
    let source_bytes = source.as_bytes();

    let item_idx = query.capture_index_for_name("item").unwrap();
    let name_idx = query.capture_index_for_name("name");
    let body_idx = query.capture_index_for_name("body");

    let mut items_map: BTreeMap<usize, Item> = BTreeMap::new();
    // Deduplicate by @name byte range (exported symbols match twice)
    let mut seen_names = std::collections::HashSet::new();

    let root = tree.root_node();
    let mut matches_iter = cursor.matches(&query, root, source_bytes);

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

        let mut kind_str = item_node.kind();
        // For TS export_statement, use the inner declaration's kind
        let inner_node = if kind_str == "export_statement" {
            let mut inner = None;
            let mut c = item_node.walk();
            for child in item_node.children(&mut c) {
                let ck = child.kind();
                if ck != "export"
                    && ck != ";"
                    && ck != "default"
                    && ck != "comment"
                    && ck != "decorator"
                {
                    inner = Some(child);
                    break;
                }
            }
            inner
        } else {
            None
        };
        if let Some(inner) = inner_node {
            kind_str = inner.kind();
        }

        let visibility = handler.visibility(item_node, source);

        let info = match handler.classify_node(item_node, source) {
            Some(i) => i,
            None => continue,
        };

        let name = info.name;
        let kind = info.kind;

        let body_node = body_idx
            .and_then(|idx| m.captures.iter().find(|c| c.index == idx))
            .map(|c| c.node);

        let (effective_start_byte, line_start) = find_attr_start(item_node);
        let line_end = item_node.end_position().row + 1;

        let (content, line_mappings, has_body) = match kind_str {
            "impl_item"
            | "trait_item"
            | "class_declaration"
            | "abstract_class_declaration"
            | "interface_declaration"
            | "class_specifier"
            | "struct_specifier"
            | "struct_declaration"
            | "record_declaration"
            | "enum_declaration" => {
                let actual_node = if let Some(inner) = inner_node {
                    inner
                } else {
                    item_node
                };
                if pub_only {
                    let exclude = collect_private_member_ranges(actual_node, source, handler);
                    let (c, m) =
                        collapse_block_filtered(source, effective_start_byte, item_node, &exclude);
                    (c, m, false)
                } else {
                    let (c, m) = collapse_block(source, effective_start_byte, item_node);
                    (c, m, false)
                }
            }
            _ if body_node.is_some() => {
                let body = body_node.unwrap();
                let (c, m) = collapse_body(
                    source,
                    effective_start_byte,
                    item_node.end_byte(),
                    body.start_byte(),
                    body.end_byte(),
                );
                (c, m, true)
            }
            _ => {
                let text = &source[effective_start_byte..item_node.end_byte()];
                (text.to_string(), Vec::new(), false)
            }
        };

        let line_mappings = if line_mappings.is_empty() {
            Some(build_source_line_mappings(&content, line_start))
        } else {
            Some(line_mappings)
        };

        items_map.entry(line_start).or_insert(Item {
            kind: kind.clone(),
            name: name.clone(),
            visibility: visibility.clone(),
            line_start,
            line_end,
            signature: None,
            body: if has_body {
                Some("{ ... }".to_string())
            } else {
                None
            },
            content: content.clone(),
            doc_comment: None,
            line_mappings: line_mappings.clone(),
        });

        if matches!(
            kind_str,
            "impl_item"
                | "trait_item"
                | "class_declaration"
                | "abstract_class_declaration"
                | "class_specifier"
                | "struct_specifier"
                | "struct_declaration"
                | "record_declaration"
                | "enum_declaration"
                | "interface_declaration"
        ) {
            let block_node = if let Some(inner) = inner_node {
                inner
            } else {
                item_node
            };
            for child in handler.child_symbols(block_node, source) {
                if !matches!(child.kind, ItemKind::Method | ItemKind::Function) {
                    continue;
                }
                let vis = handler.member_visibility(child.node, source);
                let child_line_start = child.node.start_position().row + 1;
                let child_line_end = child.node.end_position().row + 1;
                let child_content =
                    source[child.node.start_byte()..child.node.end_byte()].to_string();
                let signature = if matches!(child.kind, ItemKind::Method | ItemKind::Function) {
                    Some(handler.signature(child.node, source))
                } else {
                    None
                };
                items_map.entry(child_line_start).or_insert(Item {
                    kind: child.kind,
                    name: child.name,
                    visibility: vis,
                    line_start: child_line_start,
                    line_end: child_line_end,
                    signature,
                    body: None,
                    content: child_content,
                    doc_comment: None,
                    line_mappings: None,
                });
            }
        }
    }

    items_map.into_values().collect()
}

/// Collect byte ranges of private/protected members in a class/impl body.
fn collect_private_member_ranges(
    class_node: tree_sitter::Node,
    source: &str,
    handler: &dyn LanguageHandler,
) -> Vec<(usize, usize)> {
    let body = match class_node.child_by_field_name("body") {
        Some(b) => b,
        None => return vec![],
    };
    let mut ranges = Vec::new();
    let mut cursor = body.walk();
    for child in body.children(&mut cursor) {
        let kind = child.kind();
        if kind == "method_definition"
            || kind == "public_field_definition"
            || kind == "property_definition"
            || kind == "abstract_method_signature"
            || kind == "function_item"
        {
            let vis = handler.member_visibility(child, source);
            if !matches!(vis, Visibility::Public) {
                // Extend range to cover the full line(s) including leading whitespace and trailing newline
                let start = {
                    let mut s = child.start_byte();
                    while s > 0 && source.as_bytes()[s - 1] != b'\n' {
                        s -= 1;
                    }
                    s
                };
                let end = {
                    let mut e = child.end_byte();
                    while e < source.len() && matches!(source.as_bytes()[e], b';' | b' ' | b'\t') {
                        e += 1;
                    }
                    if e < source.len() && source.as_bytes()[e] == b'\n' {
                        e += 1;
                    }
                    e
                };
                ranges.push((start, end));
            }
        }
    }
    ranges
}
