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
        stats: false, summary_only: false,
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
    }

}

fn write_ts(content: &str) -> NamedTempFile {
    let mut f = tempfile::Builder::new().suffix(".ts").tempfile().unwrap();
    f.write_all(content.as_bytes()).unwrap();
    f.flush().unwrap();
    f
}

fn write_tsx(content: &str) -> NamedTempFile {
    let mut f = tempfile::Builder::new().suffix(".tsx").tempfile().unwrap();
    f.write_all(content.as_bytes()).unwrap();
    f.flush().unwrap();
    f
}

const SAMPLE_TS: &str = r#"import { EventEmitter } from "events";

export interface User {
    name: string;
    age: number;
    email?: string;
}

export type UserId = string | number;

export enum Role {
    Admin = "ADMIN",
    User = "USER",
    Guest = "GUEST",
}

export const MAX_USERS = 100;

export class UserService {
    private db: Map<string, User>;

    constructor() {
        this.db = new Map();
    }

    public getUser(id: string): User | undefined {
        return this.db.get(id);
    }

    public createUser(name: string, age: number): User {
        const user: User = { name, age };
        this.db.set(name, user);
        return user;
    }

    private validate(user: User): boolean {
        return user.name.length > 0 && user.age > 0;
    }
}

function helperFunction(x: number): number {
    return x * 2;
}

export function publicApi(input: string): string {
    return input.trim().toLowerCase();
}
"#;

// --- Interface mode ---

#[test]
fn ts_interface_mode_basic() {
    let f = write_ts(SAMPLE_TS);
    let output = process_path(f.path().to_str().unwrap(), opts()).unwrap();

    // All top-level items should appear
    assert!(output.contains("interface User"), "Missing interface User");
    assert!(output.contains("type UserId"), "Missing type alias UserId");
    assert!(output.contains("enum Role"), "Missing enum Role");
    assert!(output.contains("const MAX_USERS"), "Missing const");
    assert!(output.contains("class UserService"), "Missing class");
    assert!(output.contains("import"), "Missing import");
    assert!(output.contains("function helperFunction"), "Missing helperFunction");
    assert!(output.contains("function publicApi"), "Missing publicApi");
    // Bodies should be collapsed
    assert!(output.contains("{ ... }"), "Missing collapsed bodies");
}

// --- Expand mode ---

#[test]
fn ts_expand_symbol() {
    let f = write_ts(SAMPLE_TS);
    let mut o = opts();
    o.symbols = vec!["publicApi".to_string()];
    let output = process_path(f.path().to_str().unwrap(), o).unwrap();

    assert!(output.contains("function publicApi"), "Missing publicApi");
    assert!(output.contains("trim().toLowerCase()"), "Missing function body");
}

#[test]
fn ts_expand_class() {
    let f = write_ts(SAMPLE_TS);
    let mut o = opts();
    o.symbols = vec!["UserService".to_string()];
    let output = process_path(f.path().to_str().unwrap(), o).unwrap();

    assert!(output.contains("class UserService"), "Missing class");
    assert!(output.contains("new Map()") || output.contains("this.db"), "Missing class body");
}

// --- --pub filter ---

#[test]
fn ts_pub_filter() {
    let f = write_ts(SAMPLE_TS);
    let mut o = opts();
    o.pub_only = true;
    let output = process_path(f.path().to_str().unwrap(), o).unwrap();

    // Exported items should appear
    assert!(output.contains("interface User"), "Missing exported interface");
    assert!(output.contains("function publicApi"), "Missing exported function");
    // Non-exported items should not
    assert!(!output.contains("helperFunction"), "Should not contain non-exported helperFunction");
}

// --- --fns filter ---

#[test]
fn ts_fns_filter() {
    let f = write_ts(SAMPLE_TS);
    let mut o = opts();
    o.fns_only = true;
    let output = process_path(f.path().to_str().unwrap(), o).unwrap();

    // Functions and methods should appear
    assert!(output.contains("function helperFunction") || output.contains("function publicApi"),
            "Missing functions");
    // Types should not
    assert!(!output.contains("interface User"), "Should not contain interface");
    assert!(!output.contains("enum Role"), "Should not contain enum");
}

// --- --types filter ---

