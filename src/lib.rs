mod error;
pub mod agent;
pub mod skill;
pub(crate) mod parser;
pub(crate) mod extractor;
pub(crate) mod languages;
mod output;
pub mod test_detect;
mod walk;
pub mod editor;
pub mod search;
pub mod tree;
pub mod references;
pub mod xrefs;
pub(crate) mod sfc;
pub(crate) mod pipeline;
pub mod handler;
pub mod dispatch;
pub mod git;
pub mod diff;
pub mod diff_cli;

use std::fs;
use std::path::Path;

pub use error::CodehudError;
pub use output::OutputFormat;
pub use languages::{Language, detect_language};
use extractor::{Item, ItemKind};

/// Options for processing paths
#[derive(Default)]
pub struct ProcessOptions {
    pub symbols: Vec<String>,
    pub pub_only: bool,
    pub fns_only: bool,
    pub types_only: bool,
    pub no_tests: bool,
    pub depth: Option<usize>,
    pub format: OutputFormat,
    pub stats: bool,
    pub summary_only: bool,
    pub ext: Vec<String>,
    pub signatures: bool,
    pub max_lines: Option<usize>,
    pub list_symbols: bool,
    pub no_imports: bool,
    pub smart_depth: bool,
    pub symbol_depth: Option<usize>,
    pub exclude: Vec<String>,
    pub outline: bool,
    pub compact: bool,
}

/// Process a file or directory and return formatted output
pub fn process_path(
    path: &str,
    options: ProcessOptions,
) -> Result<String, CodehudError> {
    let path = Path::new(path);
    
    if !path.exists() {
        return Err(CodehudError::PathNotFound(path.display().to_string()));
    }

    // Fast path: stats mode doesn't need AST parsing
    if options.stats {
        let fast_stats = pipeline::collect_stats_fast(path, &options)?;
        return pipeline::format_stats_fast(&fast_stats, options.format, options.summary_only);
    }

    // Stage 1+2: Collect files and extract items
    let file_items = pipeline::collect_and_extract(path, &options)?;

    // Check if requested symbols were actually found
    if !options.symbols.is_empty() && !options.stats && !options.list_symbols {
        let found_any = file_items.iter().any(|fi| !fi.items.is_empty());
        if !found_any {
            let sym_list = options.symbols.join("', '");
            return Err(CodehudError::SymbolNotFound {
                symbols: sym_list,
                path: path.display().to_string(),
            });
        }
    }

    // Collect source sizes for stats before filtering consumes the items
    let source_sizes: Vec<(usize, usize)> = file_items.iter().map(|fi| (fi.lines, fi.bytes)).collect();

    // Stage 3: Apply filters
    let filtered = pipeline::apply_filters(file_items, &options);

    // Stage 4: Format output
    pipeline::format_output(&filtered, &source_sizes, &options)
}

/// Returns (items, lines, bytes)
/// Extract a line range from a file with structural context.
///
/// `lines_arg` should be in the format "N-M" (1-indexed, inclusive).
/// Returns formatted output with an enclosing-symbol context header and line numbers.
pub fn extract_lines(path_str: &str, lines_arg: &str, json: bool) -> Result<String, CodehudError> {
    use std::fmt::Write;

    let path = Path::new(path_str);
    if !path.exists() {
        return Err(CodehudError::PathNotFound(path.display().to_string()));
    }
    if path.is_dir() {
        return Err(CodehudError::InvalidPath(
            "--lines only works on single files, not directories".to_string(),
        ));
    }

    // Parse the range
    let (start, end) = parse_line_range(lines_arg)?;

    let source = fs::read_to_string(path).map_err(|e| CodehudError::ReadError {
        path: path.display().to_string(),
        source: e,
    })?;

    let total_lines = source.lines().count();
    if start > total_lines {
        return Err(CodehudError::ParseError(format!(
            "Start line {} is beyond end of file ({} lines)",
            start, total_lines
        )));
    }
    let end = end.min(total_lines);

    let mut symbol_path = Vec::new();

    // Only attempt structural context for supported languages
    if languages::is_supported_file(path) {
        let language = languages::detect_language(path)?;
        let tree = parser::parse(&source, language)?;
        symbol_path = search::find_enclosing_symbols(&tree, &source, start - 1, language);
    }

    if json {
        #[derive(serde::Serialize)]
        struct LinesOutput {
            file: String,
            start: usize,
            end: usize,
            #[serde(skip_serializing_if = "Vec::is_empty")]
            symbol_path: Vec<String>,
            lines: Vec<LineEntry>,
        }
        #[derive(serde::Serialize)]
        struct LineEntry {
            line: usize,
            content: String,
        }
        let lines_vec: Vec<&str> = source.lines().collect();
        let entries: Vec<LineEntry> = lines_vec.iter().enumerate()
            .take(end).skip(start - 1)
            .map(|(i, line)| LineEntry { line: i + 1, content: line.to_string() })
            .collect();
        let output = LinesOutput {
            file: path_str.to_string(),
            start,
            end,
            symbol_path,
            lines: entries,
        };
        return Ok(serde_json::to_string(&output).map_err(|e| CodehudError::ParseError(e.to_string()))?);
    }

    let mut output = String::new();
    if !symbol_path.is_empty() {
        writeln!(output, "// Inside: {}", symbol_path.join(" > ")).unwrap();
    }

    // Extract and format lines
    let lines: Vec<&str> = source.lines().collect();
    let width = end.to_string().len().max(start.to_string().len());
    for (i, line) in lines.iter().enumerate().take(end).skip(start - 1) {
        writeln!(output, "L{:<width$}: {}", i + 1, line, width = width).unwrap();
    }

    Ok(output)
}


