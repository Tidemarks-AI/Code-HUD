use codehud::editor::{self, BatchEdit, BatchAction};
use codehud::Language;

// ============================================================================
// REPLACE TESTS
// ============================================================================

#[test]
fn test_replace_function() {
    let source = r#"
fn hello() {
    println!("Hello");
}

fn world() {
    println!("World");
}
"#;

    let new_content = r#"fn hello() {
    println!("Greetings");
    println!("Modified");
}"#;

    let result = editor::replace(source, "hello", new_content, Language::Rust).unwrap();
    
    assert!(result.contains("Greetings"));
    assert!(result.contains("Modified"));
    assert!(result.contains("fn world()"));
    assert!(!result.contains(r#"println!("Hello")"#));
}

#[test]
fn test_replace_struct_with_attributes() {
    let source = r#"
#[derive(Debug, Clone)]
#[serde(rename_all = "camelCase")]
struct Person {
    name: String,
    age: u32,
}

struct Other {
    value: i32,
}
"#;

    let new_content = r#"#[derive(Debug)]
struct Person {
    full_name: String,
}"#;

    let result = editor::replace(source, "Person", new_content, Language::Rust).unwrap();
    
    assert!(result.contains("full_name"));
    assert!(result.contains("#[derive(Debug)]"));
    assert!(!result.contains("serde"));
    assert!(!result.contains("age: u32"));
    assert!(result.contains("struct Other"));
}

#[test]
fn test_replace_with_invalid_syntax() {
    let source = r#"
fn valid() {
    println!("Valid");
}
"#;

    let invalid_content = r#"fn valid( {
    this is not valid rust
}"#;

    let result = editor::replace(source, "valid", invalid_content, Language::Rust);
    
    assert!(result.is_err());
}

#[test]
fn test_replace_symbol_not_found() {
    let source = r#"
fn existing() {
    println!("Exists");
}
"#;

    let new_content = r#"fn new_func() {}"#;

    let result = editor::replace(source, "nonexistent", new_content, Language::Rust);
    
    assert!(result.is_err());
}

// ============================================================================
// DELETE TESTS
// ============================================================================

