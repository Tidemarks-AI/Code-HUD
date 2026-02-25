use std::process::Command;

fn codehud() -> Command {
    Command::new(env!("CARGO_BIN_EXE_codehud"))
}

#[test]
fn install_skill_list_shows_platforms() {
    let output = codehud()
        .args(["install-skill", "--list"])
        .output()
        .expect("failed to run");
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(output.status.success());
    assert!(stdout.contains("openclaw"));
    assert!(stdout.contains("claude-code"));
    assert!(stdout.contains("codex"));
    assert!(stdout.contains("cursor"));
    assert!(stdout.contains("aider"));
}

#[test]
fn install_skill_unknown_platform_gives_error() {
    let output = codehud()
        .args(["install-skill", "foobar"])
        .output()
        .expect("failed to run");
    assert!(!output.status.success());
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(stderr.contains("Unknown platform 'foobar'"));
    assert!(stderr.contains("Available platforms"));
}

#[test]
fn install_uninstall_stubs_return_not_implemented() {
    // All adapters are stubs for now, so install should fail with "not yet implemented"
    for platform in &["openclaw", "claude-code", "codex", "cursor", "aider"] {
        let output = codehud()
            .args(["install-skill", platform])
            .output()
            .expect("failed to run");
        assert!(!output.status.success(), "expected failure for {}", platform);
        let stderr = String::from_utf8_lossy(&output.stderr);
        assert!(stderr.contains("not yet implemented"), "platform {} stderr: {}", platform, stderr);
    }
}

#[test]
fn uninstall_skill_unknown_platform_gives_error() {
    let output = codehud()
        .args(["uninstall-skill", "foobar"])
        .output()
        .expect("failed to run");
    assert!(!output.status.success());
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(stderr.contains("Unknown platform 'foobar'"));
}

#[test]
fn skill_content_compiled_in() {
    // Verify the content module works
    assert!(!codehud::skill::content::SKILL_CONTENT.is_empty());
    assert!(codehud::skill::content::SKILL_CONTENT.contains("codehud"));
    assert!(codehud::skill::content::SKILL_CONTENT.contains("Tree-sitter"));
}
