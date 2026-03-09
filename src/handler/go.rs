use super::{ChildSymbol, LanguageHandler, SymbolInfo};
use crate::extractor::{ItemKind, Visibility};
use std::path::Path;
use tree_sitter::Node;

pub struct GoHandler;

/// Symbol query for Go — captures top-level declarations.
const SYMBOL_QUERY: &str = r#"
(function_declaration
  name: (identifier) @name
  body: (block) @body) @item

(method_declaration
  name: (field_identifier) @name
  body: (block) @body) @item

(type_declaration
  (type_spec
    name: (type_identifier) @name
    type: (_) @body)) @item

(const_declaration) @item

(var_declaration) @item

(import_declaration) @item

(package_clause
  (package_identifier) @name) @item
"#;

impl LanguageHandler for GoHandler {
    fn symbol_query(&self) -> &str {
        SYMBOL_QUERY
    }

    fn classify_node(&self, node: Node, source: &str) -> Option<SymbolInfo> {
        match node.kind() {
            "function_declaration" => {
                let name = node
                    .child_by_field_name("name")
                    .map(|n| source[n.byte_range()].to_string());
                Some(SymbolInfo {
                    kind: ItemKind::Function,
                    name,
                })
            }
            "method_declaration" => {
                let name = node
                    .child_by_field_name("name")
                    .map(|n| source[n.byte_range()].to_string());
                Some(SymbolInfo {
                    kind: ItemKind::Function,
                    name,
                })
            }
            "type_declaration" => {
                // Find the type_spec child to get name and determine kind
                let mut cursor = node.walk();
                for child in node.named_children(&mut cursor) {
                    if child.kind() == "type_spec" {
                        let name = child
                            .child_by_field_name("name")
                            .map(|n| source[n.byte_range()].to_string());
                        let type_node = child.child_by_field_name("type");
                        let kind = match type_node.map(|n| n.kind()) {
                            Some("struct_type") => ItemKind::Struct,
                            Some("interface_type") => ItemKind::Trait,
                            _ => ItemKind::TypeAlias,
                        };
                        return Some(SymbolInfo { kind, name });
                    }
                }
                None
            }
            "const_declaration" => {
                // Try to get the first const name
                let name = extract_first_spec_name(node, source, "const_spec");
                Some(SymbolInfo {
                    kind: ItemKind::Const,
                    name,
                })
            }
            "var_declaration" => {
                let name = extract_first_spec_name(node, source, "var_spec");
                Some(SymbolInfo {
                    kind: ItemKind::Static,
                    name,
                })
            }
            "import_declaration" => Some(SymbolInfo {
                kind: ItemKind::Use,
                name: None,
            }),
            "package_clause" => {
                let mut cursor = node.walk();
                let name = node
                    .named_children(&mut cursor)
                    .find(|c| c.kind() == "package_identifier")
                    .map(|n| source[n.byte_range()].to_string());
                Some(SymbolInfo {
                    kind: ItemKind::Mod,
                    name,
                })
            }
            _ => None,
        }
    }

    fn visibility(&self, node: Node, source: &str) -> Visibility {
        go_visibility_from_node(node, source)
    }

    fn member_visibility(&self, node: Node, source: &str) -> Visibility {
        go_visibility_from_node(node, source)
    }

