//! Cross-file reference resolution via import graph analysis.
//!
//! Parses import statements (TS/JS, Rust, Python) to build a graph of which files
//! import which symbols from where, then follows those edges to find all usages
//! of a symbol across a project.

use crate::error::CodehudError;
use crate::languages::{self};
use crate::parser;
use crate::references::{self, RefKind, Reference, ReferenceOptions};
use crate::walk;
use rayon::prelude::*;
use std::collections::HashSet;
use std::fs;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicBool, AtomicUsize, Ordering};
use tree_sitter::Node;

/// An import edge: file X imports symbol S from source path P.
#[derive(Debug, Clone)]
pub struct ImportEdge {
    /// The file containing the import statement
    pub importing_file: PathBuf,
    /// The symbols imported (empty = wildcard/namespace)
    pub symbols: Vec<String>,
    /// The raw source string (e.g. "./foo", "std::collections::HashMap")
    pub source: String,
    /// Resolved absolute path of the imported module (if resolvable)
    pub resolved_path: Option<PathBuf>,
}

/// Options for cross-file reference search.
pub struct XrefOptions {
    pub symbol: String,
    pub depth: Option<usize>,
    pub ext: Vec<String>,
    pub context_lines: usize,
    pub json: bool,
    pub exclude: Vec<String>,
    pub max_results: Option<usize>,
}

