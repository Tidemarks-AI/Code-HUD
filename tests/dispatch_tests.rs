use codehud::dispatch;
use codehud::handler::{self, ItemKind, Visibility, Language, ts_language};

const TS_SOURCE: &str = r#"
import { Something } from './somewhere';

export class MyClass {
    private secret: string;
    public name: string;

    constructor(name: string) {
        this.name = name;
    }

    public getStartNode(): string {
        return this.name;
    }

    private helperMethod(): void {
        // ...
    }
}

function localFunction(): number {
    return 42;
}

export interface MyInterface {
    id: number;
    getName(): string;
}

export type MyType = string | number;

const MY_CONST = "hello";

export enum Color {
    Red,
    Green,
    Blue,
}
"#;

fn parse_ts(source: &str) -> tree_sitter::Tree {
    let lang = ts_language(Language::TypeScript);
    let mut parser = tree_sitter::Parser::new();
    parser.set_language(&lang).unwrap();
    parser.parse(source, None).unwrap()
}

fn get_handler() -> Box<dyn handler::LanguageHandler> {
    handler::handler_for(Language::TypeScript).unwrap()
}

#[test]
fn list_symbols_top_level() {
    let tree = parse_ts(TS_SOURCE);
    let handler = get_handler();
    let items = dispatch::list_symbols(TS_SOURCE, &tree, handler.as_ref(), Language::TypeScript, 1);

    let names: Vec<Option<&str>> = items.iter().map(|i| i.name.as_deref()).collect();
    assert!(names.contains(&Some("MyClass")), "should contain MyClass, got: {:?}", names);
    assert!(names.contains(&Some("localFunction")), "should contain localFunction, got: {:?}", names);
    assert!(names.contains(&Some("MyInterface")), "should contain MyInterface, got: {:?}", names);
    assert!(names.contains(&Some("MyType")), "should contain MyType, got: {:?}", names);
    assert!(names.contains(&Some("MY_CONST")), "should contain MY_CONST, got: {:?}", names);
    assert!(names.contains(&Some("Color")), "should contain Color, got: {:?}", names);

    // Imports should be excluded
    assert!(!items.iter().any(|i| i.kind == ItemKind::Use), "imports should be excluded");
}

#[test]
fn list_symbols_visibility() {
    let tree = parse_ts(TS_SOURCE);
    let handler = get_handler();
    let items = dispatch::list_symbols(TS_SOURCE, &tree, handler.as_ref(), Language::TypeScript, 1);

    let my_class = items.iter().find(|i| i.name.as_deref() == Some("MyClass")).unwrap();
    assert_eq!(my_class.visibility, Visibility::Public);

    let local_fn = items.iter().find(|i| i.name.as_deref() == Some("localFunction")).unwrap();
    assert_eq!(local_fn.visibility, Visibility::Private);
}

#[test]
fn list_symbols_depth2_includes_children() {
    let tree = parse_ts(TS_SOURCE);
    let handler = get_handler();
    let items = dispatch::list_symbols(TS_SOURCE, &tree, handler.as_ref(), Language::TypeScript, 2);

    let names: Vec<Option<&str>> = items.iter().map(|i| i.name.as_deref()).collect();
    // Should include child methods
    assert!(names.contains(&Some("getStartNode")), "depth 2 should include getStartNode, got: {:?}", names);
    assert!(names.contains(&Some("helperMethod")), "depth 2 should include helperMethod, got: {:?}", names);
    // Interface property members
    assert!(names.contains(&Some("id")), "depth 2 should include interface property 'id', got: {:?}", names);
}

#[test]
fn expand_symbol_class() {
    let tree = parse_ts(TS_SOURCE);
    let handler = get_handler();
    let result = dispatch::expand_symbol(TS_SOURCE, &tree, handler.as_ref(), Language::TypeScript, "MyClass");
    assert!(result.is_some());
    let items = result.unwrap();
    assert_eq!(items.len(), 1);
    assert_eq!(items[0].kind, ItemKind::Class);
    assert!(items[0].content.contains("getStartNode"));
}

#[test]
fn expand_symbol_qualified_method() {
    let tree = parse_ts(TS_SOURCE);
    let handler = get_handler();
    let result = dispatch::expand_symbol(TS_SOURCE, &tree, handler.as_ref(), Language::TypeScript, "MyClass.getStartNode");
    assert!(result.is_some());
    let items = result.unwrap();
    assert_eq!(items.len(), 1);
    assert_eq!(items[0].kind, ItemKind::Method);
    assert!(items[0].content.contains("getStartNode"));
    assert!(!items[0].content.contains("helperMethod"), "should only contain the method, not the whole class");
}

#[test]
fn expand_symbol_nonexistent_member() {
    let tree = parse_ts(TS_SOURCE);
    let handler = get_handler();
    let result = dispatch::expand_symbol(TS_SOURCE, &tree, handler.as_ref(), Language::TypeScript, "MyClass.nonExistent");
    assert!(result.is_none());
}

#[test]
fn expand_symbol_nonexistent_root() {
    let tree = parse_ts(TS_SOURCE);
    let handler = get_handler();
    let result = dispatch::expand_symbol(TS_SOURCE, &tree, handler.as_ref(), Language::TypeScript, "NoSuchClass");
    assert!(result.is_none());
}

#[test]
fn find_unqualified_member_finds_method() {
    let tree = parse_ts(TS_SOURCE);
    let handler = get_handler();
    let results = dispatch::find_unqualified_member(TS_SOURCE, &tree, handler.as_ref(), Language::TypeScript, "getStartNode");
    assert_eq!(results.len(), 1);
    assert_eq!(results[0].kind, ItemKind::Method);
    assert!(results[0].content.contains("getStartNode"));
}

#[test]
fn find_unqualified_member_not_found() {
    let tree = parse_ts(TS_SOURCE);
    let handler = get_handler();
    let results = dispatch::find_unqualified_member(TS_SOURCE, &tree, handler.as_ref(), Language::TypeScript, "nonExistent");
    assert!(results.is_empty());
}

#[test]
fn expand_symbol_double_colon_notation() {
    let tree = parse_ts(TS_SOURCE);
    let handler = get_handler();
    let result = dispatch::expand_symbol(TS_SOURCE, &tree, handler.as_ref(), Language::TypeScript, "MyClass::getStartNode");
    assert!(result.is_some());
    assert_eq!(result.unwrap()[0].kind, ItemKind::Method);
}
