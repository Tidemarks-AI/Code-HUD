use std::fs;
use tempfile::TempDir;

fn run_codehud(args: &[&str]) -> String {
    let bin = env!("CARGO_BIN_EXE_codehud");
    let output = std::process::Command::new(bin)
        .args(args)
        .output()
        .expect("failed to run codehud");
    String::from_utf8(output.stdout).unwrap()
}

fn write_ts_file(dir: &TempDir, name: &str, content: &str) -> String {
    let path = dir.path().join(name);
    fs::write(&path, content).unwrap();
    path.to_string_lossy().to_string()
}

const BASIC_CLASS: &str = r#"class Greeter {
  name: string;

  constructor(name: string) {
    this.name = name;
  }

  greet(): string {
    return `Hello, ${this.name}!`;
  }

  farewell(): string {
    return `Goodbye, ${this.name}!`;
  }
}
"#;

#[test]
fn signatures_basic_class() {
    let dir = TempDir::new().unwrap();
    let path = write_ts_file(&dir, "basic.ts", BASIC_CLASS);
    let output = run_codehud(&[&path, "Greeter", "--signatures"]);
    // Should show class with collapsed bodies
    assert!(output.contains("class Greeter {"));
    assert!(output.contains("constructor(name: string) { ... }"));
    assert!(output.contains("greet(): string { ... }"));
    assert!(output.contains("farewell(): string { ... }"));
    // Should NOT contain method bodies
    assert!(!output.contains("Hello,"));
    assert!(!output.contains("Goodbye,"));
}

#[test]
fn signatures_exported_class() {
    let dir = TempDir::new().unwrap();
    let src = format!("export {}", BASIC_CLASS);
    let path = write_ts_file(&dir, "exported.ts", &src);
    let output = run_codehud(&[&path, "Greeter", "--signatures"]);
    assert!(output.contains("class Greeter {"));
    assert!(output.contains("greet(): string { ... }"));
    assert!(!output.contains("Hello,"));
}

const DECORATED_CLASS: &str = r#"function Component(target: any) {}

@Component
class MyWidget {
  render(): string {
    return '<div>widget</div>';
  }

  update(data: any): void {
    console.log(data);
  }
}
"#;

#[test]
fn signatures_decorated_class() {
    let dir = TempDir::new().unwrap();
    let path = write_ts_file(&dir, "decorated.tsx", DECORATED_CLASS);
    let output = run_codehud(&[&path, "MyWidget", "--signatures"]);
    assert!(output.contains("@Component"));
    assert!(output.contains("class MyWidget {"));
    assert!(output.contains("render(): string { ... }"));
    assert!(!output.contains("<div>widget</div>"));
}

#[test]
fn signatures_combined_expand_method() {
    let dir = TempDir::new().unwrap();
    let path = write_ts_file(&dir, "combined.ts", BASIC_CLASS);
    let output = run_codehud(&[&path, "Greeter", "--signatures", "greet"]);
    // greet should be fully expanded
    assert!(output.contains("Hello,"));
    // farewell should still be collapsed
    assert!(output.contains("farewell(): string { ... }"));
    assert!(!output.contains("Goodbye,"));
}

#[test]
fn signatures_combined_multiple_methods() {
    let dir = TempDir::new().unwrap();
    let path = write_ts_file(&dir, "multi.ts", BASIC_CLASS);
    let output = run_codehud(&[&path, "Greeter", "--signatures", "greet", "farewell"]);
    // Both should be expanded
    assert!(output.contains("Hello,"));
    assert!(output.contains("Goodbye,"));
    // Constructor should still be collapsed
    assert!(output.contains("constructor(name: string) { ... }"));
}

#[test]
fn signatures_preserves_properties() {
    let dir = TempDir::new().unwrap();
    let path = write_ts_file(&dir, "props.ts", BASIC_CLASS);
    let output = run_codehud(&[&path, "Greeter", "--signatures"]);
    assert!(output.contains("name: string;"));
}
