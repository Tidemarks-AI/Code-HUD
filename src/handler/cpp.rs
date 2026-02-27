use super::{ChildSymbol, LanguageHandler, SymbolInfo};
use crate::extractor::{ItemKind, Visibility};
use std::path::Path;
use tree_sitter::Node;

pub struct CppHandler;

/// Symbol query for C++ — captures top-level declarations.
///
/// C++ tree-sitter grammar wraps templates as `template_declaration` containing
/// the actual declaration, so we capture both template and non-template forms.
const SYMBOL_QUERY: &str = r#"
(function_definition
  declarator: (_) @name) @item

(class_specifier
  name: (type_identifier) @name) @item

(struct_specifier
  name: (type_identifier) @name) @item

(enum_specifier
  name: (type_identifier) @name) @item

(namespace_definition
  name: (namespace_identifier) @name) @item

(template_declaration) @item

(using_declaration) @item

(type_definition
  declarator: (_) @name) @item

(alias_declaration
  name: (type_identifier) @name) @item

(preproc_include) @item

(declaration
  declarator: (_) @name) @item
"#;

impl LanguageHandler for CppHandler {
    fn symbol_query(&self) -> &str {
        SYMBOL_QUERY
    }

    fn classify_node(&self, node: Node, source: &str) -> Option<SymbolInfo> {
        classify_cpp_node(node, source)
    }

    fn visibility(&self, node: Node, source: &str) -> Visibility {
        // C++ top-level symbols don't have visibility modifiers in the same way.
        // Everything at namespace scope is effectively "public" unless in an anonymous namespace.
        let _ = (node, source);
        Visibility::Public
    }

    fn member_visibility(&self, node: Node, source: &str) -> Visibility {
        // Walk upward to find the access specifier
        cpp_member_visibility(node, source)
    }

    fn child_symbols<'a>(&self, node: Node<'a>, source: &str) -> Vec<ChildSymbol<'a>> {
        cpp_child_symbols(node, source)
    }

    fn signature(&self, node: Node, source: &str) -> String {
        cpp_signature(node, source)
    }

    fn body_field_name(&self) -> &str {
        "body"
    }

    fn definition_parent_kinds(&self) -> &[&str] {
        &[
            "function_definition",
            "class_specifier",
            "struct_specifier",
            "enum_specifier",
            "namespace_definition",
            "template_declaration",
            "declaration",
            "field_declaration",
        ]
    }

    fn identifier_node_kinds(&self) -> &[&str] {
        &[
            "identifier",
            "type_identifier",
            "field_identifier",
            "namespace_identifier",
            "destructor_name",
        ]
    }

    fn parse_imports(
        &self,
        root: Node,
        source: &str,
        file_path: &Path,
        _project_root: &Path,
    ) -> Vec<crate::xrefs::ImportEdge> {
        parse_cpp_includes(&root, source, file_path)
    }

    fn is_test_file(&self, path: &Path) -> bool {
        let path_str = path.to_string_lossy();
        crate::test_detect::has_test_dir_component(&path_str)
            || path
                .file_stem()
                .and_then(|s| s.to_str())
                .map(|s| s.ends_with("_test") || s.starts_with("test_"))
                .unwrap_or(false)
    }

    fn is_test_item(&self, _node: Node, _source: &str) -> bool {
        // C++ doesn't have a standard test attribute; framework-specific detection
        // would require checking for TEST(), TEST_F(), BOOST_AUTO_TEST_CASE(), etc.
        false
    }
}

