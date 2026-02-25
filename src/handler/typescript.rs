use super::{ChildSymbol, LanguageHandler, SymbolInfo};
use crate::extractor::{ItemKind, Visibility};
use tree_sitter::Node;
use std::path::Path;

pub struct TypeScriptHandler;

/// The symbol query for TypeScript — captures top-level declarations at any depth.
/// Symbol query for TypeScript — captures all declaration types.
const SYMBOL_QUERY: &str = r#"
(function_declaration
  name: (identifier) @name
  body: (statement_block) @body) @item

(class_declaration
  name: (type_identifier) @name
  body: (class_body) @body) @item

(abstract_class_declaration
  name: (type_identifier) @name
  body: (class_body) @body) @item

(interface_declaration
  name: (type_identifier) @name
  body: (interface_body) @body) @item

(type_alias_declaration
  name: (type_identifier) @name) @item

(enum_declaration
  name: (identifier) @name
  body: (enum_body) @body) @item

(lexical_declaration
  (variable_declarator
    name: (identifier) @name)) @item

(import_statement) @item

(export_statement
  (function_declaration
    name: (identifier) @name
    body: (statement_block) @body)) @item

(export_statement
  (class_declaration
    name: (type_identifier) @name
    body: (class_body) @body)) @item

(export_statement
  (abstract_class_declaration
    name: (type_identifier) @name
    body: (class_body) @body)) @item

(export_statement
  (interface_declaration
    name: (type_identifier) @name
    body: (interface_body) @body)) @item

(export_statement
  (type_alias_declaration
    name: (type_identifier) @name)) @item

(export_statement
  (enum_declaration
    name: (identifier) @name
    body: (enum_body) @body)) @item

(export_statement
  (lexical_declaration
    (variable_declarator
      name: (identifier) @name))) @item
"#;

impl LanguageHandler for TypeScriptHandler {
    fn symbol_query(&self) -> &str {
        SYMBOL_QUERY
    }

    fn classify_node(&self, node: Node, source: &str) -> Option<SymbolInfo> {
        let (kind_node, name_source) = if node.kind() == "export_statement" {
            // Unwrap to inner declaration
            let mut cursor = node.walk();
            let inner = node.named_children(&mut cursor)
                .find(|c| c.kind() != "decorator")?;
            (inner, source)
        } else {
            (node, source)
        };

        let kind = match kind_node.kind() {
            "function_declaration" => ItemKind::Function,
            "class_declaration" | "abstract_class_declaration" => ItemKind::Class,
            "interface_declaration" => ItemKind::Trait,
            "type_alias_declaration" => ItemKind::TypeAlias,
            "enum_declaration" => ItemKind::Enum,
            "import_statement" => ItemKind::Use,
            "lexical_declaration" => ItemKind::Const,
            _ => return None,
        };

        let name = extract_name(kind_node, name_source);
        Some(SymbolInfo { kind, name })
    }

    fn visibility(&self, node: Node, _source: &str) -> Visibility {
        if node.kind() == "export_statement" {
            Visibility::Public
        } else if let Some(parent) = node.parent() {
            if parent.kind() == "export_statement" {
                Visibility::Public
            } else {
                Visibility::Private
            }
        } else {
            Visibility::Private
        }
    }

    fn member_visibility(&self, node: Node, source: &str) -> Visibility {
        // Check for #private names (ES2022)
        if let Some(name_node) = node.child_by_field_name("name") {
            if name_node.kind() == "private_property_identifier" {
                return Visibility::Private;
            }
            let name_text = &source[name_node.byte_range()];
            if name_text.starts_with('#') {
                return Visibility::Private;
            }
            // Convention: __ prefix = private
            if name_text.starts_with("__") {
                return Visibility::Private;
            }
        }
        // Check for accessibility modifier
        let mut cursor = node.walk();
        for child in node.children(&mut cursor) {
            if child.kind() == "accessibility_modifier" {
                let text = &source[child.byte_range()];
                return match text {
                    "public" => Visibility::Public,
                    "protected" => Visibility::Protected,
                    "private" => Visibility::Private,
                    _ => Visibility::Private,
                };
            }
        }
        // Default: public in TS classes (no modifier = public)
        Visibility::Public
    }