/// Find cross-file references to a symbol by following import graphs.
///
/// Strategy:
/// 1. Find all files where the symbol is *defined* (using AST-aware references)
/// 2. Parse all import statements in the project
/// 3. Find files that import from the definition file and reference the symbol
/// 4. Return combined results: definitions + all cross-file usages
pub fn find_xrefs(path: &str, options: &XrefOptions) -> Result<Vec<Reference>, CodehudError> {
    let path = Path::new(path);
    if !path.exists() {
        return Err(CodehudError::PathNotFound(path.display().to_string()));
    }

    // Method-level xrefs: if symbol contains `.` or `::`, route to method xrefs
    if options.symbol.contains('.') || options.symbol.contains("::") {
        return find_method_xrefs(path, options);
    }

    // For single files, fall back to regular references
    if path.is_file() {
        let ref_opts = ReferenceOptions {
            symbol: options.symbol.clone(),
            depth: options.depth,
            ext: options.ext.clone(),
            context_lines: options.context_lines,
            defs_only: false,
            refs_only: false,
            json: options.json,
            exclude: options.exclude.clone(),
        };
        return references::find_references(path.to_str().unwrap_or(""), &ref_opts);
    }

    let files = walk::walk_directory(path, options.depth, &options.ext)?;
    let files = walk::filter_excludes(files, path, &options.exclude);

    let max_results = options.max_results.unwrap_or(usize::MAX);
    let result_count = AtomicUsize::new(0);
    let limit_reached = AtomicBool::new(false);

    // Step 1+2 combined: Parse each file once, collecting both refs and imports in parallel
    let per_file_results: Vec<(Vec<Reference>, Vec<ImportEdge>, bool)> = files
        .par_iter()
        .filter_map(|file_path| {
            if limit_reached.load(Ordering::Relaxed) {
                return None;
            }

            let source = fs::read_to_string(file_path).ok()?;
            if !languages::is_supported_file(file_path) {
                return None;
            }
            let language = languages::detect_language(file_path).ok()?;
            let tree = parser::parse(&source, language).ok()?;

            let lines: Vec<&str> = source.lines().collect();
            let mut file_refs = Vec::new();
            references::find_refs_in_tree(
                &tree.root_node(),
                &source,
                &lines,
                &options.symbol,
                language,
                file_path.to_string_lossy().as_ref(),
                options.context_lines,
                &mut file_refs,
            );

            let has_def = file_refs.iter().any(|r| r.kind == RefKind::Definition);

            // Parse imports from the same already-parsed tree
            let handler = crate::handler::handler_for(language);
            let file_edges = if let Some(ref h) = handler {
                h.parse_imports(tree.root_node(), &source, file_path, path)
            } else {
                vec![]
            };

            // Track result count for early termination
            if !file_refs.is_empty() {
                let prev = result_count.fetch_add(file_refs.len(), Ordering::Relaxed);
                if prev + file_refs.len() >= max_results {
                    limit_reached.store(true, Ordering::Relaxed);
                }
            }

            Some((file_refs, file_edges, has_def))
        })
        .collect();

    // Gather results
    let mut all_refs: Vec<Reference> = Vec::new();
    let mut import_edges: Vec<ImportEdge> = Vec::new();
    let mut def_files: HashSet<PathBuf> = HashSet::new();
    let mut seen_files: HashSet<PathBuf> = HashSet::new();

    for (file_refs, edges, has_def) in &per_file_results {
        if !file_refs.is_empty()
            && let Some(first) = file_refs.first() {
                seen_files.insert(PathBuf::from(&first.file));
            }
        if *has_def
            && let Some(first) = file_refs.first() {
                def_files.insert(PathBuf::from(&first.file));
            }
        all_refs.extend(file_refs.iter().cloned());
        import_edges.extend(edges.iter().cloned());
    }

    // Step 3: Find files that import from definition files (only if under limit)
    if !limit_reached.load(Ordering::Relaxed) {
        // Collect edges that need scanning
        let edges_to_scan: Vec<&ImportEdge> = import_edges
            .iter()
            .filter(|edge| {
                let imports_our_symbol = edge.symbols.contains(&options.symbol)
                    || edge.symbols.contains(&"*".to_string());
                if !imports_our_symbol {
                    return false;
                }
                let points_to_def = if let Some(ref resolved) = edge.resolved_path {
                    def_files.contains(resolved)
                } else {
                    def_files.iter().any(|def| source_matches_file(&edge.source, def, path))
                };
                if !points_to_def {
                    return false;
                }
                !seen_files.contains(&edge.importing_file)
            })
            .collect();

        let import_refs: Vec<Vec<Reference>> = edges_to_scan
            .par_iter()
            .filter_map(|edge| {
                if limit_reached.load(Ordering::Relaxed) {
                    return None;
                }
                let source = fs::read_to_string(&edge.importing_file).ok()?;
                if !languages::is_supported_file(&edge.importing_file) {
                    return None;
                }
                let language = languages::detect_language(&edge.importing_file).ok()?;
                let tree = parser::parse(&source, language).ok()?;
                let lines: Vec<&str> = source.lines().collect();
                let mut refs = Vec::new();
                references::find_refs_in_tree(
                    &tree.root_node(),
                    &source,
                    &lines,
                    &options.symbol,
                    language,
                    edge.importing_file.to_string_lossy().as_ref(),
                    options.context_lines,
                    &mut refs,
                );
                if !refs.is_empty() {
                    let prev = result_count.fetch_add(refs.len(), Ordering::Relaxed);
                    if prev + refs.len() >= max_results {
                        limit_reached.store(true, Ordering::Relaxed);
                    }
                }
                Some(refs)
            })
            .collect();

        for refs in import_refs {
            all_refs.extend(refs);
        }
    }

    // Deduplicate by (file, line, column)
    let mut seen = HashSet::new();
    all_refs.retain(|r| seen.insert((r.file.clone(), r.line, r.column)));

    // Sort: definitions first, then by file and line
    all_refs.sort_by(|a, b| {
        let def_ord = |k: &RefKind| match k {
            RefKind::Definition => 0,
            RefKind::Reference => 1,
            RefKind::Text => 2,
        };
        def_ord(&a.kind)
            .cmp(&def_ord(&b.kind))
            .then_with(|| a.file.cmp(&b.file))
            .then_with(|| a.line.cmp(&b.line))
    });

    Ok(all_refs)
}

