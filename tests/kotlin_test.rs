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

fn write_kotlin(content: &str) -> NamedTempFile {
    let mut f = tempfile::Builder::new().suffix(".kt").tempfile().unwrap();
    f.write_all(content.as_bytes()).unwrap();
    f.flush().unwrap();
    f
}

const SAMPLE_KOTLIN: &str = r#"package com.example.app

import kotlin.collections.List
import kotlin.collections.Map

interface Repository<T> {
    fun findById(id: String): T?
    fun findAll(): List<T>
    fun save(entity: T)
}

data class User(val name: String, val age: Int) {
    fun greet(): String {
        return "Hello, $name!"
    }
}

sealed class Result<out T> {
    data class Success<T>(val data: T) : Result<T>()
    data class Error(val message: String) : Result<Nothing>()
}

enum class Status {
    ACTIVE,
    INACTIVE,
    PENDING;

    fun label(): String = name.lowercase()
}

object AppConfig {
    val version = "1.0.0"

    fun init() {
        println("Initializing...")
    }
}

class UserService(private val repo: Repository<User>) {
    val cache: MutableMap<String, User> = mutableMapOf()

    fun getUser(id: String): User? {
        return cache[id]
    }

    private fun refreshCache() {
        cache.clear()
    }

    companion object {
        fun create(): UserService {
            return UserService(object : Repository<User> {
                override fun findById(id: String): User? = null
                override fun findAll(): List<User> = emptyList()
                override fun save(entity: User) {}
            })
        }
    }
}

fun topLevelFunction(x: Int): Int = x * 2

val topLevelProperty: String = "hello"

typealias StringList = List<String>
"#;

// --- Interface mode (default) ---

#[test]
fn kotlin_interface_mode_basic() {
    let f = write_kotlin(SAMPLE_KOTLIN);
    let output = process_path(f.path().to_str().unwrap(), opts()).unwrap();

    assert!(output.contains("package"), "Missing package declaration");
    assert!(output.contains("import"), "Missing import");
    assert!(output.contains("interface Repository"), "Missing interface");
    assert!(output.contains("data class User"), "Missing data class");
    assert!(
        output.contains("sealed class Result"),
        "Missing sealed class"
    );
    assert!(output.contains("enum class Status"), "Missing enum class");
    assert!(output.contains("object AppConfig"), "Missing object");
    assert!(output.contains("class UserService"), "Missing class");
    assert!(
        output.contains("topLevelFunction"),
        "Missing top-level function"
    );
    assert!(
        output.contains("topLevelProperty"),
        "Missing top-level property"
    );
    assert!(output.contains("typealias"), "Missing type alias");
}

// --- Expand class ---

#[test]
fn kotlin_expand_class() {
    let f = write_kotlin(SAMPLE_KOTLIN);
    let mut o = opts();
    o.symbols = vec!["UserService".to_string()];
    let output = process_path(f.path().to_str().unwrap(), o).unwrap();

    assert!(output.contains("class UserService"), "Missing class");
    assert!(output.contains("getUser"), "Missing getUser method");
    assert!(
        output.contains("refreshCache"),
        "Missing refreshCache method"
    );
    assert!(
        output.contains("companion object"),
        "Missing companion object"
    );
    assert!(output.contains("cache"), "Missing property");
}

// --- Expand data class ---

#[test]
fn kotlin_expand_data_class() {
    let f = write_kotlin(SAMPLE_KOTLIN);
    let mut o = opts();
    o.symbols = vec!["User".to_string()];
    let output = process_path(f.path().to_str().unwrap(), o).unwrap();

    assert!(output.contains("data class User"), "Missing data class");
    assert!(output.contains("greet"), "Missing method in data class");
}

// --- Expand enum ---

#[test]
fn kotlin_expand_enum() {
    let f = write_kotlin(SAMPLE_KOTLIN);
    let mut o = opts();
    o.symbols = vec!["Status".to_string()];
    let output = process_path(f.path().to_str().unwrap(), o).unwrap();

    assert!(output.contains("enum class Status"), "Missing enum");
    assert!(output.contains("label"), "Missing enum method");
}

// --- Expand object ---

#[test]
fn kotlin_expand_object() {
    let f = write_kotlin(SAMPLE_KOTLIN);
    let mut o = opts();
    o.symbols = vec!["AppConfig".to_string()];
    let output = process_path(f.path().to_str().unwrap(), o).unwrap();

    assert!(output.contains("object AppConfig"), "Missing object");
    assert!(output.contains("version"), "Missing property");
    assert!(output.contains("init"), "Missing function");
}