fn classify_cpp_node(node: Node, source: &str) -> Option<SymbolInfo> {
    match node.kind() {
        "function_definition" => {
            let name = extract_function_name(node, source);
            // Check if it's a constructor or destructor
            let kind = if name.as_deref().is_some_and(|n| n.starts_with('~')) {
                ItemKind::Method // destructor
            } else {
                ItemKind::Function
            };
            Some(SymbolInfo { kind, name })
        }
        "class_specifier" => {
            let name = node
                .child_by_field_name("name")
                .map(|n| source[n.byte_range()].to_string());
            Some(SymbolInfo {
                kind: ItemKind::Class,
                name,
            })
        }
        "struct_specifier" => {
            let name = node
                .child_by_field_name("name")
                .map(|n| source[n.byte_range()].to_string());
            Some(SymbolInfo {
                kind: ItemKind::Struct,
                name,
            })
        }
        "enum_specifier" => {
            let name = node
                .child_by_field_name("name")
                .map(|n| source[n.byte_range()].to_string());
            Some(SymbolInfo {
                kind: ItemKind::Enum,
                name,
            })
        }
        "namespace_definition" => {
            let name = node
                .child_by_field_name("name")
                .map(|n| source[n.byte_range()].to_string());
            Some(SymbolInfo {
                kind: ItemKind::Mod,
                name,
            })
        }
        "template_declaration" => {
            // A template wraps another declaration; classify the inner declaration
            let mut cursor = node.walk();
            for child in node.named_children(&mut cursor) {
                match child.kind() {
                    "function_definition" => {
                        let name = extract_function_name(child, source);
                        return Some(SymbolInfo {
                            kind: ItemKind::Function,
                            name,
                        });
                    }
                    "class_specifier" => {
                        let name = child
                            .child_by_field_name("name")
                            .map(|n| source[n.byte_range()].to_string());
                        return Some(SymbolInfo {
                            kind: ItemKind::Class,
                            name,
                        });
                    }
                    "struct_specifier" => {
                        let name = child
                            .child_by_field_name("name")
                            .map(|n| source[n.byte_range()].to_string());
                        return Some(SymbolInfo {
                            kind: ItemKind::Struct,
                            name,
                        });
                    }
                    "declaration" => {
                        let name = child
                            .child_by_field_name("declarator")
                            .and_then(|d| extract_declarator_name(d, source));
                        return Some(SymbolInfo {
                            kind: ItemKind::Function,
                            name,
                        });
                    }
                    "alias_declaration" => {
                        let name = child
                            .child_by_field_name("name")
                            .map(|n| source[n.byte_range()].to_string());
                        return Some(SymbolInfo {
                            kind: ItemKind::TypeAlias,
                            name,
                        });
                    }
                    _ => {}
                }
            }
            None
        }
        "using_declaration" => {
            let text = &source[node.byte_range()];
            Some(SymbolInfo {
                kind: ItemKind::Use,
                name: Some(text.trim().to_string()),
            })
        }
        "type_definition" | "alias_declaration" => {
            let name = if node.kind() == "alias_declaration" {
                node.child_by_field_name("name")
                    .map(|n| source[n.byte_range()].to_string())
            } else {
                node.child_by_field_name("declarator")
                    .and_then(|d| extract_declarator_name(d, source))
            };
            Some(SymbolInfo {
                kind: ItemKind::TypeAlias,
                name,
            })
        }
        "preproc_include" => {
            let text = &source[node.byte_range()];
            Some(SymbolInfo {
                kind: ItemKind::Use,
                name: Some(text.trim().to_string()),
            })
        }
        "declaration" => {
            // Top-level declarations (global variables, forward declarations, etc.)
            let declarator = node.child_by_field_name("declarator");
            let name = declarator.and_then(|d| extract_declarator_name(d, source));

            // Check if this is a function declaration (has function_declarator)
            let is_func = declarator
                .map(|d| {
                    d.kind() == "function_declarator"
                        || d.kind() == "pointer_declarator"
                            && has_child_kind(d, "function_declarator")
                })
                .unwrap_or(false);

            if is_func {
                Some(SymbolInfo {
                    kind: ItemKind::Function,
                    name,
                })
            } else {
                // Could be a variable/constant declaration
                Some(SymbolInfo {
                    kind: ItemKind::Const,
                    name,
                })
            }
        }
        _ => None,
    }
}

fn extract_function_name(node: Node, source: &str) -> Option<String> {
    node.child_by_field_name("declarator")
        .and_then(|d| extract_declarator_name(d, source))
}

