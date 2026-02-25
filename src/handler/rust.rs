use super::{ChildSymbol, LanguageHandler, SymbolInfo};
use crate::extractor::{ItemKind, Visibility};
use tree_sitter::Node;
use std::path::Path;

pub struct RustHandler;

/// Symbol query for Rust — captures top-level declarations.
/// Impl blocks are captured as @item without @name (the type is resolved in classify_node).
const SYMBOL_QUERY: &str = r#"
(function_item
  name: (identifier) @name
  body: (block) @body) @item

(struct_item
  name: (type_identifier) @name) @item

(enum_item
  name: (type_identifier) @name) @item

(trait_item
  name: (type_identifier) @name) @item

(impl_item
  type: (_) @name) @item

(mod_item
  name: (identifier) @name) @item

(const_item
  name: (identifier) @name) @item

(static_item
  name: (identifier) @name) @item

(type_item
  name: (type_identifier) @name) @item

(macro_definition
  name: (identifier) @name) @item

(use_declaration) @item
"#;

impl LanguageHandler for RustHandler {
    fn symbol_query(&self) -> &str {
        SYMBOL_QUERY
    }

    fn classify_node(&self, node: Node, source: &str) -> Option<SymbolInfo> {
        let kind = match node.kind() {
            "function_item" => ItemKind::Function,
            "struct_item" => ItemKind::Struct,
            "enum_item" => ItemKind::Enum,
            "trait_item" => ItemKind::Trait,
            "impl_item" => ItemKind::Impl,
            "mod_item" => ItemKind::Mod,
            "const_item" => ItemKind::Const,
            "static_item" => ItemKind::Static,
            "type_item" => ItemKind::TypeAlias,
            "macro_definition" => ItemKind::MacroDef,
            "use_declaration" => ItemKind::Use,
            _ => return None,
        };

        let name = match node.kind() {
            "impl_item" => {
                // For impl blocks, use the type name (e.g., "MyStruct" from `impl MyStruct`)
                node.child_by_field_name("type")
                    .map(|n| source[n.byte_range()].to_string())
            }
            "use_declaration" => None,
            _ => {
                node.child_by_field_name("name")
                    .map(|n| source[n.byte_range()].to_string())
            }
        };

        Some(SymbolInfo { kind, name })
    }

    fn visibility(&self, node: Node, source: &str) -> Visibility {
        has_pub_modifier(node, source)
    }

    fn member_visibility(&self, node: Node, source: &str) -> Visibility {
        has_pub_modifier(node, source)
    }

