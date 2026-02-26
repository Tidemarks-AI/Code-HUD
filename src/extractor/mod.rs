pub mod collapse;
pub mod interface;
pub mod expand;
pub mod outline;

use serde::Serialize;

#[derive(Debug, Clone, Serialize)]
pub struct Item {
    pub kind: ItemKind,
    pub name: Option<String>,
    pub visibility: Visibility,
    pub line_start: usize,
    pub line_end: usize,
    pub signature: Option<String>,
    pub body: Option<String>,
    pub content: String,
    /// Explicit line mappings for content lines (line_num, text)
    /// Used when content has been modified (e.g., collapsed bodies)
    #[serde(skip)]
    pub line_mappings: Option<Vec<(usize, String)>>,
}

impl Item {
    pub fn is_public(&self) -> bool {
        matches!(self.visibility, Visibility::Public)
    }
}

#[derive(Debug, Clone, PartialEq, Serialize)]
#[serde(rename_all = "lowercase")]
pub enum ItemKind {
    Function,
    Method,
    Struct,
    Enum,
    Trait,
    Impl,
    Mod,
    Use,
    Const,
    Static,
    TypeAlias,
    MacroDef,
    Class,
}

impl ItemKind {
    /// Language-appropriate display name for this symbol kind.
    ///
    /// The internal enum uses Rust-centric names (`Trait`, `Use`, `TypeAlias`).
    /// This method returns the idiomatic label for the target language.
    pub fn display_name(&self, lang: crate::languages::Language) -> &'static str {
        use crate::languages::Language;
        match lang {
            Language::TypeScript | Language::Tsx => match self {
                ItemKind::Function => "fn",
                ItemKind::Method => "fn",
                ItemKind::Struct => "interface",
                ItemKind::Enum => "enum",
                ItemKind::Trait => "interface",
                ItemKind::Impl => "impl",
                ItemKind::Mod => "module",
                ItemKind::Use => "import",
                ItemKind::Const => "const",
                ItemKind::Static => "static",
                ItemKind::TypeAlias => "type alias",
                ItemKind::MacroDef => "macro",
                ItemKind::Class => "class",
            },
            Language::JavaScript | Language::Jsx => match self {
                ItemKind::Function => "fn",
                ItemKind::Method => "fn",
                ItemKind::Struct => "object",
                ItemKind::Enum => "enum",
                ItemKind::Trait => "interface",
                ItemKind::Impl => "impl",
                ItemKind::Mod => "module",
                ItemKind::Use => "import",
                ItemKind::Const => "const",
                ItemKind::Static => "static",
                ItemKind::TypeAlias => "type alias",
                ItemKind::MacroDef => "macro",
                ItemKind::Class => "class",
            },
            Language::Python => match self {
                ItemKind::Function => "fn",
                ItemKind::Method => "fn",
                ItemKind::Struct => "class",
                ItemKind::Enum => "enum",
                ItemKind::Trait => "protocol",
                ItemKind::Impl => "impl",
                ItemKind::Mod => "module",
                ItemKind::Use => "import",
                ItemKind::Const => "const",
                ItemKind::Static => "static",
                ItemKind::TypeAlias => "TypeAlias",
                ItemKind::MacroDef => "macro",
                ItemKind::Class => "class",
            },
            Language::Rust => match self {
                ItemKind::Function => "fn",
                ItemKind::Method => "fn",
                ItemKind::Struct => "struct",
                ItemKind::Enum => "enum",
                ItemKind::Trait => "trait",
                ItemKind::Impl => "impl",
                ItemKind::Mod => "mod",
                ItemKind::Use => "use",
                ItemKind::Const => "const",
                ItemKind::Static => "static",
                ItemKind::TypeAlias => "type",
                ItemKind::MacroDef => "macro",
                ItemKind::Class => "class",
            },
        }
    }
}


#[derive(Debug, Clone, PartialEq, Serialize)]
#[serde(rename_all = "lowercase")]
pub enum Visibility {
    Public,
    Protected,
    Private,
    Crate,
    Super,
}

/// Walk backwards through preceding `attribute_item` siblings to find the true start
/// of an attributed item (byte offset, 1-based line number).
pub fn find_attr_start(node: tree_sitter::Node) -> (usize, usize) {
    let mut start_byte = node.start_byte();
    let mut start_row = node.start_position().row;
    // Walk backward through siblings looking for attributes/decorators
    let mut current = node;
    loop {
        match current.prev_sibling() {
            Some(prev) if prev.kind() == "attribute_item" || prev.kind() == "decorator" => {
                start_byte = prev.start_byte();
                start_row = prev.start_position().row;
                current = prev;
            }
            _ => break,
        }
    }
    // For nodes inside export_statement (e.g. class_declaration), check if the
    // parent export_statement has decorator children that precede this node
    if let Some(parent) = node.parent()
        && parent.kind() == "export_statement"
            && let Some(first_child) = parent.child(0)
                && first_child.kind() == "decorator" && first_child.start_byte() < start_byte {
                    start_byte = first_child.start_byte();
                    start_row = first_child.start_position().row;
                }
    (start_byte, start_row + 1)
}

impl Visibility {
    pub fn from_node(node: Option<tree_sitter::Node>, source: &str) -> Self {
        if let Some(vis_node) = node {
            let vis_text = &source[vis_node.byte_range()];
            if vis_text.starts_with("pub") {
                if vis_text.contains("crate") {
                    return Visibility::Crate;
                } else if vis_text.contains("super") {
                    return Visibility::Super;
                }
                return Visibility::Public;
            }
        }
        Visibility::Private
    }