/// Recursively extract the name from a declarator node.
/// Handles `function_declarator`, `pointer_declarator`, `reference_declarator`,
/// `qualified_identifier`, `destructor_name`, etc.
fn extract_declarator_name(node: Node, source: &str) -> Option<String> {
    match node.kind() {
        "function_declarator" => node
            .child_by_field_name("declarator")
            .and_then(|d| extract_declarator_name(d, source)),
        "pointer_declarator" | "reference_declarator" => {
            let mut cursor = node.walk();
            for child in node.named_children(&mut cursor) {
                if child.kind() != "type_qualifier" {
                    return extract_declarator_name(child, source);
                }
            }
            None
        }
        "qualified_identifier" => {
            // e.g. MyClass::method — return the full qualified name
            Some(source[node.byte_range()].to_string())
        }
        "destructor_name" => Some(source[node.byte_range()].to_string()),
        "identifier" | "field_identifier" | "type_identifier" => {
            Some(source[node.byte_range()].to_string())
        }
        "parenthesized_declarator" => {
            let mut cursor = node.walk();
            for child in node.named_children(&mut cursor) {
                if let Some(name) = extract_declarator_name(child, source) {
                    return Some(name);
                }
            }
            None
        }
        _ => None,
    }
}

fn has_child_kind(node: Node, kind: &str) -> bool {
    let mut cursor = node.walk();
    node.named_children(&mut cursor)
        .any(|c| c.kind() == kind || has_child_kind(c, kind))
}

fn cpp_member_visibility(node: Node, source: &str) -> Visibility {
    // Walk backwards from this node to find the most recent access specifier
    let mut prev = node.prev_named_sibling();
    while let Some(sib) = prev {
        if sib.kind() == "access_specifier" {
            let text = source[sib.byte_range()].trim().trim_end_matches(':');
            return match text {
                "public" => Visibility::Public,
                "protected" => Visibility::Protected,
                "private" => Visibility::Private,
                _ => Visibility::Private,
            };
        }
        prev = sib.prev_named_sibling();
    }
    // Default: struct members are public, class members are private
    // We'd need to check the parent, but default to private for safety
    Visibility::Private
}

