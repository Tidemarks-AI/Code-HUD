//! Git integration layer for `--diff`.
//!
//! Shells out to `git` — no libgit2 dependency.

use std::path::Path;
use std::process::Command;

use crate::error::CodehudError;

// ---------------------------------------------------------------------------
// Types
// ---------------------------------------------------------------------------

/// How a file changed between two refs.
#[derive(Debug, Clone, PartialEq)]
pub enum ChangeStatus {
    Added,
    Modified,
    Deleted,
    /// Renamed from the given old path.
    Renamed(String),
}

/// A single file that changed.
#[derive(Debug, Clone)]
pub struct FileChange {
    pub path: String,
    pub status: ChangeStatus,
}

// ---------------------------------------------------------------------------
// Helpers
// ---------------------------------------------------------------------------

fn git(repo: &Path, args: &[&str]) -> Result<String, CodehudError> {
    let output = Command::new("git")
        .args(["-C", &repo.display().to_string()])
        .args(args)
        .output()
        .map_err(|e| CodehudError::ParseError(format!("failed to run git: {e}")))?;

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        return Err(CodehudError::ParseError(format!(
            "git error: {}",
            stderr.trim()
        )));
    }
    Ok(String::from_utf8_lossy(&output.stdout).into_owned())
}

fn parse_name_status(raw: &str) -> Vec<FileChange> {
    let mut changes = Vec::new();
    for line in raw.lines() {
        let line = line.trim();
        if line.is_empty() {
            continue;
        }
        let mut parts = line.split('\t');
        let status_str = match parts.next() {
            Some(s) => s,
            None => continue,
        };
        let path = match parts.next() {
            Some(p) => p.to_string(),
            None => continue,
        };

        let status = if status_str == "A" {
            ChangeStatus::Added
        } else if status_str == "M" {
            ChangeStatus::Modified
        } else if status_str == "D" {
            ChangeStatus::Deleted
        } else if status_str.starts_with('R') {
            // R100\told_path\tnew_path
            let new_path = parts.next().unwrap_or(&path).to_string();
            let old_path = path;
            changes.push(FileChange {
                path: new_path,
                status: ChangeStatus::Renamed(old_path),
            });
            continue;
        } else {
            // Copy or other — treat as modified
            ChangeStatus::Modified
        };

        changes.push(FileChange { path, status });
    }
    changes
}

// ---------------------------------------------------------------------------
// Public API
// ---------------------------------------------------------------------------

/// Auto-detect the repository root.
pub fn repo_root(start: &Path) -> Result<String, CodehudError> {
    let out = git(start, &["rev-parse", "--show-toplevel"])?;
    Ok(out.trim().to_string())
}

/// Verify a ref exists.
pub fn verify_ref(repo: &Path, refspec: &str) -> Result<(), CodehudError> {
    git(repo, &["rev-parse", "--verify", refspec])?;
    Ok(())
}

/// List files changed between `refspec` and the working tree.
pub fn changed_files(repo: &Path, refspec: &str) -> Result<Vec<FileChange>, CodehudError> {
    let out = git(repo, &["diff", "--name-status", refspec])?;
    Ok(parse_name_status(&out))
}

/// List staged (cached) file changes.
pub fn staged_files(repo: &Path) -> Result<Vec<FileChange>, CodehudError> {
    let out = git(repo, &["diff", "--cached", "--name-status"])?;
    Ok(parse_name_status(&out))
}

/// Retrieve file content at a specific ref.
pub fn file_at_ref(repo: &Path, refspec: &str, file_path: &str) -> Result<String, CodehudError> {
    git(repo, &["show", &format!("{refspec}:{file_path}")])
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use tempfile::TempDir;

    /// Create a minimal git repo and return its TempDir.
    fn init_repo() -> TempDir {
        let dir = TempDir::new().unwrap();
        let p = dir.path();
        git(p, &["init"]).unwrap();
        git(p, &["config", "user.email", "test@test.com"]).unwrap();
        git(p, &["config", "user.name", "Test"]).unwrap();
        // initial commit
        fs::write(p.join("init.txt"), "init").unwrap();
        git(p, &["add", "."]).unwrap();
        git(p, &["commit", "-m", "init"]).unwrap();
        dir
    }

    #[test]
    fn test_repo_root() {
        let dir = init_repo();
        let root = repo_root(dir.path()).unwrap();
        assert!(root.len() > 0);
    }

    #[test]
    fn test_verify_ref_valid() {
        let dir = init_repo();
        verify_ref(dir.path(), "HEAD").unwrap();
    }

    #[test]
    fn test_verify_ref_invalid() {
        let dir = init_repo();
        assert!(verify_ref(dir.path(), "nonexistent-ref-12345").is_err());
    }

    #[test]
    fn test_changed_files_added() {
        let dir = init_repo();
        let p = dir.path();
        fs::write(p.join("new.txt"), "hello").unwrap();
        git(p, &["add", "new.txt"]).unwrap();
        git(p, &["commit", "-m", "add new"]).unwrap();
        let changes = changed_files(p, "HEAD~1").unwrap();
        assert_eq!(changes.len(), 1);
        assert_eq!(changes[0].path, "new.txt");
        assert_eq!(changes[0].status, ChangeStatus::Added);
    }

    #[test]
    fn test_changed_files_modified() {
        let dir = init_repo();
        let p = dir.path();
        fs::write(p.join("init.txt"), "changed").unwrap();
        git(p, &["add", "."]).unwrap();
        git(p, &["commit", "-m", "modify"]).unwrap();
        let changes = changed_files(p, "HEAD~1").unwrap();
        assert_eq!(changes.len(), 1);
        assert_eq!(changes[0].status, ChangeStatus::Modified);
    }

    #[test]
    fn test_changed_files_deleted() {
        let dir = init_repo();
        let p = dir.path();
        fs::remove_file(p.join("init.txt")).unwrap();
        git(p, &["add", "."]).unwrap();
        git(p, &["commit", "-m", "delete"]).unwrap();
        let changes = changed_files(p, "HEAD~1").unwrap();
        assert_eq!(changes.len(), 1);
        assert_eq!(changes[0].status, ChangeStatus::Deleted);
    }

    #[test]
    fn test_file_at_ref() {
        let dir = init_repo();
        let p = dir.path();
        let content = file_at_ref(p, "HEAD", "init.txt").unwrap();
        assert_eq!(content, "init");
    }

    #[test]
    fn test_staged_files() {
        let dir = init_repo();
        let p = dir.path();
        fs::write(p.join("staged.txt"), "staging").unwrap();
        git(p, &["add", "staged.txt"]).unwrap();
        let changes = staged_files(p).unwrap();
        assert_eq!(changes.len(), 1);
        assert_eq!(changes[0].path, "staged.txt");
        assert_eq!(changes[0].status, ChangeStatus::Added);
    }

    #[test]
    fn test_parse_name_status_rename() {
        let raw = "R100\told.txt\tnew.txt\n";
        let changes = parse_name_status(raw);
        assert_eq!(changes.len(), 1);
        assert_eq!(changes[0].path, "new.txt");
        assert!(matches!(changes[0].status, ChangeStatus::Renamed(ref old) if old == "old.txt"));
    }
}