#[test]
fn ts_types_filter() {
    let f = write_ts(SAMPLE_TS);
    let mut o = opts();
    o.types_only = true;
    let output = process_path(f.path().to_str().unwrap(), o).unwrap();

    assert!(output.contains("interface User"), "Missing interface");
    assert!(output.contains("enum Role"), "Missing enum");
    assert!(output.contains("type UserId"), "Missing type alias");
    assert!(output.contains("class UserService"), "Missing class");
    // Standalone functions should not appear
    assert!(!output.contains("function helperFunction"), "Should not contain standalone fn");
}

// --- --no-tests (no-op for TS, shouldn't break) ---

#[test]
fn ts_no_tests_noop() {
    let f = write_ts(SAMPLE_TS);
    let mut o = opts();
    o.no_tests = true;
    let output = process_path(f.path().to_str().unwrap(), o).unwrap();

    assert!(output.contains("class UserService"), "Missing class with --no-tests");
    assert!(output.contains("interface User"), "Missing interface with --no-tests");
}

// --- Abstract class ---

#[test]
fn ts_abstract_class() {
    let src = r#"
export abstract class Shape {
    abstract area(): number;
    abstract perimeter(): number;

    public describe(): string {
        return `Area: ${this.area()}`;
    }
}
"#;
    let f = write_ts(src);
    let output = process_path(f.path().to_str().unwrap(), opts()).unwrap();

    assert!(output.contains("abstract class Shape"), "Missing abstract class");
    assert!(output.contains("area()"), "Missing abstract method area");
    assert!(output.contains("perimeter()"), "Missing abstract method perimeter");
    assert!(output.contains("describe()"), "Missing concrete method describe");
}

// --- TSX detection ---

#[test]
fn tsx_file_detection() {
    let src = r#"
import React from "react";

interface Props {
    name: string;
    count: number;
}

export function Greeting({ name, count }: Props): JSX.Element {
    return <div>Hello {name}, count: {count}</div>;
}

export class Counter extends React.Component<Props> {
    render() {
        return <span>{this.props.count}</span>;
    }
}
"#;
    let f = write_tsx(src);
    let output = process_path(f.path().to_str().unwrap(), opts()).unwrap();

    assert!(output.contains("interface Props"), "Missing interface in TSX");
    assert!(output.contains("function Greeting"), "Missing function in TSX");
    assert!(output.contains("class Counter"), "Missing class in TSX");
}

// --- --stats ---

#[test]
fn ts_stats_mode() {
    let f = write_ts(SAMPLE_TS);
    let mut o = opts();
    o.stats = true;
    let output = process_path(f.path().to_str().unwrap(), o).unwrap();

    assert!(output.contains("files:"), "Missing files count in stats");
    assert!(output.contains("lines:"), "Missing lines count in stats");
    assert!(output.contains("bytes:"), "Missing bytes count in stats");
}

#[test]
fn ts_stats_json() {
    let f = write_ts(SAMPLE_TS);
    let mut o = opts();
    o.stats = true;
    o.format = OutputFormat::Json;
    let output = process_path(f.path().to_str().unwrap(), o).unwrap();

    let parsed: serde_json::Value = serde_json::from_str(&output)
        .expect("Stats JSON should be valid");
    assert!(parsed.is_object() || parsed.is_array(), "Should be structured JSON");
}

// --- Decorator support (issue #29) ---

#[test]
fn ts_decorated_exported_class_interface() {
    let src = r#"import { Service } from 'typedi';

@Service()
export class MyService {
  getName(): string {
    return 'hello';
  }
}
"#;
    let f = write_ts(src);
    let output = process_path(f.path().to_str().unwrap(), opts()).unwrap();

    assert!(output.contains("@Service()"), "Missing decorator");
    assert!(output.contains("class MyService"), "Missing class");
    assert!(output.contains("getName()"), "Missing method");
}

#[test]
fn ts_decorated_non_exported_class() {
    let src = r#"@Injectable()
class PlainDecorated {
  value: number = 42;
}
"#;
    let f = write_ts(src);
    let output = process_path(f.path().to_str().unwrap(), opts()).unwrap();

    assert!(output.contains("@Injectable()"), "Missing decorator");
    assert!(output.contains("class PlainDecorated"), "Missing class");
}

#[test]
fn ts_multiple_stacked_decorators() {
    let src = r#"@A()
@B()
class MultiDecorator {
  run(): void {}
}
"#;
    let f = write_ts(src);
    let output = process_path(f.path().to_str().unwrap(), opts()).unwrap();

    assert!(output.contains("@A()"), "Missing first decorator");
    assert!(output.contains("@B()"), "Missing second decorator");
    assert!(output.contains("class MultiDecorator"), "Missing class");
}