fn parse_line_range(arg: &str) -> Result<(usize, usize), CodehudError> {
    let parts: Vec<&str> = arg.split('-').collect();
    if parts.len() != 2 {
        return Err(CodehudError::ParseError(format!(
            "Invalid line range '{}': expected format N-M (e.g. 50-75)",
            arg
        )));
    }
    let start: usize = parts[0].parse().map_err(|_| {
        CodehudError::ParseError(format!("Invalid start line '{}' in range", parts[0]))
    })?;
    let end: usize = parts[1].parse().map_err(|_| {
        CodehudError::ParseError(format!("Invalid end line '{}' in range", parts[1]))
    })?;
    if start == 0 {
        return Err(CodehudError::ParseError(
            "Line numbers are 1-indexed; start line cannot be 0".to_string(),
        ));
    }
    if start > end {
        return Err(CodehudError::ParseError(format!(
            "Inverted range: start line {} is after end line {}",
            start, end
        )));
    }
    Ok((start, end))
}

/// Expand symbols with dispatch-based fallback for dot-notation and bare method names.
fn expand_with_dispatch(
    source: &str,
    tree: &tree_sitter::Tree,
    symbols: &[String],
    language: Language,
) -> Vec<Item> {
    let handler = handler::handler_for(language);

    // Partition symbols into qualified (dot/::) and simple names
    let mut qualified: Vec<&String> = Vec::new();
    let mut simple: Vec<String> = Vec::new();

    for sym in symbols {
        if sym.contains('.') || sym.contains("::") {
            qualified.push(sym);
        } else {
            simple.push(sym.clone());
        }
    }

    let mut all_items: Vec<Item> = Vec::new();

    // 1. Qualified names → dispatch first
    if let Some(ref h) = handler {
        for sym in &qualified {
            if let Some(mut items) = dispatch::expand_symbol(source, tree, h.as_ref(), language, sym) {
                all_items.append(&mut items);
            }
        }
    }

    // 2. Simple names → handler dispatch first, old path for remainder
    if !simple.is_empty() {
        let mut found_by_handler: std::collections::HashSet<String> = std::collections::HashSet::new();

        if let Some(ref h) = handler {
            for name in &simple {
                // Try top-level symbol lookup via handler
                if let Some(node) = dispatch::find_symbol_node_by_query(source, tree, h.as_ref(), language, name) {
                    if let Some(info) = h.classify_node(node, source) {
                        let vis = h.visibility(node, source);
                        all_items.push(dispatch::node_to_item(node, source, h.as_ref(), info.kind, info.name, vis));
                        found_by_handler.insert(name.clone());
                    }
                }
                // Try unqualified member lookup via handler
                if !found_by_handler.contains(name) {
                    let mut items = dispatch::find_unqualified_member(source, tree, h.as_ref(), language, name);
                    if !items.is_empty() {
                        found_by_handler.insert(name.clone());
                        all_items.append(&mut items);
                    }
                }
            }
        }

        // Fall back to expand query for anything not found by handler dispatch
        let remaining: Vec<String> = simple.into_iter().filter(|s| !found_by_handler.contains(s)).collect();
        if !remaining.is_empty() {
            let mut fallback_items = extractor::expand::extract(source, tree, &remaining, language);
            all_items.append(&mut fallback_items);
        }
    }

    all_items.sort_by_key(|item| item.line_start);
    all_items
}

