pub mod cpp;
pub mod csharp;
pub mod go;
pub mod java;
pub mod javascript;
pub mod kotlin;
pub mod python;
pub mod rust;
pub mod typescript;

// Re-export types used in the handler API so consumers don't need to reach into extractor
pub use crate::extractor::{ItemKind, Visibility};
pub use crate::languages::{Language, ts_language};
use std::path::Path;
use tree_sitter::Node;

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

/// Collect preceding comment sibling nodes in source order.
/// Shared helper for doc comment extraction across languages.
pub fn collect_prev_comment_siblings(source: &str, node: Node) -> Vec<String> {
    let comment_kinds = [
        "comment",
        "line_comment",
        "block_comment",
        "multiline_comment",
    ];
    let mut comments = Vec::new();
    let mut current = node;
    while let Some(prev) = current.prev_sibling() {
        if comment_kinds.contains(&prev.kind()) {
            comments.push(source[prev.start_byte()..prev.end_byte()].to_string());
            current = prev;
        } else {
            break;
        }
    }
    comments.reverse();
    comments
}

/// Skip backwards past attribute_item / decorator siblings to find the effective node.
pub fn skip_past_attributes(node: Node) -> Node {
    let mut current = node;
    loop {
        match current.prev_sibling() {
            Some(prev) if prev.kind() == "attribute_item" || prev.kind() == "decorator" => {
                current = prev;
            }
            _ => break,
        }
    }
    current
}

/// Default doc comment extraction: walk back past attrs, collect preceding comments,
/// filter for `///` or `/**` style (works for Rust, C#, Kotlin, Java).
pub fn default_get_doc_comment(source: &str, node: Node) -> Option<String> {
    let effective = skip_past_attributes(node);
    let comments = collect_prev_comment_siblings(source, effective);

    if comments.is_empty() {
        return None;
    }

    // Filter for doc comments (/// or /**)
    let doc_comments: Vec<&String> = comments
        .iter()
        .filter(|c| {
            let trimmed = c.trim();
            trimmed.starts_with("///") || trimmed.starts_with("/**") || trimmed.starts_with("* ")
        })
        .collect();

    if doc_comments.is_empty() {
        // Check for standalone /** block
        if comments.iter().any(|c| c.trim().starts_with("/**")) {
            return Some(comments.join("\n"));
        }
        return None;
    }

    Some(
        doc_comments
            .into_iter()
            .cloned()
            .collect::<Vec<_>>()
            .join("\n"),
    )
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

    /// Maximum tree depth for symbol query matching.
    /// Default is 2, which handles top-level + export wrappers.
    /// Languages with namespace blocks (e.g. C#) may need higher values.
    fn max_query_depth(&self) -> u32 {
        2
    }

    /// Extract the doc comment attached to the given item node, if any.
    /// Default implementation handles `///` and `/**` style comments (Rust, C#, Kotlin, Java).
    /// Languages with different conventions should override this.
    fn get_doc_comment(&self, source: &str, node: Node) -> Option<String> {
        default_get_doc_comment(source, node)
    }
}

/// Get a `LanguageHandler` implementation for the given language, if available.
pub fn handler_for(language: Language) -> Option<Box<dyn LanguageHandler>> {
    match language {
        Language::TypeScript | Language::Tsx => Some(Box::new(typescript::TypeScriptHandler)),
        Language::JavaScript | Language::Jsx => Some(Box::new(javascript::JavaScriptHandler)),
        Language::Python => Some(Box::new(python::PythonHandler)),
        Language::Rust => Some(Box::new(rust::RustHandler)),
        Language::Java => Some(Box::new(java::JavaHandler)),
        Language::Go => Some(Box::new(go::GoHandler)),
        Language::Cpp => Some(Box::new(cpp::CppHandler)),
        Language::CSharp => Some(Box::new(csharp::CSharpHandler)),
        Language::Kotlin => Some(Box::new(kotlin::KotlinHandler)),
    }
}
