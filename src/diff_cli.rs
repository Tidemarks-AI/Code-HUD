//! CLI orchestration for `--diff`: ties together git, diff, and output.

use std::path::Path;

use crate::diff::{self, FileDiff, SymbolChange};
use crate::error::CodehudError;
use crate::git::{self, ChangeStatus, FileChange};
use crate::languages;

// ---------------------------------------------------------------------------
// Options
// ---------------------------------------------------------------------------

/// Options for the diff command.
pub struct DiffOptions {
    pub refspec: Option<String>,
    pub staged: bool,
    pub path_scope: Option<String>,
    pub json: bool,
    pub pub_only: bool,
    pub fns_only: bool,
    pub types_only: bool,
    pub no_tests: bool,
    pub ext: Vec<String>,
    pub exclude: Vec<String>,
}

// ---------------------------------------------------------------------------
// Core
// ---------------------------------------------------------------------------

/// Run the diff and return formatted output.
pub fn run_diff(opts: &DiffOptions) -> Result<String, CodehudError> {
    // Determine the repo root from the path scope or cwd
    let start = match &opts.path_scope {
        Some(p) => {
            let pb = std::path::PathBuf::from(p);
            // git -C requires a directory; if the path is a file, use its parent
            if pb.is_file() {
                pb.parent()
                    .map(|d| d.to_path_buf())
                    .unwrap_or(pb)
            } else {
                pb
            }
        }
        None => std::env::current_dir()
            .map_err(|e| CodehudError::ParseError(format!("cannot get cwd: {e}")))?,
    };
    let root_str = git::repo_root(&start)?;
    let root = Path::new(&root_str);

    // Get changed files
    let changes = if opts.staged {
        git::staged_files(root)?
    } else {
        let refspec = opts.refspec.as_deref().unwrap_or("HEAD");
        git::verify_ref(root, refspec)?;
        git::changed_files(root, refspec)?
    };

    // Filter by path scope
    let changes = filter_by_scope(changes, &opts.path_scope, &root_str);

    // Filter by extension
    let changes = filter_by_ext(changes, &opts.ext);

    // Filter by exclude patterns
    let changes = filter_by_exclude(changes, &opts.exclude);

    // Diff each file
    let refspec_for_old = if opts.staged {
        "HEAD".to_string()
    } else {
        opts.refspec.clone().unwrap_or_else(|| "HEAD".to_string())
    };

    let mut file_diffs = Vec::new();
    for fc in &changes {
        let diff = diff_one_file(root, fc, &refspec_for_old, opts.staged)?;
        if let Some(mut fd) = diff {
            // Apply symbol-level filters
            if opts.no_tests {
                fd.changes.retain(|c| !is_test_symbol(c));
            }
            if opts.fns_only {
                fd.changes.retain(is_fn_symbol);
            }
            if opts.types_only {
                fd.changes.retain(is_type_symbol);
            }
            if opts.pub_only {
                // We don't track visibility in SymbolInfo currently — skip this filter
            }
            if !fd.changes.is_empty() {
                file_diffs.push(fd);
            }
        }
    }

    if opts.json {
        format_json(&file_diffs)
    } else {
        Ok(format_plain(&file_diffs))
    }
}

// ---------------------------------------------------------------------------
// Per-file diffing
// ---------------------------------------------------------------------------

/// Get file content from the staging area (index) using `git show :path`.
fn staged_file_content(root: &Path, file_path: &str) -> Option<String> {
    // git show :<path> — the colon prefix means "from the index"
    // We use file_at_ref with an empty string and prepend : to the path
    // Actually, git show :path works as a single argument
    use std::process::Command;
    let output = Command::new("git")
        .args(["-C", &root.display().to_string()])
        .args(["show", &format!(":{file_path}")])
        .output()
        .ok()?;
    if output.status.success() {
        Some(String::from_utf8_lossy(&output.stdout).into_owned())
    } else {
        None
    }
}

fn diff_one_file(
    root: &Path,
    fc: &FileChange,
    refspec: &str,
    staged: bool,
) -> Result<Option<FileDiff>, CodehudError> {
    // Only diff supported languages
    let file_path = Path::new(&fc.path);
    if !languages::is_supported_file(file_path) {
        return Ok(None);
    }
    let language = match languages::detect_language(file_path) {
        Ok(l) => l,
        Err(_) => return Ok(None),
    };

    let (old_src, new_src) = match &fc.status {
        ChangeStatus::Added => {
            let new = if staged {
                staged_file_content(root, &fc.path)
            } else {
                std::fs::read_to_string(root.join(&fc.path)).ok()
            };
            (None, new)
        }
        ChangeStatus::Deleted => {
            let old = git::file_at_ref(root, refspec, &fc.path).ok();
            (old, None)
        }
        ChangeStatus::Modified => {
            let old = git::file_at_ref(root, refspec, &fc.path).ok();
            let new = if staged {
                staged_file_content(root, &fc.path)
            } else {
                std::fs::read_to_string(root.join(&fc.path)).ok()
            };
            (old, new)
        }
        ChangeStatus::Renamed(old_path) => {
            let old = git::file_at_ref(root, refspec, old_path).ok();
            let new = if staged {
                staged_file_content(root, &fc.path)
            } else {
                std::fs::read_to_string(root.join(&fc.path)).ok()
            };
            (old, new)
        }
    };

    let changes = diff::diff_symbols_tolerant(
        old_src.as_deref(),
        new_src.as_deref(),
        language,
    );

    if changes.is_empty() {
        return Ok(None);
    }

    Ok(Some(FileDiff {
        path: fc.path.clone(),
        changes,
    }))
}

// ---------------------------------------------------------------------------
// Filters
// ---------------------------------------------------------------------------

