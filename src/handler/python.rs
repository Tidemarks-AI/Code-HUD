use super::{ChildSymbol, LanguageHandler, SymbolInfo};
use crate::extractor::{ItemKind, Visibility};
use std::path::Path;
use tree_sitter::Node;

pub struct PythonHandler;

/// Symbol query for Python — captures top-level declarations at any depth.
const SYMBOL_QUERY: &str = r#"
(function_definition
  name: (identifier) @name
  body: (block) @body) @item

(class_definition
  name: (identifier) @name
  body: (block) @body) @item

(decorated_definition
  (function_definition
    name: (identifier) @name
    body: (block) @body)) @item

(decorated_definition
  (class_definition
    name: (identifier) @name
    body: (block) @body)) @item

(import_statement) @item

(import_from_statement) @item

(expression_statement
  (assignment
    left: (identifier) @name)) @item
"#;

fn python_visibility(name: &str) -> Visibility {
    if name.starts_with('_') {
        Visibility::Private
    } else {
        Visibility::Public
    }
}

/// Unwrap a decorated_definition to its inner declaration (skipping decorators).
fn unwrap_decorated(node: Node) -> Option<Node> {
    let mut cursor = node.walk();
    let mut result = None;
    for child in node.named_children(&mut cursor) {
        if child.kind() != "decorator" {
            result = Some(child);
            break;
        }
    }
    result
}

/// Find a child of a specific kind inside a decorated_definition.
fn find_inner_of_kind<'a>(node: Node<'a>, kind: &str) -> Option<Node<'a>> {
    let mut cursor = node.walk();
    let mut result = None;
    for child in node.named_children(&mut cursor) {
        if child.kind() == kind {
            result = Some(child);
            break;
        }
    }
    result
}

impl LanguageHandler for PythonHandler {
    fn symbol_query(&self) -> &str {
        SYMBOL_QUERY
    }

    fn classify_node(&self, node: Node, source: &str) -> Option<SymbolInfo> {
        let kind_node = if node.kind() == "decorated_definition" {
            unwrap_decorated(node)?
        } else {
            node
        };

        let kind = match kind_node.kind() {
            "function_definition" => ItemKind::Function,
            "class_definition" => ItemKind::Class,
            "import_statement" | "import_from_statement" => ItemKind::Use,
            "expression_statement" => ItemKind::Const,
            _ => return None,
        };

        let name = extract_name(kind_node, source);
        Some(SymbolInfo { kind, name })
    }

    fn visibility(&self, node: Node, source: &str) -> Visibility {
        let inner = if node.kind() == "decorated_definition" {
            unwrap_decorated(node).unwrap_or(node)
        } else {
            node
        };

        if let Some(name_node) = inner.child_by_field_name("name") {
            let name = &source[name_node.byte_range()];
            return python_visibility(name);
        }

        // For assignments, check the left side
        if inner.kind() == "expression_statement" {
            let mut cursor = inner.walk();
            for child in inner.named_children(&mut cursor) {
                if child.kind() == "assignment"
                    && let Some(left) = child.child_by_field_name("left") {
                        let name = &source[left.byte_range()];
                        return python_visibility(name);
                    }
            }
        }

        Visibility::Public
    }

    fn member_visibility(&self, node: Node, source: &str) -> Visibility {
        let func_node = if node.kind() == "decorated_definition" {
            find_inner_of_kind(node, "function_definition").unwrap_or(node)
        } else {
            node
        };

        if let Some(name_node) = func_node.child_by_field_name("name") {
            let name = &source[name_node.byte_range()];
            return python_visibility(name);
        }
        Visibility::Public
    }

