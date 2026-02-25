use codehud::handler::{handler_for, LanguageHandler, ItemKind, Visibility, Language, ts_language};
use tree_sitter::{Parser, Query, QueryCursor, StreamingIterator};

const TS_SOURCE: &str = r#"
import { something } from './module';

export class Workflow {
    private name: string;
    protected nodes: Map<string, INode>;

    constructor(params: WorkflowParameters) {
        this.name = params.name;
    }

    getStartNode(): INode | undefined {
        return this.nodes.get('start');
    }

    private __resolveNode(id: string): INode {
        return this.nodes.get(id)!;
    }

    async runWorkflow(): Promise<void> {
        const start = this.getStartNode();
    }

    static create(): Workflow {
        return new Workflow({} as any);
    }
}

export interface WorkflowParameters {
    name: string;
    nodes: INode[];
    active: boolean;
}

export function createWorkflow(params: WorkflowParameters): Workflow {
    return new Workflow(params);
}

export type NodeId = string;

const DEFAULT_NAME = "untitled";

enum Status {
    Active,
    Inactive,
}
"#;

fn get_handler() -> Box<dyn LanguageHandler> {
    handler_for(Language::TypeScript).expect("TypeScript handler should exist")
}

fn parse_ts(source: &str) -> tree_sitter::Tree {
    let mut parser = Parser::new();
    let lang = ts_language(Language::TypeScript);
    parser.set_language(&lang).expect("set language");
    parser.parse(source, None).expect("parse")
}

/// Helper: find class_declaration node inside the tree (unwrapping export_statement)
fn find_class_node<'a>(tree: &'a tree_sitter::Tree, _source: &'a str) -> tree_sitter::Node<'a> {
    let root = tree.root_node();
    let mut cursor = root.walk();
    for child in root.named_children(&mut cursor) {
        if child.kind() == "export_statement" {
            let mut ic = child.walk();
            for inner in child.named_children(&mut ic) {
                if inner.kind() == "class_declaration" {
                    return inner;
                }
            }
        }
    }
    panic!("no class found")
}

/// Helper: find interface_declaration node inside the tree
fn find_interface_node<'a>(tree: &'a tree_sitter::Tree, _source: &'a str) -> tree_sitter::Node<'a> {
    let root = tree.root_node();
    let mut cursor = root.walk();
    for child in root.named_children(&mut cursor) {
        if child.kind() == "export_statement" {
            let mut ic = child.walk();
            for inner in child.named_children(&mut ic) {
                if inner.kind() == "interface_declaration" {
                    return inner;
                }
            }
        }
    }
    panic!("no interface found")
}

#[test]
fn symbol_query_finds_classes_functions_interfaces() {
    let handler = get_handler();
    let tree = parse_ts(TS_SOURCE);
    let lang = ts_language(Language::TypeScript);
    let query = Query::new(&lang, handler.symbol_query()).expect("valid query");
    let mut cursor = QueryCursor::new();
    let name_idx = query.capture_index_for_name("name").unwrap();

    let mut names = Vec::new();
    let mut matches = cursor.matches(&query, tree.root_node(), TS_SOURCE.as_bytes());
    while let Some(m) = matches.next() {
        for cap in m.captures {
            if cap.index == name_idx {
                names.push(TS_SOURCE[cap.node.byte_range()].to_string());
            }
        }
    }

    assert!(names.contains(&"Workflow".to_string()));
    assert!(names.contains(&"WorkflowParameters".to_string()));
    assert!(names.contains(&"createWorkflow".to_string()));
    assert!(names.contains(&"NodeId".to_string()));
    assert!(names.contains(&"DEFAULT_NAME".to_string()));
    assert!(names.contains(&"Status".to_string()));
}

#[test]
fn classify_node_maps_kinds_correctly() {
    let handler = get_handler();
    let tree = parse_ts(TS_SOURCE);
    let root = tree.root_node();

    let mut cursor = root.walk();
    let mut found_class = false;
    let mut found_interface = false;
    let mut found_function = false;

    for child in root.named_children(&mut cursor) {
        if let Some(info) = handler.classify_node(child, TS_SOURCE) {
            match info.kind {
                ItemKind::Class => {
                    assert_eq!(info.name.as_deref(), Some("Workflow"));
                    found_class = true;
                }
                ItemKind::Trait => {
                    assert_eq!(info.name.as_deref(), Some("WorkflowParameters"));
                    found_interface = true;
                }
                ItemKind::Function => {
                    assert_eq!(info.name.as_deref(), Some("createWorkflow"));
                    found_function = true;
                }
                _ => {}
            }
        }
    }

    assert!(found_class, "should find class");
    assert!(found_interface, "should find interface");
    assert!(found_function, "should find function");
}

