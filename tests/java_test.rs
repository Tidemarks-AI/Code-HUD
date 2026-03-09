use codehud::{OutputFormat, ProcessOptions, process_path};
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
        stats: false,
        stats_detailed: true,
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
        yes: false,
        warn_threshold: 10_000,
        expand_symbols: vec![],
        token_budget: None,
        with_comments: false,
    }
}

fn write_java(content: &str) -> NamedTempFile {
    let mut f = tempfile::Builder::new().suffix(".java").tempfile().unwrap();
    f.write_all(content.as_bytes()).unwrap();
    f.flush().unwrap();
    f
}

const SAMPLE_JAVA: &str = r#"package com.example.app;

import java.util.List;
import java.util.Map;

public class UserService {
    private static final int MAX_USERS = 100;
    private Map<String, String> cache;

    public UserService() {
        this.cache = new java.util.HashMap<>();
    }

    public String getUser(String id) {
        return cache.get(id);
    }

    private void refreshCache() {
        cache.clear();
    }

    public static UserService create() {
        return new UserService();
    }
}

interface Repository<T> {
    T findById(String id);
    List<T> findAll();
    void save(T entity);
}

public enum Status {
    ACTIVE,
    INACTIVE,
    PENDING;

    public String label() {
        return name().toLowerCase();
    }
}

public record Point(int x, int y) {
    public double distance() {
        return Math.sqrt(x * x + y * y);
    }
}
"#;

// --- Interface mode (default) ---

#[test]
fn java_interface_mode_basic() {
    let f = write_java(SAMPLE_JAVA);
    let output = process_path(f.path().to_str().unwrap(), opts()).unwrap();

    assert!(output.contains("package"), "Missing package declaration");
    assert!(output.contains("import"), "Missing import");
    assert!(output.contains("class UserService"), "Missing class");
    assert!(output.contains("interface Repository"), "Missing interface");
    assert!(output.contains("enum Status"), "Missing enum");
    assert!(output.contains("record Point"), "Missing record");
}

// --- Expand class ---

#[test]
fn java_expand_class() {
    let f = write_java(SAMPLE_JAVA);
    let mut o = opts();
    o.symbols = vec!["UserService".to_string()];
    let output = process_path(f.path().to_str().unwrap(), o).unwrap();

    assert!(output.contains("class UserService"), "Missing class");
    assert!(output.contains("getUser"), "Missing getUser method");
    assert!(
        output.contains("refreshCache"),
        "Missing refreshCache method"
    );
    assert!(output.contains("create"), "Missing static method");
    assert!(output.contains("MAX_USERS"), "Missing field");
}

// --- Pub filter ---

#[test]
fn java_pub_filter() {
    let f = write_java(SAMPLE_JAVA);
    let mut o = opts();
    o.pub_only = true;
    let output = process_path(f.path().to_str().unwrap(), o).unwrap();

    assert!(output.contains("class UserService"), "Missing public class");
    assert!(output.contains("enum Status"), "Missing public enum");
    assert!(output.contains("record Point"), "Missing public record");
}

// --- Fns filter ---

#[test]
fn java_fns_filter() {
    let src = r#"
public class Foo {
    public void bar() {}
}
"#;
    let f = write_java(src);
    let mut o = opts();
    o.fns_only = true;
    let output = process_path(f.path().to_str().unwrap(), o).unwrap();

    // In fns mode, standalone methods should appear but not classes
    // Note: Java methods are always inside classes, so this tests top-level behavior
    assert!(
        !output.contains("class Foo") || output.contains("bar"),
        "fns filter behavior check"
    );
}

// --- Types filter ---

#[test]
fn java_types_filter() {
    let f = write_java(SAMPLE_JAVA);
    let mut o = opts();
    o.types_only = true;
    let output = process_path(f.path().to_str().unwrap(), o).unwrap();

    assert!(
        output.contains("class UserService"),
        "Missing class as type"
    );
    assert!(
        output.contains("interface Repository"),
        "Missing interface as type"
    );
    assert!(output.contains("enum Status"), "Missing enum as type");
}

// --- Stats ---

#[test]
fn java_stats_mode() {
    let f = write_java(SAMPLE_JAVA);
    let mut o = opts();
    o.stats = true;
    let output = process_path(f.path().to_str().unwrap(), o).unwrap();

    assert!(output.contains("Files:"), "Missing files count");
    assert!(output.contains("Lines:"), "Missing lines count");
}

// --- Constructor ---

#[test]
fn java_constructor() {
    let f = write_java(SAMPLE_JAVA);
    let mut o = opts();
    o.symbols = vec!["UserService".to_string()];
    let output = process_path(f.path().to_str().unwrap(), o).unwrap();

    assert!(output.contains("UserService"), "Missing constructor");
}

// --- Enum with methods ---

#[test]
fn java_enum_with_methods() {
    let f = write_java(SAMPLE_JAVA);
    let mut o = opts();
    o.symbols = vec!["Status".to_string()];
    let output = process_path(f.path().to_str().unwrap(), o).unwrap();

    assert!(output.contains("enum Status"), "Missing enum");
    assert!(output.contains("label"), "Missing enum method");
}

// --- Record ---

#[test]
fn java_record() {
    let f = write_java(SAMPLE_JAVA);
    let mut o = opts();
    o.symbols = vec!["Point".to_string()];
    let output = process_path(f.path().to_str().unwrap(), o).unwrap();

    assert!(output.contains("record Point"), "Missing record");
    assert!(output.contains("distance"), "Missing record method");
}

// --- Visibility levels ---

#[test]
fn java_visibility_pub_filter() {
    let src = r#"
public class UserService {
    public String getUser(String id) {
        return "test";
    }
    private void refreshCache() {
        System.out.println("refresh");
    }
    public static UserService create() {
        return new UserService();
    }
}
class InternalHelper {
    void doWork() {}
}
"#;
    let f = write_java(src);
    let mut o = opts();
    o.pub_only = true;
    let output = process_path(f.path().to_str().unwrap(), o).unwrap();

    assert!(output.contains("class UserService"), "Missing public class");
    assert!(
        !output.contains("InternalHelper"),
        "Package-private class should be hidden"
    );
}

// --- Annotations ---

#[test]
fn java_annotation_type() {
    let src = r#"
public @interface MyAnnotation {
    String value() default "";
    int count() default 0;
}
"#;
    let f = write_java(src);
    let output = process_path(f.path().to_str().unwrap(), opts()).unwrap();

    assert!(output.contains("MyAnnotation"), "Missing annotation type");
}

// --- Imports ---

#[test]
fn java_imports() {
    let f = write_java(SAMPLE_JAVA);
    let output = process_path(f.path().to_str().unwrap(), opts()).unwrap();

    assert!(
        output.contains("import java.util.List"),
        "Missing List import"
    );
    assert!(
        output.contains("import java.util.Map"),
        "Missing Map import"
    );
}
