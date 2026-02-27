use crate::extractor::{Item, ItemKind};

use crate::languages::Language;
use std::path::Path;

/// Per-language test detection: file-level (path matching) and block-level (item matching).
pub trait TestDetector {
    /// Returns true if the file path indicates a test file.
    fn is_test_file(&self, path: &Path) -> bool;

    /// Returns true if the item represents a test block (e.g., test module, test function).
    fn is_test_item(&self, item: &Item) -> bool;
}

/// Get a test detector for the given language.
pub fn detector_for(lang: Language) -> Box<dyn TestDetector> {
    match lang {
        Language::Rust => Box::new(RustTestDetector),
        Language::TypeScript | Language::Tsx => Box::new(JsTsTestDetector),
        Language::JavaScript | Language::Jsx => Box::new(JsTsTestDetector),
        Language::Python => Box::new(PythonTestDetector),
        Language::Java => Box::new(JavaTestDetector),
        Language::Go => Box::new(GoTestDetector),
        Language::Cpp => Box::new(CppTestDetector),
        Language::CSharp => Box::new(CSharpTestDetector),
        Language::Kotlin => Box::new(KotlinTestDetector),
    }
}

struct RustTestDetector;
impl TestDetector for RustTestDetector {
    fn is_test_file(&self, _path: &Path) -> bool {
        false // Rust uses inline test modules, not test files
    }
    fn is_test_item(&self, item: &Item) -> bool {
        if matches!(item.kind, ItemKind::Mod) && item.name.as_deref() == Some("tests") {
            return true;
        }
        if matches!(item.kind, ItemKind::Function | ItemKind::Method)
            && (item.content.contains("#[test]") || item.content.contains("#[tokio::test]")) {
                return true;
            }
        false
    }
}

struct JsTsTestDetector;
impl TestDetector for JsTsTestDetector {
    fn is_test_file(&self, path: &Path) -> bool {
        let path_str = path.to_string_lossy();
        let stem = path.file_stem().and_then(|s| s.to_str()).unwrap_or("");
        has_test_dir_component(&path_str) || is_js_ts_test_filename(stem, "")
    }
    fn is_test_item(&self, item: &Item) -> bool {
        is_js_test_call(item)
    }
}

struct CppTestDetector;
impl TestDetector for CppTestDetector {
    fn is_test_file(&self, path: &Path) -> bool {
        let path_str = path.to_string_lossy();
        let stem = path.file_stem().and_then(|s| s.to_str()).unwrap_or("");
        has_test_dir_component(&path_str)
            || stem.ends_with("_test")
            || stem.starts_with("test_")
    }
    fn is_test_item(&self, _item: &Item) -> bool {
        false
    }
}

struct CSharpTestDetector;
impl TestDetector for CSharpTestDetector {
    fn is_test_file(&self, path: &Path) -> bool {
        let path_str = path.to_string_lossy();
        let stem = path.file_stem().and_then(|s| s.to_str()).unwrap_or("");
        has_test_dir_component(&path_str)
            || stem.ends_with("Tests")
            || stem.ends_with("Test")
            || stem.starts_with("Test")
    }
    fn is_test_item(&self, _item: &Item) -> bool {
        false
    }
}

struct KotlinTestDetector;
impl TestDetector for KotlinTestDetector {
    fn is_test_file(&self, path: &Path) -> bool {
        let path_str = path.to_string_lossy();
        let stem = path.file_stem().and_then(|s| s.to_str()).unwrap_or("");
        has_test_dir_component(&path_str)
            || stem.ends_with("Test")
            || stem.ends_with("Tests")
            || stem.ends_with("Spec")
    }
    fn is_test_item(&self, _item: &Item) -> bool {
        false
    }
}

struct PythonTestDetector;
impl TestDetector for PythonTestDetector {
    fn is_test_file(&self, path: &Path) -> bool {
        let path_str = path.to_string_lossy();
        let stem = path.file_stem().and_then(|s| s.to_str()).unwrap_or("");
        let file_name = path.file_name().and_then(|n| n.to_str()).unwrap_or("");
        has_test_dir_component(&path_str)
            || stem.starts_with("test_")
            || stem.ends_with("_test")
            || file_name == "conftest.py"
    }
    fn is_test_item(&self, item: &Item) -> bool {
        if matches!(item.kind, ItemKind::Function | ItemKind::Method)
            && let Some(ref name) = item.name
                && name.starts_with("test_") {
                    return true;
                }
        if matches!(item.kind, ItemKind::Class)
            && let Some(ref name) = item.name
                && name.starts_with("Test") {
                    return true;
                }
        false
    }
}

