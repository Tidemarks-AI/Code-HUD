//! Orchestrates `--diff`: git layer → symbol diff → filtering → output formatting.

use std::fmt::Write;
use std::path::Path;

use serde::Serialize;

use crate::diff::{self, FileDiff, SymbolChange, SymbolInfo};
use crate::error::CodehudError;
use crate::extractor::ItemKind;
use crate::git::{self, ChangeStatus, FileChange};
use crate::languages;

// ---------------------------------------------------------------------------
// Options
// ---------------------------------------------------------------------------

/// CLI-level options for diff mode.
pub struct DiffOptions {
    /// Git ref to diff against (e.g. "HEAD", "main", "HEAD~3").
    pub refspec: String,
    /// Diff staged changes instead of working tree.
    pub staged: bool,
    /// Filter: only public symbols.
    pub pub_only: bool,
    /// Filter: only functions/methods.
    pub fns_only: bool,
    /// Filter: only types.
    pub types_only: bool,
    /// Filter: exclude test files.
    pub no_tests: bool,
    /// Filter: file extensions.
    pub ext: Vec<String>,
    /// Filter: exclude glob patterns.
    pub exclude: Vec<String>,
    /// JSON output.
    pub json: bool,
}

// ---------------------------------------------------------------------------
// JSON types
// ---------------------------------------------------------------------------

#[derive(Serialize)]
struct JsonFileDiff {
    file: String,
    symbols: Vec<JsonSymbolChange>,
}

#[derive(Serialize)]
struct JsonSymbolChange {
    name: String,
    kind: String,
    change: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    lines_changed: Option<usize>,
    #[serde(skip_serializing_if = "Option::is_none")]
    range: Option<[usize; 2]>,
    #[serde(skip_serializing_if = "Option::is_none")]
    signature_changed: Option<bool>,
}

// ---------------------------------------------------------------------------
// Public API
// ---------------------------------------------------------------------------

/// Run the full diff pipeline, returning formatted output and whether changes were found.
pub fn run_diff(path: &str, opts: &DiffOptions) -> Result<(String, bool), CodehudError> {
    let start_path = Path::new(path);

    // Find repo root
    let search_dir = if start_path.is_file() {
        start_path.parent().unwrap_or(Path::new("."))
    } else {
        start_path
    };
    let root_str = git::repo_root(search_dir)?;
    let repo = Path::new(&root_str);

    // Verify ref
    if !opts.staged {
        git::verify_ref(repo, &opts.refspec)?;
    }

    // Get changed files
    let changes = if opts.staged {
        git::staged_files(repo)?
    } else {
        git::changed_files(repo, &opts.refspec)?
    };

    // Scope to the requested path (relative to repo root)
    let scope_prefix = if start_path.is_file() || start_path.is_dir() {
        // Make path relative to repo root
        let abs = std::fs::canonicalize(start_path)
            .unwrap_or_else(|_| start_path.to_path_buf());
        let repo_abs = std::fs::canonicalize(repo)
            .unwrap_or_else(|_| repo.to_path_buf());
        abs.strip_prefix(&repo_abs)
            .map(|p| p.to_string_lossy().to_string())
            .unwrap_or_default()
    } else {
        String::new()
    };

    // Filter and diff each file
    let mut file_diffs: Vec<FileDiff> = Vec::new();

    for fc in &changes {
        // Scope filter
        if !scope_prefix.is_empty() && !fc.path.starts_with(&scope_prefix) {
            continue;
        }

        // Extension filter
        if !opts.ext.is_empty() {
            let file_ext = Path::new(&fc.path)
                .extension()
                .and_then(|e| e.to_str())
                .unwrap_or("");
            if !opts.ext.iter().any(|e| e == file_ext) {
                continue;
            }
        }

        // Exclude filter
        if opts.exclude.iter().any(|pat| {
            fc.path.contains(pat) || glob_match(&fc.path, pat)
        }) {
            continue;
        }

        // Test file filter
        if opts.no_tests && crate::test_detect::is_test_file_any_language(Path::new(&fc.path)) {
            continue;
        }

        // Only diff files we can parse
        let file_path = repo.join(&fc.path);
        let can_parse = languages::is_supported_file(&file_path);
        if !can_parse {
            continue;
        }

        let language = match languages::detect_language(&file_path) {
            Ok(l) => l,
            Err(_) => continue,
        };

        // Get old and new source
        let old_source = match &fc.status {
            ChangeStatus::Added => None,
            ChangeStatus::Renamed(old_path) => {
                git::file_at_ref(repo, &opts.refspec, old_path).ok()
            }
            _ => git::file_at_ref(repo, &opts.refspec, &fc.path).ok(),
        };

        let new_source = match &fc.status {
            ChangeStatus::Deleted => None,
            _ => std::fs::read_to_string(&file_path).ok(),
        };

        let mut changes = diff::diff_symbols_tolerant(
            old_source.as_deref(),
            new_source.as_deref(),
            language,
        );

        // Apply symbol-level filters
        changes.retain(|c| filter_change(c, opts));

        if !changes.is_empty() {
            file_diffs.push(FileDiff {
                path: fc.path.clone(),
                changes,
            });
        }
    }

    let has_changes = !file_diffs.is_empty();

    let output = if opts.json {
        format_json(&file_diffs)
    } else {
        format_plain(&file_diffs)
    };

    Ok((output, has_changes))
}

