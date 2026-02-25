use codehud::editor::{self, EditResult};
use codehud::Language;

#[test]
fn test_symbol_line_range_simple() {
    let source = "fn foo() {\n    1\n}\n\nfn bar() {\n    2\n}\n";
    let (start, end) = editor::symbol_line_range(source, "foo", Language::Rust).unwrap();
    assert_eq!(start, 1);
    assert_eq!(end, 3);
    let (start, end) = editor::symbol_line_range(source, "bar", Language::Rust).unwrap();
    assert_eq!(start, 5);
    assert_eq!(end, 7);
}

#[test]
fn test_symbol_line_range_with_attributes() {
    let source = "#[inline]\n#[must_use]\nfn foo() -> i32 {\n    42\n}\n";
    let (start, end) = editor::symbol_line_range(source, "foo", Language::Rust).unwrap();
    assert_eq!(start, 1);
    assert_eq!(end, 5);
}

#[test]
fn test_symbol_line_range_not_found() {
    let source = "fn foo() {}\n";
    let result = editor::symbol_line_range(source, "nonexistent", Language::Rust);
    assert!(result.is_err());
}

#[test]
fn test_edit_result_serialization() {
    let result = EditResult {
        symbol: "foo".to_string(),
        action: "replaced".to_string(),
        line_start: 1,
        line_end: 3,
    };
    let json = serde_json::to_string(&result).unwrap();
    assert!(json.contains("symbol"));
    assert!(json.contains("foo"));
    assert!(json.contains("line_start"));
}
