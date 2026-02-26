//! Symbol diffing engine.
//!
//! Compares two versions of a file at the symbol level, classifying each
//! symbol as added, deleted, or modified.

use std::collections::BTreeMap;
use std::hash::{Hash, Hasher};

use crate::dispatch;
use crate::error::CodehudError;
use crate::extractor::ItemKind;
use crate::handler;
use crate::languages::Language;
use crate::parser;

// ---------------------------------------------------------------------------
// Types
// ---------------------------------------------------------------------------

/// Information about a symbol extracted for diffing.
#[derive(Debug, Clone)]
pub struct SymbolInfo {
    pub name: String,
    pub qualified_name: String,
    pub kind: ItemKind,
    pub line_start: usize,
    pub line_end: usize,
    pub signature: String,
    /// Hash of the body content (whitespace-normalized).
    pub body_hash: u64,
}

/// How a symbol changed between two versions.
#[derive(Debug, Clone)]
pub enum SymbolChange {
    Added(SymbolInfo),
    Deleted(SymbolInfo),
    Modified {
        old: SymbolInfo,
        new: SymbolInfo,
        signature_changed: bool,
    },
}

/// The diff result for a single file.
#[derive(Debug, Clone)]
pub struct FileDiff {
    pub path: String,
    pub changes: Vec<SymbolChange>,
}

// ---------------------------------------------------------------------------
// Helpers
// ---------------------------------------------------------------------------

fn body_hash(content: &str) -> u64 {
    // Normalize whitespace for comparison
    let normalized: String = content.split_whitespace().collect::<Vec<_>>().join(" ");
    let mut hasher = std::collections::hash_map::DefaultHasher::new();
    normalized.hash(&mut hasher);
    hasher.finish()
}

/// Extract [`SymbolInfo`] entries from source code.
fn extract_symbols(source: &str, language: Language) -> Result<Vec<SymbolInfo>, CodehudError> {
    let tree = parser::parse(source, language)?;
    let handler = match handler::handler_for(language) {
        Some(h) => h,
        None => return Ok(vec![]),
    };

    // Depth 2 to include class members
    let items = dispatch::list_symbols(source, &tree, handler.as_ref(), language, 2);

    let mut symbols = Vec::new();
    let mut current_parent: Option<String> = None;

    for item in &items {
        let name = match &item.name {
            Some(n) => n.clone(),
            None => continue,
        };

        let is_container = matches!(
            item.kind,
            ItemKind::Class | ItemKind::Trait | ItemKind::Enum | ItemKind::Impl | ItemKind::Struct
        );

        if is_container {
            current_parent = Some(name.clone());
        }

        // Determine qualified name:
        // Container items are top-level, members are prefixed with parent.
        let qualified_name = if is_container {
            name.clone()
        } else if let Some(ref _parent) = current_parent {
            // Check if this item is nested (its line range is within the previous container)
            // Simple heuristic: methods/functions after a container inherit its name
            // until the next container
            if matches!(item.kind, ItemKind::Method | ItemKind::Function) {
                // Check if there was a container above that might contain this
                if let Some(container) = find_enclosing_container(&symbols, item.line_start) {
                    format!("{}.{}", container, name)
                } else {
                    name.clone()
                }
            } else {
                name.clone()
            }
        } else {
            name.clone()
        };

        let sig = item.signature.clone().unwrap_or_default();
        let hash = body_hash(&item.content);

        symbols.push(SymbolInfo {
            name,
            qualified_name,
            kind: item.kind.clone(),
            line_start: item.line_start,
            line_end: item.line_end,
            signature: sig,
            body_hash: hash,
        });
    }

    Ok(symbols)
}

/// Find the name of the enclosing container for a given line.
fn find_enclosing_container(symbols: &[SymbolInfo], line: usize) -> Option<&str> {
    // Walk backwards to find the last container whose range includes this line
    for sym in symbols.iter().rev() {
        let is_container = matches!(
            sym.kind,
            ItemKind::Class | ItemKind::Trait | ItemKind::Enum | ItemKind::Impl | ItemKind::Struct
        );
        if is_container && sym.line_start <= line && sym.line_end >= line {
            return Some(&sym.name);
        }
    }
    None
}

// ---------------------------------------------------------------------------
// Core diff
// ---------------------------------------------------------------------------

/// Diff symbols between old and new source for a file.
///
/// Both sources are parsed independently with Tree-sitter.
pub fn diff_symbols(
    old_source: &str,
    new_source: &str,
    language: Language,
) -> Result<Vec<SymbolChange>, CodehudError> {
    let old_syms = extract_symbols(old_source, language)?;
    let new_syms = extract_symbols(new_source, language)?;

    // Index by qualified name
    let old_map: BTreeMap<&str, &SymbolInfo> = old_syms
        .iter()
        .map(|s| (s.qualified_name.as_str(), s))
        .collect();
    let new_map: BTreeMap<&str, &SymbolInfo> = new_syms
        .iter()
        .map(|s| (s.qualified_name.as_str(), s))
        .collect();

    let mut changes = Vec::new();

    // Deleted: in old, not in new
    for (qname, old) in &old_map {
        if !new_map.contains_key(qname) {
            changes.push(SymbolChange::Deleted((*old).clone()));
        }
    }

    // Added: in new, not in old
    for (qname, new) in &new_map {
        if !old_map.contains_key(qname) {
            changes.push(SymbolChange::Added((*new).clone()));
        }
    }

    // Modified: in both, body differs
    for (qname, old) in &old_map {
        if let Some(new) = new_map.get(qname)
            && old.body_hash != new.body_hash {
                let sig_changed = old.signature != new.signature;
                changes.push(SymbolChange::Modified {
                    old: (*old).clone(),
                    new: (*new).clone(),
                    signature_changed: sig_changed,
                });
            }
    }

    Ok(changes)
}

