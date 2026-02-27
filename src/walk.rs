use crate::error::CodehudError;
use crate::languages;
use globset::{Glob, GlobSetBuilder};
use ignore::WalkBuilder;
use std::collections::HashSet;
use std::path::{Path, PathBuf};

const LARGE_REPO_THRESHOLD: usize = 1000;

/// Emit a stderr warning if file count exceeds the large repo threshold
/// and stderr is a TTY (not piped).
pub fn warn_if_large_repo(file_count: usize) {
    use std::io::IsTerminal;
    if file_count > LARGE_REPO_THRESHOLD && std::io::stderr().is_terminal() {
        eprintln!(
            "⚠️  Large repo detected ({file_count} files). Tip: Use -d 2, --limit N, or --compact to reduce output"
        );
    }
}

/// Filter out paths matching any of the exclude glob patterns.
///
/// Patterns are matched against relative paths (relative to `base`).
/// A pattern without `*` or `/` is treated as matching any path component
/// (e.g. `dist` matches `packages/foo/dist/index.js`).
pub fn filter_excludes(files: Vec<PathBuf>, base: &Path, exclude: &[String]) -> Vec<PathBuf> {
    if exclude.is_empty() {
        return files;
    }

    let mut builder = GlobSetBuilder::new();
    for pattern in exclude {
        // If pattern has no glob chars or path separators, wrap as **/{pattern}/**
        // to match as a directory component, plus **/{pattern} for leaf matches
        let patterns = if !pattern.contains('*') && !pattern.contains('/') && !pattern.contains('?') {
            vec![format!("**/{pattern}"), format!("**/{pattern}/**")]
        } else {
            vec![pattern.clone()]
        };
        for p in patterns {
            if let Ok(glob) = Glob::new(&p) {
                builder.add(glob);
            }
        }
    }

    let globset = match builder.build() {
        Ok(gs) => gs,
        Err(_) => return files,
    };

    files
        .into_iter()
        .filter(|f| {
            let rel = f.strip_prefix(base).unwrap_or(f);
            let rel_str = rel.to_string_lossy();
            !globset.is_match(rel_str.as_ref())
        })
        .collect()
}
use std::sync::Mutex;

/// Inner source directories within a package/crate.
const INNER_SOURCE_DIRS: &[&str] = &["src", "lib"];

/// Detect if a directory is a monorepo root by checking for workspace indicators.
/// Returns a list of source root directories found.
pub fn detect_monorepo_source_roots(root: &Path) -> Vec<PathBuf> {
    let mut source_roots = Vec::new();

    // Check for Node.js/pnpm workspaces
    let pkg_json = root.join("package.json");
    if pkg_json.is_file()
        && let Ok(content) = std::fs::read_to_string(&pkg_json)
            && content.contains("\"workspaces\"") {
                collect_glob_source_roots(root, &["packages", "apps", "libs", "modules", "services"], &mut source_roots);
            }

    // Check pnpm-workspace.yaml
    if root.join("pnpm-workspace.yaml").is_file() || root.join("pnpm-workspace.yml").is_file() {
        collect_glob_source_roots(root, &["packages", "apps", "libs", "modules", "services"], &mut source_roots);
    }

    // Check Cargo.toml workspace
    let cargo_toml = root.join("Cargo.toml");
    if cargo_toml.is_file()
        && let Ok(content) = std::fs::read_to_string(&cargo_toml)
            && content.contains("[workspace]") {
                collect_glob_source_roots(root, &["crates", "packages", "libs", "modules"], &mut source_roots);
            }

    // Check go.work
    if root.join("go.work").is_file() {
        collect_glob_source_roots(root, &["cmd", "internal", "pkg", "services"], &mut source_roots);
    }

    // Check lerna.json
    if root.join("lerna.json").is_file() {
        collect_glob_source_roots(root, &["packages", "apps", "libs", "modules"], &mut source_roots);
    }

    source_roots
}

