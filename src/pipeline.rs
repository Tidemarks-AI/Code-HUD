//! Pipeline stages for `process_path`: collect → extract → filter → format.

use std::path::Path;

use crate::error::CodehudError;
use crate::extractor::{Item, ItemKind};
use crate::output::OutputFormat;
use crate::{languages, test_detect, walk, output, process_file, ProcessOptions};

use std::collections::BTreeMap;

/// A collected file with its extracted items and source metrics.
#[derive(Debug, Clone)]
pub(crate) struct FileItems {
    pub path: String,
    pub items: Vec<Item>,
    pub lines: usize,
    pub bytes: usize,
}

/// Stage 1: Collect and extract items from files.
///
/// Walks directories (or handles a single file), parses each file, and returns
/// extracted items together with source size metrics. This combines the "collect"
/// and "extract" stages because extraction is interleaved with early-exit logic
/// for symbol expansion.
pub(crate) fn collect_and_extract(
    path: &Path,
    options: &ProcessOptions,
) -> Result<Vec<FileItems>, CodehudError> {
    let expand_mode = !options.symbols.is_empty();

    let (symbols, expand_methods) = if options.signatures && options.symbols.len() > 1 {
        (vec![options.symbols[0].clone()], options.symbols[1..].to_vec())
    } else {
        (options.symbols.clone(), Vec::new())
    };

    if path.is_file() {
        let (items, lines, bytes) = process_file(path, &symbols, expand_mode, options.signatures, &expand_methods, options.pub_only, options.outline, options.compact, &options.expand_symbols)?;
        return Ok(vec![FileItems {
            path: path.to_string_lossy().to_string(),
            items,
            lines,
            bytes,
        }]);
    }

    if !path.is_dir() {
        return Err(CodehudError::InvalidPath(path.display().to_string()));
    }

    let files = walk::walk_directory_smart(path, options.depth, &options.ext, options.smart_depth)?;
    let files = walk::filter_excludes(files, path, &options.exclude);

    walk::warn_if_large_repo(files.len());
    walk::warn_if_costly(files.len(), options.yes, options.warn_threshold);

    let mut results = Vec::new();
    let mut remaining_symbols: Vec<&str> = if expand_mode {
        options.symbols.iter().map(|s| s.as_str()).collect()
    } else {
        Vec::new()
    };

    for file_path in files {
        if options.no_tests && test_detect::is_test_file_any_language(&file_path) {
            continue;
        }
        match process_file(&file_path, &symbols, expand_mode, options.signatures, &expand_methods, options.pub_only, options.outline, options.compact, &options.expand_symbols) {
            Ok((items, lines, bytes)) => {
                if expand_mode && !items.is_empty() {
                    for item in &items {
                        if let Some(name) = &item.name {
                            remaining_symbols.retain(|s| *s != name.as_str());
                        }
                    }
                }
                results.push(FileItems {
                    path: file_path.to_string_lossy().to_string(),
                    items,
                    lines,
                    bytes,
                });
                if expand_mode && remaining_symbols.is_empty() {
                    break;
                }
            }
            Err(e) => {
                eprintln!("Warning: Failed to process {}: {}", file_path.display(), e);
            }
        }
    }

    Ok(results)
}

/// Lightweight per-file stats collected without AST parsing.
#[derive(Debug, Clone)]
pub(crate) struct FastFileStats {
    pub path: String,
    pub lines: usize,
    pub bytes: usize,
    pub language: String,
}

/// Fast stats collection: walks files, counts lines/bytes, detects language by extension.
/// Skips Tree-sitter parsing entirely. Uses rayon for parallel file I/O.
pub(crate) fn collect_stats_fast(
    path: &Path,
    options: &ProcessOptions,
) -> Result<Vec<FastFileStats>, CodehudError> {
    use rayon::prelude::*;

    if path.is_file() {
        let meta = std::fs::metadata(path).map_err(|e| CodehudError::ReadError {
            path: path.display().to_string(),
            source: e,
        })?;
        let bytes = meta.len() as usize;
        let lines = count_lines_fast(path).unwrap_or(0);
        let lang = language_label(path);
        return Ok(vec![FastFileStats {
            path: path.to_string_lossy().to_string(),
            lines,
            bytes,
            language: lang,
        }]);
    }

    if !path.is_dir() {
        return Err(CodehudError::InvalidPath(path.display().to_string()));
    }

    // Use parallel walk for stats mode — order doesn't matter
    let files = if options.smart_depth && options.depth.is_some() {
        walk::walk_directory_smart(path, options.depth, &options.ext, options.smart_depth)?
    } else {
        walk::walk_directory_parallel(path, options.depth, &options.ext)?
    };

    walk::warn_if_large_repo(files.len());
    walk::warn_if_costly(files.len(), options.yes, options.warn_threshold);

    let results: Vec<FastFileStats> = files
        .into_par_iter()
        .filter(|file_path| {
            !(options.no_tests && test_detect::is_test_file_any_language(file_path))
        })
        .filter_map(|file_path| {
            let meta = std::fs::metadata(&file_path).ok()?;
            let bytes = meta.len() as usize;
            let lines = count_lines_fast(&file_path).unwrap_or(0);
            let lang = language_label(&file_path);
            Some(FastFileStats {
                path: file_path.to_string_lossy().to_string(),
                lines,
                bytes,
                language: lang,
            })
        })
        .collect();

    Ok(results)
}

