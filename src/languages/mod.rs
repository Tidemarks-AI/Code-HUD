pub mod cpp;
pub mod go;
pub mod java;
pub mod rust;
pub mod typescript;
pub mod python;
pub mod javascript;

use crate::error::CodehudError;
use std::path::Path;

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum Language {
    Rust,
    TypeScript,
    Tsx,
    Python,
    JavaScript,
    Jsx,
    Java,
    Go,
    Cpp,
}

/// Registration entry for a language variant.
struct LangEntry {
    lang: Language,
    extensions: &'static [&'static str],
    uses_braces: bool,
    ts_language_fn: fn() -> tree_sitter::Language,
}

/// Single declarative table of all supported languages.
/// To add a new language, add one entry here (plus the handler/query modules).
static LANG_TABLE: &[LangEntry] = &[
    LangEntry {
        lang: Language::Rust,
        extensions: &["rs"],
        uses_braces: true,
        ts_language_fn: || tree_sitter_rust::LANGUAGE.into(),
    },
    LangEntry {
        lang: Language::TypeScript,
        extensions: &["ts"],
        uses_braces: true,
        ts_language_fn: || tree_sitter_typescript::LANGUAGE_TYPESCRIPT.into(),
    },
    LangEntry {
        lang: Language::Tsx,
        extensions: &["tsx"],
        uses_braces: true,
        ts_language_fn: || tree_sitter_typescript::LANGUAGE_TSX.into(),
    },
    LangEntry {
        lang: Language::JavaScript,
        extensions: &["js"],
        uses_braces: true,
        ts_language_fn: || tree_sitter_javascript::LANGUAGE.into(),
    },
    LangEntry {
        lang: Language::Jsx,
        extensions: &["jsx"],
        uses_braces: true,
        ts_language_fn: || tree_sitter_javascript::LANGUAGE.into(),
    },
    LangEntry {
        lang: Language::Python,
        extensions: &["py"],
        uses_braces: false,
        ts_language_fn: || tree_sitter_python::LANGUAGE.into(),
    },
    LangEntry {
        lang: Language::Java,
        extensions: &["java"],
        uses_braces: true,
        ts_language_fn: || tree_sitter_java::LANGUAGE.into(),
    },
    LangEntry {
        lang: Language::Go,
        extensions: &["go"],
        uses_braces: true,
        ts_language_fn: || tree_sitter_go::LANGUAGE.into(),
    },
    LangEntry {
        lang: Language::Cpp,
        extensions: &["cpp", "cc", "cxx", "hpp", "h"],
        uses_braces: true,
        ts_language_fn: || tree_sitter_cpp::LANGUAGE.into(),
    },
];

fn find_entry(lang: Language) -> &'static LangEntry {
    LANG_TABLE.iter().find(|e| e.lang == lang).expect("Language not registered")
}

impl Language {
    /// Returns true for languages that use braces `{ }` for blocks (Rust, JS, TS, C, etc.).
    /// Returns false for indentation-based languages (Python).
    pub fn uses_braces_for_blocks(self) -> bool {
        find_entry(self).uses_braces
    }
}

/// Detect language from file extension
pub fn detect_language(path: &Path) -> Result<Language, CodehudError> {
    let extension = path
        .extension()
        .and_then(|e| e.to_str())
        .ok_or_else(|| CodehudError::NoExtension(path.display().to_string()))?;

    for entry in LANG_TABLE {
        if entry.extensions.contains(&extension) {
            return Ok(entry.lang);
        }
    }
    Err(CodehudError::UnsupportedExtension(extension.to_string()))
}

/// Check if a file has a supported Tree-sitter language based on its extension
pub fn is_supported_file(path: &Path) -> bool {
    path.extension()
        .and_then(|e| e.to_str())
        .map(|ext| LANG_TABLE.iter().any(|e| e.extensions.contains(&ext)))
        .unwrap_or(false)
}

