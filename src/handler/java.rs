use super::{ChildSymbol, LanguageHandler, SymbolInfo};
use crate::extractor::{ItemKind, Visibility};
use std::path::Path;
use tree_sitter::Node;

pub struct JavaHandler;

/// Symbol query for Java — captures top-level declarations.
const SYMBOL_QUERY: &str = r#"
(class_declaration
  name: (identifier) @name
  body: (class_body) @body) @item

(interface_declaration
  name: (identifier) @name
  body: (interface_body) @body) @item

(enum_declaration
  name: (identifier) @name
  body: (enum_body) @body) @item

(record_declaration
  name: (identifier) @name
  body: (class_body) @body) @item

(annotation_type_declaration
  name: (identifier) @name
  body: (annotation_type_body) @body) @item

(method_declaration
  name: (identifier) @name
  body: (block) @body) @item

(constructor_declaration
  name: (identifier) @name
  body: (constructor_body) @body) @item

(import_declaration) @item

(package_declaration) @item

(field_declaration) @item
"#;

/// Check if a node has a specific modifier keyword.
fn has_modifier(node: Node, source: &str, keyword: &str) -> bool {
    let mut cursor = node.walk();
    for child in node.children(&mut cursor) {
        if child.kind() == "modifiers" {
            let text = &source[child.byte_range()];
            return text.split_whitespace().any(|w| w == keyword);
        }
    }
    false
}

fn java_visibility(node: Node, source: &str) -> Visibility {
    let mut cursor = node.walk();
    for child in node.children(&mut cursor) {
        if child.kind() == "modifiers" {
            let text = &source[child.byte_range()];
            if text.contains("public") {
                return Visibility::Public;
            } else if text.contains("protected") {
                return Visibility::Protected;
            } else if text.contains("private") {
                return Visibility::Private;
            }
            // package-private (no access modifier)
            return Visibility::Crate;
        }
    }
    // No modifiers → package-private
    Visibility::Crate
}

/// Extract field name from a field_declaration node.
fn field_name(node: Node, source: &str) -> Option<String> {
    let mut cursor = node.walk();
    for child in node.named_children(&mut cursor) {
        if child.kind() == "variable_declarator"
            && let Some(name_node) = child.child_by_field_name("name")
        {
            return Some(source[name_node.byte_range()].to_string());
        }
    }
    None
}

impl LanguageHandler for JavaHandler {
    fn symbol_query(&self) -> &str {
        SYMBOL_QUERY
    }

    fn classify_node(&self, node: Node, source: &str) -> Option<SymbolInfo> {
        let kind = match node.kind() {
            "class_declaration" => ItemKind::Class,
            "interface_declaration" => ItemKind::Trait,
            "enum_declaration" => ItemKind::Enum,
            "record_declaration" => ItemKind::Struct,
            "annotation_type_declaration" => ItemKind::MacroDef,
            "method_declaration" => ItemKind::Function,
            "constructor_declaration" => ItemKind::Function,
            "import_declaration" => ItemKind::Use,
            "package_declaration" => ItemKind::Mod,
            "field_declaration" => ItemKind::Const,
            _ => return None,
        };

        let name = match node.kind() {
            "import_declaration" | "package_declaration" => None,
            "field_declaration" => field_name(node, source),
            _ => node
                .child_by_field_name("name")
                .map(|n| source[n.byte_range()].to_string()),
        };

        Some(SymbolInfo { kind, name })
    }

    fn visibility(&self, node: Node, source: &str) -> Visibility {
        java_visibility(node, source)
    }

    fn member_visibility(&self, node: Node, source: &str) -> Visibility {
        java_visibility(node, source)
    }