/// Count newlines in a file efficiently using buffered byte reads.
fn count_lines_fast(path: &Path) -> std::io::Result<usize> {
    use std::io::{BufRead, BufReader};
    let file = std::fs::File::open(path)?;
    let reader = BufReader::with_capacity(32 * 1024, file);
    let mut count = 0;
    for line in reader.split(b'\n') {
        line?;
        count += 1;
    }
    Ok(count)
}

/// Get a human-readable language label from file extension.
fn language_label(path: &Path) -> String {
    if let Ok(lang) = languages::detect_language(path) {
        match lang {
            languages::Language::Rust => "Rust",
            languages::Language::TypeScript => "TypeScript",
            languages::Language::Tsx => "TSX",
            languages::Language::JavaScript => "JavaScript",
            languages::Language::Jsx => "JSX",
            languages::Language::Python => "Python",
        }.to_string()
    } else {
        path.extension()
            .and_then(|e| e.to_str())
            .unwrap_or("other")
            .to_string()
    }
}

/// Format a number with comma separators or abbreviated (e.g. 8.2k).
fn format_count(n: usize) -> String {
    if n >= 10_000 {
        let k = n as f64 / 1000.0;
        format!("{:.1}k", k)
    } else if n >= 1_000 {
        // Use comma formatting
        let s = n.to_string();
        let mut result = String::new();
        for (i, c) in s.chars().rev().enumerate() {
            if i > 0 && i % 3 == 0 { result.push(','); }
            result.push(c);
        }
        result.chars().rev().collect()
    } else {
        n.to_string()
    }
}

