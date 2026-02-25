//! Collapse function bodies to `{ ... }` placeholders.
//!
//! Language-agnostic text surgery for producing interface views.

use tree_sitter::Node;
pub fn collapse_body(
    source: &str,
    item_start: usize,
    item_end: usize,
    body_start: usize,
    body_end: usize,
) -> (String, Vec<(usize, String)>) {
    let before = &source[item_start..body_start];
    let after = &source[body_end..item_end];

    // Preserve trailing space before body, trim only trailing newlines
    let before_trimmed = before.trim_end_matches(['\n', '\r']);

    // Ensure space before `{`
    let collapsed = if before_trimmed.ends_with(' ') || before_trimmed.ends_with('\t') {
        format!("{}{{ ... }}{}", before_trimmed, after.trim())
    } else {
        format!("{} {{ ... }}{}", before_trimmed, after.trim())
    };

    let start_line = source[..item_start].matches('\n').count() + 1;
    let mappings = build_source_line_mappings(&collapsed, start_line);
    (collapsed, mappings)
}

/// Collapse all function bodies inside an impl/trait block.
/// Preserves the block structure but replaces each fn body with `{ ... }`.
pub fn collapse_block(source: &str, start_byte: usize, block_node: Node) -> (String, Vec<(usize, String)>) {
    collapse_block_filtered(source, start_byte, block_node, &[])
}

/// Collapse a block, excluding specified byte ranges entirely (e.g., private members).
pub fn collapse_block_filtered(source: &str, start_byte: usize, block_node: Node, exclude_ranges: &[(usize, usize)]) -> (String, Vec<(usize, String)>) {
    // Collect function body ranges
    let mut body_ranges: Vec<(usize, usize)> = Vec::new();
    collect_fn_bodies(block_node, &mut body_ranges);
    body_ranges.sort_by_key(|&(s, _)| s);

    // Build a unified list of skip actions sorted by position
    #[derive(Clone, Copy)]
    enum SkipKind { CollapseBody, Exclude }
    let mut skips: Vec<(usize, usize, SkipKind)> = Vec::new();
    for &(s, e) in &body_ranges {
        // Only add body collapse if not inside an excluded range
        if !exclude_ranges.iter().any(|&(es, ee)| s >= es && e <= ee) {
            skips.push((s, e, SkipKind::CollapseBody));
        }
    }
    for &(s, e) in exclude_ranges {
        skips.push((s, e, SkipKind::Exclude));
    }
    skips.sort_by_key(|&(s, _, _)| s);

    let end_byte = block_node.end_byte();
    let mut result = String::new();
    let mut pos = start_byte;

    for (skip_start, skip_end, kind) in &skips {
        if *skip_start < pos { continue; }
        result.push_str(&source[pos..*skip_start]);
        match kind {
            SkipKind::CollapseBody => result.push_str("{ ... }"),
            SkipKind::Exclude => {} // skip entirely
        }
        pos = *skip_end;
    }
    result.push_str(&source[pos..end_byte]);

    // Clean up double blank lines from exclusions
    while result.contains("\n\n\n") {
        result = result.replace("\n\n\n", "\n\n");
    }

    let start_line = source[..start_byte].matches('\n').count() + 1;
    let mappings = build_collapsed_block_mappings(source, end_byte, &body_ranges, start_line, &result);

    (result, mappings)
}

/// Recursively collect function body byte ranges inside a node.
fn collect_fn_bodies(node: Node, ranges: &mut Vec<(usize, usize)>) {
    let mut cursor = node.walk();
    for child in node.children(&mut cursor) {
        if child.kind() == "function_item" || child.kind() == "method_definition" {
            if let Some(body) = child.child_by_field_name("body") {
                ranges.push((body.start_byte(), body.end_byte()));
            }
        } else if child.kind() == "declaration_list" || child.kind() == "class_body" || child.kind() == "interface_body" || child.kind() == "class_declaration" || child.kind() == "abstract_class_declaration" || child.kind() == "interface_declaration" || child.kind() == "export_statement" {
            // Recurse into block containers
            collect_fn_bodies(child, ranges);
        }
    }
}

/// Build line mappings for a collapsed block.
/// Uses the already-collapsed content string and maps each output line
/// back to its original source line number.
pub fn build_collapsed_block_mappings_pub(
    source: &str,
    end_byte: usize,
    body_ranges: &[(usize, usize)],
    start_line: usize,
    collapsed_content: &str,
) -> Vec<(usize, String)> {
    build_collapsed_block_mappings(source, end_byte, body_ranges, start_line, collapsed_content)
}