/// Find cross-file references to a method (dot-notation like `Workflow.getStartNode`).
///
/// Strategy:
/// 1. Split the symbol on `.` or `::` to get parent class and method name
/// 2. Use LanguageHandler to find the method definition in the parent class
/// 3. Find files importing the parent class
/// 4. In those files, search for member_expression nodes where property == method_name
fn find_method_xrefs(path: &Path, options: &XrefOptions) -> Result<Vec<Reference>, CodehudError> {
    // Split symbol: "Workflow.getStartNode" -> ("Workflow", "getStartNode")
    let (parent_name, method_name) = if let Some(dot_pos) = options.symbol.rfind('.') {
        (&options.symbol[..dot_pos], &options.symbol[dot_pos + 1..])
    } else if let Some(cc_pos) = options.symbol.rfind("::") {
        (&options.symbol[..cc_pos], &options.symbol[cc_pos + 2..])
    } else {
        return Ok(vec![]);
    };

    let files = if path.is_file() {
        vec![path.to_path_buf()]
    } else if path.is_dir() {
        let files = walk::walk_directory(path, options.depth, &options.ext)?;
        walk::filter_excludes(files, path, &options.exclude)
    } else {
        return Err(CodehudError::InvalidPath(path.display().to_string()));
    };

    let _root = if path.is_dir() { path } else { path.parent().unwrap_or(path) };

    let max_results = options.max_results.unwrap_or(usize::MAX);
    let result_count = AtomicUsize::new(0);
    let limit_reached = AtomicBool::new(false);

    let per_file: Vec<Vec<Reference>> = files
        .par_iter()
        .filter_map(|file_path| {
            if limit_reached.load(Ordering::Relaxed) {
                return None;
            }
            let source = fs::read_to_string(file_path).ok()?;
            if !languages::is_supported_file(file_path) {
                return None;
            }
            let language = languages::detect_language(file_path).ok()?;
            let tree = parser::parse(&source, language).ok()?;
            let lines: Vec<&str> = source.lines().collect();
            let file_str = file_path.to_string_lossy();

            let mut refs = Vec::new();

            // Use LanguageHandler to find method definition in the parent class
            if let Some(handler) = crate::handler::handler_for(language)
                && let Some(items) = crate::dispatch::expand_symbol(
                    &source, &tree, handler.as_ref(), language,
                    &format!("{}.{}", parent_name, method_name),
                ) {
                    for item in &items {
                        refs.push(Reference {
                            file: file_str.to_string(),
                            line: item.line_start,
                            column: 0,
                            kind: RefKind::Definition,
                            line_content: lines.get(item.line_start.saturating_sub(1)).unwrap_or(&"").to_string(),
                            context: get_context(&lines, item.line_start.saturating_sub(1), options.context_lines),
                        });
                    }
                }

            // Search for member_expression nodes where property == method_name
            find_member_refs(
                &tree.root_node(),
                &source,
                &lines,
                method_name,
                &file_str,
                options.context_lines,
                &mut refs,
            );

            if !refs.is_empty() {
                let prev = result_count.fetch_add(refs.len(), Ordering::Relaxed);
                if prev + refs.len() >= max_results {
                    limit_reached.store(true, Ordering::Relaxed);
                }
            }

            Some(refs)
        })
        .collect();

    let mut all_refs: Vec<Reference> = per_file.into_iter().flatten().collect();

    // Deduplicate by (file, line, column)
    let mut seen = HashSet::new();
    all_refs.retain(|r| seen.insert((r.file.clone(), r.line, r.column)));

    // Sort: definitions first
    all_refs.sort_by(|a, b| {
        let def_ord = |k: &RefKind| match k {
            RefKind::Definition => 0,
            RefKind::Reference => 1,
            RefKind::Text => 2,
        };
        def_ord(&a.kind)
            .cmp(&def_ord(&b.kind))
            .then_with(|| a.file.cmp(&b.file))
            .then_with(|| a.line.cmp(&b.line))
    });

    Ok(all_refs)
}