/// Format fast stats output (no items/kinds, shows language breakdown instead).
pub(crate) fn format_stats_fast(
    file_stats: &[FastFileStats],
    format: OutputFormat,
    summary_only: bool,
) -> Result<String, CodehudError> {
    use std::fmt::Write;

    let total_files = file_stats.len();
    let total_lines: usize = file_stats.iter().map(|f| f.lines).sum();
    let total_bytes: usize = file_stats.iter().map(|f| f.bytes).sum();
    let total_tokens: usize = total_bytes / 4;

    let mut lang_counts: BTreeMap<String, usize> = BTreeMap::new();
    let mut lang_lines: BTreeMap<String, usize> = BTreeMap::new();
    for f in file_stats {
        *lang_counts.entry(f.language.clone()).or_default() += 1;
        *lang_lines.entry(f.language.clone()).or_default() += f.lines;
    }

    // Compute directory counts and top directories
    let mut dir_set: std::collections::BTreeSet<String> = std::collections::BTreeSet::new();
    let mut dir_file_counts: BTreeMap<String, usize> = BTreeMap::new();
    for f in file_stats {
        if let Some(parent) = std::path::Path::new(&f.path).parent() {
            let dir = parent.to_string_lossy().to_string();
            dir_set.insert(dir.clone());
            *dir_file_counts.entry(dir).or_default() += 1;
        }
    }
    let total_dirs = dir_set.len();

    // Top directories by file count
    let mut top_dirs: Vec<(String, usize)> = dir_file_counts.into_iter().collect();
    top_dirs.sort_by(|a, b| b.1.cmp(&a.1));
    top_dirs.truncate(5);

    let is_large = total_files > 1000;

    match format {
        OutputFormat::Plain => {
            let mut out = String::new();
            writeln!(out, "Files: {} | Dirs: {} | Lines: {} | Bytes: {} | Tokens: ~{}",
                format_count(total_files), format_count(total_dirs),
                format_count(total_lines), format_count(total_bytes),
                format_count(total_tokens)).unwrap();
            if !lang_counts.is_empty() {
                let mut langs: Vec<(&String, &usize)> = lang_counts.iter().collect();
                langs.sort_by(|a, b| b.1.cmp(a.1));
                let lang_strs: Vec<String> = langs.iter()
                    .map(|(k, v)| format!("{} ({})", k, format_count(**v)))
                    .collect();
                writeln!(out, "  Languages: {}", lang_strs.join(", ")).unwrap();
            }
            if !top_dirs.is_empty() && total_dirs > 1 {
                let dir_strs: Vec<String> = top_dirs.iter()
                    .map(|(d, c)| format!("{} ({})", d, c))
                    .collect();
                writeln!(out, "  Top dirs: {}", dir_strs.join(", ")).unwrap();
            }
            if !summary_only && file_stats.len() > 1 {
                writeln!(out).unwrap();
                for f in file_stats {
                    writeln!(out, "  {} — {} lines, {} bytes [{}]", f.path, f.lines, f.bytes, f.language).unwrap();
                }
            } else if file_stats.len() > 1 && is_large {
                writeln!(out, "\n[Use --stats-detailed for full file list]").unwrap();
            }
            Ok(out)
        }
        OutputFormat::Json => {
            use serde::Serialize;
            #[derive(Serialize)]
            struct StatsOut {
                files: usize,
                lines: usize,
                bytes: usize,
                tokens_approx: usize,
                languages: BTreeMap<String, LangStat>,
                per_file: Vec<FileStat>,
            }
            #[derive(Serialize)]
            struct LangStat { files: usize, lines: usize }
            #[derive(Serialize)]
            struct FileStat { path: String, lines: usize, bytes: usize, language: String }

            let languages: BTreeMap<String, LangStat> = lang_counts.into_iter()
                .map(|(k, v)| (k.clone(), LangStat { files: v, lines: lang_lines[&k] }))
                .collect();
            let per_file = if summary_only { vec![] } else {
                file_stats.iter().map(|f| FileStat {
                    path: f.path.clone(), lines: f.lines, bytes: f.bytes, language: f.language.clone()
                }).collect()
            };
            let output = StatsOut { files: total_files, lines: total_lines, bytes: total_bytes, tokens_approx: total_tokens, languages, per_file };
            Ok(serde_json::to_string_pretty(&output)?)
        }
    }
}

/// Stage 2: Apply filters (visibility, kind, test, import) to extracted items.
pub(crate) fn apply_filters(
    file_items: Vec<FileItems>,
    options: &ProcessOptions,
) -> Vec<(String, Vec<Item>)> {
    let has_kind_filter = options.fns_only || options.types_only;

    file_items
        .into_iter()
        .map(|fi| {
            let detector: Option<Box<dyn crate::test_detect::TestDetector>> = if options.no_tests {
                languages::detect_language(Path::new(&fi.path))
                    .ok()
                    .map(crate::test_detect::detector_for)
            } else {
                None
            };

            let filtered_items = fi.items
                .into_iter()
                .filter(|item| {
                    if let Some(ref det) = detector
                        && det.is_test_item(item) {
                            return false;
                        }
                    if options.pub_only && !item.is_public() {
                        return false;
                    }
                    if options.no_imports && matches!(item.kind, ItemKind::Use) {
                        return false;
                    }
                    if has_kind_filter {
                        let is_fn = matches!(item.kind, ItemKind::Function | ItemKind::Method);
                        let is_type = matches!(
                            item.kind,
                            ItemKind::Struct | ItemKind::Enum | ItemKind::Trait | ItemKind::TypeAlias | ItemKind::Class
                        );
                        let mut matched = false;
                        if options.fns_only && is_fn { matched = true; }
                        if options.types_only && is_type { matched = true; }
                        if !matched { return false; }
                        if matches!(item.kind, ItemKind::Method) && !options.fns_only {
                            return false;
                        }
                    } else if matches!(item.kind, ItemKind::Method) {
                        // Show methods when expanding symbols or list-symbols with symbol-depth >= 2
                        let expand_mode = !options.symbols.is_empty();
                        let show_methods = expand_mode
                            || (options.list_symbols
                                && options.symbol_depth.is_some_and(|d| d >= 2));
                        if !show_methods {
                            return false;
                        }
                    }
                    true
                })
                .collect();
            (fi.path, filtered_items)
        })
        .collect()
}

