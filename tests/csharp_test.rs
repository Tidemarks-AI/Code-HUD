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
        yes: false,
        warn_threshold: 10_000,
        expand_symbols: vec![],
        token_budget: None,
    }
}

fn write_csharp(content: &str) -> NamedTempFile {
    let mut f = tempfile::Builder::new().suffix(".cs").tempfile().unwrap();
    f.write_all(content.as_bytes()).unwrap();
    f.flush().unwrap();
    f
}

const SAMPLE_CSHARP: &str = r#"using System;
using System.Collections.Generic;

namespace MyApp.Models
{
    public class UserService
    {
        private static readonly int MaxUsers = 100;
        private Dictionary<string, string> _cache;

        public UserService()
        {
            _cache = new Dictionary<string, string>();
        }

        public string GetUser(string id)
        {
            return _cache[id];
        }

        private void RefreshCache()
        {
            _cache.Clear();
        }

        public static UserService Create()
        {
            return new UserService();
        }
    }

    public interface IRepository<T>
    {
        T FindById(string id);
        List<T> FindAll();
        void Save(T entity);
    }

    public enum Status
    {
        Active,
        Inactive,
        Pending
    }

    public struct Point
    {
        public int X { get; set; }
        public int Y { get; set; }

        public double Distance()
        {
            return Math.Sqrt(X * X + Y * Y);
        }
    }

    public record Person(string FirstName, string LastName)
    {
        public string FullName()
        {
            return $"{FirstName} {LastName}";
        }
    }
}
"#;

// --- Interface mode (default) ---

#[test]
fn csharp_interface_mode_basic() {
    let f = write_csharp(SAMPLE_CSHARP);
    let output = process_path(f.path().to_str().unwrap(), opts()).unwrap();

    assert!(output.contains("using"), "Missing using directive");
    assert!(output.contains("namespace"), "Missing namespace");
    assert!(output.contains("class UserService"), "Missing class");
    assert!(output.contains("interface IRepository"), "Missing interface");
    assert!(output.contains("enum Status"), "Missing enum");
    assert!(output.contains("struct Point"), "Missing struct");
    assert!(output.contains("record Person"), "Missing record");
}

// --- Expand class ---

#[test]
fn csharp_expand_class() {
    let f = write_csharp(SAMPLE_CSHARP);
    let mut o = opts();
    o.symbols = vec!["UserService".to_string()];
    let output = process_path(f.path().to_str().unwrap(), o).unwrap();

    assert!(output.contains("class UserService"), "Missing class");
    assert!(output.contains("GetUser"), "Missing GetUser method");
    assert!(output.contains("RefreshCache"), "Missing RefreshCache method");
    assert!(output.contains("Create"), "Missing static method");
}

// --- Pub filter ---

#[test]
fn csharp_pub_filter() {
    let f = write_csharp(SAMPLE_CSHARP);
    let mut o = opts();
    o.pub_only = true;
    let output = process_path(f.path().to_str().unwrap(), o).unwrap();

    assert!(output.contains("class UserService"), "Missing public class");
    assert!(output.contains("enum Status"), "Missing public enum");
    assert!(output.contains("struct Point"), "Missing public struct");
    assert!(output.contains("record Person"), "Missing public record");
}

// --- Types filter ---

#[test]
fn csharp_types_filter() {
    let f = write_csharp(SAMPLE_CSHARP);
    let mut o = opts();
    o.types_only = true;
    let output = process_path(f.path().to_str().unwrap(), o).unwrap();

    assert!(output.contains("class UserService"), "Missing class as type");
    assert!(output.contains("interface IRepository"), "Missing interface as type");
    assert!(output.contains("enum Status"), "Missing enum as type");
}

// --- Stats ---

#[test]
fn csharp_stats_mode() {
    let f = write_csharp(SAMPLE_CSHARP);
    let mut o = opts();
    o.stats = true;
    let output = process_path(f.path().to_str().unwrap(), o).unwrap();

    assert!(output.contains("Files:"), "Missing files count");
    assert!(output.contains("Lines:"), "Missing lines count");
}

// --- Struct with properties ---

#[test]
fn csharp_struct_with_properties() {
    let f = write_csharp(SAMPLE_CSHARP);
    let mut o = opts();
    o.symbols = vec!["Point".to_string()];
    let output = process_path(f.path().to_str().unwrap(), o).unwrap();

    assert!(output.contains("struct Point"), "Missing struct");
    assert!(output.contains("Distance"), "Missing struct method");
}

// --- Record ---

#[test]
fn csharp_record() {
    let f = write_csharp(SAMPLE_CSHARP);
    let mut o = opts();
    o.symbols = vec!["Person".to_string()];
    let output = process_path(f.path().to_str().unwrap(), o).unwrap();

    assert!(output.contains("record Person"), "Missing record");
    assert!(output.contains("FullName"), "Missing record method");
}

// --- Visibility ---

#[test]
fn csharp_visibility_pub_filter() {
    let src = r#"
public class UserService
{
    public string GetUser(string id) { return "test"; }
    private void RefreshCache() { }
    public static UserService Create() { return new UserService(); }
}
internal class InternalHelper
{
    void DoWork() { }
}
"#;
    let f = write_csharp(src);
    let mut o = opts();
    o.pub_only = true;
    let output = process_path(f.path().to_str().unwrap(), o).unwrap();

    assert!(output.contains("class UserService"), "Missing public class");
    assert!(!output.contains("InternalHelper"), "Internal class should be hidden");
}

// --- Using directives ---

#[test]
fn csharp_usings() {
    let f = write_csharp(SAMPLE_CSHARP);
    let output = process_path(f.path().to_str().unwrap(), opts()).unwrap();

    assert!(output.contains("using System;"), "Missing System using");
    assert!(output.contains("using System.Collections.Generic"), "Missing Collections using");
}