/// Check if a file is a text file worth including in passthrough mode.
/// Returns true for supported languages AND common text file extensions.
pub fn is_text_file(path: &Path) -> bool {
    if is_supported_file(path) || crate::sfc::is_sfc_file(path) {
        return true;
    }
    // Common text file extensions for passthrough
    const TEXT_EXTENSIONS: &[&str] = &[
        "toml", "yaml", "yml", "json", "md", "txt", "csv", "xml", "html", "css",
        "scss", "less", "sql", "sh", "bash", "zsh", "fish", "bat", "ps1",
        "env", "ini", "cfg", "conf", "config", "properties",
        "dockerfile", "dockerignore", "gitignore", "gitattributes",
        "editorconfig", "prettierrc", "eslintrc",
        "lock", "log", "diff", "patch",
        "c", "h", "cpp", "hpp", "cc", "java", "go", "rb", "php", "swift",
        "kt", "kts", "scala", "r", "lua", "pl", "pm", "ex", "exs",
        "hs", "ml", "mli", "clj", "cljs", "edn", "elm", "erl", "hrl",
        "vim", "makefile", "cmake", "gradle", "sbt",
        "tf", "tfvars", "hcl", "nix",
        "graphql", "gql", "proto", "thrift", "avsc",
    ];
    let ext = path.extension()
        .and_then(|e| e.to_str())
        .map(|e| e.to_lowercase());
    match ext {
        Some(e) => TEXT_EXTENSIONS.contains(&e.as_str()),
        None => {
            // Files without extension: check known filenames
            let name = path.file_name().and_then(|n| n.to_str()).unwrap_or("");
            let lower = name.to_lowercase();
            matches!(lower.as_str(),
                "makefile" | "dockerfile" | "vagrantfile" | "gemfile" |
                "rakefile" | "procfile" | "brewfile" | "justfile" |
                ".env" | ".gitignore" | ".gitattributes" | ".editorconfig" |
                ".dockerignore" | ".prettierrc" | ".eslintrc" |
                "license" | "readme" | "changelog" | "authors" | "contributors"
            )
        }
    }
}

/// Get tree-sitter Language for a given language enum
pub fn ts_language(lang: Language) -> tree_sitter::Language {
    (find_entry(lang).ts_language_fn)()
}


#[cfg(test)]
mod tests {
    use super::*;
    use std::path::Path;

    #[test]
    fn detect_language_rust() {
        let lang = detect_language(Path::new("foo.rs")).unwrap();
        assert_eq!(lang, Language::Rust);
    }

    #[test]
    fn detect_language_unsupported() {
        let err = detect_language(Path::new("foo.rb")).unwrap_err();
        assert!(err.to_string().contains("Unsupported"));
    }

    #[test]
    fn detect_language_no_extension() {
        let err = detect_language(Path::new("Makefile")).unwrap_err();
        assert!(err.to_string().contains("No file extension"));
    }

    #[test]
    fn detect_language_nested_path() {
        let lang = detect_language(Path::new("/a/b/c/main.rs")).unwrap();
        assert_eq!(lang, Language::Rust);
    }

    #[test]
    fn is_supported_file_rs() {
        assert!(is_supported_file(Path::new("lib.rs")));
        assert!(is_supported_file(Path::new("/deep/path/mod.rs")));
    }

    #[test]
    fn is_supported_file_not_rs() {
        assert!(!is_supported_file(Path::new("main.rb")));
        assert!(!is_supported_file(Path::new("README.md")));
        assert!(!is_supported_file(Path::new("Makefile")));
        assert!(!is_supported_file(Path::new(".hidden")));
        assert!(is_supported_file(Path::new("app.ts")));
        assert!(is_supported_file(Path::new("component.tsx")));
    }

    #[test]
    fn is_supported_file_no_extension() {
        assert!(!is_supported_file(Path::new("noext")));
    }
}