/// Scan for actual source root directories that exist under the given parent dirs.
/// A "source root" is a directory like packages/foo/src/ or crates/bar/src/.
fn collect_glob_source_roots(root: &Path, parent_dirs: &[&str], out: &mut Vec<PathBuf>) {
    for parent in parent_dirs {
        let parent_path = root.join(parent);
        if !parent_path.is_dir() {
            continue;
        }
        // Each child of the parent dir is a package/crate
        if let Ok(entries) = std::fs::read_dir(&parent_path) {
            for entry in entries.flatten() {
                let pkg_path = entry.path();
                if !pkg_path.is_dir() {
                    continue;
                }
                // Check for inner source dirs (src/, lib/)
                let mut found_inner = false;
                for inner in INNER_SOURCE_DIRS {
                    let inner_path = pkg_path.join(inner);
                    if inner_path.is_dir() {
                        out.push(inner_path);
                        found_inner = true;
                    }
                }
                // If no inner src dir, the package dir itself is a source root
                if !found_inner {
                    out.push(pkg_path);
                }
            }
        }
    }
}

/// Walk a directory with smart depth awareness for monorepos.
///
/// When `smart_depth` is true and `max_depth` is set, the walker will:
/// 1. Walk the root at the specified depth (normal behavior)
/// 2. Additionally detect monorepo source roots and walk into those
///    with the full depth budget, regardless of their nesting level
///
/// This means `--depth 1 --smart-depth` at a monorepo root will find
/// root configs AND source files inside packages/foo/src/.
pub fn walk_directory_smart(
    path: &Path,
    max_depth: Option<usize>,
    ext_filter: &[String],
    smart_depth: bool,
) -> Result<Vec<PathBuf>, CodehudError> {
    if !smart_depth || max_depth.is_none() {
        return walk_directory(path, max_depth, ext_filter);
    }

    let depth = max_depth.unwrap();

    // First: normal walk at requested depth
    let mut files = walk_directory(path, Some(depth), ext_filter)?;
    let existing: HashSet<PathBuf> = files.iter().cloned().collect();

    // Second: detect monorepo source roots and walk each with the user's depth
    let source_roots = detect_monorepo_source_roots(path);
    for source_root in &source_roots {
        if !source_root.is_dir() {
            continue;
        }
        let extra = walk_directory(source_root, Some(depth), ext_filter)?;
        for f in extra {
            if !existing.contains(&f) {
                files.push(f);
            }
        }
    }

    // Re-sort for consistent output
    files.sort();
    Ok(files)
}

/// Walk a directory and collect all supported source files.
/// Respects .gitignore, .ignore, and global gitignore rules.
pub fn walk_directory(path: &Path, max_depth: Option<usize>, ext_filter: &[String]) -> Result<Vec<PathBuf>, CodehudError> {
    walk_directory_inner(path, max_depth, ext_filter, true)
}

/// Walk a directory using parallel traversal (unsorted). Much faster for large repos.
pub fn walk_directory_parallel(path: &Path, max_depth: Option<usize>, ext_filter: &[String]) -> Result<Vec<PathBuf>, CodehudError> {
    walk_directory_inner(path, max_depth, ext_filter, false)
}