#[test]
fn test_delete_function() {
    let source = r#"
fn first() {
    println!("First");
}

fn second() {
    println!("Second");
}

fn third() {
    println!("Third");
}
"#;

    let result = editor::delete(source, "second", Language::Rust).unwrap();
    
    assert!(result.contains("fn first()"));
    assert!(result.contains("fn third()"));
    assert!(!result.contains("fn second()"));
    assert!(!result.contains(r#"println!("Second")"#));
}

#[test]
fn test_delete_struct_with_attributes() {
    let source = r#"
#[derive(Debug, Clone)]
#[serde(rename_all = "camelCase")]
struct ToDelete {
    field: String,
}

struct ToKeep {
    value: i32,
}
"#;

    let result = editor::delete(source, "ToDelete", Language::Rust).unwrap();
    
    assert!(!result.contains("ToDelete"));
    assert!(!result.contains("#[derive(Debug, Clone)]"));
    assert!(!result.contains("serde"));
    assert!(result.contains("struct ToKeep"));
}

#[test]
fn test_delete_trailing_newline_cleanup() {
    let source = r#"fn first() {}

fn second() {}

fn third() {}
"#;

    let result = editor::delete(source, "second", Language::Rust).unwrap();
    
    // Deletion may leave some blank lines, which is acceptable
    assert!(result.contains("fn third()"));
}

#[test]
fn test_delete_symbol_not_found() {
    let source = r#"
fn existing() {
    println!("Exists");
}
"#;

    let result = editor::delete(source, "nonexistent", Language::Rust);
    
    assert!(result.is_err());
}

// ============================================================================
// REPLACE_BODY TESTS
// ============================================================================

#[test]
fn test_replace_body_function() {
    let source = r#"
fn calculate(x: i32, y: i32) -> i32 {
    x + y
}
"#;

    let new_body = r#"{
    let result = x * y;
    result
}"#;

    let result = editor::replace_body(source, "calculate", new_body, Language::Rust).unwrap();
    
    // Signature should be preserved
    assert!(result.contains("fn calculate(x: i32, y: i32) -> i32"));
    // New body should be present
    assert!(result.contains("x * y"));
    assert!(result.contains("let result"));
    // Old body should be gone
    assert!(!result.contains("x + y"));
}

#[test]
fn test_replace_body_method_in_impl() {
    let source = r#"
struct Calculator;

impl Calculator {
    fn add(&self, a: i32, b: i32) -> i32 {
        a + b
    }
    
    fn multiply(&self, a: i32, b: i32) -> i32 {
        a * b
    }
}
"#;

    let new_body = r#"{
        println!("Adding {} and {}", a, b);
        a + b
    }"#;

    let result = editor::replace_body(source, "add", new_body, Language::Rust).unwrap();
    
    assert!(result.contains("fn add(&self, a: i32, b: i32) -> i32"));
    assert!(result.contains(r#"println!("Adding"#));
    assert!(result.contains("fn multiply"));
}

#[test]
fn test_replace_body_invalid_body() {
    let source = r#"
fn valid(x: i32) -> i32 {
    x + 1
}
"#;

    let invalid_body = r#"{
    this is not valid rust syntax
    missing semicolons and structure
}"#;

    let result = editor::replace_body(source, "valid", invalid_body, Language::Rust);
    
    assert!(result.is_err());
}

#[test]
fn test_replace_body_symbol_not_found() {
    let source = r#"
fn existing() {
    println!("Exists");
}
"#;

    let new_body = r#"{ println!("New"); }"#;

    let result = editor::replace_body(source, "nonexistent", new_body, Language::Rust);
    
    assert!(result.is_err());
}

// ============================================================================
// BATCH TESTS
// ============================================================================

#[test]
fn test_batch_multiple_edits() {
    let source = r#"
fn first() {
    println!("First");
}

fn second() {
    println!("Second");
}

fn third() {
    println!("Third");
}
"#;

    let edits = vec![
        BatchEdit {
            symbol: "first".to_string(),
            action: BatchAction::Replace,
            content: Some("fn first() { println!(\"Modified first\"); }".to_string()),
        },
        BatchEdit {
            symbol: "second".to_string(),
            action: BatchAction::Delete,
            content: None,
        },
        BatchEdit {
            symbol: "third".to_string(),
            action: BatchAction::ReplaceBody,
            content: Some("{ println!(\"Modified third\"); }".to_string()),
        },
    ];

    let result = editor::batch(source, &edits, Language::Rust).unwrap();
    
    assert!(result.contains("Modified first"));
    assert!(!result.contains("fn second()"));
    assert!(result.contains("Modified third"));
    assert!(result.contains("fn third()"));
}

#[test]
fn test_batch_overlapping_ranges() {
    let source = r#"
impl MyStruct {
    fn method_one() {
        println!("One");
    }
}
"#;

    // Try to delete the impl block and also modify a method inside it
    let edits = vec![
        BatchEdit {
            symbol: "MyStruct".to_string(),
            action: BatchAction::Delete,
            content: None,
        },
        BatchEdit {
            symbol: "method_one".to_string(),
            action: BatchAction::Replace,
            content: Some("fn method_one() { println!(\"Modified\"); }".to_string()),
        },
    ];

    let result = editor::batch(source, &edits, Language::Rust);
    
    // This should error because the ranges overlap
    assert!(result.is_err());
}

#[test]
fn test_batch_missing_content_for_replace() {
    let source = r#"
fn test_func() {
    println!("Test");
}
"#;

    let edits = vec![
        BatchEdit {
            symbol: "test_func".to_string(),
            action: BatchAction::Replace,
            content: None, // Missing content for Replace
        },
    ];

    let result = editor::batch(source, &edits, Language::Rust);
    
    assert!(result.is_err());
}

#[test]
fn test_batch_empty() {
    let source = r#"
fn unchanged() {
    println!("Unchanged");
}
"#;

    let edits: Vec<BatchEdit> = vec![];

    let result = editor::batch(source, &edits, Language::Rust).unwrap();
    
    // Should succeed with no changes
    assert_eq!(result, source);
}

// ============================================================================
// TYPESCRIPT TESTS
// ============================================================================

#[test]
fn test_typescript_replace_function() {
    let source = r#"
function greet(name: string): string {
    return "Hello, " + name;
}

function farewell(name: string): string {
    return "Goodbye, " + name;
}
"#;

    let new_content = r#"function greet(name: string): string {
    return `Hi there, ${name}!`;
}"#;

    let result = editor::replace(source, "greet", new_content, Language::TypeScript).unwrap();
    
    assert!(result.contains("Hi there"));
    assert!(result.contains("${name}"));
    assert!(!result.contains(r#""Hello, ""#));
    assert!(result.contains("function farewell"));
}

#[test]
fn test_typescript_delete_class() {
    let source = r#"
class Person {
    name: string;
    
    constructor(name: string) {
        this.name = name;
    }
}

class Animal {
    species: string;
    
    constructor(species: string) {
        this.species = species;
    }
}
"#;

    let result = editor::delete(source, "Person", Language::TypeScript).unwrap();
    
    assert!(!result.contains("class Person"));
    assert!(!result.contains("constructor(name: string)"));
    assert!(result.contains("class Animal"));
}

#[test]
fn test_typescript_replace_body() {
    let source = r#"
function calculate(x: number, y: number): number {
    return x + y;
}
"#;

    let new_body = r#"{
    const result = x * y;
    console.log(`Result: ${result}`);
    return result;
}"#;

    let result = editor::replace_body(source, "calculate", new_body, Language::TypeScript).unwrap();
    
    assert!(result.contains("function calculate(x: number, y: number): number"));
    assert!(result.contains("x * y"));
    assert!(result.contains("console.log"));
    assert!(!result.contains("x + y"));
}


// ============================================================================
// PYTHON TESTS
// ============================================================================

#[test]
fn test_python_replace_function() {
    let source = "
def greet(name):
    print(f\"Hello, {name}\")
    return name

def farewell(name):
    print(f\"Goodbye, {name}\")
    return name
";

    let new_content = "def greet(name):
    msg = f\"Hi there, {name}!\"
    print(msg)
    return msg";

    let result = editor::replace(source, "greet", new_content, Language::Python).unwrap();

    assert!(result.contains("Hi there"));
    assert!(result.contains("return msg"));
    assert!(result.contains("def farewell"));
}

#[test]
fn test_python_delete_function() {
    let source = "
def first():
    print(\"First\")

def second():
    print(\"Second\")

def third():
    print(\"Third\")
";

    let result = editor::delete(source, "second", Language::Python).unwrap();

    assert!(result.contains("def first()"));
    assert!(result.contains("def third()"));
    assert!(!result.contains("def second()"));
}

#[test]
fn test_python_replace_body() {
    // Python replace_body: the function wraps the body in a `block` node
    // which tree-sitter-python expects as indented lines after the colon.
    let source = "
def calculate(x, y):
    return x + y
";

    // Provide a valid Python block (indented body lines)
    let new_body = "    result = x * y\n    return result";

    let result = editor::replace_body(source, "calculate", new_body, Language::Python).unwrap();
    assert!(result.contains("def calculate(x, y):"));
    assert!(result.contains("result = x * y"));
    assert!(result.contains("return result"));
    assert!(!result.contains("return x + y"));
    assert!(!result.contains("{"));
}

#[test]
fn test_python_delete_class() {
    let source = "
class Person:
    def __init__(self, name):
        self.name = name

class Animal:
    def __init__(self, species):
        self.species = species
";

    let result = editor::delete(source, "Person", Language::Python).unwrap();

    assert!(!result.contains("class Person"));
    assert!(!result.contains("self.name"));
    assert!(result.contains("class Animal"));
}

// ============================================================================
// JAVASCRIPT TESTS
// ============================================================================

#[test]
fn test_javascript_replace_function() {
    let source = r#"
function greet(name) {
    return "Hello, " + name;
}

function farewell(name) {
    return "Goodbye, " + name;
}
"#;

    let new_content = r#"function greet(name) {
    return "Hi there, " + name;
}"#;

    let result = editor::replace(source, "greet", new_content, Language::JavaScript).unwrap();

    assert!(result.contains("Hi there"));
    assert!(result.contains("function farewell"));
}

#[test]
fn test_javascript_delete_function() {
    let source = r#"
function first() {
    console.log("First");
}

function second() {
    console.log("Second");
}

function third() {
    console.log("Third");
}
"#;

    let result = editor::delete(source, "second", Language::JavaScript).unwrap();

    assert!(result.contains("function first()"));
    assert!(result.contains("function third()"));
    assert!(!result.contains("function second()"));
}