    /// Find visibility by searching children for `visibility_modifier` node kind
    pub fn from_parent(parent: tree_sitter::Node, source: &str) -> Self {
        let mut cursor = parent.walk();
        for child in parent.children(&mut cursor) {
            if child.kind() == "visibility_modifier" {
                return Self::from_node(Some(child), source);
            }
        }
        Visibility::Private
    }
}




#[cfg(test)]
mod tests {
    use super::*;
    use crate::parser::parse;
    use crate::languages::Language;

    #[test]
    fn display_name_rust_uses_rust_terms() {
        assert_eq!(ItemKind::Trait.display_name(Language::Rust), "trait");
        assert_eq!(ItemKind::Use.display_name(Language::Rust), "use");
        assert_eq!(ItemKind::TypeAlias.display_name(Language::Rust), "type");
        assert_eq!(ItemKind::Mod.display_name(Language::Rust), "mod");
        assert_eq!(ItemKind::Struct.display_name(Language::Rust), "struct");
    }

    #[test]
    fn display_name_typescript_uses_ts_terms() {
        assert_eq!(ItemKind::Trait.display_name(Language::TypeScript), "interface");
        assert_eq!(ItemKind::Use.display_name(Language::TypeScript), "import");
        assert_eq!(ItemKind::TypeAlias.display_name(Language::TypeScript), "type alias");
        assert_eq!(ItemKind::Mod.display_name(Language::TypeScript), "module");
        assert_eq!(ItemKind::Struct.display_name(Language::TypeScript), "interface");
    }

    #[test]
    fn display_name_tsx_same_as_typescript() {
        assert_eq!(ItemKind::Trait.display_name(Language::Tsx), "interface");
        assert_eq!(ItemKind::Use.display_name(Language::Tsx), "import");
        assert_eq!(ItemKind::TypeAlias.display_name(Language::Tsx), "type alias");
    }

    #[test]
    fn display_name_python_uses_python_terms() {
        assert_eq!(ItemKind::Trait.display_name(Language::Python), "protocol");
        assert_eq!(ItemKind::Use.display_name(Language::Python), "import");
        assert_eq!(ItemKind::TypeAlias.display_name(Language::Python), "TypeAlias");
        assert_eq!(ItemKind::Struct.display_name(Language::Python), "class");
    }

    #[test]
    fn display_name_javascript_uses_js_terms() {
        assert_eq!(ItemKind::Trait.display_name(Language::JavaScript), "interface");
        assert_eq!(ItemKind::Use.display_name(Language::JavaScript), "import");
        assert_eq!(ItemKind::Mod.display_name(Language::JavaScript), "module");
    }

    #[test]
    fn display_name_shared_kinds_unchanged() {
        // These should be the same across all languages
        for lang in [Language::Rust, Language::TypeScript, Language::Python, Language::JavaScript] {
            assert_eq!(ItemKind::Function.display_name(lang), "fn");
            assert_eq!(ItemKind::Method.display_name(lang), "fn");
            assert_eq!(ItemKind::Enum.display_name(lang), "enum");
            assert_eq!(ItemKind::Const.display_name(lang), "const");
            assert_eq!(ItemKind::Class.display_name(lang), "class");
        }
    }

    #[test]
    fn visibility_from_node_none_is_private() {
        let vis = Visibility::from_node(None, "");
        assert_eq!(vis, Visibility::Private);
    }

    #[test]
    fn visibility_from_parent_pub() {
        let source = "pub fn foo() {}";
        let tree = parse(source, Language::Rust).unwrap();
        let root = tree.root_node();
        let fn_node = root.child(0).unwrap();
        let vis = Visibility::from_parent(fn_node, source);
        assert_eq!(vis, Visibility::Public);
    }

    #[test]
    fn visibility_from_parent_private() {
        let source = "fn foo() {}";
        let tree = parse(source, Language::Rust).unwrap();
        let root = tree.root_node();
        let fn_node = root.child(0).unwrap();
        let vis = Visibility::from_parent(fn_node, source);
        assert_eq!(vis, Visibility::Private);
    }

    #[test]
    fn visibility_from_parent_pub_crate() {
        let source = "pub(crate) fn foo() {}";
        let tree = parse(source, Language::Rust).unwrap();
        let root = tree.root_node();
        let fn_node = root.child(0).unwrap();
        let vis = Visibility::from_parent(fn_node, source);
        assert_eq!(vis, Visibility::Crate);
    }

    #[test]
    fn visibility_from_parent_pub_super() {
        let source = "pub(super) fn foo() {}";
        let tree = parse(source, Language::Rust).unwrap();
        let root = tree.root_node();
        let fn_node = root.child(0).unwrap();
        let vis = Visibility::from_parent(fn_node, source);
        assert_eq!(vis, Visibility::Super);
    }

    #[test]
    fn find_attr_start_no_attrs() {
        let source = "fn foo() {}";
        let tree = parse(source, Language::Rust).unwrap();
        let root = tree.root_node();
        let fn_node = root.child(0).unwrap();
        let (byte, line) = find_attr_start(fn_node);
        assert_eq!(byte, 0);
        assert_eq!(line, 1);
    }

    #[test]
    fn find_attr_start_with_attr() {
        let source = "#[inline]\nfn foo() {}";
        let tree = parse(source, Language::Rust).unwrap();
        let root = tree.root_node();
        // The fn_node should be the function_item
        let fn_node = root.child(1).unwrap();
        assert_eq!(fn_node.kind(), "function_item");
        let (byte, line) = find_attr_start(fn_node);
        assert_eq!(byte, 0); // attribute starts at byte 0
        assert_eq!(line, 1);
    }
}