// --- Pub filter ---

#[test]
fn kotlin_pub_filter() {
    let src = r#"
class MyClass {
    fun publicMethod() {}
    private fun privateMethod() {}
    internal fun internalMethod() {}
}

private class PrivateClass {
    fun doWork() {}
}
"#;
    let f = write_kotlin(src);
    let mut o = opts();
    o.pub_only = true;
    let output = process_path(f.path().to_str().unwrap(), o).unwrap();

    assert!(
        output.contains("class MyClass"),
        "Missing public class (Kotlin default is public)"
    );
    assert!(
        !output.contains("PrivateClass"),
        "Private class should be hidden"
    );
}

// --- Types filter ---

#[test]
fn kotlin_types_filter() {
    let f = write_kotlin(SAMPLE_KOTLIN);
    let mut o = opts();
    o.types_only = true;
    let output = process_path(f.path().to_str().unwrap(), o).unwrap();

    assert!(output.contains("Repository"), "Missing interface as type");
    assert!(output.contains("User"), "Missing data class as type");
    assert!(output.contains("Status"), "Missing enum as type");
}

// --- Fns filter ---

#[test]
fn kotlin_fns_filter() {
    let f = write_kotlin(SAMPLE_KOTLIN);
    let mut o = opts();
    o.fns_only = true;
    let output = process_path(f.path().to_str().unwrap(), o).unwrap();

    assert!(
        output.contains("topLevelFunction"),
        "Missing top-level function"
    );
}

// --- Stats ---

#[test]
fn kotlin_stats_mode() {
    let f = write_kotlin(SAMPLE_KOTLIN);
    let mut o = opts();
    o.stats = true;
    let output = process_path(f.path().to_str().unwrap(), o).unwrap();

    assert!(output.contains("Files:"), "Missing files count");
    assert!(output.contains("Lines:"), "Missing lines count");
}

// --- Imports ---

#[test]
fn kotlin_imports() {
    let f = write_kotlin(SAMPLE_KOTLIN);
    let output = process_path(f.path().to_str().unwrap(), opts()).unwrap();

    assert!(
        output.contains("import kotlin.collections.List"),
        "Missing List import"
    );
    assert!(
        output.contains("import kotlin.collections.Map"),
        "Missing Map import"
    );
}

// --- List symbols ---

#[test]
fn kotlin_list_symbols() {
    let f = write_kotlin(SAMPLE_KOTLIN);
    let mut o = opts();
    o.list_symbols = true;
    let output = process_path(f.path().to_str().unwrap(), o).unwrap();

    assert!(output.contains("Repository"), "Missing interface in list");
    assert!(output.contains("User"), "Missing data class in list");
    assert!(output.contains("Result"), "Missing sealed class in list");
    assert!(output.contains("Status"), "Missing enum in list");
    assert!(output.contains("AppConfig"), "Missing object in list");
    assert!(output.contains("UserService"), "Missing class in list");
    assert!(
        output.contains("topLevelFunction"),
        "Missing function in list"
    );
    assert!(
        output.contains("topLevelProperty"),
        "Missing property in list"
    );
    assert!(output.contains("StringList"), "Missing typealias in list");
}

// --- .kts extension ---

#[test]
fn kotlin_kts_extension() {
    let mut f = tempfile::Builder::new().suffix(".kts").tempfile().unwrap();
    f.write_all(b"fun main() { println(\"hello\") }").unwrap();
    f.flush().unwrap();
    let output = process_path(f.path().to_str().unwrap(), opts()).unwrap();

    assert!(output.contains("main"), "Missing function from .kts file");
}

// --- Visibility ---

#[test]
fn kotlin_visibility_levels() {
    let src = r#"
public class PublicClass
private class PrivateClass
internal class InternalClass
protected class ProtectedClass
class DefaultPublicClass
"#;
    let f = write_kotlin(src);
    let mut o = opts();
    o.pub_only = true;
    let output = process_path(f.path().to_str().unwrap(), o).unwrap();

    assert!(
        output.contains("PublicClass"),
        "Missing explicit public class"
    );
    assert!(
        output.contains("DefaultPublicClass"),
        "Missing default-public class"
    );
    assert!(
        !output.contains("PrivateClass"),
        "Private class should be hidden"
    );
    assert!(
        !output.contains("InternalClass"),
        "Internal class should be hidden with pub filter"
    );
}