fn walk_directory_inner(path: &Path, max_depth: Option<usize>, ext_filter: &[String], sorted: bool) -> Result<Vec<PathBuf>, CodehudError> {
    // Verify path exists and is readable before walking
    if !path.is_dir() {
        return Err(CodehudError::ReadError {
            path: path.display().to_string(),
            source: std::io::Error::new(std::io::ErrorKind::NotFound, "not a directory"),
        });
    }

    let mut builder = WalkBuilder::new(path);
    builder
        .hidden(true)          // skip hidden files/dirs
        .git_ignore(true)      // respect .gitignore
        .git_global(true)      // respect global gitignore
        .git_exclude(true);    // respect .git/info/exclude

    if sorted {
        builder.sort_by_file_path(|a, b| a.cmp(b));
    } else {
        builder.threads(rayon::current_num_threads().min(12));
    }

    // The `ignore` crate's max_depth includes the root directory itself,
    // so depth=1 means root + one level. Our API defines depth as levels
    // *below* root (depth=0 → root only, depth=1 → root + one sub-level),
    // which maps to ignore's max_depth = depth + 1.
    if let Some(d) = max_depth {
        builder.max_depth(Some(d + 1));
    }

    if !sorted {
        // Use parallel walker
        let files = Mutex::new(Vec::new());
        let ext_filter_owned: Vec<String> = ext_filter.to_vec();
        builder.build_parallel().run(|| {
            let files = &files;
            let ext_filter = &ext_filter_owned;
            Box::new(move |entry| {
                let entry = match entry {
                    Ok(e) => e,
                    Err(_) => return ignore::WalkState::Continue,
                };
                let entry_path = entry.path();
                if entry_path.is_file() && languages::is_text_file(entry_path) {
                    if !ext_filter.is_empty() {
                        if let Some(ext) = entry_path.extension().and_then(|e| e.to_str()) {
                            if !ext_filter.iter().any(|f| f == ext) {
                                return ignore::WalkState::Continue;
                            }
                        } else {
                            return ignore::WalkState::Continue;
                        }
                    }
                    files.lock().unwrap().push(entry_path.to_path_buf());
                }
                ignore::WalkState::Continue
            })
        });
        return Ok(files.into_inner().unwrap());
    }

    let mut files = Vec::new();
    for entry in builder.build() {
        let entry = entry.map_err(|e| CodehudError::ReadError {
            path: path.display().to_string(),
            source: std::io::Error::other(e.to_string()),
        })?;

        let entry_path = entry.path();
        if entry_path.is_file() && languages::is_text_file(entry_path) {
            if !ext_filter.is_empty() {
                if let Some(ext) = entry_path.extension().and_then(|e| e.to_str()) {
                    if !ext_filter.iter().any(|f| f == ext) {
                        continue;
                    }
                } else {
                    continue;
                }
            }
            files.push(entry_path.to_path_buf());
        }
    }

    Ok(files)
}


