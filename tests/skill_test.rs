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
    // Stub adapters should fail with "not yet implemented"
    for platform in &["claude-code", "codex", "cursor", "aider"] {
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
fn openclaw_install_and_uninstall() {
    // Install
    let output = codehud()
        .args(["install-skill", "openclaw"])
        .output()
        .expect("failed to run");
    assert!(output.status.success(), "install failed: {}", String::from_utf8_lossy(&output.stderr));

    let skill_path = dirs::home_dir().unwrap().join(".openclaw/workspace/skills/codehud/SKILL.md");
    assert!(skill_path.exists(), "SKILL.md should exist after install");

    let content = std::fs::read_to_string(&skill_path).unwrap();
    assert!(content.starts_with("---"));
    assert!(content.contains("name: codehud"));
    assert!(content.contains("Tree-sitter"));
    assert!(content.contains("codehud"));

    // Idempotent re-install
    let output2 = codehud()
        .args(["install-skill", "openclaw"])
        .output()
        .expect("failed to run");
    assert!(output2.status.success());

    // Uninstall
    let output3 = codehud()
        .args(["uninstall-skill", "openclaw"])
        .output()
        .expect("failed to run");
    assert!(output3.status.success(), "uninstall failed: {}", String::from_utf8_lossy(&output3.stderr));
    assert!(!skill_path.exists(), "SKILL.md should be removed after uninstall");
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
