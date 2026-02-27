use codehud::{process_path, ProcessOptions, OutputFormat};
use std::io::Write;
use tempfile::NamedTempFile;

fn opts() -> ProcessOptions {
    ProcessOptions {
        symbols: vec![],
        pub_only: false,
        fns_only: false,
        types_only: false,
        no_tests: false,
        depth: None,
        format: OutputFormat::Plain,
        stats: false, stats_detailed: true,
        ext: vec![],
        signatures: false,
        max_lines: None,
        list_symbols: false,
        no_imports: false,
        smart_depth: false,
        symbol_depth: None,
        exclude: vec![],
        outline: false,
        compact: false,
        minimal: false,
        expand_symbols: vec![],
    }

}

fn write_js(content: &str) -> NamedTempFile {
    let mut f = tempfile::Builder::new().suffix(".js").tempfile().unwrap();
    f.write_all(content.as_bytes()).unwrap();
    f.flush().unwrap();
    f
}

fn write_jsx(content: &str) -> NamedTempFile {
    let mut f = tempfile::Builder::new().suffix(".jsx").tempfile().unwrap();
    f.write_all(content.as_bytes()).unwrap();
    f.flush().unwrap();
    f
}

const SAMPLE_JS: &str = r#"import { EventEmitter } from "events";
import fs from "fs";

export function publicApi(input) {
    return input.trim().toLowerCase();
}

export class UserService {
    constructor() {
        this.db = new Map();
    }

    getUser(id) {
        return this.db.get(id);
    }

    async fetchUser(id) {
        const resp = await fetch(`/api/users/${id}`);
        return resp.json();
    }

    static defaultInstance() {
        return new UserService();
    }
}

export const MAX_USERS = 100;

export let currentUser = null;

var legacyFlag = true;

function helperFunction(x) {
    return x * 2;
}

const arrowFn = (a, b) => {
    return a + b;
};

let mutableVal = 42;
"#;

// --- Interface mode ---

#[test]
fn javascript_interface_mode_basic() {
    let f = write_js(SAMPLE_JS);
    let output = process_path(f.path().to_str().unwrap(), opts()).unwrap();

    assert!(output.contains("import"), "Missing import");
    assert!(output.contains("function publicApi"), "Missing publicApi");
    assert!(output.contains("class UserService"), "Missing class");
    assert!(output.contains("MAX_USERS"), "Missing const MAX_USERS");
    assert!(output.contains("currentUser"), "Missing let currentUser");
    assert!(output.contains("legacyFlag"), "Missing var legacyFlag");
    assert!(output.contains("function helperFunction"), "Missing helperFunction");
    assert!(output.contains("arrowFn"), "Missing arrowFn const");
    assert!(output.contains("mutableVal"), "Missing mutableVal");
    assert!(output.contains("{ ... }"), "Missing collapsed bodies");
}

// --- Expand mode ---

#[test]
fn javascript_expand_symbol() {
    let f = write_js(SAMPLE_JS);
    let mut o = opts();
    o.symbols = vec!["publicApi".to_string()];
    let output = process_path(f.path().to_str().unwrap(), o).unwrap();

    assert!(output.contains("function publicApi"), "Missing publicApi");
    assert!(output.contains("trim().toLowerCase()"), "Missing function body");
}

#[test]
fn javascript_expand_class() {
    let f = write_js(SAMPLE_JS);
    let mut o = opts();
    o.symbols = vec!["UserService".to_string()];
    let output = process_path(f.path().to_str().unwrap(), o).unwrap();

    assert!(output.contains("class UserService"), "Missing class");
    assert!(output.contains("new Map()") || output.contains("this.db"), "Missing class body");
}

// --- Class methods ---

#[test]
fn javascript_class_methods() {
    let f = write_js(SAMPLE_JS);
    let mut o = opts();
    o.symbols = vec!["UserService".to_string()];
    let output = process_path(f.path().to_str().unwrap(), o).unwrap();

    assert!(output.contains("constructor"), "Missing constructor");
    assert!(output.contains("getUser"), "Missing getUser method");
    assert!(output.contains("fetchUser"), "Missing fetchUser method");
    assert!(output.contains("defaultInstance"), "Missing static method");
}

// --- Async/static method signatures ---

#[test]
fn javascript_async_static_methods() {
    let src = r#"class Service {
    async loadData(url) {
        return await fetch(url);
    }

    static create() {
        return new Service();
    }
}
"#;
    let f = write_js(src);
    let mut o = opts();
    o.symbols = vec!["Service".to_string()];
    let output = process_path(f.path().to_str().unwrap(), o).unwrap();

    assert!(output.contains("async"), "Missing async keyword in method signature");
    assert!(output.contains("static"), "Missing static keyword in method signature");
}

// --- Export variants ---

#[test]
fn javascript_export_variants() {
    let f = write_js(SAMPLE_JS);
    let mut o = opts();
    o.pub_only = true;
    let output = process_path(f.path().to_str().unwrap(), o).unwrap();

    assert!(output.contains("function publicApi"), "Missing export function");
    assert!(output.contains("class UserService"), "Missing export class");
    assert!(output.contains("MAX_USERS"), "Missing export const");
    assert!(output.contains("currentUser"), "Missing export let");
}