#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use tempfile::TempDir;

    #[test]
    fn large_repo_threshold_is_1000() {
        assert_eq!(LARGE_REPO_THRESHOLD, 1000);
    }

    #[test]
    fn warn_if_large_repo_does_not_panic() {
        // Should not panic for any value; in CI (non-TTY) it silently does nothing
        warn_if_large_repo(0);
        warn_if_large_repo(999);
        warn_if_large_repo(1000);
        warn_if_large_repo(1001);
        warn_if_large_repo(100_000);
    }

    #[test]
    fn walk_empty_directory() {
        let dir = TempDir::new().unwrap();
        let files = walk_directory(dir.path(), None, &[]).unwrap();
        assert!(files.is_empty());
    }

    #[test]
    fn walk_finds_rs_files() {
        let dir = TempDir::new().unwrap();
        fs::write(dir.path().join("main.rs"), "fn main() {}").unwrap();
        fs::write(dir.path().join("readme.md"), "# hi").unwrap();
        let files = walk_directory(dir.path(), None, &[]).unwrap();
        // Both .rs and .md files are now included (passthrough support)
        assert_eq!(files.len(), 2);
        assert!(files.iter().any(|f| f.ends_with("main.rs")));
        assert!(files.iter().any(|f| f.ends_with("readme.md")));
    }

    #[test]
    fn walk_recurses_into_subdirs() {
        let dir = TempDir::new().unwrap();
        fs::create_dir(dir.path().join("sub")).unwrap();
        fs::write(dir.path().join("sub/lib.rs"), "").unwrap();
        let files = walk_directory(dir.path(), None, &[]).unwrap();
        assert_eq!(files.len(), 1);
    }

    #[test]
    fn walk_depth_limit_zero() {
        let dir = TempDir::new().unwrap();
        fs::write(dir.path().join("main.rs"), "").unwrap();
        fs::create_dir(dir.path().join("sub")).unwrap();
        fs::write(dir.path().join("sub/lib.rs"), "").unwrap();
        let files = walk_directory(dir.path(), Some(0), &[]).unwrap();
        assert_eq!(files.len(), 1);
        assert!(files[0].ends_with("main.rs"));
    }

    #[test]
    fn walk_depth_limit_one() {
        let dir = TempDir::new().unwrap();
        fs::write(dir.path().join("top.rs"), "").unwrap();
        fs::create_dir(dir.path().join("sub")).unwrap();
        fs::write(dir.path().join("sub/nested.rs"), "").unwrap();
        let files = walk_directory(dir.path(), Some(1), &[]).unwrap();
        assert_eq!(files.len(), 2);
    }

    #[test]
    fn walk_sorted_output() {
        let dir = TempDir::new().unwrap();
        fs::write(dir.path().join("z.rs"), "").unwrap();
        fs::write(dir.path().join("a.rs"), "").unwrap();
        let files = walk_directory(dir.path(), None, &[]).unwrap();
        assert!(files[0] < files[1]);
    }

    #[test]
    fn walk_nonexistent_dir() {
        let result = walk_directory(Path::new("/nonexistent_dir_xyz"), None, &[]);
        assert!(result.is_err());
    }

    #[test]
    fn walk_respects_gitignore() {
        let dir = TempDir::new().unwrap();
        // Init a git repo so .gitignore is respected
        fs::create_dir(dir.path().join(".git")).unwrap();
        fs::write(dir.path().join(".gitignore"), "ignored/\n").unwrap();
        fs::write(dir.path().join("keep.rs"), "").unwrap();
        fs::create_dir(dir.path().join("ignored")).unwrap();
        fs::write(dir.path().join("ignored/skip.rs"), "").unwrap();
        let files = walk_directory(dir.path(), None, &[]).unwrap();
        assert_eq!(files.len(), 1);
        assert!(files[0].ends_with("keep.rs"));
    }

    #[test]
    fn walk_skips_hidden_dirs() {
        let dir = TempDir::new().unwrap();
        fs::write(dir.path().join("visible.rs"), "").unwrap();
        fs::create_dir(dir.path().join(".hidden")).unwrap();
        fs::write(dir.path().join(".hidden/secret.rs"), "").unwrap();
        let files = walk_directory(dir.path(), None, &[]).unwrap();
        assert_eq!(files.len(), 1);
        assert!(files[0].ends_with("visible.rs"));
    }

    #[test]
    fn walk_ext_filter_rs_only() {
        let dir = TempDir::new().unwrap();
        fs::write(dir.path().join("main.rs"), "fn main() {}").unwrap();
        fs::write(dir.path().join("lib.ts"), "export {}").unwrap();
        let exts = vec!["rs".to_string()];
        let files = walk_directory(dir.path(), None, &exts).unwrap();
        assert_eq!(files.len(), 1);
        assert!(files[0].ends_with("main.rs"));
    }

    #[test]
    fn walk_ext_filter_multiple() {
        let dir = TempDir::new().unwrap();
        fs::write(dir.path().join("main.rs"), "fn main() {}").unwrap();
        fs::write(dir.path().join("app.ts"), "export {}").unwrap();
        fs::write(dir.path().join("comp.tsx"), "export {}").unwrap();
        let exts = vec!["rs".to_string(), "tsx".to_string()];
        let files = walk_directory(dir.path(), None, &exts).unwrap();
        assert_eq!(files.len(), 2);
    }

    #[test]
    fn walk_ext_filter_empty_means_all() {
        let dir = TempDir::new().unwrap();
        fs::write(dir.path().join("main.rs"), "fn main() {}").unwrap();
        fs::write(dir.path().join("app.ts"), "export {}").unwrap();
        let files = walk_directory(dir.path(), None, &[]).unwrap();
        assert_eq!(files.len(), 2);
    }

    // --- Smart depth tests ---

    #[test]
    fn smart_depth_disabled_same_as_normal() {
        let dir = TempDir::new().unwrap();
        fs::write(dir.path().join("root.rs"), "").unwrap();
        fs::create_dir_all(dir.path().join("packages/foo/src")).unwrap();
        fs::write(dir.path().join("packages/foo/src/lib.rs"), "").unwrap();
        let normal = walk_directory(dir.path(), Some(0), &[]).unwrap();
        let smart = walk_directory_smart(dir.path(), Some(0), &[], false).unwrap();
        assert_eq!(normal, smart);
    }

    #[test]
    fn smart_depth_detects_node_monorepo() {
        let dir = TempDir::new().unwrap();
        // Root config
        fs::write(dir.path().join("package.json"), r#"{"workspaces": ["packages/*"]}"#).unwrap();
        fs::write(dir.path().join("tsconfig.json"), "{}").unwrap();
        // Deep source file
        fs::create_dir_all(dir.path().join("packages/core/src")).unwrap();
        fs::write(dir.path().join("packages/core/src/index.ts"), "export {}").unwrap();

        // Without smart depth, depth 0 only finds root files
        let normal = walk_directory(dir.path(), Some(0), &[]).unwrap();
        assert!(!normal.iter().any(|f| f.to_string_lossy().contains("index.ts")));

        // With smart depth, depth 1 finds root files AND source files in detected roots
        let smart = walk_directory_smart(dir.path(), Some(1), &[], true).unwrap();
        assert!(smart.iter().any(|f| f.to_string_lossy().contains("index.ts")));
        assert!(smart.iter().any(|f| f.to_string_lossy().contains("package.json")));
    }

    #[test]
    fn smart_depth_detects_rust_workspace() {
        let dir = TempDir::new().unwrap();
        fs::write(dir.path().join("Cargo.toml"), "[workspace]\nmembers = [\"crates/*\"]").unwrap();
        fs::create_dir_all(dir.path().join("crates/core/src")).unwrap();
        fs::write(dir.path().join("crates/core/src/lib.rs"), "").unwrap();

        let smart = walk_directory_smart(dir.path(), Some(1), &[], true).unwrap();
        assert!(smart.iter().any(|f| f.to_string_lossy().contains("lib.rs")));
        assert!(smart.iter().any(|f| f.to_string_lossy().contains("Cargo.toml")));
    }

    #[test]
    fn smart_depth_no_monorepo_indicator_same_as_normal() {
        let dir = TempDir::new().unwrap();
        fs::write(dir.path().join("main.rs"), "").unwrap();
        fs::create_dir_all(dir.path().join("packages/foo/src")).unwrap();
        fs::write(dir.path().join("packages/foo/src/lib.rs"), "").unwrap();

        // No package.json with workspaces, so smart depth shouldn't expand
        let smart = walk_directory_smart(dir.path(), Some(0), &[], true).unwrap();
        assert_eq!(smart.len(), 1);
        assert!(smart[0].to_string_lossy().contains("main.rs"));
    }

    #[test]
    fn smart_depth_no_duplicates() {
        let dir = TempDir::new().unwrap();
        fs::write(dir.path().join("package.json"), r#"{"workspaces": ["packages/*"]}"#).unwrap();
        fs::create_dir_all(dir.path().join("packages/foo/src")).unwrap();
        fs::write(dir.path().join("packages/foo/src/lib.rs"), "").unwrap();

        // With unlimited normal depth, files would already be found
        let smart = walk_directory_smart(dir.path(), None, &[], true).unwrap();
        // Should have no duplicates
        let unique: HashSet<_> = smart.iter().collect();
        assert_eq!(smart.len(), unique.len());
    }

    #[test]
    fn smart_depth_with_ext_filter() {
        let dir = TempDir::new().unwrap();
        fs::write(dir.path().join("package.json"), r#"{"workspaces": ["packages/*"]}"#).unwrap();
        fs::create_dir_all(dir.path().join("packages/core/src")).unwrap();
        fs::write(dir.path().join("packages/core/src/index.ts"), "export {}").unwrap();
        fs::write(dir.path().join("packages/core/src/style.css"), "body {}").unwrap();

        let smart = walk_directory_smart(dir.path(), Some(1), &["ts".to_string()], true).unwrap();
        assert!(smart.iter().any(|f| f.to_string_lossy().contains("index.ts")));
        assert!(!smart.iter().any(|f| f.to_string_lossy().contains("style.css")));
    }

    #[test]
    fn smart_depth_pnpm_workspace() {
        let dir = TempDir::new().unwrap();
        fs::write(dir.path().join("pnpm-workspace.yaml"), "packages:\n  - 'packages/*'").unwrap();
        fs::create_dir_all(dir.path().join("packages/ui/src")).unwrap();
        fs::write(dir.path().join("packages/ui/src/Button.tsx"), "export {}").unwrap();

        let smart = walk_directory_smart(dir.path(), Some(1), &[], true).unwrap();
        assert!(smart.iter().any(|f| f.to_string_lossy().contains("Button.tsx")));
    }

    #[test]
    fn smart_depth_go_work() {
        let dir = TempDir::new().unwrap();
        fs::write(dir.path().join("go.work"), "go 1.21\nuse ./cmd/app").unwrap();
        fs::create_dir_all(dir.path().join("cmd/app")).unwrap();
        fs::write(dir.path().join("cmd/app/main.go"), "package main").unwrap();

        // go.work triggers detection; cmd/ children don't have src/ so they're source roots themselves
        let smart = walk_directory_smart(dir.path(), Some(1), &[], true).unwrap();
        assert!(smart.iter().any(|f| f.to_string_lossy().contains("main.go")));
    }

    #[test]
    fn walk_finds_vue_files() {
        let dir = TempDir::new().unwrap();
        fs::write(dir.path().join("App.vue"), "<script>export default {}</script>").unwrap();
        fs::write(dir.path().join("main.ts"), "import App from './App.vue'").unwrap();
        let files = walk_directory(dir.path(), None, &[]).unwrap();
        assert_eq!(files.len(), 2);
        assert!(files.iter().any(|f| f.ends_with("App.vue")));
    }

    #[test]
    fn walk_finds_svelte_files() {
        let dir = TempDir::new().unwrap();
        fs::write(dir.path().join("Counter.svelte"), "<script>let count = 0</script>").unwrap();
        let files = walk_directory(dir.path(), None, &[]).unwrap();
        assert_eq!(files.len(), 1);
        assert!(files[0].ends_with("Counter.svelte"));
    }

    #[test]
    fn walk_finds_astro_files() {
        let dir = TempDir::new().unwrap();
        fs::write(dir.path().join("index.astro"), "---\nconst title = 'hi'\n---\n<h1>{title}</h1>").unwrap();
        let files = walk_directory(dir.path(), None, &[]).unwrap();
        assert_eq!(files.len(), 1);
        assert!(files[0].ends_with("index.astro"));
    }

    #[test]
    fn walk_ext_filter_vue() {
        let dir = TempDir::new().unwrap();
        fs::write(dir.path().join("App.vue"), "<script>export default {}</script>").unwrap();
        fs::write(dir.path().join("main.ts"), "import App from './App.vue'").unwrap();
        let exts = vec!["vue".to_string()];
        let files = walk_directory(dir.path(), None, &exts).unwrap();
        assert_eq!(files.len(), 1);
        assert!(files[0].ends_with("App.vue"));
    }

    #[test]
    fn walk_ext_filter_sfc_multiple() {
        let dir = TempDir::new().unwrap();
        fs::write(dir.path().join("App.vue"), "<script></script>").unwrap();
        fs::write(dir.path().join("Page.svelte"), "<script></script>").unwrap();
        fs::write(dir.path().join("Index.astro"), "---\n---").unwrap();
        fs::write(dir.path().join("main.ts"), "").unwrap();
        let exts = vec!["vue".to_string(), "svelte".to_string(), "astro".to_string()];
        let files = walk_directory(dir.path(), None, &exts).unwrap();
        assert_eq!(files.len(), 3);
    }

    #[test]
    fn detect_monorepo_source_roots_finds_inner_src() {
        let dir = TempDir::new().unwrap();
        fs::write(dir.path().join("package.json"), r#"{"workspaces": ["packages/*"]}"#).unwrap();
        fs::create_dir_all(dir.path().join("packages/foo/src")).unwrap();
        fs::create_dir_all(dir.path().join("packages/bar/lib")).unwrap();
        fs::create_dir_all(dir.path().join("packages/baz")).unwrap(); // no src/lib

        let roots = detect_monorepo_source_roots(dir.path());
        assert!(roots.iter().any(|r| r.ends_with("foo/src")));
        assert!(roots.iter().any(|r| r.ends_with("bar/lib")));
        assert!(roots.iter().any(|r| r.ends_with("packages/baz")));
    }
}