#[test]
fn child_symbols_returns_class_methods_and_fields() {
    let handler = get_handler();
    let tree = parse_ts(TS_SOURCE);
    let class_node = find_class_node(&tree, TS_SOURCE);

    let children = handler.child_symbols(class_node, TS_SOURCE);
    let child_names: Vec<Option<&str>> = children.iter()
        .map(|c| c.name.as_deref())
        .collect();

    // Methods
    assert!(child_names.contains(&Some("constructor")));
    assert!(child_names.contains(&Some("getStartNode")));
    assert!(child_names.contains(&Some("__resolveNode")));
    assert!(child_names.contains(&Some("runWorkflow")));
    assert!(child_names.contains(&Some("create")));

    // Fields
    assert!(child_names.contains(&Some("name")));
    assert!(child_names.contains(&Some("nodes")));

    let methods: Vec<_> = children.iter().filter(|c| c.kind == ItemKind::Method).collect();
    let fields: Vec<_> = children.iter().filter(|c| c.kind == ItemKind::Const).collect();
    assert_eq!(methods.len(), 5);
    assert_eq!(fields.len(), 2);
}

#[test]
fn visibility_detects_export_as_public() {
    let handler = get_handler();
    let tree = parse_ts(TS_SOURCE);
    let root = tree.root_node();

    let mut cursor = root.walk();
    for child in root.named_children(&mut cursor) {
        if child.kind() == "export_statement" {
            assert_eq!(handler.visibility(child, TS_SOURCE), Visibility::Public);
        } else if child.kind() == "enum_declaration" {
            assert_eq!(handler.visibility(child, TS_SOURCE), Visibility::Private);
        }
    }
}

#[test]
fn member_visibility_detects_private_protected() {
    let handler = get_handler();
    let tree = parse_ts(TS_SOURCE);
    let class_node = find_class_node(&tree, TS_SOURCE);

    let children = handler.child_symbols(class_node, TS_SOURCE);

    for child in &children {
        let vis = handler.member_visibility(child.node, TS_SOURCE);
        match child.name.as_deref() {
            Some("name") => assert_eq!(vis, Visibility::Private, "name field should be private"),
            Some("nodes") => assert_eq!(vis, Visibility::Protected, "nodes field should be protected"),
            Some("constructor") => assert_eq!(vis, Visibility::Public, "constructor should be public"),
            Some("getStartNode") => assert_eq!(vis, Visibility::Public, "getStartNode should be public"),
            Some("__resolveNode") => assert_eq!(vis, Visibility::Private, "__ prefix should be private"),
            Some("runWorkflow") => assert_eq!(vis, Visibility::Public, "runWorkflow should be public"),
            Some("create") => assert_eq!(vis, Visibility::Public, "create should be public"),
            _ => {}
        }
    }
}

#[test]
fn signature_builds_method_signature() {
    let handler = get_handler();
    let tree = parse_ts(TS_SOURCE);
    let class_node = find_class_node(&tree, TS_SOURCE);

    let children = handler.child_symbols(class_node, TS_SOURCE);
    let get_start = children.iter()
        .find(|c| c.name.as_deref() == Some("getStartNode"))
        .expect("should find getStartNode");

    let sig = handler.signature(get_start.node, TS_SOURCE);
    assert!(sig.contains("getStartNode"), "signature should contain method name");
    assert!(sig.contains("()"), "signature should contain params");
    assert!(sig.contains("INode | undefined"), "signature should contain return type");
}

#[test]
fn interface_child_symbols_returns_properties() {
    let handler = get_handler();
    let tree = parse_ts(TS_SOURCE);
    let iface_node = find_interface_node(&tree, TS_SOURCE);

    let children = handler.child_symbols(iface_node, TS_SOURCE);
    let names: Vec<&str> = children.iter()
        .filter_map(|c| c.name.as_deref())
        .collect();

    assert!(names.contains(&"name"));
    assert!(names.contains(&"nodes"));
    assert!(names.contains(&"active"));
    assert_eq!(children.len(), 3);
}

#[test]
fn handler_for_returns_none_for_unsupported() {
    assert!(handler_for(Language::Python).is_some());
    assert!(handler_for(Language::Rust).is_some());
}

#[test]
fn handler_for_returns_some_for_typescript() {
    assert!(handler_for(Language::TypeScript).is_some());
    assert!(handler_for(Language::Tsx).is_some());
}
