pub mod javascript;
pub mod python;
pub mod rust;
pub mod typescript;

// Re-export types used in the handler API so consumers don't need to reach into extractor
pub use crate::extractor::{ItemKind, Visibility};
pub use crate::languages::{Language, ts_language};
use tree_sitter::Node;
use std::path::Path;

/// Information about a classified symbol node.
#[derive(Debug, Clone)]
pub struct SymbolInfo {
    pub kind: ItemKind,
    pub name: Option<String>,
}

/// A child symbol discovered by walking the AST.
/// Holds a reference to the tree-sitter Node — no copying.
#[derive(Debug, Clone)]
pub struct ChildSymbol<'a> {
    pub node: Node<'a>,
    pub kind: ItemKind,
    pub name: Option<String>,
}

/// A language-specific handler for codehud operations using tree-sitter's native Node API.
///
/// The handler uses tree-sitter queries to identify symbols, then
/// navigates the AST (Node API) for hierarchy and content.
pub trait LanguageHandler: Send + Sync {
    /// Query that captures all symbol nodes.
    /// Captures: @item (the node), @name (identifier), @body (block, optional)
    fn symbol_query(&self) -> &str;

    /// Given a node, return the SymbolKind and name, or None to skip.
    fn classify_node(&self, node: Node, source: &str) -> Option<SymbolInfo>;

    /// Determine visibility of a top-level symbol node.
    fn visibility(&self, node: Node, source: &str) -> Visibility;

    /// Determine visibility of a class/struct member node.
    fn member_visibility(&self, node: Node, source: &str) -> Visibility;

    /// Walk a container node's children to find child symbols.
    /// Uses the AST directly — node.named_children() — not queries.
    fn child_symbols<'a>(&self, node: Node<'a>, source: &str) -> Vec<ChildSymbol<'a>>;

    /// Build a signature string for a method/function node.
    fn signature(&self, node: Node, source: &str) -> String;

    /// The field name used to access body blocks (for collapsing).
    fn body_field_name(&self) -> &str {
        "body"
    }

    /// Node kinds that indicate a definition context.
    fn definition_parent_kinds(&self) -> &[&str];

    /// Node kinds that count as identifiers for reference search.
    fn identifier_node_kinds(&self) -> &[&str];

    /// Parse import statements from the tree root.
    fn parse_imports(
        &self,
        root: Node,
        source: &str,
        file_path: &Path,
        project_root: &Path,
    ) -> Vec<crate::xrefs::ImportEdge>;

    /// Returns true if the file path indicates a test file.
    fn is_test_file(&self, path: &Path) -> bool;

    /// Returns true if the given node represents a test item.
    fn is_test_item(&self, node: Node, source: &str) -> bool;
}

/// Get a `LanguageHandler` implementation for the given language, if available.
pub fn handler_for(language: Language) -> Option<Box<dyn LanguageHandler>> {
    match language {
        Language::TypeScript | Language::Tsx => {
            Some(Box::new(typescript::TypeScriptHandler))
        }
        Language::JavaScript | Language::Jsx => {
            Some(Box::new(javascript::JavaScriptHandler))
        }
        Language::Python => {
            Some(Box::new(python::PythonHandler))
        }
        Language::Rust => {
            Some(Box::new(rust::RustHandler))
        }
    }
}