fn cpp_child_symbols<'a>(node: Node<'a>, source: &str) -> Vec<ChildSymbol<'a>> {
    let mut result = Vec::new();

    // For template_declaration, delegate to the inner declaration
    let actual_node = if node.kind() == "template_declaration" {
        let mut cursor = node.walk();
        let mut inner = None;
        for child in node.named_children(&mut cursor) {
            match child.kind() {
                "class_specifier" | "struct_specifier" => {
                    inner = Some(child);
                    break;
                }
                _ => {}
            }
        }
        match inner {
            Some(n) => n,
            None => return result,
        }
    } else {
        node
    };

    match actual_node.kind() {
        "class_specifier" | "struct_specifier" => {
            if let Some(body) = actual_node.child_by_field_name("body") {
                let mut cursor = body.walk();
                for child in body.named_children(&mut cursor) {
                    match child.kind() {
                        "function_definition" => {
                            let name = extract_function_name(child, source);
                            let kind = ItemKind::Method;
                            result.push(ChildSymbol {
                                node: child,
                                kind,
                                name,
                            });
                        }
                        "declaration" => {
                            let declarator = child.child_by_field_name("declarator");
                            let name =
                                declarator.and_then(|d| extract_declarator_name(d, source));
                            let is_func = declarator
                                .map(|d| d.kind() == "function_declarator")
                                .unwrap_or(false);
                            let kind = if is_func {
                                ItemKind::Method
                            } else {
                                ItemKind::Const // field
                            };
                            result.push(ChildSymbol {
                                node: child,
                                kind,
                                name,
                            });
                        }
                        "field_declaration" => {
                            let declarator = child.child_by_field_name("declarator");
                            let name =
                                declarator.and_then(|d| extract_declarator_name(d, source));
                            let is_func = declarator
                                .map(|d| d.kind() == "function_declarator")
                                .unwrap_or(false);
                            let kind = if is_func {
                                ItemKind::Method
                            } else {
                                ItemKind::Const // field
                            };
                            result.push(ChildSymbol {
                                node: child,
                                kind,
                                name,
                            });
                        }
                        "template_declaration" => {
                            // Template method inside class
                            let mut tc = child.walk();
                            for inner in child.named_children(&mut tc) {
                                if inner.kind() == "function_definition" {
                                    let name = extract_function_name(inner, source);
                                    result.push(ChildSymbol {
                                        node: child,
                                        kind: ItemKind::Method,
                                        name,
                                    });
                                } else if inner.kind() == "declaration" {
                                    let name = inner
                                        .child_by_field_name("declarator")
                                        .and_then(|d| extract_declarator_name(d, source));
                                    result.push(ChildSymbol {
                                        node: child,
                                        kind: ItemKind::Method,
                                        name,
                                    });
                                }
                            }
                        }
                        "type_definition" | "alias_declaration" => {
                            let name = if child.kind() == "alias_declaration" {
                                child
                                    .child_by_field_name("name")
                                    .map(|n| source[n.byte_range()].to_string())
                            } else {
                                child
                                    .child_by_field_name("declarator")
                                    .and_then(|d| extract_declarator_name(d, source))
                            };
                            result.push(ChildSymbol {
                                node: child,
                                kind: ItemKind::TypeAlias,
                                name,
                            });
                        }
                        "enum_specifier" => {
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
            }
        }
        "enum_specifier" => {
            if let Some(body) = actual_node.child_by_field_name("body") {
                let mut cursor = body.walk();
                for child in body.named_children(&mut cursor) {
                    if child.kind() == "enumerator" {
                        let name = child
                            .child_by_field_name("name")
                            .map(|n| source[n.byte_range()].to_string());
                        result.push(ChildSymbol {
                            node: child,
                            kind: ItemKind::Const,
                            name,
                        });
                    }
                }
            }
        }
        "namespace_definition" => {
            if let Some(body) = actual_node.child_by_field_name("body") {
                let mut cursor = body.walk();
                for child in body.named_children(&mut cursor) {
                    if let Some(info) = classify_cpp_node(child, source) {
                        result.push(ChildSymbol {
                            node: child,
                            kind: info.kind,
                            name: info.name,
                        });
                    }
                }
            }
        }
        _ => {}
    }

    result
}

fn cpp_signature(node: Node, source: &str) -> String {
    // For function_definition, take everything before the body
    if let Some(body) = node.child_by_field_name("body") {
        let sig = &source[node.start_byte()..body.start_byte()];
        return sig.trim().to_string();
    }
    // For declarations (method declarations without body), use the full text
    let text = &source[node.byte_range()];
    text.trim().trim_end_matches(';').trim().to_string()
}

fn parse_cpp_includes(
    root: &Node,
    source: &str,
    file_path: &Path,
) -> Vec<crate::xrefs::ImportEdge> {
    let mut edges = Vec::new();
    let mut cursor = root.walk();
    for child in root.named_children(&mut cursor) {
        if child.kind() == "preproc_include"
            && let Some(path_node) = child.child_by_field_name("path") {
                let raw = source[path_node.byte_range()].to_string();
                let clean = raw.trim_matches(|c| c == '"' || c == '<' || c == '>');
                edges.push(crate::xrefs::ImportEdge {
                    importing_file: file_path.to_path_buf(),
                    source: clean.to_string(),
                    symbols: vec![],
                    resolved_path: None,
                });
        }
    }
    edges
}

#[cfg(test)]
mod tests {
    use tree_sitter::Parser;

    fn parse_cpp(source: &str) -> tree_sitter::Tree {
        let mut parser = Parser::new();
        parser.set_language(&tree_sitter_cpp::LANGUAGE.into()).unwrap();
        parser.parse(source, None).unwrap()
    }

    #[test]
    fn test_cpp_extract_filtered() {
        use crate::extractor::interface::extract_filtered;
        use crate::languages::Language;
        let source = "class MyClass {\npublic:\n    void method() const {}\n};\n\nvoid free_function(int x) {}\n";
        let tree = parse_cpp(source);
        let items = extract_filtered(source, &tree, Language::Cpp, false);
        assert!(items.iter().any(|i| i.name.as_deref() == Some("MyClass")), "should find MyClass");
        assert!(items.iter().any(|i| i.name.as_deref() == Some("free_function")), "should find free_function");
    }
}