/// Find member_expression nodes where .property matches the method name.
/// Handles `this.method()`, `obj.method()`, etc.
fn find_member_refs(
    node: &Node,
    source: &str,
    lines: &[&str],
    method_name: &str,
    file_path: &str,
    context_lines: usize,
    refs: &mut Vec<Reference>,
) {
    if node.kind() == "member_expression" || node.kind() == "field_expression" {
        // Check the property (right-hand side)
        if let Some(prop) = node.child_by_field_name("property") {
            let prop_text = &source[prop.start_byte()..prop.end_byte()];
            if prop_text == method_name {
                let line_idx = prop.start_position().row;
                refs.push(Reference {
                    file: file_path.to_string(),
                    line: line_idx + 1,
                    column: prop.start_position().column,
                    kind: RefKind::Reference,
                    line_content: lines.get(line_idx).unwrap_or(&"").to_string(),
                    context: get_context(lines, line_idx, context_lines),
                });
            }
        }
    }

    let mut cursor = node.walk();
    for child in node.children(&mut cursor) {
        find_member_refs(&child, source, lines, method_name, file_path, context_lines, refs);
    }
}

fn get_context(lines: &[&str], idx: usize, context_lines: usize) -> Vec<String> {
    if context_lines == 0 {
        return vec![];
    }
    let start = idx.saturating_sub(context_lines);
    let end = (idx + context_lines + 1).min(lines.len());
    lines[start..end]
        .iter()
        .enumerate()
        .filter_map(|(offset, line)| {
            let i = start + offset;
            if i != idx {
                Some(format!("L{}: {}", i + 1, line))
            } else {
                None
            }
        })
        .collect()
}

/// Parse import statements from all supported files in a directory.
fn parse_all_imports(
    root: &Path,
    files: &[PathBuf],
) -> Result<Vec<ImportEdge>, CodehudError> {
    let mut edges = Vec::new();

    for file_path in files {
        let source = match fs::read_to_string(file_path) {
            Ok(s) => s,
            Err(_) => continue,
        };

        if !languages::is_supported_file(file_path) {
            continue;
        }

        let language = match languages::detect_language(file_path) {
            Ok(l) => l,
            Err(_) => continue,
        };

        let tree = match parser::parse(&source, language) {
            Ok(t) => t,
            Err(_) => continue,
        };

        let handler = crate::handler::handler_for(language);
        let file_edges = if let Some(ref h) = handler {
            h.parse_imports(tree.root_node(), &source, file_path, root)
        } else {
            vec![]
        };

        edges.extend(file_edges);
    }

    Ok(edges)
}

/// Parse JS/TS import statements (public for use by extractors).
pub fn parse_js_ts_imports_impl(
    root: &Node,
    source: &str,
    file_path: &Path,
    project_root: &Path,
) -> Vec<ImportEdge> {
    let mut edges = Vec::new();
    let mut cursor = root.walk();

    for child in root.children(&mut cursor) {
        if child.kind() != "import_statement" {
            continue;
        }

        let mut symbols = Vec::new();
        let mut source_path = String::new();

        let mut child_cursor = child.walk();
        for node in child.children(&mut child_cursor) {
            match node.kind() {
                "import_clause" => {
                    collect_import_identifiers(&node, source, &mut symbols);
                }
                "string" | "string_fragment" => {
                    let text = node_text(&node, source);
                    source_path = text.trim_matches(|c| c == '\'' || c == '"').to_string();
                }
                _ => {
                    // Also check direct children for named imports
                    if node.kind() == "import_specifier"
                        || node.kind() == "identifier"
                        || node.kind() == "named_imports"
                    {
                        collect_import_identifiers(&node, source, &mut symbols);
                    }
                }
            }
        }

        // Try source field
        if source_path.is_empty()
            && let Some(src_node) = child.child_by_field_name("source") {
                let text = node_text(&src_node, source);
                source_path = text.trim_matches(|c| c == '\'' || c == '"').to_string();
            }

        if !source_path.is_empty() {
            let resolved = resolve_js_ts_path(file_path, &source_path, project_root);
            edges.push(ImportEdge {
                importing_file: file_path.to_path_buf(),
                symbols,
                source: source_path,
                resolved_path: resolved,
            });
        }
    }

    edges
}