    fn child_symbols<'a>(&self, node: Node<'a>, source: &str) -> Vec<ChildSymbol<'a>> {
        let body = match node.child_by_field_name("body") {
            Some(b) => b,
            None => return vec![],
        };

        let mut result = Vec::new();
        let mut cursor = body.walk();
        for child in body.named_children(&mut cursor) {
            match child.kind() {
                "method_definition" => {
                    let name = child.child_by_field_name("name")
                        .map(|n| source[n.byte_range()].to_string());
                    result.push(ChildSymbol {
                        node: child,
                        kind: ItemKind::Method,
                        name,
                    });
                }
                "abstract_method_signature" => {
                    let name = child.child_by_field_name("name")
                        .map(|n| source[n.byte_range()].to_string());
                    result.push(ChildSymbol {
                        node: child,
                        kind: ItemKind::Method,
                        name,
                    });
                }
                "public_field_definition" | "property_definition" => {
                    let name = child.child_by_field_name("name")
                        .map(|n| source[n.byte_range()].to_string());
                    result.push(ChildSymbol {
                        node: child,
                        kind: ItemKind::Const,
                        name,
                    });
                }
                "property_signature" => {
                    let name = child.child_by_field_name("name")
                        .map(|n| source[n.byte_range()].to_string());
                    result.push(ChildSymbol {
                        node: child,
                        kind: ItemKind::Const,
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

        let mut cursor = node.walk();
        for child in node.children(&mut cursor) {
            match child.kind() {
                "accessibility_modifier" | "readonly" | "async" | "static" => {
                    parts.push(source[child.byte_range()].to_string());
                }
                _ => {}
            }
        }

        if let Some(name) = node.child_by_field_name("name") {
            parts.push(source[name.byte_range()].to_string());
        }

        // type parameters
        let mut cursor2 = node.walk();
        for child in node.children(&mut cursor2) {
            if child.kind() == "type_parameters" {
                parts.push(source[child.byte_range()].to_string());
            }
        }

        if let Some(params) = node.child_by_field_name("parameters") {
            parts.push(source[params.byte_range()].to_string());
        }

        // return type
        let mut cursor3 = node.walk();
        for child in node.children(&mut cursor3) {
            if child.kind() == "type_annotation" {
                parts.push(source[child.byte_range()].to_string());
            }
        }

        parts.join(" ")
    }

    fn definition_parent_kinds(&self) -> &[&str] {
        &[
            "function_declaration", "class_declaration", "interface_declaration",
            "type_alias_declaration", "enum_declaration", "method_definition",
            "variable_declarator", "property_signature", "public_field_definition",
            "required_parameter", "optional_parameter",
        ]
    }

    fn identifier_node_kinds(&self) -> &[&str] {
        &["identifier", "type_identifier", "property_identifier", "shorthand_property_identifier_pattern"]
    }

    fn parse_imports(
        &self,
        root: Node,
        source: &str,
        file_path: &Path,
        project_root: &Path,
    ) -> Vec<crate::xrefs::ImportEdge> {
        crate::xrefs::parse_js_ts_imports_impl(&root, source, file_path, project_root)
    }

    fn is_test_file(&self, path: &Path) -> bool {
        let path_str = path.to_string_lossy();
        let stem = path.file_stem().and_then(|s| s.to_str()).unwrap_or("");
        crate::test_detect::has_test_dir_component(&path_str)
            || crate::test_detect::is_js_ts_test_filename(stem, "")
    }

    fn is_test_item(&self, _node: Node, _source: &str) -> bool {
        false
    }
}

/// Extract the name from a declaration node.
fn extract_name(node: Node, source: &str) -> Option<String> {
    match node.kind() {
        "lexical_declaration" => {
            let mut cursor = node.walk();
            for child in node.children(&mut cursor) {
                if child.kind() == "variable_declarator" {
                    return child.child_by_field_name("name")
                        .map(|n| source[n.byte_range()].to_string());
                }
            }
            None
        }
        "import_statement" => None,
        _ => {
            node.child_by_field_name("name")
                .map(|n| source[n.byte_range()].to_string())
        }
    }
}