struct JavaTestDetector;
impl TestDetector for JavaTestDetector {
    fn is_test_file(&self, path: &Path) -> bool {
        let path_str = path.to_string_lossy();
        let stem = path.file_stem().and_then(|s| s.to_str()).unwrap_or("");
        has_test_dir_component(&path_str)
            || stem.starts_with("Test")
            || stem.ends_with("Test")
            || stem.ends_with("Tests")
            || stem.ends_with("IT")
    }
    fn is_test_item(&self, item: &Item) -> bool {
        // Java test methods are typically annotated with @Test — detected via name heuristics here
        if matches!(item.kind, ItemKind::Function | ItemKind::Method)
            && let Some(ref name) = item.name
                && (name.starts_with("test") || name.starts_with("should")) {
                    return true;
                }
        if matches!(item.kind, ItemKind::Class)
            && let Some(ref name) = item.name
                && (name.ends_with("Test") || name.ends_with("Tests")) {
                    return true;
                }
        false
    }
}

/// Check if a file path looks like a test file for *any* supported language.
pub fn is_test_file_any_language(path: &Path) -> bool {
    if let Ok(lang) = crate::languages::detect_language(path) {
        let det = detector_for(lang);
        return det.is_test_file(path);
    }

    let path_str = path.to_string_lossy();
    let file_name = path.file_name().and_then(|n| n.to_str()).unwrap_or("");
    let ext = path.extension().and_then(|e| e.to_str()).unwrap_or("");

    if has_test_dir_component(&path_str) {
        return true;
    }

    match ext {
        "go" => file_name.ends_with("_test.go"),
        _ => false,
    }
}

pub fn has_test_dir_component(path_str: &str) -> bool {
    path_str.contains("/__tests__/")
        || path_str.contains("/tests/")
        || path_str.starts_with("__tests__/")
        || path_str.starts_with("tests/")
}

pub fn is_js_ts_test_filename(stem: &str, _file_name: &str) -> bool {
    stem.ends_with(".test")
        || stem.ends_with(".spec")
        || stem.ends_with(".tests")
        || stem.ends_with(".specs")
}

/// Check if an item looks like a JS/TS test block: describe(), it(), test()
pub fn is_js_test_call(item: &Item) -> bool {
    if matches!(item.kind, ItemKind::Function)
        && let Some(ref name) = item.name {
            let n = name.as_str();
            return n == "describe" || n == "it" || n == "test"
                || n.starts_with("describe(") || n.starts_with("it(") || n.starts_with("test(");
        }
    false
}

