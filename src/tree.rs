use crate::error::CodehudError;
use crate::languages;
use crate::walk;
use serde::Serialize;
use std::collections::BTreeMap;
use std::fmt::Write;
use std::fs;
use std::path::{Path, PathBuf};

#[derive(Debug, Clone, Serialize, serde::Deserialize)]
pub struct FileEntry {
    pub path: String,
    pub size: u64,
    pub extension: Option<String>,
    pub is_supported: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub symbols: Option<usize>,
}

pub struct TreeOptions {
    pub depth: Option<usize>,
    pub ext: Vec<String>,
    pub stats: bool,
    pub json: bool,
    pub smart_depth: bool,
    pub no_tests: bool,
    pub exclude: Vec<String>,
}

/// Flat file listing mode (--files)
pub fn list_files(dir: &str, opts: &TreeOptions) -> Result<String, CodehudError> {
    let path = Path::new(dir);
    if !path.exists() {
        return Err(CodehudError::PathNotFound(dir.to_string()));
    }
    if !path.is_dir() {
        return Err(CodehudError::InvalidPath(format!(
            "{} is not a directory",
            dir
        )));
    }

    let files = walk::walk_directory_smart(path, opts.depth, &opts.ext, opts.smart_depth)?;
    let files = walk::filter_excludes(files, path, &opts.exclude);
    let files = if opts.no_tests {
        files
            .into_iter()
            .filter(|f| !crate::test_detect::is_test_file_any_language(f))
            .collect()
    } else {
        files
    };
    let entries = build_entries(&files, path, opts.stats)?;

    if opts.json {
        Ok(serde_json::to_string_pretty(&entries)?)
    } else {
        let mut out = String::new();
        for e in &entries {
            if opts.stats {
                let size_str = format_size(e.size);
                if let Some(syms) = e.symbols {
                    writeln!(out, "{:>8}  {:>3} syms  {}", size_str, syms, e.path).unwrap();
                } else {
                    writeln!(out, "{:>8}           {}", size_str, e.path).unwrap();
                }
            } else {
                writeln!(out, "{}", e.path).unwrap();
            }
        }
        Ok(out)
    }
}

/// Tree view mode (--tree)
pub fn tree_view(dir: &str, opts: &TreeOptions) -> Result<String, CodehudError> {
    let path = Path::new(dir);
    if !path.exists() {
        return Err(CodehudError::PathNotFound(dir.to_string()));
    }
    if !path.is_dir() {
        return Err(CodehudError::InvalidPath(format!(
            "{} is not a directory",
            dir
        )));
    }

    let files = walk::walk_directory_smart(path, opts.depth, &opts.ext, opts.smart_depth)?;
    let files = walk::filter_excludes(files, path, &opts.exclude);
    let files = if opts.no_tests {
        files
            .into_iter()
            .filter(|f| !crate::test_detect::is_test_file_any_language(f))
            .collect()
    } else {
        files
    };

    if opts.json {
        let entries = build_entries(&files, path, opts.stats)?;
        return Ok(serde_json::to_string_pretty(&entries)?);
    }

    // Build a tree structure from flat file list
    let tree = build_tree(&files, path);
    let dir_name = path.file_name().and_then(|n| n.to_str()).unwrap_or(".");

    let mut out = String::new();
    writeln!(out, "{}/", dir_name).unwrap();
    render_tree(&tree, "", &mut out, path, opts.stats)?;

    // Summary line
    let (dir_count, file_count) = count_tree(&tree);
    writeln!(out, "\n{} directories, {} files", dir_count, file_count).unwrap();

    Ok(out)
}

fn build_entries(
    files: &[PathBuf],
    base: &Path,
    with_stats: bool,
) -> Result<Vec<FileEntry>, CodehudError> {
    let mut entries = Vec::with_capacity(files.len());
    for file in files {
        let rel = file.strip_prefix(base).unwrap_or(file);
        let size = fs::metadata(file).map(|m| m.len()).unwrap_or(0);
        let ext = file
            .extension()
            .and_then(|e| e.to_str())
            .map(|s| s.to_string());
        let is_supported = languages::is_supported_file(file);

        let symbols = if with_stats && is_supported {
            Some(count_symbols(file))
        } else {
            None
        };

        entries.push(FileEntry {
            path: rel.to_string_lossy().to_string(),
            size,
            extension: ext,
            is_supported,
            symbols,
        });
    }
    Ok(entries)
}