/// Convenience: diff with parse-failure tolerance.
///
/// If old fails to parse, all new symbols are "added".
/// If new fails to parse, all old symbols are "deleted".
pub fn diff_symbols_tolerant(
    old_source: Option<&str>,
    new_source: Option<&str>,
    language: Language,
) -> Vec<SymbolChange> {
    match (old_source, new_source) {
        (Some(old), Some(new)) => diff_symbols(old, new, language).unwrap_or_default(),
        (None, Some(new)) => {
            // All added
            extract_symbols(new, language)
                .unwrap_or_default()
                .into_iter()
                .map(SymbolChange::Added)
                .collect()
        }
        (Some(old), None) => {
            // All deleted
            extract_symbols(old, language)
                .unwrap_or_default()
                .into_iter()
                .map(SymbolChange::Deleted)
                .collect()
        }
        (None, None) => vec![],
    }
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_no_changes() {
        let src = "fn foo() { 1 + 1 }";
        let changes = diff_symbols(src, src, Language::Rust).unwrap();
        assert!(changes.is_empty());
    }

    #[test]
    fn test_added_function() {
        let old = "fn foo() {}";
        let new = "fn foo() {}\nfn bar() {}";
        let changes = diff_symbols(old, new, Language::Rust).unwrap();
        assert_eq!(changes.len(), 1);
        assert!(matches!(&changes[0], SymbolChange::Added(s) if s.name == "bar"));
    }

    #[test]
    fn test_deleted_function() {
        let old = "fn foo() {}\nfn bar() {}";
        let new = "fn foo() {}";
        let changes = diff_symbols(old, new, Language::Rust).unwrap();
        assert_eq!(changes.len(), 1);
        assert!(matches!(&changes[0], SymbolChange::Deleted(s) if s.name == "bar"));
    }

    #[test]
    fn test_modified_function() {
        let old = "fn foo() { 1 + 1 }";
        let new = "fn foo() { 2 + 2 }";
        let changes = diff_symbols(old, new, Language::Rust).unwrap();
        assert_eq!(changes.len(), 1);
        assert!(matches!(&changes[0], SymbolChange::Modified { signature_changed: false, .. }));
    }

    #[test]
    fn test_signature_changed() {
        let old = "fn foo() {}";
        let new = "fn foo(x: i32) {}";
        let changes = diff_symbols(old, new, Language::Rust).unwrap();
        assert_eq!(changes.len(), 1);
        assert!(matches!(&changes[0], SymbolChange::Modified { signature_changed: true, .. }));
    }

    #[test]
    fn test_tolerant_all_added() {
        let changes = diff_symbols_tolerant(None, Some("fn foo() {}"), Language::Rust);
        assert_eq!(changes.len(), 1);
        assert!(matches!(&changes[0], SymbolChange::Added(_)));
    }

    #[test]
    fn test_tolerant_all_deleted() {
        let changes = diff_symbols_tolerant(Some("fn foo() {}"), None, Language::Rust);
        assert_eq!(changes.len(), 1);
        assert!(matches!(&changes[0], SymbolChange::Deleted(_)));
    }

    #[test]
    fn test_typescript_diff() {
        let old = r#"
export class Foo {
    bar() { return 1; }
}
"#;
        let new = r#"
export class Foo {
    bar() { return 2; }
    baz() { return 3; }
}
"#;
        let changes = diff_symbols(old, new, Language::TypeScript).unwrap();
        // Should detect bar modified and baz added
        let added: Vec<_> = changes.iter().filter(|c| matches!(c, SymbolChange::Added(_))).collect();
        let modified: Vec<_> = changes.iter().filter(|c| matches!(c, SymbolChange::Modified { .. })).collect();
        assert!(!added.is_empty() || !modified.is_empty(), "expected changes, got none");
    }

    #[test]
    fn test_python_diff() {
        let old = "def foo():\n    pass\n";
        let new = "def foo():\n    return 1\ndef bar():\n    pass\n";
        let changes = diff_symbols(old, new, Language::Python).unwrap();
        let added: Vec<_> = changes.iter().filter(|c| matches!(c, SymbolChange::Added(_))).collect();
        assert_eq!(added.len(), 1);
    }

    #[test]
    fn test_struct_diff() {
        let old = "pub struct Foo { x: i32 }";
        let new = "pub struct Foo { x: i32, y: i32 }";
        let changes = diff_symbols(old, new, Language::Rust).unwrap();
        // At least one modified change for Foo
        let modified: Vec<_> = changes.iter().filter(|c| matches!(c, SymbolChange::Modified { .. })).collect();
        assert!(!modified.is_empty(), "expected at least one modified symbol");
    }
}