// --- Import statements ---

#[test]
fn javascript_imports() {
    let f = write_js(SAMPLE_JS);
    let output = process_path(f.path().to_str().unwrap(), opts()).unwrap();

    assert!(output.contains("EventEmitter"), "Missing named import");
    assert!(output.contains("fs"), "Missing default import");
}

// --- Variable declarations ---

#[test]
fn javascript_variable_declarations() {
    let f = write_js(SAMPLE_JS);
    let output = process_path(f.path().to_str().unwrap(), opts()).unwrap();

    assert!(output.contains("const") && output.contains("MAX_USERS"), "Missing const declaration");
    assert!(output.contains("let") && output.contains("currentUser"), "Missing let declaration");
    assert!(output.contains("var") && output.contains("legacyFlag"), "Missing var declaration");
}

// --- Arrow functions in const ---

#[test]
fn javascript_arrow_function_const() {
    let f = write_js(SAMPLE_JS);
    let output = process_path(f.path().to_str().unwrap(), opts()).unwrap();

    assert!(output.contains("arrowFn"), "Missing arrow fn const");
    assert!(output.contains("const"), "Arrow fn should show as const");
}

// --- JSX file ---

#[test]
fn javascript_jsx_file() {
    let src = r#"import React from "react";

export function Greeting({ name }) {
    return <div>Hello {name}</div>;
}

export class Counter extends React.Component {
    render() {
        return <span>{this.props.count}</span>;
    }
}
"#;
    let f = write_jsx(src);
    let output = process_path(f.path().to_str().unwrap(), opts()).unwrap();

    assert!(output.contains("function Greeting"), "Missing function in JSX");
    assert!(output.contains("class Counter"), "Missing class in JSX");
    assert!(output.contains("import"), "Missing import in JSX");
}

// --- Filters ---

#[test]
fn javascript_pub_filter() {
    let f = write_js(SAMPLE_JS);
    let mut o = opts();
    o.pub_only = true;
    let output = process_path(f.path().to_str().unwrap(), o).unwrap();

    assert!(output.contains("function publicApi"), "Missing exported function");
    assert!(output.contains("class UserService"), "Missing exported class");
    assert!(!output.contains("helperFunction"), "Should not contain non-exported fn");
    assert!(!output.contains("legacyFlag"), "Should not contain non-exported var");
}

#[test]
fn javascript_fns_filter() {
    let f = write_js(SAMPLE_JS);
    let mut o = opts();
    o.fns_only = true;
    let output = process_path(f.path().to_str().unwrap(), o).unwrap();

    assert!(output.contains("function publicApi") || output.contains("function helperFunction"),
            "Missing functions");
    assert!(!output.contains("class UserService"), "Should not contain class");
    assert!(!output.contains("MAX_USERS"), "Should not contain const");
}

#[test]
fn javascript_types_filter() {
    let f = write_js(SAMPLE_JS);
    let mut o = opts();
    o.types_only = true;
    let output = process_path(f.path().to_str().unwrap(), o).unwrap();

    assert!(output.contains("class UserService"), "Missing class as type");
    assert!(!output.contains("function helperFunction"), "Should not contain standalone fn");
}

#[test]
fn javascript_pub_fns_combined() {
    let f = write_js(SAMPLE_JS);
    let mut o = opts();
    o.pub_only = true;
    o.fns_only = true;
    let output = process_path(f.path().to_str().unwrap(), o).unwrap();

    assert!(output.contains("function publicApi"), "Missing exported function");
    assert!(!output.contains("helperFunction"), "Should not contain non-exported fn");
    assert!(!output.contains("class UserService"), "Should not contain class with fns filter");
}

// --- Stats ---

#[test]
fn javascript_stats_mode() {
    let f = write_js(SAMPLE_JS);
    let mut o = opts();
    o.stats = true;
    let output = process_path(f.path().to_str().unwrap(), o).unwrap();

    assert!(output.contains("Files:"), "Missing files count. Got: {}", output);
    assert!(output.contains("Lines:"), "Missing lines count");
    assert!(output.contains("Bytes:"), "Missing bytes count");
}

// === Per-language visibility filtering (issue #82) ===

#[test]
fn javascript_pub_filter_hides_hash_private() {
    let f = write_js(r#"
export class Foo {
    greet() {}
    #secret() { return 42; }
}
"#);
    let out = process_path(f.path().to_str().unwrap(), ProcessOptions {
        pub_only: true,
        fns_only: true,
        ..opts()
    }).unwrap();
    assert!(out.contains("greet"), "public method should be visible");
    assert!(!out.contains("secret"), "#private method should be hidden");
}

#[test]
fn javascript_pub_filter_non_exported_hidden() {
    let f = write_js(r#"
export function publicFn() {}
function privateFn() {}
export class PublicClass {}
class PrivateClass {}
"#);
    let out = process_path(f.path().to_str().unwrap(), ProcessOptions {
        pub_only: true,
        ..opts()
    }).unwrap();
    assert!(out.contains("publicFn"), "exported function should be visible");
    assert!(out.contains("PublicClass"), "exported class should be visible");
    assert!(!out.contains("privateFn"), "non-exported function should be hidden");
    assert!(!out.contains("PrivateClass"), "non-exported class should be hidden");
}