    fn child_symbols<'a>(&self, node: Node<'a>, source: &str) -> Vec<ChildSymbol<'a>> {
        let body = match node.kind() {
            "class_declaration" | "record_declaration" => node.child_by_field_name("body"),
            "interface_declaration" => node.child_by_field_name("body"),
            "enum_declaration" => node.child_by_field_name("body"),
            "annotation_type_declaration" => node.child_by_field_name("body"),
            _ => return vec![],
        };

        let body = match body {
            Some(b) => b,
            None => return vec![],
        };

        let mut result = Vec::new();
        let mut cursor = body.walk();
        for child in body.named_children(&mut cursor) {
            match child.kind() {
                "method_declaration" => {
                    let name = child
                        .child_by_field_name("name")
                        .map(|n| source[n.byte_range()].to_string());
                    result.push(ChildSymbol {
                        node: child,
                        kind: ItemKind::Method,
                        name,
                    });
                }
                "constructor_declaration" => {
                    let name = child
                        .child_by_field_name("name")
                        .map(|n| source[n.byte_range()].to_string());
                    result.push(ChildSymbol {
                        node: child,
                        kind: ItemKind::Method,
                        name,
                    });
                }
                "field_declaration" => {
                    let name = field_name(child, source);
                    result.push(ChildSymbol {
                        node: child,
                        kind: ItemKind::Const,
                        name,
                    });
                }
                "class_declaration" => {
                    let name = child
                        .child_by_field_name("name")
                        .map(|n| source[n.byte_range()].to_string());
                    result.push(ChildSymbol {
                        node: child,
                        kind: ItemKind::Class,
                        name,
                    });
                }
                "interface_declaration" => {
                    let name = child
                        .child_by_field_name("name")
                        .map(|n| source[n.byte_range()].to_string());
                    result.push(ChildSymbol {
                        node: child,
                        kind: ItemKind::Trait,
                        name,
                    });
                }
                "enum_declaration" => {
                    let name = child
                        .child_by_field_name("name")
                        .map(|n| source[n.byte_range()].to_string());
                    result.push(ChildSymbol {
                        node: child,
                        kind: ItemKind::Enum,
                        name,
                    });
                }
                _ => {}
            }
        }
        result
    }

    fn signature(&self, node: Node, source: &str) -> String {
        let mut parts = Vec::new();

        // Collect modifiers
        let mut cursor = node.walk();
        for child in node.children(&mut cursor) {
            if child.kind() == "modifiers" {
                parts.push(source[child.byte_range()].to_string());
                break;
            }
        }

        // Return type (for methods)
        if node.kind() == "method_declaration"
            && let Some(ret) = node.child_by_field_name("type")
        {
            parts.push(source[ret.byte_range()].to_string());
        }

        // Name
        if let Some(name) = node.child_by_field_name("name") {
            parts.push(source[name.byte_range()].to_string());
        }

        // Parameters
        if let Some(params) = node.child_by_field_name("parameters") {
            let last = parts.pop().unwrap_or_default();
            parts.push(format!("{}{}", last, &source[params.byte_range()]));
        }

        parts.join(" ")
    }

    fn body_field_name(&self) -> &str {
        "body"
    }

    fn definition_parent_kinds(&self) -> &[&str] {
        &[
            "class_declaration",
            "interface_declaration",
            "enum_declaration",
            "record_declaration",
            "method_declaration",
            "constructor_declaration",
            "annotation_type_declaration",
            "formal_parameters",
            "local_variable_declaration",
            "for_statement",
            "enhanced_for_statement",
        ]
    }

    fn identifier_node_kinds(&self) -> &[&str] {
        &["identifier", "type_identifier"]
    }

    fn parse_imports(
        &self,
        _root: Node,
        _source: &str,
        _file_path: &Path,
        _project_root: &Path,
    ) -> Vec<crate::xrefs::ImportEdge> {
        // Java imports don't map to file paths in the same way as other languages
        vec![]
    }

    fn is_test_file(&self, path: &Path) -> bool {
        let path_str = path.to_string_lossy();
        let stem = path.file_stem().and_then(|s| s.to_str()).unwrap_or("");
        crate::test_detect::has_test_dir_component(&path_str)
            || stem.starts_with("Test")
            || stem.ends_with("Test")
            || stem.ends_with("Tests")
            || stem.ends_with("IT")
    }

    fn is_test_item(&self, node: Node, source: &str) -> bool {
        // Check for @Test annotation
        if node.kind() == "method_declaration" {
            if let Some(parent) = node.parent() {
                let _ = parent; // methods with @Test
            }
            // Check preceding siblings for annotations
            let mut prev = node.prev_named_sibling();
            while let Some(p) = prev {
                if p.kind() == "marker_annotation" || p.kind() == "annotation" {
                    let text = &source[p.byte_range()];
                    if text.contains("Test") {
                        return true;
                    }
                }
                prev = p.prev_named_sibling();
            }
        }
        // Check for @Test in modifiers
        has_modifier(node, source, "@Test") || {
            let mut cursor = node.walk();
            node.children(&mut cursor).any(|child| {
                if child.kind() == "modifiers" {
                    let mut inner_cursor = child.walk();
                    child.named_children(&mut inner_cursor).any(|ann| {
                        (ann.kind() == "marker_annotation" || ann.kind() == "annotation")
                            && source[ann.byte_range()].contains("Test")
                    })
                } else {
                    false
                }
            })
        }
    }
}
