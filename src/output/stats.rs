use crate::error::CodehudError;
use crate::extractor::Item;
use crate::languages::detect_language;
use super::OutputFormat;
use std::collections::BTreeMap;
use std::fmt::Write;
use std::path::Path;

/// Per-file statistics
struct FileStats {
    path: String,
    lines: usize,
    bytes: usize,
    items: usize,
    kinds: BTreeMap<String, usize>,
}

/// Gather common totals from files + source_sizes.
fn gather_stats(
    files: &[(String, Vec<Item>)],
    source_sizes: &[(usize, usize)],
) -> (Vec<FileStats>, usize, usize, usize, BTreeMap<String, usize>) {
    let mut total_lines = 0usize;
    let mut total_bytes = 0usize;
    let mut total_items = 0usize;
    let mut total_kinds: BTreeMap<String, usize> = BTreeMap::new();

    let file_stats: Vec<FileStats> = files
        .iter()
        .zip(source_sizes.iter())
        .map(|((path, items), &(lines, bytes))| {
            let lang = detect_language(Path::new(path)).ok();
            let mut kinds: BTreeMap<String, usize> = BTreeMap::new();
            for item in items {
                let kind = match lang {
                    Some(l) => item.kind.display_name(l).to_string(),
                    None => format!("{:?}", item.kind).to_lowercase(),
                };
                *kinds.entry(kind.clone()).or_default() += 1;
                *total_kinds.entry(kind).or_default() += 1;
            }
            total_lines += lines;
            total_bytes += bytes;
            total_items += items.len();
            FileStats {
                path: path.clone(),
                lines,
                bytes,
                items: items.len(),
                kinds,
            }
        })
        .collect();

    (file_stats, total_lines, total_bytes, total_items, total_kinds)
}

/// Format stats output in the requested format.
pub fn format_output(
    files: &[(String, Vec<Item>)],
    source_sizes: &[(usize, usize)],
    format: OutputFormat,
    summary_only: bool,
) -> Result<String, CodehudError> {
    match format {
        OutputFormat::Plain => format_plain(files, source_sizes, summary_only),
        OutputFormat::Json => format_json(files, source_sizes, summary_only),
    }
}

fn format_plain(
    files: &[(String, Vec<Item>)],
    source_sizes: &[(usize, usize)],
    summary_only: bool,
) -> Result<String, CodehudError> {
    let (file_stats, total_lines, total_bytes, total_items, total_kinds) =
        gather_stats(files, source_sizes);

    let mut out = String::new();
    let file_count = file_stats.iter().filter(|f| f.items > 0 || file_stats.len() == 1).count();

    writeln!(out, "files: {}  lines: {}  bytes: {}  items: {}",
        file_count, total_lines, total_bytes, total_items).unwrap();

    if !total_kinds.is_empty() {
        let kinds_str: Vec<String> = total_kinds
            .iter()
            .map(|(k, v)| format!("{}: {}", k, v))
            .collect();
        writeln!(out, "  {}", kinds_str.join("  ")).unwrap();
    }

    if file_stats.len() > 1 && !summary_only {
        writeln!(out).unwrap();
        for f in &file_stats {
            if f.items == 0 {
                continue;
            }
            let kinds_str: Vec<String> = f.kinds
                .iter()
                .map(|(k, v)| format!("{} {}", v, k))
                .collect();
            writeln!(out, "  {} — {} lines, {} bytes, {} items ({})",
                f.path, f.lines, f.bytes, f.items, kinds_str.join(", ")).unwrap();
        }
    }

    Ok(out)
}

fn format_json(
    files: &[(String, Vec<Item>)],
    source_sizes: &[(usize, usize)],
    summary_only: bool,
) -> Result<String, CodehudError> {
    use serde::Serialize;

    #[derive(Serialize)]
    struct StatsOutput {
        files: usize,
        lines: usize,
        bytes: usize,
        items: usize,
        kinds: BTreeMap<String, usize>,
        per_file: Vec<FileStatJson>,
    }

    #[derive(Serialize)]
    struct FileStatJson {
        path: String,
        lines: usize,
        bytes: usize,
        items: usize,
        kinds: BTreeMap<String, usize>,
    }

    let (file_stats, total_lines, total_bytes, total_items, total_kinds) =
        gather_stats(files, source_sizes);

    let per_file: Vec<FileStatJson> = if summary_only {
        vec![]
    } else {
        file_stats
            .into_iter()
            .filter(|f| f.items > 0)
            .map(|f| FileStatJson {
                path: f.path,
                lines: f.lines,
                bytes: f.bytes,
                items: f.items,
                kinds: f.kinds,
            })
            .collect()
    };

    let file_count = if summary_only {
        files.iter().filter(|(_, items)| !items.is_empty()).count()
    } else {
        per_file.len()
    };

    let output = StatsOutput {
        files: file_count,
        lines: total_lines,
        bytes: total_bytes,
        items: total_items,
        kinds: total_kinds,
        per_file,
    };

    Ok(serde_json::to_string_pretty(&output)?)
}
