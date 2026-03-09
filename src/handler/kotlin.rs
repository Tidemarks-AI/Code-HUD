use super::{ChildSymbol, LanguageHandler, SymbolInfo};
use crate::extractor::{ItemKind, Visibility};
use std::path::Path;
use tree_sitter::Node;

pub struct KotlinHandler;

/// Symbol query for Kotlin — captures top-level declarations.
const SYMBOL_QUERY: &str = r#"
(class_declaration) @item

(object_declaration) @item

(function_declaration) @item

(property_declaration) @item

(import_header) @item

(package_header) @item

(type_alias) @item
"#;

/// Find the first child of a given kind.
fn first_child_of_kind<'a>(node: Node<'a>, kind: &str) -> Option<Node<'a>> {
    let mut cursor = node.walk();
    node.named_children(&mut cursor)
        .find(|&child| child.kind() == kind)
}

/// Extract the name from a node that uses `type_identifier` for its name (classes, objects).
fn type_name(node: Node, source: &str) -> Option<String> {
    first_child_of_kind(node, "type_identifier").map(|n| source[n.byte_range()].to_string())
}

/// Extract the name from a node that uses `simple_identifier` (functions, properties).
fn simple_name(node: Node, source: &str) -> Option<String> {
    first_child_of_kind(node, "simple_identifier").map(|n| source[n.byte_range()].to_string())
}

/// Extract property name from a property_declaration.
/// Properties may use `variable_declaration` which contains `simple_identifier`.
fn property_name(node: Node, source: &str) -> Option<String> {
    // Try direct simple_identifier first
    if let Some(name) = simple_name(node, source) {
        return Some(name);
    }
    // Try variable_declaration > simple_identifier
    if let Some(var_decl) = first_child_of_kind(node, "variable_declaration") {
        return simple_name(var_decl, source);
    }
    None
}

/// Check if modifiers contain a specific text.
fn has_modifier_text(node: Node, source: &str, keyword: &str) -> bool {
    if let Some(mods) = first_child_of_kind(node, "modifiers") {
        let text = &source[mods.byte_range()];
        return text.split_whitespace().any(|w| w == keyword);
    }
    false
}

/// Check if this is a data class.
fn is_data_class(node: Node, source: &str) -> bool {
    has_modifier_text(node, source, "data")
}

/// Check if this is a sealed class.
fn is_sealed_class(node: Node, source: &str) -> bool {
    has_modifier_text(node, source, "sealed")
}

/// Check if this is an enum class.
fn is_enum_class(node: Node, source: &str) -> bool {
    // Kotlin enums have an enum_class_body child
    first_child_of_kind(node, "enum_class_body").is_some()
        || has_modifier_text(node, source, "enum")
}

/// Determine visibility from modifiers.
fn kotlin_visibility(node: Node, source: &str) -> Visibility {
    if let Some(mods) = first_child_of_kind(node, "modifiers") {
        let mut cursor = mods.walk();
        for child in mods.named_children(&mut cursor) {
            if child.kind() == "visibility_modifier" {
                let text = &source[child.byte_range()];
                return match text {
                    "public" => Visibility::Public,
                    "private" => Visibility::Private,
                    "protected" => Visibility::Protected,
                    "internal" => Visibility::Crate,
                    _ => Visibility::Public,
                };
            }
        }
    }
    // Kotlin default is public
    Visibility::Public
}

/// Classify a class_declaration into the right ItemKind.
fn classify_class(node: Node, source: &str) -> ItemKind {
    if is_enum_class(node, source) {
        ItemKind::Enum
    } else if is_data_class(node, source) {
        ItemKind::Struct
    } else if is_sealed_class(node, source) {
        ItemKind::Trait
    } else {
        // Check for "interface" keyword — Kotlin interfaces are also class_declaration
        // Actually interfaces are separate... let me check
        // In tree-sitter-kotlin, interfaces are class_declaration with "interface" keyword
        if has_modifier_text(node, source, "interface") {
            ItemKind::Trait
        } else {
            // Check if the node text starts with "interface"
            let text = &source[node.byte_range()];
            let trimmed = text.trim_start();
            // After stripping modifiers, check for interface keyword
            if trimmed.contains("interface ") && !trimmed.starts_with("class ") {
                ItemKind::Trait
            } else {
                ItemKind::Class
            }
        }
    }
}

impl LanguageHandler for KotlinHandler {
    fn symbol_query(&self) -> &str {
        SYMBOL_QUERY
    }

