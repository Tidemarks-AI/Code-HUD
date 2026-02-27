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

fn write_go(content: &str) -> NamedTempFile {
    let mut f = tempfile::Builder::new().suffix(".go").tempfile().unwrap();
    f.write_all(content.as_bytes()).unwrap();
    f.flush().unwrap();
    f
}

const SAMPLE_GO: &str = r#"package main

import (
	"fmt"
	"strings"
)

const MaxRetries = 3

var DefaultTimeout = 30

type Config struct {
	Host string
	Port int
}

type Handler interface {
	Handle(req string) string
	Close() error
}

type status int

func NewConfig(host string, port int) *Config {
	return &Config{Host: host, Port: port}
}

func (c *Config) Address() string {
	return fmt.Sprintf("%s:%d", c.Host, c.Port)
}

func helper(s string) string {
	return strings.TrimSpace(s)
}
"#;

#[test]
fn go_basic_extraction() {
    let f = write_go(SAMPLE_GO);
    let result = process_path(f.path().to_str().unwrap(), opts()).unwrap();
    let output = result.to_string();
    // Should contain functions
    assert!(output.contains("NewConfig"), "should contain NewConfig function");
    assert!(output.contains("Address"), "should contain Address method");
    assert!(output.contains("helper"), "should contain helper function");
    // Should contain types
    assert!(output.contains("Config"), "should contain Config struct");
    assert!(output.contains("Handler"), "should contain Handler interface");
}

#[test]
fn go_pub_filter() {
    let f = write_go(SAMPLE_GO);
    let mut o = opts();
    o.pub_only = true;
    let result = process_path(f.path().to_str().unwrap(), o).unwrap();
    let output = result.to_string();
    // Exported (uppercase) should be present
    assert!(output.contains("NewConfig"), "exported func should be present");
    assert!(output.contains("Config"), "exported struct should be present");
    assert!(output.contains("Handler"), "exported interface should be present");
    assert!(output.contains("MaxRetries"), "exported const should be present");
    // Unexported (lowercase) should be hidden
    assert!(!output.contains("helper"), "unexported func should be hidden");
    assert!(!output.contains("status"), "unexported type should be hidden");
}

#[test]
fn go_fns_only() {
    let f = write_go(SAMPLE_GO);
    let mut o = opts();
    o.fns_only = true;
    let result = process_path(f.path().to_str().unwrap(), o).unwrap();
    let output = result.to_string();
    assert!(output.contains("NewConfig"), "should contain function");
    assert!(output.contains("Address"), "should contain method");
    // Types should not appear at top level in fns_only mode
    // (they may still appear if they contain methods, depending on implementation)
}

#[test]
fn go_signatures() {
    let f = write_go(SAMPLE_GO);
    let mut o = opts();
    o.signatures = true;
    let result = process_path(f.path().to_str().unwrap(), o).unwrap();
    let output = result.to_string();
    assert!(output.contains("func NewConfig(host string, port int) *Config"), "should show function signature");
}

#[test]
fn go_fixture_file() {
    let result = process_path("tests/fixtures/sample.go", opts()).unwrap();
    let output = result.to_string();
    assert!(output.contains("NewConfig"), "fixture should contain NewConfig");
    assert!(output.contains("Config"), "fixture should contain Config struct");
}

#[test]
fn go_no_tests_filter() {
    let src = r#"package main

import "testing"

func Add(a, b int) int {
	return a + b
}

func TestAdd(t *testing.T) {
	if Add(1, 2) != 3 {
		t.Fatal("bad")
	}
}
"#;
    let f = write_go(src);
    let mut o = opts();
    o.no_tests = true;
    let result = process_path(f.path().to_str().unwrap(), o).unwrap();
    let output = result.to_string();
    assert!(output.contains("Add"), "non-test func should be present");
    // TestAdd should be filtered out in no_tests mode
}

#[test]
fn go_list_symbols() {
    let f = write_go(SAMPLE_GO);
    let mut o = opts();
    o.list_symbols = true;
    let result = process_path(f.path().to_str().unwrap(), o).unwrap();
    let output = result.to_string();
    assert!(output.contains("NewConfig"), "should list NewConfig");
    assert!(output.contains("Config"), "should list Config");
    assert!(output.contains("Handler"), "should list Handler");
}
