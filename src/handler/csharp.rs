use super::{ChildSymbol, LanguageHandler, SymbolInfo};
use crate::extractor::{ItemKind, Visibility};
use std::path::Path;
use tree_sitter::Node;

pub struct CSharpHandler;

/// Symbol query for C# — captures top-level declarations.
const SYMBOL_QUERY: &str = r#"
(class_declaration
  name: (identifier) @name
  body: (declaration_list) @body) @item

(interface_declaration
  name: (identifier) @name
  body: (declaration_list) @body) @item

(enum_declaration
  name: (identifier) @name
  body: (enum_member_declaration_list) @body) @item

(struct_declaration
  name: (identifier) @name
  body: (declaration_list) @body) @item

(record_declaration
  name: (identifier) @name
  body: (declaration_list) @body) @item

(method_declaration
  name: (identifier) @name
  body: (block) @body) @item

(constructor_declaration
  name: (identifier) @name
  body: (block) @body) @item

(namespace_declaration
  name: (_) @name) @item

(using_directive) @item

(property_declaration
  name: (identifier) @name) @item

(field_declaration) @item
"#;

fn csharp_visibility(node: Node, source: &str) -> Visibility {
    let mut cursor = node.walk();
    for child in node.children(&mut cursor) {
        if child.kind() == "modifier" {
            let text = &source[child.byte_range()];
            match text {
                "public" => return Visibility::Public,
                "protected" => return Visibility::Protected,
                "private" => return Visibility::Private,
                "internal" => return Visibility::Crate,
                _ => {}
            }
        }
    }
    // Default: internal for top-level, private for members
    Visibility::Crate
}

/// Extract field name from a field_declaration node.
fn field_name(node: Node, source: &str) -> Option<String> {
    let mut cursor = node.walk();
    for child in node.named_children(&mut cursor) {
        if child.kind() == "variable_declaration" {
            let mut inner = child.walk();
            for var in child.named_children(&mut inner) {
                if var.kind() == "variable_declarator" {
                    if let Some(name_node) = var.child_by_field_name("name") {
                        return Some(source[name_node.byte_range()].to_string());
                    }
                    // Sometimes the identifier is the first child
                    let mut vc = var.walk();
                    for vc_child in var.named_children(&mut vc) {
                        if vc_child.kind() == "identifier" {
                            return Some(source[vc_child.byte_range()].to_string());
                        }
                    }
                }
            }
        }
    }
    None
}

impl LanguageHandler for CSharpHandler {
    fn symbol_query(&self) -> &str {
        SYMBOL_QUERY
    }

    fn classify_node(&self, node: Node, source: &str) -> Option<SymbolInfo> {
        let kind = match node.kind() {
            "class_declaration" => ItemKind::Class,
            "interface_declaration" => ItemKind::Trait,
            "enum_declaration" => ItemKind::Enum,
            "struct_declaration" => ItemKind::Struct,
            "record_declaration" => ItemKind::Struct,
            "method_declaration" => ItemKind::Function,
            "constructor_declaration" => ItemKind::Function,
            "namespace_declaration" => ItemKind::Mod,
            "using_directive" => ItemKind::Use,
            "property_declaration" => ItemKind::Const,
            "field_declaration" => ItemKind::Const,
            _ => return None,
        };

        let name = match node.kind() {
            "using_directive" => None,
            "field_declaration" => field_name(node, source),
            "namespace_declaration" => {
                node.child_by_field_name("name")
                    .map(|n| source[n.byte_range()].to_string())
            }
            _ => node
                .child_by_field_name("name")
                .map(|n| source[n.byte_range()].to_string()),
        };

        Some(SymbolInfo { kind, name })
    }

    fn visibility(&self, node: Node, source: &str) -> Visibility {
        csharp_visibility(node, source)
    }

    fn member_visibility(&self, node: Node, source: &str) -> Visibility {
        csharp_visibility(node, source)
    }

    fn child_symbols<'a>(&self, node: Node<'a>, source: &str) -> Vec<ChildSymbol<'a>> {
        let body = match node.kind() {
            "class_declaration"
            | "struct_declaration"
            | "record_declaration"
            | "interface_declaration" => node.child_by_field_name("body"),
            "enum_declaration" => node.child_by_field_name("body"),
            "namespace_declaration" => node.child_by_field_name("body"),
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
                "property_declaration" => {
                    let name = child
                        .child_by_field_name("name")
                        .map(|n| source[n.byte_range()].to_string());
                    result.push(ChildSymbol {
                        node: child,
                        kind: ItemKind::Const,
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
                "struct_declaration" => {
                    let name = child
                        .child_by_field_name("name")
                        .map(|n| source[n.byte_range()].to_string());
                    result.push(ChildSymbol {
                        node: child,
                        kind: ItemKind::Struct,
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
            if child.kind() == "modifier" {
                parts.push(source[child.byte_range()].to_string());
            }
        }

        // Return type (for methods)
        if node.kind() == "method_declaration"
            && let Some(ret) = node.child_by_field_name("type") {
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
            "struct_declaration",
            "record_declaration",
            "method_declaration",
            "constructor_declaration",
            "namespace_declaration",
            "property_declaration",
            "local_declaration_statement",
            "for_statement",
            "foreach_statement",
        ]
    }

    fn identifier_node_kinds(&self) -> &[&str] {
        &["identifier", "generic_name"]
    }

    fn parse_imports(
        &self,
        _root: Node,
        _source: &str,
        _file_path: &Path,
        _project_root: &Path,
    ) -> Vec<crate::xrefs::ImportEdge> {
        vec![]
    }

    fn is_test_file(&self, path: &Path) -> bool {
        let path_str = path.to_string_lossy();
        let stem = path.file_stem().and_then(|s| s.to_str()).unwrap_or("");
        crate::test_detect::has_test_dir_component(&path_str)
            || stem.ends_with("Tests")
            || stem.ends_with("Test")
            || stem.starts_with("Test")
    }

    fn max_query_depth(&self) -> u32 {
        // C# declarations can be inside namespace { declaration_list { ... } }
        4
    }

    fn is_test_item(&self, node: Node, source: &str) -> bool {
        // Check for [Test], [Fact], [Theory] attributes
        if node.kind() == "method_declaration" {
            let mut cursor = node.walk();
            for child in node.children(&mut cursor) {
                if child.kind() == "attribute_list" {
                    let text = &source[child.byte_range()];
                    if text.contains("Test") || text.contains("Fact") || text.contains("Theory") {
                        return true;
                    }
                }
            }
        }
        false
    }
}