// ---------------------------------------------------------------------------
// Filtering
// ---------------------------------------------------------------------------

fn filter_change(change: &SymbolChange, opts: &DiffOptions) -> bool {
    let info = match change {
        SymbolChange::Added(s) | SymbolChange::Deleted(s) => s,
        SymbolChange::Modified { new, .. } => new,
    };

    // Kind filters
    if opts.fns_only || opts.types_only {
        let is_fn = matches!(info.kind, ItemKind::Function | ItemKind::Method);
        let is_type = matches!(
            info.kind,
            ItemKind::Struct | ItemKind::Enum | ItemKind::Trait | ItemKind::TypeAlias | ItemKind::Class
        );
        if opts.fns_only && !is_fn {
            return false;
        }
        if opts.types_only && !is_type {
            return false;
        }
    }

    // Note: pub_only filtering would require visibility info from the extractor.
    // SymbolInfo doesn't currently carry visibility, so we skip it for now.
    // This could be enhanced later.

    true
}

// ---------------------------------------------------------------------------
// Formatting
// ---------------------------------------------------------------------------

fn format_plain(file_diffs: &[FileDiff]) -> String {
    if file_diffs.is_empty() {
        return String::from("No symbol changes found.\n");
    }

    let mut out = String::new();
    writeln!(out, "Modified symbols:").unwrap();

    for fd in file_diffs {
        writeln!(out, "  {}", fd.path).unwrap();
        for change in &fd.changes {
            match change {
                SymbolChange::Added(s) => {
                    writeln!(
                        out,
                        "    + {} (L{}-{}) — added",
                        s.qualified_name, s.line_start, s.line_end
                    )
                    .unwrap();
                }
                SymbolChange::Deleted(s) => {
                    writeln!(out, "    - {} — deleted", s.qualified_name).unwrap();
                }
                SymbolChange::Modified {
                    old,
                    new,
                    signature_changed,
                } => {
                    let lines_changed = lines_diff(old, new);
                    let detail = if *signature_changed {
                        "signature changed".to_string()
                    } else {
                        format!("{} lines changed", lines_changed)
                    };
                    writeln!(
                        out,
                        "    ~ {} (L{}-{}) — {}",
                        new.qualified_name, new.line_start, new.line_end, detail
                    )
                    .unwrap();
                }
            }
        }
        writeln!(out).unwrap();
    }

    out
}

fn format_json(file_diffs: &[FileDiff]) -> String {
    let json_files: Vec<JsonFileDiff> = file_diffs
        .iter()
        .map(|fd| JsonFileDiff {
            file: fd.path.clone(),
            symbols: fd
                .changes
                .iter()
                .map(|c| match c {
                    SymbolChange::Added(s) => JsonSymbolChange {
                        name: s.qualified_name.clone(),
                        kind: kind_str(&s.kind).to_string(),
                        change: "added".into(),
                        lines_changed: None,
                        range: Some([s.line_start, s.line_end]),
                        signature_changed: None,
                    },
                    SymbolChange::Deleted(s) => JsonSymbolChange {
                        name: s.qualified_name.clone(),
                        kind: kind_str(&s.kind).to_string(),
                        change: "deleted".into(),
                        lines_changed: None,
                        range: None,
                        signature_changed: None,
                    },
                    SymbolChange::Modified {
                        old,
                        new,
                        signature_changed,
                    } => JsonSymbolChange {
                        name: new.qualified_name.clone(),
                        kind: kind_str(&new.kind).to_string(),
                        change: "modified".into(),
                        lines_changed: Some(lines_diff(old, new)),
                        range: Some([new.line_start, new.line_end]),
                        signature_changed: Some(*signature_changed),
                    },
                })
                .collect(),
        })
        .collect();

    serde_json::to_string_pretty(&json_files).unwrap_or_else(|_| "[]".into())
}

fn kind_str(kind: &ItemKind) -> &'static str {
    match kind {
        ItemKind::Function => "fn",
        ItemKind::Method => "method",
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
    }
}

fn lines_diff(old: &SymbolInfo, new: &SymbolInfo) -> usize {
    let old_len = old.line_end.saturating_sub(old.line_start) + 1;
    let new_len = new.line_end.saturating_sub(new.line_start) + 1;
    if new_len > old_len {
        new_len - old_len
    } else {
        old_len - new_len
    }
    .max(1) // at least 1 line changed if body differs
}

fn glob_match(path: &str, pattern: &str) -> bool {
    // Simple glob: support * as wildcard
    if pattern.contains('*') {
        let parts: Vec<&str> = pattern.split('*').collect();
        if parts.len() == 2 {
            let (prefix, suffix) = (parts[0], parts[1]);
            return path.contains(prefix) && path.ends_with(suffix)
                || (prefix.is_empty() && path.ends_with(suffix))
                || (suffix.is_empty() && path.starts_with(prefix));
        }
    }
    path.contains(pattern)
}