/// Collect identifier names from import clause nodes.
fn collect_import_identifiers(node: &Node, source: &str, symbols: &mut Vec<String>) {
    match node.kind() {
        "identifier" | "type_identifier" => {
            symbols.push(node_text(node, source));
        }
        "import_specifier" => {
            // `import { X as Y }` — we want the original name X
            if let Some(name) = node.child_by_field_name("name") {
                symbols.push(node_text(&name, source));
            } else {
                // Single identifier specifier
                let mut cursor = node.walk();
                for child in node.children(&mut cursor) {
                    if child.kind() == "identifier" || child.kind() == "type_identifier" {
                        symbols.push(node_text(&child, source));
                        break;
                    }
                }
            }
        }
        "namespace_import" => {
            symbols.push("*".to_string());
        }
        _ => {
            // Recurse
            let mut cursor = node.walk();
            for child in node.children(&mut cursor) {
                collect_import_identifiers(&child, source, symbols);
            }
        }
    }
}

/// Resolve a JS/TS relative import to an absolute file path.
fn resolve_js_ts_path(
    importing_file: &Path,
    source: &str,
    _project_root: &Path,
) -> Option<PathBuf> {
    if !source.starts_with('.') {
        return None; // Skip node_modules / bare specifiers
    }

    let dir = importing_file.parent()?;
    let base = dir.join(source);

    // Try exact, then with extensions
    let extensions = ["", ".ts", ".tsx", ".js", ".jsx", "/index.ts", "/index.tsx", "/index.js", "/index.jsx"];
    for ext in &extensions {
        let candidate = if ext.is_empty() {
            base.clone()
        } else {
            PathBuf::from(format!("{}{}", base.display(), ext))
        };
        if candidate.is_file() {
            return Some(candidate.canonicalize().unwrap_or(candidate));
        }
    }

    None
}

/// Parse Rust `use` statements (public for use by extractors).
pub fn parse_rust_imports_impl(
    root: &Node,
    source: &str,
    file_path: &Path,
) -> Vec<ImportEdge> {
    let mut edges = Vec::new();
    let mut cursor = root.walk();

    for child in root.children(&mut cursor) {
        if child.kind() != "use_declaration" {
            continue;
        }

        let mut symbols = Vec::new();
        let full_text = node_text(&child, source);

        // Extract the path and symbols from use declarations
        // e.g., `use crate::foo::Bar;` -> source="crate::foo", symbols=["Bar"]
        // e.g., `use crate::foo::{Bar, Baz};` -> source="crate::foo", symbols=["Bar", "Baz"]
        collect_rust_use_symbols(&child, source, &mut symbols);

        let source_path = full_text
            .trim_start_matches("use ")
            .trim_end_matches(';')
            .trim()
            .to_string();

        edges.push(ImportEdge {
            importing_file: file_path.to_path_buf(),
            symbols,
            source: source_path,
            resolved_path: None, // Rust module resolution is complex; rely on name matching
        });
    }

    edges
}

/// Collect symbol names from a Rust use declaration.
fn collect_rust_use_symbols(node: &Node, source: &str, symbols: &mut Vec<String>) {
    match node.kind() {
        "identifier" | "type_identifier" => {
            symbols.push(node_text(node, source));
        }
        "use_as_clause" => {
            // `Foo as Bar` — we want the original name
            let mut cursor = node.walk();
            for child in node.children(&mut cursor) {
                if child.kind() == "identifier" || child.kind() == "type_identifier" {
                    symbols.push(node_text(&child, source));
                    break;
                }
            }
        }
        "use_wildcard" => {
            symbols.push("*".to_string());
        }
        _ => {
            let mut cursor = node.walk();
            for child in node.children(&mut cursor) {
                collect_rust_use_symbols(&child, source, symbols);
            }
        }
    }
}