struct GoTestDetector;
impl TestDetector for GoTestDetector {
    fn is_test_file(&self, path: &Path) -> bool {
        let file_name = path.file_name().and_then(|n| n.to_str()).unwrap_or("");
        file_name.ends_with("_test.go")
    }
    fn is_test_item(&self, item: &Item) -> bool {
        if matches!(item.kind, ItemKind::Function)
            && let Some(ref name) = item.name {
                return name.starts_with("Test") || name.starts_with("Benchmark") || name.starts_with("Example");
            }
        false
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::extractor::{Item, ItemKind, Visibility};
    use crate::languages::Language;

    fn make_item(kind: ItemKind, name: &str, content: &str) -> Item {
        Item {
            kind,
            name: Some(name.to_string()),
            visibility: Visibility::Public,
            line_start: 1,
            line_end: 1,
            content: content.to_string(),
            signature: None,
            body: None,
            line_mappings: None,
        }
    }

    #[test]
    fn rust_no_test_files() {
        let d = detector_for(Language::Rust);
        assert!(!d.is_test_file(Path::new("src/main.rs")));
        assert!(!d.is_test_file(Path::new("tests/integration.rs")));
    }

    #[test]
    fn ts_test_file_patterns() {
        let d = detector_for(Language::TypeScript);
        assert!(d.is_test_file(Path::new("src/utils.test.ts")));
        assert!(d.is_test_file(Path::new("src/utils.spec.ts")));
        assert!(d.is_test_file(Path::new("src/__tests__/foo.ts")));
        assert!(!d.is_test_file(Path::new("src/utils.ts")));
    }

    #[test]
    fn js_test_file_patterns() {
        let d = detector_for(Language::JavaScript);
        assert!(d.is_test_file(Path::new("src/utils.test.js")));
        assert!(d.is_test_file(Path::new("src/utils.spec.js")));
        assert!(d.is_test_file(Path::new("__tests__/foo.js")));
        assert!(!d.is_test_file(Path::new("src/index.js")));
    }

    #[test]
    fn python_test_file_patterns() {
        let d = detector_for(Language::Python);
        assert!(d.is_test_file(Path::new("test_main.py")));
        assert!(d.is_test_file(Path::new("main_test.py")));
        assert!(d.is_test_file(Path::new("tests/test_foo.py")));
        assert!(d.is_test_file(Path::new("conftest.py")));
        assert!(!d.is_test_file(Path::new("main.py")));
    }

    #[test]
    fn go_test_file_detection() {
        assert!(is_test_file_any_language(Path::new("main_test.go")));
        assert!(!is_test_file_any_language(Path::new("main.go")));
    }

    #[test]
    fn rust_test_mod() {
        let d = detector_for(Language::Rust);
        let item = make_item(ItemKind::Mod, "tests", "#[cfg(test)]\nmod tests { }");
        assert!(d.is_test_item(&item));
    }

    #[test]
    fn rust_test_fn() {
        let d = detector_for(Language::Rust);
        let item = make_item(ItemKind::Function, "test_foo", "#[test]\nfn test_foo() {}");
        assert!(d.is_test_item(&item));
    }

    #[test]
    fn rust_non_test_fn() {
        let d = detector_for(Language::Rust);
        let item = make_item(ItemKind::Function, "foo", "fn foo() {}");
        assert!(!d.is_test_item(&item));
    }

    #[test]
    fn ts_describe_block() {
        let d = detector_for(Language::TypeScript);
        let item = make_item(ItemKind::Function, "describe", "describe('suite', () => {})");
        assert!(d.is_test_item(&item));
    }

    #[test]
    fn ts_it_block() {
        let d = detector_for(Language::TypeScript);
        let item = make_item(ItemKind::Function, "it", "it('works', () => {})");
        assert!(d.is_test_item(&item));
    }

    #[test]
    fn ts_test_block() {
        let d = detector_for(Language::TypeScript);
        let item = make_item(ItemKind::Function, "test", "test('works', () => {})");
        assert!(d.is_test_item(&item));
    }

    #[test]
    fn ts_normal_fn() {
        let d = detector_for(Language::TypeScript);
        let item = make_item(ItemKind::Function, "calculate", "function calculate() {}");
        assert!(!d.is_test_item(&item));
    }

    #[test]
    fn python_test_function() {
        let d = detector_for(Language::Python);
        let item = make_item(ItemKind::Function, "test_login", "def test_login(): pass");
        assert!(d.is_test_item(&item));
    }

    #[test]
    fn python_test_class() {
        let d = detector_for(Language::Python);
        let item = make_item(ItemKind::Class, "TestLogin", "class TestLogin: pass");
        assert!(d.is_test_item(&item));
    }

    #[test]
    fn python_normal_function() {
        let d = detector_for(Language::Python);
        let item = make_item(ItemKind::Function, "login", "def login(): pass");
        assert!(!d.is_test_item(&item));
    }

    #[test]
    fn any_language_test_files() {
        assert!(is_test_file_any_language(Path::new("src/utils.test.ts")));
        assert!(is_test_file_any_language(Path::new("src/utils.spec.js")));
        assert!(is_test_file_any_language(Path::new("test_main.py")));
        assert!(is_test_file_any_language(Path::new("pkg/main_test.go")));
        assert!(is_test_file_any_language(Path::new("src/__tests__/foo.ts")));
        assert!(is_test_file_any_language(Path::new("tests/test_foo.py")));
    }

    #[test]
    fn any_language_non_test_files() {
        assert!(!is_test_file_any_language(Path::new("src/main.rs")));
        assert!(!is_test_file_any_language(Path::new("src/index.ts")));
        assert!(!is_test_file_any_language(Path::new("main.py")));
        assert!(!is_test_file_any_language(Path::new("main.go")));
    }
}