fn process_file(
    path: &Path,
    symbols: &[String],
    expand_mode: bool,
    signatures: bool,
    expand_methods: &[String],
    pub_only: bool,
    outline: bool,
    compact: bool,
) -> Result<(Vec<Item>, usize, usize), CodehudError> {
    let source = fs::read_to_string(path)
        .map_err(|e| CodehudError::ReadError {
            path: path.display().to_string(),
            source: e,
        })?;

    let lines = source.lines().count();
    let bytes = source.len();

    // SFC files: extract script blocks and parse them with TS/JS
    if let Some(sfc_kind) = sfc::detect_sfc(path) {
        let blocks = sfc::extract_scripts(&source, sfc_kind);
        if blocks.is_empty() {
            // No script blocks found — fall through to passthrough
            if expand_mode {
                return Err(CodehudError::ParseError(format!(
                    "No script blocks found in SFC file: {}", path.display()
                )));
            }
            let numbered = source
                .lines()
                .enumerate()
                .map(|(i, line)| format!("{:>4}: {}", i + 1, line))
                .collect::<Vec<_>>()
                .join("\n");
            let item = Item {
                kind: ItemKind::Use,
                name: None,
                visibility: extractor::Visibility::Public,
                line_start: 1,
                line_end: lines,
                content: numbered,
                signature: None,
                body: None,
                line_mappings: None,
            };
            return Ok((vec![item], lines, bytes));
        }

        let mut all_items = Vec::new();
        for block in &blocks {
            let tree = parser::parse(&block.content, block.language)?;
            let mut block_items = if signatures && !symbols.is_empty() {
                extractor::expand::extract_signatures(&block.content, &tree, &symbols[0], expand_methods, block.language)
            } else if expand_mode {
                expand_with_dispatch(&block.content, &tree, symbols, block.language)
            } else {
                if outline {
                    extractor::outline::extract_outline(&block.content, &tree, block.language, pub_only, compact)
                } else {
                    extractor::interface::extract_filtered(&block.content, &tree, block.language, pub_only)
                }
            };
            // Adjust line numbers to map back to original SFC file
            let offset = block.start_line - 1;
            for item in &mut block_items {
                item.line_start += offset;
                item.line_end += offset;
                if let Some(ref mut mappings) = item.line_mappings {
                    for (line_num, _) in mappings.iter_mut() {
                        *line_num += offset;
                    }
                }
            }
            all_items.extend(block_items);
        }
        return Ok((all_items, lines, bytes));
    }

    // If not a supported Tree-sitter language, use passthrough
    if !languages::is_supported_file(path) {
        if expand_mode {
            return Err(CodehudError::ParseError(format!(
                "Symbol expansion not available for unsupported file type: {}",
                path.display()
            )));
        }
        // Return a single item representing the whole file content with line numbers
        let numbered = source
            .lines()
            .enumerate()
            .map(|(i, line)| format!("{:>4}: {}", i + 1, line))
            .collect::<Vec<_>>()
            .join("\n");
        let item = Item {
            kind: ItemKind::Use, // reuse as a generic "content" kind
            name: None,
            visibility: extractor::Visibility::Public,
            line_start: 1,
            line_end: lines,
            content: numbered,
            signature: None,
            body: None,
            line_mappings: None,
        };
        return Ok((vec![item], lines, bytes));
    }

    let language = languages::detect_language(path)?;
    let tree = parser::parse(&source, language)?;

    let items = if signatures && !symbols.is_empty() {
        // For signatures mode with dot-notation, extract the class name
        let first = &symbols[0];
        let class_name = if first.contains('.') || first.contains("::") {
            first.split(['.', ':']).find(|s| !s.is_empty()).unwrap_or(first)
        } else {
            first.as_str()
        };
        extractor::expand::extract_signatures(&source, &tree, class_name, expand_methods, language)
    } else if expand_mode {
        expand_with_dispatch(&source, &tree, symbols, language)
    } else if outline {
        extractor::outline::extract_outline(&source, &tree, language, pub_only, compact)
    } else {
        extractor::interface::extract_filtered(&source, &tree, language, pub_only)
    };

    Ok((items, lines, bytes))
}