fn count_symbols(path: &Path) -> usize {
    let source = match fs::read_to_string(path) {
        Ok(s) => s,
        Err(_) => return 0,
    };
    let language = match languages::detect_language(path) {
        Ok(l) => l,
        Err(_) => return 0,
    };
    let tree = match crate::parser::parse(&source, language) {
        Ok(t) => t,
        Err(_) => return 0,
    };
    let handler = crate::handler::handler_for(language);
    if let Some(ref h) = handler {
        crate::dispatch::list_symbols(&source, &tree, h.as_ref(), language, 1).len()
    } else {
        0
    }
}

// Tree node for directory structure
enum TreeNode {
    Dir(BTreeMap<String, TreeNode>),
    File(PathBuf),
}

fn build_tree(files: &[PathBuf], base: &Path) -> BTreeMap<String, TreeNode> {
    let mut root: BTreeMap<String, TreeNode> = BTreeMap::new();
    for file in files {
        let rel = file.strip_prefix(base).unwrap_or(file);
        let components: Vec<&str> = rel.iter().filter_map(|c| c.to_str()).collect();
        insert_into_tree(&mut root, &components, file.clone());
    }
    root
}

fn insert_into_tree(
    node: &mut BTreeMap<String, TreeNode>,
    components: &[&str],
    full_path: PathBuf,
) {
    if components.is_empty() {
        return;
    }
    if components.len() == 1 {
        node.insert(components[0].to_string(), TreeNode::File(full_path));
    } else {
        let dir_name = components[0].to_string();
        let child = node
            .entry(dir_name)
            .or_insert_with(|| TreeNode::Dir(BTreeMap::new()));
        if let TreeNode::Dir(children) = child {
            insert_into_tree(children, &components[1..], full_path);
        }
    }
}

fn render_tree(
    tree: &BTreeMap<String, TreeNode>,
    prefix: &str,
    out: &mut String,
    _base: &Path,
    stats: bool,
) -> Result<(), CodehudError> {
    let entries: Vec<(&String, &TreeNode)> = tree.iter().collect();
    let len = entries.len();

    for (i, (name, node)) in entries.iter().enumerate() {
        let is_last = i == len - 1;
        let connector = if is_last { "└── " } else { "├── " };
        let child_prefix = if is_last {
            format!("{}    ", prefix)
        } else {
            format!("{}│   ", prefix)
        };

        match node {
            TreeNode::Dir(children) => {
                writeln!(out, "{}{}{}/", prefix, connector, name).unwrap();
                render_tree(children, &child_prefix, out, _base, stats)?;
            }
            TreeNode::File(path) => {
                if stats {
                    let size = fs::metadata(path).map(|m| m.len()).unwrap_or(0);
                    let size_str = format_size(size);
                    let is_supported = languages::is_supported_file(path);
                    if is_supported {
                        let syms = count_symbols(path);
                        writeln!(
                            out,
                            "{}{}{}  ({}, {} syms)",
                            prefix, connector, name, size_str, syms
                        )
                        .unwrap();
                    } else {
                        writeln!(out, "{}{}{}  ({})", prefix, connector, name, size_str).unwrap();
                    }
                } else {
                    writeln!(out, "{}{}{}", prefix, connector, name).unwrap();
                }
            }
        }
    }
    Ok(())
}

fn count_tree(tree: &BTreeMap<String, TreeNode>) -> (usize, usize) {
    let mut dirs = 0;
    let mut files = 0;
    for node in tree.values() {
        match node {
            TreeNode::Dir(children) => {
                dirs += 1;
                let (d, f) = count_tree(children);
                dirs += d;
                files += f;
            }
            TreeNode::File(_) => files += 1,
        }
    }
    (dirs, files)
}