    fn child_symbols<'a>(&self, node: Node<'a>, source: &str) -> Vec<ChildSymbol<'a>> {
        let class_node = if node.kind() == "decorated_definition" {
            match find_inner_of_kind(node, "class_definition") {
                Some(c) => c,
                None => return vec![],
            }
        } else {
            node
        };

        let body = match class_node.child_by_field_name("body") {
            Some(b) => b,
            None => return vec![],
        };

        let mut result = Vec::new();
        let mut cursor = body.walk();
        for child in body.named_children(&mut cursor) {
            match child.kind() {
                "function_definition" => {
                    let name = child
                        .child_by_field_name("name")
                        .map(|n| source[n.byte_range()].to_string());
                    result.push(ChildSymbol {
                        node: child,
                        kind: ItemKind::Method,
                        name,
                    });
                }
                "decorated_definition" => {
                    let mut inner_cursor = child.walk();
                    for inner in child.named_children(&mut inner_cursor) {
                        match inner.kind() {
                            "function_definition" => {
                                let name = inner
                                    .child_by_field_name("name")
                                    .map(|n| source[n.byte_range()].to_string());
                                result.push(ChildSymbol {
                                    node: child,
                                    kind: ItemKind::Method,
                                    name,
                                });
                                break;
                            }
                            "class_definition" => {
                                let name = inner
                                    .child_by_field_name("name")
                                    .map(|n| source[n.byte_range()].to_string());
                                result.push(ChildSymbol {
                                    node: child,
                                    kind: ItemKind::Class,
                                    name,
                                });
                                break;
                            }
                            _ => {}
                        }
                    }
                }
                "class_definition" => {
                    let name = child
                        .child_by_field_name("name")
                        .map(|n| source[n.byte_range()].to_string());
                    result.push(ChildSymbol {
                        node: child,
                        kind: ItemKind::Class,
                        name,
                    });
                }
                "expression_statement" => {
                    let mut inner_cursor = child.walk();
                    for inner in child.named_children(&mut inner_cursor) {
                        if inner.kind() == "assignment"
                            && let Some(left) = inner.child_by_field_name("left")
                                && left.kind() == "identifier" {
                                    let name = Some(source[left.byte_range()].to_string());
                                    result.push(ChildSymbol {
                                        node: child,
                                        kind: ItemKind::Const,
                                        name,
                                    });
                                }
                    }
                }
                _ => {}
            }
        }
        result
    }

    fn signature(&self, node: Node, source: &str) -> String {
        let func_node = if node.kind() == "decorated_definition" {
            find_inner_of_kind(node, "function_definition").unwrap_or(node)
        } else {
            node
        };

        let mut parts = Vec::new();

        // Check for async
        let mut cursor = func_node.walk();
        for child in func_node.children(&mut cursor) {
            if child.kind() == "async" {
                parts.push("async".to_string());
                break;
            }
        }

        parts.push("def".to_string());

        if let Some(name) = func_node.child_by_field_name("name") {
            parts.push(source[name.byte_range()].to_string());
        }

        if let Some(params) = func_node.child_by_field_name("parameters") {
            let last = parts.pop().unwrap_or_default();
            parts.push(format!("{}{}", last, &source[params.byte_range()]));
        }

        if let Some(ret) = func_node.child_by_field_name("return_type") {
            parts.push("->".to_string());
            parts.push(source[ret.byte_range()].to_string());
        }

        parts.join(" ")
    }

    fn body_field_name(&self) -> &str {
        "body"
    }

    fn definition_parent_kinds(&self) -> &[&str] {
        &[
            "function_definition",
            "class_definition",
            "parameters",
            "assignment",
            "for_statement",
        ]
    }

    fn identifier_node_kinds(&self) -> &[&str] {
        &["identifier"]
    }

    fn parse_imports(
        &self,
        root: Node,
        source: &str,
        file_path: &Path,
        _project_root: &Path,
    ) -> Vec<crate::xrefs::ImportEdge> {
        crate::xrefs::parse_python_imports_impl(&root, source, file_path)
    }

    fn is_test_file(&self, path: &Path) -> bool {
        let path_str = path.to_string_lossy();
        let stem = path.file_stem().and_then(|s| s.to_str()).unwrap_or("");
        let file_name = path.file_name().and_then(|n| n.to_str()).unwrap_or("");
        crate::test_detect::has_test_dir_component(&path_str)
            || stem.starts_with("test_")
            || stem.ends_with("_test")
            || file_name == "conftest.py"
    }

    fn is_test_item(&self, node: Node, source: &str) -> bool {
        let check_node = if node.kind() == "decorated_definition" {
            unwrap_decorated(node).unwrap_or(node)
        } else {
            node
        };

        if let Some(name_node) = check_node.child_by_field_name("name") {
            let name = &source[name_node.byte_range()];
            match check_node.kind() {
                "function_definition" => return name.starts_with("test_"),
                "class_definition" => return name.starts_with("Test"),
                _ => {}
            }
        }
        false
    }
}

fn extract_name(node: Node, source: &str) -> Option<String> {
    match node.kind() {
        "expression_statement" => {
            let mut cursor = node.walk();
            for child in node.named_children(&mut cursor) {
                if child.kind() == "assignment" {
                    return child
                        .child_by_field_name("left")
                        .filter(|n| n.kind() == "identifier")
                        .map(|n| source[n.byte_range()].to_string());
                }
            }
            None
        }
        "import_statement" | "import_from_statement" => None,
        _ => node
            .child_by_field_name("name")
            .map(|n| source[n.byte_range()].to_string()),
    }
}