    fn child_symbols<'a>(&self, node: Node<'a>, source: &str) -> Vec<ChildSymbol<'a>> {
        let mut result = Vec::new();

        // For type_declaration, walk the type body (struct fields, interface methods)
        if node.kind() == "type_declaration" {
            let mut cursor = node.walk();
            for child in node.named_children(&mut cursor) {
                if child.kind() == "type_spec"
                    && let Some(type_node) = child.child_by_field_name("type")
                {
                    match type_node.kind() {
                        "struct_type" => {
                            if let Some(field_list) = type_node.child_by_field_name("fields") {
                                let mut fc = field_list.walk();
                                for field in field_list.named_children(&mut fc) {
                                    if field.kind() == "field_declaration" {
                                        let name = field
                                            .child_by_field_name("name")
                                            .map(|n| source[n.byte_range()].to_string());
                                        result.push(ChildSymbol {
                                            node: field,
                                            kind: ItemKind::Const,
                                            name,
                                        });
                                    }
                                }
                            }
                        }
                        "interface_type" => {
                            let mut ic = type_node.walk();
                            for child_node in type_node.named_children(&mut ic) {
                                if child_node.kind() == "method_elem" {
                                    // method_elem contains the method signature
                                    let name = child_node
                                        .child_by_field_name("name")
                                        .map(|n| source[n.byte_range()].to_string());
                                    result.push(ChildSymbol {
                                        node: child_node,
                                        kind: ItemKind::Method,
                                        name,
                                    });
                                }
                            }
                        }
                        _ => {}
                    }
                }
            }
        }

        // For const/var declarations with multiple specs
        if node.kind() == "const_declaration" || node.kind() == "var_declaration" {
            let (spec_kind, item_kind) = if node.kind() == "const_declaration" {
                ("const_spec", ItemKind::Const)
            } else {
                ("var_spec", ItemKind::Static)
            };
            let mut cursor = node.walk();
            let specs: Vec<_> = node
                .named_children(&mut cursor)
                .filter(|c| c.kind() == spec_kind)
                .collect();
            if specs.len() > 1 {
                for spec in specs {
                    let name = spec
                        .child_by_field_name("name")
                        .map(|n| source[n.byte_range()].to_string());
                    result.push(ChildSymbol {
                        node: spec,
                        kind: item_kind.clone(),
                        name,
                    });
                }
            }
        }

        result
    }

    fn signature(&self, node: Node, source: &str) -> String {
        match node.kind() {
            "function_declaration" => {
                let mut sig = String::from("func ");
                if let Some(name) = node.child_by_field_name("name") {
                    sig.push_str(&source[name.byte_range()]);
                }
                if let Some(params) = node.child_by_field_name("parameters") {
                    sig.push_str(&source[params.byte_range()]);
                }
                if let Some(result) = node.child_by_field_name("result") {
                    sig.push(' ');
                    sig.push_str(&source[result.byte_range()]);
                }
                sig
            }
            "method_declaration" => {
                let mut sig = String::from("func ");
                if let Some(receiver) = node.child_by_field_name("receiver") {
                    sig.push_str(&source[receiver.byte_range()]);
                    sig.push(' ');
                }
                if let Some(name) = node.child_by_field_name("name") {
                    sig.push_str(&source[name.byte_range()]);
                }
                if let Some(params) = node.child_by_field_name("parameters") {
                    sig.push_str(&source[params.byte_range()]);
                }
                if let Some(result) = node.child_by_field_name("result") {
                    sig.push(' ');
                    sig.push_str(&source[result.byte_range()]);
                }
                sig
            }
            _ => source[node.byte_range()]
                .lines()
                .next()
                .unwrap_or("")
                .to_string(),
        }
    }

    fn body_field_name(&self) -> &str {
        "body"
    }

    fn definition_parent_kinds(&self) -> &[&str] {
        &[
            "function_declaration",
            "method_declaration",
            "type_declaration",
            "const_declaration",
            "var_declaration",
            "short_var_declaration",
            "assignment_statement",
        ]
    }

    fn identifier_node_kinds(&self) -> &[&str] {
        &[
            "identifier",
            "field_identifier",
            "type_identifier",
            "package_identifier",
        ]
    }

    fn parse_imports(
        &self,
        root: Node,
        source: &str,
        file_path: &Path,
        _project_root: &Path,
    ) -> Vec<crate::xrefs::ImportEdge> {
        parse_go_imports(&root, source, file_path)
    }

    fn is_test_file(&self, path: &Path) -> bool {
        let file_name = path.file_name().and_then(|n| n.to_str()).unwrap_or("");
        file_name.ends_with("_test.go")
    }

    fn is_test_item(&self, node: Node, source: &str) -> bool {
        if node.kind() == "function_declaration"
            && let Some(name_node) = node.child_by_field_name("name")
        {
            let name = &source[name_node.byte_range()];
            return name.starts_with("Test")
                || name.starts_with("Benchmark")
                || name.starts_with("Example");
        }
        false
    }

    /// Go doc comments use plain `//` (not `///`). All `//` comments immediately
    /// preceding a declaration are its doc comment per godoc convention.
    fn get_doc_comment(&self, source: &str, node: Node) -> Option<String> {
        let comments = super::collect_prev_comment_siblings(source, node);
        if comments.is_empty() {
            None
        } else {
            Some(comments.join("\n"))
        }
    }
}