fn filter_by_scope(changes: Vec<FileChange>, scope: &Option<String>, root: &str) -> Vec<FileChange> {
    let scope = match scope {
        Some(s) => s,
        None => return changes,
    };

    // Resolve scope to an absolute path, then make it relative to repo root
    let scope_abs = if Path::new(scope).is_absolute() {
        std::path::PathBuf::from(scope)
    } else {
        std::env::current_dir()
            .unwrap_or_default()
            .join(scope)
    };
    let scope_abs = scope_abs.canonicalize().unwrap_or(scope_abs);
    let root_path = Path::new(root);

    let scope_rel = match scope_abs.strip_prefix(root_path) {
        Ok(rel) => rel.to_string_lossy().to_string(),
        Err(_) => return changes, // scope outside repo → no filtering
    };
    let scope_rel = scope_rel.trim_end_matches('/');

    // If scope is the repo root itself (empty string or "."), no filtering needed
    if scope_rel.is_empty() || scope_rel == "." {
        return changes;
    }

    changes
        .into_iter()
        .filter(|fc| {
            fc.path.starts_with(scope_rel)
                || fc.path.starts_with(&format!("{scope_rel}/"))
                || fc.path == scope_rel
        })
        .collect()
}

fn filter_by_ext(changes: Vec<FileChange>, exts: &[String]) -> Vec<FileChange> {
    if exts.is_empty() {
        return changes;
    }
    changes
        .into_iter()
        .filter(|fc| {
            Path::new(&fc.path)
                .extension()
                .and_then(|e| e.to_str())
                .map(|e| exts.iter().any(|x| x == e))
                .unwrap_or(false)
        })
        .collect()
}

fn filter_by_exclude(changes: Vec<FileChange>, patterns: &[String]) -> Vec<FileChange> {
    if patterns.is_empty() {
        return changes;
    }
    changes
        .into_iter()
        .filter(|fc| !patterns.iter().any(|p| fc.path.contains(p)))
        .collect()
}

fn is_test_symbol(change: &SymbolChange) -> bool {
    let name = match change {
        SymbolChange::Added(s) => &s.name,
        SymbolChange::Deleted(s) => &s.name,
        SymbolChange::Modified { new, .. } => &new.name,
    };
    name.starts_with("test_") || name.starts_with("Test") || name.contains("_test")
}

fn is_fn_symbol(change: &SymbolChange) -> bool {
    use crate::extractor::ItemKind;
    let kind = match change {
        SymbolChange::Added(s) => &s.kind,
        SymbolChange::Deleted(s) => &s.kind,
        SymbolChange::Modified { new, .. } => &new.kind,
    };
    matches!(kind, ItemKind::Function | ItemKind::Method)
}

fn is_type_symbol(change: &SymbolChange) -> bool {
    use crate::extractor::ItemKind;
    let kind = match change {
        SymbolChange::Added(s) => &s.kind,
        SymbolChange::Deleted(s) => &s.kind,
        SymbolChange::Modified { new, .. } => &new.kind,
    };
    matches!(kind, ItemKind::Class | ItemKind::Struct | ItemKind::Enum | ItemKind::Trait)
}

// ---------------------------------------------------------------------------
// Output formatting
// ---------------------------------------------------------------------------

fn format_plain(diffs: &[FileDiff]) -> String {
    if diffs.is_empty() {
        return "No symbol changes detected.\n".to_string();
    }

    let mut out = String::from("Modified symbols:\n");
    for fd in diffs {
        out.push_str(&format!("  {}\n", fd.path));
        for change in &fd.changes {
            match change {
                SymbolChange::Added(s) => {
                    out.push_str(&format!(
                        "    + {} (L{}-{}) — added\n",
                        s.qualified_name, s.line_start, s.line_end
                    ));
                }
                SymbolChange::Deleted(s) => {
                    out.push_str(&format!(
                        "    - {} — deleted\n",
                        s.qualified_name
                    ));
                }
                SymbolChange::Modified { new, signature_changed, .. } => {
                    if *signature_changed {
                        out.push_str(&format!(
                            "    ~ {} (L{}-{}) — signature changed\n",
                            new.qualified_name, new.line_start, new.line_end
                        ));
                    } else {
                        out.push_str(&format!(
                            "    ~ {} (L{}-{}) — modified\n",
                            new.qualified_name, new.line_start, new.line_end
                        ));
                    }
                }
            }
        }
    }
    out
}

fn format_json(diffs: &[FileDiff]) -> Result<String, CodehudError> {
    let mut entries = Vec::new();
    for fd in diffs {
        for change in &fd.changes {
            let entry = match change {
                SymbolChange::Added(s) => serde_json::json!({
                    "file": fd.path,
                    "symbol": s.qualified_name,
                    "kind": format!("{:?}", s.kind),
                    "change_type": "added",
                    "new_range": [s.line_start, s.line_end],
                }),
                SymbolChange::Deleted(s) => serde_json::json!({
                    "file": fd.path,
                    "symbol": s.qualified_name,
                    "kind": format!("{:?}", s.kind),
                    "change_type": "deleted",
                    "old_range": [s.line_start, s.line_end],
                }),
                SymbolChange::Modified { old, new, signature_changed } => serde_json::json!({
                    "file": fd.path,
                    "symbol": new.qualified_name,
                    "kind": format!("{:?}", new.kind),
                    "change_type": if *signature_changed { "signature_changed" } else { "modified" },
                    "old_range": [old.line_start, old.line_end],
                    "new_range": [new.line_start, new.line_end],
                }),
            };
            entries.push(entry);
        }
    }
    serde_json::to_string_pretty(&entries)
        .map_err(|e| CodehudError::ParseError(format!("JSON serialization error: {e}")))
}