fn build_collapsed_block_mappings(
    source: &str,
    end_byte: usize,
    body_ranges: &[(usize, usize)],
    start_line: usize,
    collapsed_content: &str,
) -> Vec<(usize, String)> {
    // Strategy: walk through source lines, skipping body interiors.
    // For each body, the line containing `{` is kept (with replacement in content),
    // and lines inside the body up to the closing `}` line are skipped.
    // Convert body byte ranges to line ranges (0-indexed)
    let body_line_ranges: Vec<(usize, usize)> = body_ranges
        .iter()
        .map(|&(bs, be)| {
            let start_ln = source[..bs].matches('\n').count();
            // end_byte points past `}`, so the `}` line is:
            let end_ln = source[..be].matches('\n').count();
            (start_ln, end_ln)
        })
        .collect();

    let first_line = start_line - 1; // 0-indexed
    let last_line = source[..end_byte].matches('\n').count();

    // Collect which source lines survive (not inside a body range, excluding body start)
    let mut surviving_source_lines = Vec::new();
    let mut src_line = first_line;
    while src_line <= last_line {
        if let Some(range) = body_line_ranges.iter().find(|&&(s, _)| s == src_line) {
            // Body start line — keep it (signature is here)
            surviving_source_lines.push(src_line);
            // Skip to after the body end line
            src_line = range.1 + 1;
        } else if body_line_ranges.iter().any(|&(s, e)| src_line > s && src_line <= e) {
            src_line += 1;
        } else {
            surviving_source_lines.push(src_line);
            src_line += 1;
        }
    }

    // Now zip the collapsed content lines with the surviving source line numbers
    let content_lines: Vec<&str> = collapsed_content.lines().collect();
    content_lines
        .iter()
        .enumerate()
        .map(|(i, line)| {
            let src_ln = surviving_source_lines
                .get(i)
                .map(|&ln| ln + 1)
                .unwrap_or(start_line + i);
            (src_ln, line.to_string())
        })
        .collect()
}

/// Build simple line mappings from content string.
pub fn build_source_line_mappings(content: &str, start_line: usize) -> Vec<(usize, String)> {
    content
        .lines()
        .enumerate()
        .map(|(i, line)| (start_line + i, line.to_string()))
        .collect()
}



#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn collapse_body_simple_fn() {
        let source = "fn foo() {\n    42\n}\n";
        let (collapsed, mappings) = collapse_body(source, 0, source.len(), 9, source.len() - 1);
        assert!(collapsed.contains("{ ... }"));
        assert!(!collapsed.contains("42"));
        assert_eq!(mappings[0].0, 1);
    }

    #[test]
    fn collapse_body_preserves_signature() {
        let source = "pub fn bar(x: i32) -> bool {\n    true\n}";
        let body_start = source.find('{').unwrap();
        let body_end = source.rfind('}').unwrap() + 1;
        let (collapsed, _) = collapse_body(source, 0, source.len(), body_start, body_end);
        assert!(collapsed.starts_with("pub fn bar(x: i32) -> bool"));
        assert!(collapsed.contains("{ ... }"));
    }

    #[test]
    fn collapse_body_no_space_before_brace() {
        let source = "fn foo(){\n    1\n}";
        let body_start = source.find('{').unwrap();
        let body_end = source.rfind('}').unwrap() + 1;
        let (collapsed, _) = collapse_body(source, 0, source.len(), body_start, body_end);
        assert!(collapsed.contains(" { ... }"));
    }

    #[test]
    fn build_source_line_mappings_basic() {
        let content = "line one\nline two\nline three";
        let mappings = build_source_line_mappings(content, 5);
        assert_eq!(mappings.len(), 3);
        assert_eq!(mappings[0], (5, "line one".to_string()));
        assert_eq!(mappings[1], (6, "line two".to_string()));
        assert_eq!(mappings[2], (7, "line three".to_string()));
    }

    #[test]
    fn build_source_line_mappings_single_line() {
        let mappings = build_source_line_mappings("hello", 1);
        assert_eq!(mappings, vec![(1, "hello".to_string())]);
    }

    #[test]
    fn build_source_line_mappings_empty() {
        let mappings = build_source_line_mappings("", 10);
        assert_eq!(mappings.len(), 0);
    }

    #[test]
    fn collapse_body_with_offset() {
        // Item doesn't start at byte 0
        let source = "// comment\nfn foo() {\n    42\n}";
        let item_start = source.find("fn").unwrap();
        let body_start = source.find('{').unwrap();
        let body_end = source.rfind('}').unwrap() + 1;
        let (collapsed, mappings) = collapse_body(source, item_start, source.len(), body_start, body_end);
        assert!(collapsed.contains("{ ... }"));
        assert_eq!(mappings[0].0, 2); // fn is on line 2
    }
}