    fn child_symbols<'a>(&self, node: Node<'a>, source: &str) -> Vec<ChildSymbol<'a>> {
        let mut result = Vec::new();

        match node.kind() {
            "impl_item" => {
                // Walk the body (declaration_list) for methods and associated items
                if let Some(body) = node.child_by_field_name("body") {
                    let mut cursor = body.walk();
                    for child in body.named_children(&mut cursor) {
                        match child.kind() {
                            "function_item" => {
                                let name = child.child_by_field_name("name")
                                    .map(|n| source[n.byte_range()].to_string());
                                result.push(ChildSymbol {
                                    node: child,
                                    kind: ItemKind::Method,
                                    name,
                                });
                            }
                            "const_item" => {
                                let name = child.child_by_field_name("name")
                                    .map(|n| source[n.byte_range()].to_string());
                                result.push(ChildSymbol {
                                    node: child,
                                    kind: ItemKind::Const,
                                    name,
                                });
                            }
                            "type_item" => {
                                let name = child.child_by_field_name("name")
                                    .map(|n| source[n.byte_range()].to_string());
                                result.push(ChildSymbol {
                                    node: child,
                                    kind: ItemKind::TypeAlias,
                                    name,
                                });
                            }
                            _ => {}
                        }
                    }
                }
            }
            "trait_item" => {
                if let Some(body) = node.child_by_field_name("body") {
                    let mut cursor = body.walk();
                    for child in body.named_children(&mut cursor) {
                        match child.kind() {
                            "function_item" => {
                                let name = child.child_by_field_name("name")
                                    .map(|n| source[n.byte_range()].to_string());
                                result.push(ChildSymbol {
                                    node: child,
                                    kind: ItemKind::Method,
                                    name,
                                });
                            }
                            "function_signature_item" => {
                                let name = child.child_by_field_name("name")
                                    .map(|n| source[n.byte_range()].to_string());
                                result.push(ChildSymbol {
                                    node: child,
                                    kind: ItemKind::Method,
                                    name,
                                });
                            }
                            "const_item" => {
                                let name = child.child_by_field_name("name")
                                    .map(|n| source[n.byte_range()].to_string());
                                result.push(ChildSymbol {
                                    node: child,
                                    kind: ItemKind::Const,
                                    name,
                                });
                            }
                            "type_item" => {
                                let name = child.child_by_field_name("name")
                                    .map(|n| source[n.byte_range()].to_string());
                                result.push(ChildSymbol {
                                    node: child,
                                    kind: ItemKind::TypeAlias,
                                    name,
                                });
                            }
                            _ => {}
                        }
                    }
                }
            }
            "enum_item" => {
                if let Some(body) = node.child_by_field_name("body") {
                    let mut cursor = body.walk();
                    for child in body.named_children(&mut cursor) {
                        if child.kind() == "enum_variant" {
                            let name = child.child_by_field_name("name")
                                .map(|n| source[n.byte_range()].to_string());
                            result.push(ChildSymbol {
                                node: child,
                                kind: ItemKind::Const, // Variants treated as const-like
                                name,
                            });
                        }
                    }
                }
            }
            "struct_item" => {
                // Struct fields from the field_declaration_list
                if let Some(body) = node.child_by_field_name("body") {
                    let mut cursor = body.walk();
                    for child in body.named_children(&mut cursor) {
                        if child.kind() == "field_declaration" {
                            let name = child.child_by_field_name("name")
                                .map(|n| source[n.byte_range()].to_string());
                            result.push(ChildSymbol {
                                node: child,
                                kind: ItemKind::Const, // Fields treated as const-like
                                name,
                            });
                        }
                    }
                }
            }
            _ => {}
        }

        result
    }

    fn signature(&self, node: Node, source: &str) -> String {
        // Build signature from parts up to (but not including) the body block
        let mut parts = Vec::new();

        // Check for visibility modifier
        let mut cursor = node.walk();
        for child in node.children(&mut cursor) {
            match child.kind() {
                "visibility_modifier" => {
                    parts.push(source[child.byte_range()].to_string());
                }
                "function_modifiers" => {
                    parts.push(source[child.byte_range()].to_string());
                }
                _ => {}
            }
        }

        // fn keyword + name
        parts.push("fn".to_string());

        if let Some(name) = node.child_by_field_name("name") {
            let mut name_str = source[name.byte_range()].to_string();

            // type parameters
            let mut cursor2 = node.walk();
            for child in node.children(&mut cursor2) {
                if child.kind() == "type_parameters" {
                    name_str.push_str(&source[child.byte_range()]);
                }
            }
            parts.push(name_str);
        }

        if let Some(params) = node.child_by_field_name("parameters") {
            parts.push(source[params.byte_range()].to_string());
        }

        // return type
        if let Some(ret) = node.child_by_field_name("return_type") {
            parts.push(format!("-> {}", &source[ret.byte_range()]));
        }

        // where clause
        let mut cursor3 = node.walk();
        for child in node.children(&mut cursor3) {
            if child.kind() == "where_clause" {
                parts.push(source[child.byte_range()].to_string());
            }
        }

        parts.join(" ")
    }

    fn body_field_name(&self) -> &str {
        "body"
    }

    fn definition_parent_kinds(&self) -> &[&str] {
        &[
            "function_item", "struct_item", "enum_item", "trait_item",
            "impl_item", "mod_item", "const_item", "static_item",
            "type_item", "macro_definition", "let_declaration",
            "parameter", "closure_expression",
        ]
    }

    fn identifier_node_kinds(&self) -> &[&str] {
        &["identifier", "type_identifier", "field_identifier"]
    }

    fn parse_imports(
        &self,
        root: Node,
        source: &str,
        file_path: &Path,
        _project_root: &Path,
    ) -> Vec<crate::xrefs::ImportEdge> {
        crate::xrefs::parse_rust_imports_impl(&root, source, file_path)
    }

    fn is_test_file(&self, path: &Path) -> bool {
        let path_str = path.to_string_lossy();
        crate::test_detect::has_test_dir_component(&path_str)
    }

    fn is_test_item(&self, node: Node, source: &str) -> bool {
        // Check for #[test] or #[cfg(test)] attributes
        if let Some(prev) = node.prev_sibling() {
            if prev.kind() == "attribute_item" {
                let text = &source[prev.byte_range()];
                if text.contains("#[test]") || text.contains("#[cfg(test)]") {
                    return true;
                }
            }
        }
        // Check for #[test] on the node itself via attribute field
        let mut cursor = node.walk();
        for child in node.children(&mut cursor) {
            if child.kind() == "attribute_item" {
                let text = &source[child.byte_range()];
                if text.contains("#[test]") {
                    return true;
                }
            }
        }
        false
    }
}

/// Check if a node has a `pub` visibility modifier.
fn has_pub_modifier(node: Node, source: &str) -> Visibility {
    let mut cursor = node.walk();
    for child in node.children(&mut cursor) {
        if child.kind() == "visibility_modifier" {
            let text = &source[child.byte_range()];
            if text.starts_with("pub") {
                return Visibility::Public;
            }
        }
    }
    Visibility::Private
}