fn format_size(bytes: u64) -> String {
    if bytes < 1024 {
        format!("{}B", bytes)
    } else if bytes < 1024 * 1024 {
        format!("{:.1}K", bytes as f64 / 1024.0)
    } else {
        format!("{:.1}M", bytes as f64 / (1024.0 * 1024.0))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use tempfile::TempDir;

    fn setup_test_dir() -> TempDir {
        let dir = TempDir::new().unwrap();
        fs::create_dir(dir.path().join(".git")).unwrap();
        fs::create_dir(dir.path().join("src")).unwrap();
        fs::write(
            dir.path().join("src/main.rs"),
            "fn main() {}\nfn helper() {}\n",
        )
        .unwrap();
        fs::write(dir.path().join("src/lib.rs"), "pub fn foo() {}\n").unwrap();
        fs::write(dir.path().join("README.md"), "# Hello\n").unwrap();
        fs::write(
            dir.path().join("Cargo.toml"),
            "[package]\nname = \"test\"\n",
        )
        .unwrap();
        dir
    }

    #[test]
    fn test_files_flat_listing() {
        let dir = setup_test_dir();
        let opts = TreeOptions {
            depth: None,
            ext: vec![],
            stats: false,
            json: false,
            smart_depth: false,
            no_tests: false,
            exclude: vec![],
        };
        let output = list_files(dir.path().to_str().unwrap(), &opts).unwrap();
        assert!(output.contains("src/main.rs"));
        assert!(output.contains("src/lib.rs"));
        assert!(output.contains("README.md"));
        // Each line should be a relative path
        for line in output.lines() {
            assert!(!line.starts_with('/'));
        }
    }

    #[test]
    fn test_files_ext_filter() {
        let dir = setup_test_dir();
        let opts = TreeOptions {
            depth: None,
            ext: vec!["rs".to_string()],
            stats: false,
            json: false,
            smart_depth: false,
            no_tests: false,
            exclude: vec![],
        };
        let output = list_files(dir.path().to_str().unwrap(), &opts).unwrap();
        assert!(output.contains("main.rs"));
        assert!(!output.contains("README.md"));
        assert!(!output.contains("Cargo.toml"));
    }

    #[test]
    fn test_files_depth_filter() {
        let dir = setup_test_dir();
        let opts = TreeOptions {
            depth: Some(0),
            ext: vec![],
            stats: false,
            json: false,
            smart_depth: false,
            no_tests: false,
            exclude: vec![],
        };
        let output = list_files(dir.path().to_str().unwrap(), &opts).unwrap();
        assert!(!output.contains("src/main.rs"));
        assert!(output.contains("README.md"));
    }

    #[test]
    fn test_files_json_output() {
        let dir = setup_test_dir();
        let opts = TreeOptions {
            depth: None,
            ext: vec!["rs".to_string()],
            stats: false,
            json: true,
            smart_depth: false,
            no_tests: false,
            exclude: vec![],
        };
        let output = list_files(dir.path().to_str().unwrap(), &opts).unwrap();
        let entries: Vec<FileEntry> = serde_json::from_str(&output).unwrap();
        assert!(entries.len() >= 2);
        assert!(entries.iter().all(|e| e.path.ends_with(".rs")));
        assert!(entries.iter().any(|e| e.is_supported));
    }

    #[test]
    fn test_files_stats() {
        let dir = setup_test_dir();
        let opts = TreeOptions {
            depth: None,
            ext: vec!["rs".to_string()],
            stats: true,
            json: false,
            smart_depth: false,
            no_tests: false,
            exclude: vec![],
        };
        let output = list_files(dir.path().to_str().unwrap(), &opts).unwrap();
        assert!(output.contains("syms"));
        assert!(output.contains("src/main.rs"));
    }

    #[test]
    fn test_files_stats_json() {
        let dir = setup_test_dir();
        let opts = TreeOptions {
            depth: None,
            ext: vec!["rs".to_string()],
            stats: true,
            json: true,
            smart_depth: false,
            no_tests: false,
            exclude: vec![],
        };
        let output = list_files(dir.path().to_str().unwrap(), &opts).unwrap();
        let entries: Vec<FileEntry> = serde_json::from_str(&output).unwrap();
        // With stats, supported files should have symbol counts
        let rs_entry = entries.iter().find(|e| e.path.contains("main.rs")).unwrap();
        assert!(rs_entry.symbols.is_some());
        assert!(rs_entry.symbols.unwrap() >= 2); // main + helper
    }

    #[test]
    fn test_tree_view() {
        let dir = setup_test_dir();
        let opts = TreeOptions {
            depth: None,
            ext: vec![],
            stats: false,
            json: false,
            smart_depth: false,
            no_tests: false,
            exclude: vec![],
        };
        let output = tree_view(dir.path().to_str().unwrap(), &opts).unwrap();
        // Should have tree drawing characters
        assert!(output.contains("├── ") || output.contains("└── "));
        assert!(output.contains("src/"));
        assert!(output.contains("main.rs"));
        assert!(output.contains("directories"));
        assert!(output.contains("files"));
    }

    #[test]
    fn test_tree_with_ext_filter() {
        let dir = setup_test_dir();
        let opts = TreeOptions {
            depth: None,
            ext: vec!["rs".to_string()],
            stats: false,
            json: false,
            smart_depth: false,
            no_tests: false,
            exclude: vec![],
        };
        let output = tree_view(dir.path().to_str().unwrap(), &opts).unwrap();
        assert!(output.contains("main.rs"));
        assert!(!output.contains("README.md"));
    }

    #[test]
    fn test_tree_with_stats() {
        let dir = setup_test_dir();
        let opts = TreeOptions {
            depth: None,
            ext: vec!["rs".to_string()],
            stats: true,
            json: false,
            smart_depth: false,
            no_tests: false,
            exclude: vec![],
        };
        let output = tree_view(dir.path().to_str().unwrap(), &opts).unwrap();
        assert!(output.contains("syms"));
    }

    #[test]
    fn test_tree_json_output() {
        let dir = setup_test_dir();
        let opts = TreeOptions {
            depth: None,
            ext: vec!["rs".to_string()],
            stats: false,
            json: true,
            smart_depth: false,
            no_tests: false,
            exclude: vec![],
        };
        let output = tree_view(dir.path().to_str().unwrap(), &opts).unwrap();
        let entries: Vec<FileEntry> = serde_json::from_str(&output).unwrap();
        assert!(!entries.is_empty());
    }

    #[test]
    fn test_files_nonexistent_dir() {
        let opts = TreeOptions {
            depth: None,
            ext: vec![],
            stats: false,
            json: false,
            smart_depth: false,
            no_tests: false,
            exclude: vec![],
        };
        let result = list_files("/nonexistent_xyz", &opts);
        assert!(result.is_err());
    }

    #[test]
    fn test_files_smart_depth_pnpm_monorepo() {
        let dir = tempfile::TempDir::new().unwrap();
        // Simulate pnpm monorepo
        fs::write(
            dir.path().join("pnpm-workspace.yaml"),
            "packages:\n  - 'packages/*'",
        )
        .unwrap();
        fs::write(dir.path().join("package.json"), r#"{"name": "root"}"#).unwrap();
        fs::create_dir_all(dir.path().join("packages/ui/src")).unwrap();
        fs::write(dir.path().join("packages/ui/src/Button.tsx"), "export {}").unwrap();
        fs::create_dir_all(dir.path().join("packages/core/src")).unwrap();
        fs::write(dir.path().join("packages/core/src/index.ts"), "export {}").unwrap();

        let dir_str = dir.path().to_str().unwrap();

        // Without smart_depth at depth 0, we should NOT see deep files
        let opts_no_smart = TreeOptions {
            depth: Some(0),
            ext: vec![],
            stats: false,
            json: false,
            smart_depth: false,
            no_tests: false,
            exclude: vec![],
        };
        let result = list_files(dir_str, &opts_no_smart).unwrap();
        assert!(
            !result.contains("Button.tsx"),
            "depth 0 without smart-depth should not find Button.tsx"
        );

        // With smart_depth at depth 0, we SHOULD see files inside source roots
        let opts_smart = TreeOptions {
            depth: Some(0),
            ext: vec![],
            stats: false,
            json: false,
            smart_depth: true,
            no_tests: false,
            exclude: vec![],
        };
        let result = list_files(dir_str, &opts_smart).unwrap();
        assert!(
            result.contains("Button.tsx"),
            "smart-depth should find Button.tsx in packages/ui/src/"
        );
        assert!(
            result.contains("index.ts"),
            "smart-depth should find index.ts in packages/core/src/"
        );
    }

    #[test]
    fn test_tree_smart_depth_pnpm_monorepo() {
        let dir = tempfile::TempDir::new().unwrap();
        fs::write(
            dir.path().join("pnpm-workspace.yaml"),
            "packages:\n  - 'packages/*'",
        )
        .unwrap();
        fs::create_dir_all(dir.path().join("packages/ui/src")).unwrap();
        fs::write(dir.path().join("packages/ui/src/Button.tsx"), "export {}").unwrap();

        let dir_str = dir.path().to_str().unwrap();

        let opts_smart = TreeOptions {
            depth: Some(0),
            ext: vec![],
            stats: false,
            json: false,
            smart_depth: true,
            no_tests: false,
            exclude: vec![],
        };
        let result = tree_view(dir_str, &opts_smart).unwrap();
        assert!(
            result.contains("Button.tsx"),
            "tree smart-depth should find Button.tsx"
        );
    }

    #[test]
    fn test_files_no_tests_excludes_test_files() {
        let dir = TempDir::new().unwrap();
        fs::create_dir(dir.path().join(".git")).unwrap();
        fs::create_dir_all(dir.path().join("src")).unwrap();
        fs::create_dir_all(dir.path().join("src/__tests__")).unwrap();
        fs::write(dir.path().join("src/index.ts"), "export {}").unwrap();
        fs::write(dir.path().join("src/utils.ts"), "export {}").unwrap();
        fs::write(dir.path().join("src/utils.test.ts"), "test()").unwrap();
        fs::write(dir.path().join("src/__tests__/foo.ts"), "test()").unwrap();

        let dir_str = dir.path().to_str().unwrap();

        // Without no_tests: all files present
        let opts = TreeOptions {
            depth: None,
            ext: vec![],
            stats: false,
            json: false,
            smart_depth: false,
            no_tests: false,
            exclude: vec![],
        };
        let result = list_files(dir_str, &opts).unwrap();
        assert!(result.contains("utils.test.ts"));
        assert!(result.contains("__tests__/foo.ts"));

        // With no_tests: test files excluded
        let opts = TreeOptions {
            depth: None,
            ext: vec![],
            stats: false,
            json: false,
            smart_depth: false,
            no_tests: true,
            exclude: vec![],
        };
        let result = list_files(dir_str, &opts).unwrap();
        assert!(result.contains("index.ts"));
        assert!(result.contains("src/utils.ts"));
        assert!(
            !result.contains("utils.test.ts"),
            "--no-tests should exclude .test.ts files"
        );
        assert!(
            !result.contains("__tests__"),
            "--no-tests should exclude __tests__ directory files"
        );
    }

    #[test]
    fn test_tree_no_tests_excludes_test_files() {
        let dir = TempDir::new().unwrap();
        fs::create_dir(dir.path().join(".git")).unwrap();
        fs::create_dir_all(dir.path().join("src/__tests__")).unwrap();
        fs::write(dir.path().join("src/index.ts"), "export {}").unwrap();
        fs::write(dir.path().join("src/index.test.ts"), "test()").unwrap();
        fs::write(dir.path().join("src/__tests__/bar.ts"), "test()").unwrap();

        let dir_str = dir.path().to_str().unwrap();

        let opts = TreeOptions {
            depth: None,
            ext: vec![],
            stats: false,
            json: false,
            smart_depth: false,
            no_tests: true,
            exclude: vec![],
        };
        let result = tree_view(dir_str, &opts).unwrap();
        assert!(result.contains("index.ts"));
        assert!(
            !result.contains("index.test.ts"),
            "--no-tests should exclude .test.ts in tree view"
        );
        assert!(
            !result.contains("__tests__"),
            "--no-tests should exclude __tests__ dir in tree view"
        );
    }
}