    fn classify_node(&self, node: Node, source: &str) -> Option<SymbolInfo> {
        let kind = match node.kind() {
            "class_declaration" => classify_class(node, source),
            "object_declaration" => ItemKind::Class,
            "function_declaration" => ItemKind::Function,
            "property_declaration" => ItemKind::Const,
            "import_header" => ItemKind::Use,
            "package_header" => ItemKind::Mod,
            "type_alias" => ItemKind::TypeAlias,
            _ => return None,
        };

        let name = match node.kind() {
            "import_header" | "package_header" => None,
            "class_declaration" | "object_declaration" | "type_alias" => type_name(node, source),
            "function_declaration" => simple_name(node, source),
            "property_declaration" => property_name(node, source),
            _ => None,
        };

        Some(SymbolInfo { kind, name })
    }

    fn visibility(&self, node: Node, source: &str) -> Visibility {
        kotlin_visibility(node, source)
    }

    fn member_visibility(&self, node: Node, source: &str) -> Visibility {
        kotlin_visibility(node, source)
    }

    fn child_symbols<'a>(&self, node: Node<'a>, source: &str) -> Vec<ChildSymbol<'a>> {
        let body = match node.kind() {
            "class_declaration" => first_child_of_kind(node, "class_body")
                .or_else(|| first_child_of_kind(node, "enum_class_body")),
            "object_declaration" | "companion_object" => first_child_of_kind(node, "class_body"),
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
                "function_declaration" => {
                    let name = simple_name(child, source);
                    result.push(ChildSymbol {
                        node: child,
                        kind: ItemKind::Method,
                        name,
                    });
                }
                "property_declaration" => {
                    let name = property_name(child, source);
                    result.push(ChildSymbol {
                        node: child,
                        kind: ItemKind::Const,
                        name,
                    });
                }
                "secondary_constructor" => {
                    result.push(ChildSymbol {
                        node: child,
                        kind: ItemKind::Method,
                        name: Some("constructor".to_string()),
                    });
                }
                "companion_object" => {
                    let name = type_name(child, source).unwrap_or_else(|| "Companion".to_string());
                    result.push(ChildSymbol {
                        node: child,
                        kind: ItemKind::Class,
                        name: Some(name),
                    });
                }
                "class_declaration" => {
                    let name = type_name(child, source);
                    result.push(ChildSymbol {
                        node: child,
                        kind: classify_class(child, source),
                        name,
                    });
                }
                "object_declaration" => {
                    let name = type_name(child, source);
                    result.push(ChildSymbol {
                        node: child,
                        kind: ItemKind::Class,
                        name,
                    });
                }
                "enum_entry" => {
                    let name = simple_name(child, source);
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

        // Collect modifiers
        if let Some(mods) = first_child_of_kind(node, "modifiers") {
            parts.push(source[mods.byte_range()].to_string());
        }

        // fun keyword + name
        if node.kind() == "function_declaration" {
            parts.push("fun".to_string());
            if let Some(name) = simple_name(node, source) {
                parts.push(name);
            }
            // Parameters
            if let Some(params) = first_child_of_kind(node, "function_value_parameters") {
                let last = parts.pop().unwrap_or_default();
                parts.push(format!("{}{}", last, &source[params.byte_range()]));
            }
            // Return type: find the type after parameters (user_type, nullable_type, etc.)
            let mut cursor = node.walk();
            let mut after_params = false;
            for child in node.named_children(&mut cursor) {
                if child.kind() == "function_value_parameters" {
                    after_params = true;
                    continue;
                }
                if after_params && child.kind() != "function_body" && child.kind() != "modifiers" {
                    let last = parts.pop().unwrap_or_default();
                    parts.push(format!("{}: {}", last, &source[child.byte_range()]));
                    break;
                }
            }
        } else if node.kind() == "secondary_constructor" {
            parts.push("constructor".to_string());
            if let Some(params) = first_child_of_kind(node, "function_value_parameters") {
                let last = parts.pop().unwrap_or_default();
                parts.push(format!("{}{}", last, &source[params.byte_range()]));
            }
        }

        parts.join(" ")
    }

    fn body_field_name(&self) -> &str {
        "body"
    }

    fn definition_parent_kinds(&self) -> &[&str] {
        &[
            "class_declaration",
            "object_declaration",
            "companion_object",
            "function_declaration",
            "secondary_constructor",
            "property_declaration",
            "for_statement",
            "when_entry",
            "lambda_literal",
        ]
    }

    fn identifier_node_kinds(&self) -> &[&str] {
        &["simple_identifier", "type_identifier"]
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
            || stem.ends_with("Test")
            || stem.ends_with("Tests")
            || stem.ends_with("Spec")
    }

    fn is_test_item(&self, node: Node, source: &str) -> bool {
        // Check for @Test annotation in modifiers
        if let Some(mods) = first_child_of_kind(node, "modifiers") {
            let mut cursor = mods.walk();
            for child in mods.named_children(&mut cursor) {
                if child.kind() == "annotation" {
                    let text = &source[child.byte_range()];
                    if text.contains("Test") {
                        return true;
                    }
                }
            }
        }
        false
    }
}