/// Stage 3: Format the filtered output.
pub(crate) fn format_output(
    filtered: &[(String, Vec<Item>)],
    source_sizes: &[(usize, usize)],
    options: &ProcessOptions,
) -> Result<String, CodehudError> {
    let expand_mode = !options.symbols.is_empty();

    if options.stats {
        output::stats::format_output(filtered, source_sizes, options.format, !options.stats_detailed)
    } else if options.outline {
        match options.format {
            OutputFormat::Json => output::json::format_output(filtered),
            OutputFormat::Plain => output::plain::format_outline(filtered),
        }
    } else if options.list_symbols {
        if options.minimal {
            match options.format {
                OutputFormat::Json => output::json::format_list_symbols_minimal(filtered),
                OutputFormat::Plain => output::plain::format_list_symbols_minimal(filtered),
            }
        } else {
            match options.format {
                OutputFormat::Json => output::json::format_list_symbols(filtered),
                OutputFormat::Plain => output::plain::format_list_symbols(filtered),
            }
        }


    } else {
        match options.format {
            OutputFormat::Plain => output::plain::format_output(filtered, expand_mode, options.max_lines),
            OutputFormat::Json => output::json::format_output(filtered),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::extractor::{Item, ItemKind, Visibility};

    fn make_item(name: &str, kind: ItemKind, vis: Visibility) -> Item {
        Item {
            kind,
            name: Some(name.to_string()),
            visibility: vis,
            line_start: 1,
            line_end: 1,
            content: String::new(),
            signature: None,
            body: None,
            line_mappings: None,
        }
    }

    fn default_options() -> ProcessOptions {
        ProcessOptions {
            symbols: vec![],
            pub_only: false,
            fns_only: false,
            types_only: false,
            no_tests: false,
            depth: None,
            format: OutputFormat::Plain,
            stats: false,
            stats_detailed: true,
            ext: vec![],
            signatures: false,
            max_lines: None,
            list_symbols: false,
            no_imports: false,
            smart_depth: false,
        symbol_depth: None,
        exclude: vec![],
        outline: false,
        compact: false,
        minimal: false,
        expand_symbols: vec![],
        yes: false,
        warn_threshold: 10_000,
        }
    }

    #[test]
    fn test_apply_filters_pub_only() {
        let items = vec![FileItems {
            path: "test.rs".to_string(),
            items: vec![
                make_item("public_fn", ItemKind::Function, Visibility::Public),
                make_item("private_fn", ItemKind::Function, Visibility::Private),
            ],
            lines: 10,
            bytes: 100,
        }];
        let mut opts = default_options();
        opts.pub_only = true;
        let result = apply_filters(items, &opts);
        assert_eq!(result.len(), 1);
        assert_eq!(result[0].1.len(), 1);
        assert_eq!(result[0].1[0].name.as_deref(), Some("public_fn"));
    }

    #[test]
    fn test_apply_filters_fns_only() {
        let items = vec![FileItems {
            path: "test.rs".to_string(),
            items: vec![
                make_item("my_fn", ItemKind::Function, Visibility::Public),
                make_item("MyStruct", ItemKind::Struct, Visibility::Public),
            ],
            lines: 10,
            bytes: 100,
        }];
        let mut opts = default_options();
        opts.fns_only = true;
        let result = apply_filters(items, &opts);
        assert_eq!(result[0].1.len(), 1);
        assert_eq!(result[0].1[0].name.as_deref(), Some("my_fn"));
    }

    #[test]
    fn test_apply_filters_no_imports() {
        let items = vec![FileItems {
            path: "test.rs".to_string(),
            items: vec![
                make_item("my_fn", ItemKind::Function, Visibility::Public),
                make_item("std::io", ItemKind::Use, Visibility::Private),
            ],
            lines: 10,
            bytes: 100,
        }];
        let mut opts = default_options();
        opts.no_imports = true;
        let result = apply_filters(items, &opts);
        assert_eq!(result[0].1.len(), 1);
        assert_eq!(result[0].1[0].name.as_deref(), Some("my_fn"));
    }

    #[test]
    fn test_apply_filters_hides_standalone_methods() {
        let items = vec![FileItems {
            path: "test.rs".to_string(),
            items: vec![
                make_item("my_fn", ItemKind::Function, Visibility::Public),
                make_item("my_method", ItemKind::Method, Visibility::Public),
            ],
            lines: 10,
            bytes: 100,
        }];
        let opts = default_options();
        let result = apply_filters(items, &opts);
        assert_eq!(result[0].1.len(), 1);
        assert_eq!(result[0].1[0].name.as_deref(), Some("my_fn"));
    }
}