/// Parse Python import statements (public for use by extractors).
pub fn parse_python_imports_impl(
    root: &Node,
    source: &str,
    file_path: &Path,
) -> Vec<ImportEdge> {
    let mut edges = Vec::new();
    let mut cursor = root.walk();

    for child in root.children(&mut cursor) {
        match child.kind() {
            "import_from_statement" => {
                // `from foo import bar, baz`
                let mut symbols = Vec::new();
                let mut module_name = String::new();

                if let Some(mod_node) = child.child_by_field_name("module_name") {
                    module_name = node_text(&mod_node, source);
                }
                // Also try dotted_name for the module
                let mut child_cursor = child.walk();
                for node in child.children(&mut child_cursor) {
                    match node.kind() {
                        "dotted_name" => {
                            if module_name.is_empty() {
                                module_name = node_text(&node, source);
                            } else {
                                symbols.push(node_text(&node, source));
                            }
                        }
                        "aliased_import" => {
                            if let Some(name) = node.child_by_field_name("name") {
                                symbols.push(node_text(&name, source));
                            }
                        }
                        "identifier" => {
                            // Could be imported name
                            let text = node_text(&node, source);
                            if text != "from" && text != "import" && !module_name.is_empty() {
                                symbols.push(text);
                            }
                        }
                        "wildcard_import" => {
                            symbols.push("*".to_string());
                        }
                        _ => {}
                    }
                }

                if !module_name.is_empty() {
                    edges.push(ImportEdge {
                        importing_file: file_path.to_path_buf(),
                        symbols,
                        source: module_name,
                        resolved_path: None,
                    });
                }
            }
            "import_statement" => {
                // `import foo` — the module itself is the symbol
                let mut child_cursor = child.walk();
                for node in child.children(&mut child_cursor) {
                    if node.kind() == "dotted_name" || node.kind() == "identifier" {
                        let text = node_text(&node, source);
                        if text != "import" {
                            edges.push(ImportEdge {
                                importing_file: file_path.to_path_buf(),
                                symbols: vec![text.clone()],
                                source: text,
                                resolved_path: None,
                            });
                        }
                    }
                }
            }
            _ => {}
        }
    }

    edges
}

/// Check if an import source string plausibly refers to a file.
fn source_matches_file(source: &str, def_file: &Path, root: &Path) -> bool {
    let relative = def_file
        .strip_prefix(root)
        .unwrap_or(def_file);
    let rel_str = relative.to_string_lossy();
    let stem = relative.file_stem().unwrap_or_default().to_string_lossy();

    // Normalize the source: ./foo/bar -> foo/bar
    let normalized = source
        .trim_start_matches("./")
        .trim_start_matches("../")
        .replace("::", "/")
        .replace('.', "/");

    // Check if the import source matches the file path
    // e.g., source="./utils" matches "utils.ts" or "utils/index.ts"
    rel_str.starts_with(&normalized)
        || rel_str.contains(&format!("/{}", normalized))
        || stem == normalized.rsplit('/').next().unwrap_or(&normalized)
}

fn node_text(node: &Node, source: &str) -> String {
    source[node.start_byte()..node.end_byte()].to_string()
}