#[test]
fn ts_decorated_abstract_class() {
    let src = r#"@Controller()
export abstract class BaseController {
  abstract handle(): void;
}
"#;
    let f = write_ts(src);
    let output = process_path(f.path().to_str().unwrap(), opts()).unwrap();

    assert!(output.contains("@Controller()"), "Missing decorator");
    assert!(output.contains("abstract class BaseController"), "Missing abstract class");
}

#[test]
fn ts_decorated_class_expand() {
    let src = r#"@Service()
export class MyService {
  getName(): string {
    return 'hello';
  }
}
"#;
    let f = write_ts(src);
    let mut o = opts();
    o.symbols = vec!["MyService".to_string()];
    let output = process_path(f.path().to_str().unwrap(), o).unwrap();

    assert!(output.contains("@Service()"), "Missing decorator in expand");
    assert!(output.contains("class MyService"), "Missing class in expand");
    assert!(output.contains("return 'hello'"), "Missing body in expand");
}

#[test]
fn ts_property_decorators_not_regressed() {
    let src = r#"class Entity {
  @Column()
  name: string = '';

  @Inject()
  service: any;
}
"#;
    let f = write_ts(src);
    let output = process_path(f.path().to_str().unwrap(), opts()).unwrap();

    assert!(output.contains("class Entity"), "Missing class");
    assert!(output.contains("@Column()"), "Missing property decorator");
    assert!(output.contains("name: string"), "Missing property");
}

// --- Combined filters ---

#[test]
fn ts_pub_fns_combined() {
    let f = write_ts(SAMPLE_TS);
    let mut o = opts();
    o.pub_only = true;
    o.fns_only = true;
    let output = process_path(f.path().to_str().unwrap(), o).unwrap();

    assert!(output.contains("function publicApi"), "Missing exported function");
    assert!(!output.contains("helperFunction"), "Should not contain non-exported fn");
    assert!(!output.contains("interface User"), "Should not contain types");
}

// === Per-language visibility filtering (issue #82) ===

#[test]
fn ts_pub_filter_hides_private_methods() {
    let f = write_ts(r#"
export class Foo {
    public greet(): void {}
    protected helper(): void {}
    private internal(): void {}
    open(): void {}
}
"#);
    let out = process_path(f.path().to_str().unwrap(), ProcessOptions {
        pub_only: true,
        fns_only: true,
        ..opts()
    }).unwrap();
    assert!(out.contains("greet"), "public method should be visible");
    assert!(out.contains("open"), "default (no modifier) method should be visible");
    assert!(!out.contains("helper"), "protected method should be hidden");
    assert!(!out.contains("internal"), "private method should be hidden");
}

#[test]
fn ts_pub_filter_hides_hash_private() {
    let f = write_ts(r#"
export class Foo {
    public greet(): void {}
    #secret(): number { return 42; }
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
fn ts_pub_filter_non_exported_top_level_hidden() {
    let f = write_ts(r#"
export function publicFn(): void {}
function privateFn(): void {}
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

#[test]
fn ts_visibility_protected_in_json() {
    let f = write_ts(r#"
export class Foo {
    public a(): void {}
    protected b(): void {}
    private c(): void {}
}
"#);
    let json_out = process_path(f.path().to_str().unwrap(), ProcessOptions {
        format: OutputFormat::Json,
        fns_only: true,
        ..opts()
    }).unwrap();
    assert!(json_out.contains("\"protected\""), "JSON should show protected visibility");
    assert!(json_out.contains("\"private\""), "JSON should show private visibility");
    assert!(json_out.contains("\"public\""), "JSON should show public visibility");
}

#[test]
fn ts_pub_filter_hides_private_members_in_class_body() {
    let f = write_ts(r#"
export class MyClass {
    private secret: string;
    public name: string;

    private internalMethod() {
        return 1;
    }

    publicMethod() {
        return 2;
    }
}
"#);
    let out = process_path(f.path().to_str().unwrap(), ProcessOptions {
        pub_only: true,
        ..opts()
    }).unwrap();
    // Class content should not contain private members
    assert!(!out.contains("secret"), "private field 'secret' should be hidden from class body with --pub");
    assert!(!out.contains("internalMethod"), "private method should be hidden from class body with --pub");
    assert!(out.contains("name"), "public field should remain visible");
    assert!(out.contains("publicMethod"), "public method should remain visible");
    assert!(out.contains("MyClass"), "class name should be visible");
}