/// In Go, exported names start with an uppercase letter.
fn go_visibility_from_node(node: Node, source: &str) -> Visibility {
    let name_str = match node.kind() {
        "function_declaration" => node
            .child_by_field_name("name")
            .map(|n| &source[n.byte_range()]),
        "method_declaration" => node
            .child_by_field_name("name")
            .map(|n| &source[n.byte_range()]),
        "type_declaration" => {
            let mut cursor = node.walk();
            node.named_children(&mut cursor)
                .find(|c| c.kind() == "type_spec")
                .and_then(|ts| ts.child_by_field_name("name"))
                .map(|n| &source[n.byte_range()])
        }
        "const_declaration" => extract_first_spec_name_str(node, source, "const_spec"),
        "var_declaration" => extract_first_spec_name_str(node, source, "var_spec"),
        "package_clause" => return Visibility::Public,
        "import_declaration" => return Visibility::Private,
        _ => None,
    };

    match name_str {
        Some(name) if name.starts_with(|c: char| c.is_uppercase()) => Visibility::Public,
        Some(_) => Visibility::Private,
        None => Visibility::Public,
    }
}

fn extract_first_spec_name(node: Node, source: &str, spec_kind: &str) -> Option<String> {
    let mut cursor = node.walk();
    node.named_children(&mut cursor)
        .find(|c| c.kind() == spec_kind)
        .and_then(|spec| spec.child_by_field_name("name"))
        .map(|n| source[n.byte_range()].to_string())
}

fn extract_first_spec_name_str<'a>(
    node: Node<'a>,
    source: &'a str,
    spec_kind: &str,
) -> Option<&'a str> {
    let mut cursor = node.walk();
    node.named_children(&mut cursor)
        .find(|c| c.kind() == spec_kind)
        .and_then(|spec| spec.child_by_field_name("name"))
        .map(|n| &source[n.byte_range()])
}

fn parse_go_imports(root: &Node, source: &str, file_path: &Path) -> Vec<crate::xrefs::ImportEdge> {
    let mut edges = Vec::new();
    let mut cursor = root.walk();
    for child in root.named_children(&mut cursor) {
        if child.kind() == "import_declaration" {
            let mut ic = child.walk();
            for import_child in child.named_children(&mut ic) {
                match import_child.kind() {
                    "import_spec" => {
                        if let Some(path_node) = import_child.child_by_field_name("path") {
                            let raw = &source[path_node.byte_range()];
                            let import_path = raw.trim_matches('"');
                            edges.push(crate::xrefs::ImportEdge {
                                importing_file: file_path.to_path_buf(),
                                symbols: vec![],
                                source: import_path.to_string(),
                                resolved_path: None,
                            });
                        }
                    }
                    "import_spec_list" => {
                        let mut lc = import_child.walk();
                        for spec in import_child.named_children(&mut lc) {
                            if spec.kind() == "import_spec"
                                && let Some(path_node) = spec.child_by_field_name("path")
                            {
                                let raw = &source[path_node.byte_range()];
                                let import_path = raw.trim_matches('"');
                                edges.push(crate::xrefs::ImportEdge {
                                    importing_file: file_path.to_path_buf(),
                                    symbols: vec![],
                                    source: import_path.to_string(),
                                    resolved_path: None,
                                });
                            }
                        }
                    }
                    _ => {}
                }
            }
        }
    }
    edges
}