/// Get import edges for a directory (public API for testing/inspection).
pub fn get_import_graph(
    path: &str,
    depth: Option<usize>,
    ext: &[String],
) -> Result<Vec<ImportEdge>, CodehudError> {
    let path = Path::new(path);
    if !path.exists() {
        return Err(CodehudError::PathNotFound(path.display().to_string()));
    }
    let files = walk::walk_directory(path, depth, ext)?;
    parse_all_imports(path, &files)
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    fn write_file(dir: &TempDir, name: &str, content: &str) -> PathBuf {
        let path = dir.path().join(name);
        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent).unwrap();
        }
        fs::write(&path, content).unwrap();
        path
    }

    fn setup_ts_project(dir: &TempDir) {
        // Make it look like a project root
        fs::create_dir_all(dir.path().join(".git")).unwrap();

        write_file(dir, "utils.ts", r#"
export function greet(name: string): string {
    return `Hello, ${name}`;
}

export interface Config {
    debug: boolean;
}
"#);

        write_file(dir, "app.ts", r#"
import { greet, Config } from './utils';

const config: Config = { debug: true };
console.log(greet("world"));
"#);

        write_file(dir, "other.ts", r#"
function greet() {
    // Different greet, not imported
    return "hi";
}
greet();
"#);
    }

    #[test]
    fn test_ts_import_parsing() {
        let dir = TempDir::new().unwrap();
        setup_ts_project(&dir);

        let edges = get_import_graph(
            dir.path().to_str().unwrap(),
            None,
            &[],
        )
        .unwrap();

        // Should find import edge from app.ts
        let app_edges: Vec<_> = edges
            .iter()
            .filter(|e| e.importing_file.ends_with("app.ts"))
            .collect();
        assert!(!app_edges.is_empty(), "Should find imports in app.ts");
        assert!(
            app_edges[0].symbols.contains(&"greet".to_string()),
            "Should find 'greet' in imports: {:?}",
            app_edges[0].symbols
        );
        assert!(
            app_edges[0].symbols.contains(&"Config".to_string()),
            "Should find 'Config' in imports: {:?}",
            app_edges[0].symbols
        );
    }

    #[test]
    fn test_xrefs_finds_cross_file_usage() {
        let dir = TempDir::new().unwrap();
        setup_ts_project(&dir);

        let opts = XrefOptions {
            symbol: "greet".to_string(),
            depth: None,
            ext: vec![],
            context_lines: 0,
            json: false,
            exclude: vec![],
            max_results: None,
        };

        let refs = find_xrefs(dir.path().to_str().unwrap(), &opts).unwrap();

        // Should find: def in utils.ts, ref in app.ts (import + call), def+ref in other.ts
        let files: HashSet<String> = refs.iter().map(|r| {
            Path::new(&r.file).file_name().unwrap().to_string_lossy().to_string()
        }).collect();

        assert!(files.contains("utils.ts"), "Should find definition in utils.ts, got: {:?}", files);
        assert!(files.contains("app.ts"), "Should find usage in app.ts, got: {:?}", files);
    }

    #[test]
    fn test_rust_use_parsing() {
        let dir = TempDir::new().unwrap();
        fs::create_dir_all(dir.path().join(".git")).unwrap();

        write_file(&dir, "lib.rs", "pub fn helper() {}\n");
        write_file(&dir, "main.rs", "use crate::helper;\nfn main() { helper(); }\n");

        let edges = get_import_graph(dir.path().to_str().unwrap(), None, &[]).unwrap();

        let main_edges: Vec<_> = edges
            .iter()
            .filter(|e| e.importing_file.ends_with("main.rs"))
            .collect();
        assert!(!main_edges.is_empty(), "Should find use in main.rs");
        assert!(
            main_edges[0].symbols.contains(&"helper".to_string()),
            "Should find 'helper' in use: {:?}",
            main_edges[0].symbols
        );
    }

    #[test]
    fn test_rust_use_braces() {
        let dir = TempDir::new().unwrap();
        fs::create_dir_all(dir.path().join(".git")).unwrap();

        write_file(&dir, "lib.rs", "pub struct Foo;\npub struct Bar;\n");
        write_file(&dir, "main.rs", "use crate::{Foo, Bar};\nfn main() {}\n");

        let edges = get_import_graph(dir.path().to_str().unwrap(), None, &[]).unwrap();

        let main_edges: Vec<_> = edges
            .iter()
            .filter(|e| e.importing_file.ends_with("main.rs"))
            .collect();
        assert!(!main_edges.is_empty());
        let all_syms: Vec<_> = main_edges.iter().flat_map(|e| &e.symbols).collect();
        assert!(all_syms.contains(&&"Foo".to_string()), "Should find Foo: {:?}", all_syms);
        assert!(all_syms.contains(&&"Bar".to_string()), "Should find Bar: {:?}", all_syms);
    }

    #[test]
    fn test_python_import_parsing() {
        let dir = TempDir::new().unwrap();
        fs::create_dir_all(dir.path().join(".git")).unwrap();

        write_file(&dir, "utils.py", "def greet():\n    pass\n");
        write_file(&dir, "app.py", "from utils import greet\ngreet()\n");

        let edges = get_import_graph(dir.path().to_str().unwrap(), None, &[]).unwrap();

        let app_edges: Vec<_> = edges
            .iter()
            .filter(|e| e.importing_file.ends_with("app.py"))
            .collect();
        assert!(!app_edges.is_empty(), "Should find imports in app.py");
        assert!(
            app_edges[0].symbols.contains(&"greet".to_string()),
            "Should find 'greet': {:?}",
            app_edges[0].symbols
        );
    }

    #[test]
    fn test_xrefs_single_file_fallback() {
        let dir = TempDir::new().unwrap();
        let path = write_file(&dir, "test.rs", "fn foo() {}\nfn bar() { foo(); }\n");

        let opts = XrefOptions {
            symbol: "foo".to_string(),
            depth: None,
            ext: vec![],
            context_lines: 0,
            json: false,
            exclude: vec![],
            max_results: None,
        };

        let refs = find_xrefs(path.to_str().unwrap(), &opts).unwrap();
        assert_eq!(refs.len(), 2);
    }

    #[test]
    fn test_namespace_import() {
        let dir = TempDir::new().unwrap();
        fs::create_dir_all(dir.path().join(".git")).unwrap();

        write_file(&dir, "utils.ts", "export function greet() {}\n");
        write_file(&dir, "app.ts", "import * as utils from './utils';\nutils.greet();\n");

        let edges = get_import_graph(dir.path().to_str().unwrap(), None, &[]).unwrap();

        let app_edges: Vec<_> = edges
            .iter()
            .filter(|e| e.importing_file.ends_with("app.ts"))
            .collect();
        assert!(!app_edges.is_empty());
        assert!(app_edges[0].symbols.contains(&"*".to_string()));
    }

    #[test]
    fn test_js_ts_path_resolution() {
        let dir = TempDir::new().unwrap();
        fs::create_dir_all(dir.path().join(".git")).unwrap();

        write_file(&dir, "utils.ts", "export function helper() {}\n");
        write_file(&dir, "app.ts", "import { helper } from './utils';\nhelper();\n");

        let edges = get_import_graph(dir.path().to_str().unwrap(), None, &[]).unwrap();

        let app_edges: Vec<_> = edges
            .iter()
            .filter(|e| e.importing_file.ends_with("app.ts"))
            .collect();
        assert!(!app_edges.is_empty());
        assert!(
            app_edges[0].resolved_path.is_some(),
            "Should resolve ./utils to utils.ts"
        );
    }

    #[test]
    fn test_method_xrefs_ts() {
        let dir = TempDir::new().unwrap();
        fs::create_dir_all(dir.path().join(".git")).unwrap();

        write_file(&dir, "workflow.ts", r#"
export class Workflow {
    getStartNode() {
        return this.getStartNode();
    }
}
"#);

        write_file(&dir, "app.ts", r#"
import { Workflow } from './workflow';
const w = new Workflow();
w.getStartNode();
"#);

        let opts = XrefOptions {
            symbol: "Workflow.getStartNode".to_string(),
            depth: None,
            ext: vec![],
            context_lines: 0,
            json: false,
            exclude: vec![],
            max_results: None,
        };

        let refs = find_xrefs(dir.path().to_str().unwrap(), &opts).unwrap();

        // Should find: definition in workflow.ts + member_expression refs
        assert!(!refs.is_empty(), "Should find method references, got none");

        // Should find usage in app.ts
        let app_refs: Vec<_> = refs.iter().filter(|r| r.file.contains("app.ts")).collect();
        assert!(!app_refs.is_empty(), "Should find method usage in app.ts");
    }

    #[test]
    fn test_new_expression_is_reference() {
        let dir = TempDir::new().unwrap();
        let path = write_file(&dir, "test.ts", r#"
class Workflow {
    run() {}
}
const w = new Workflow();
"#);

        let opts = ReferenceOptions {
            symbol: "Workflow".to_string(),
            depth: None,
            ext: vec![],
            context_lines: 0,
            defs_only: false,
            refs_only: false,
            json: false,
            exclude: vec![],
        };
        let refs = references::find_references(path.to_str().unwrap(), &opts).unwrap();

        // "new Workflow()" should be Reference, not Definition
        let new_ref = refs.iter().find(|r| r.line == 5);
        assert!(new_ref.is_some(), "Should find Workflow on line 5");
        assert_eq!(new_ref.unwrap().kind, RefKind::Reference, "new Workflow() should be a Reference");
    }
}
