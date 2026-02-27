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
    for platform in &["aider"] {
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
fn cursor_install_and_uninstall() {
    let tmp = tempfile::tempdir().unwrap();
    let output = codehud()
        .args(["install-skill", "cursor"])
        .current_dir(tmp.path())
        .output()
        .expect("failed to run");
    assert!(output.status.success(), "install failed: {}", String::from_utf8_lossy(&output.stderr));

    let mdc_path = tmp.path().join(".cursor/rules/codehud.mdc");
    assert!(mdc_path.exists(), "codehud.mdc should exist after install");

    let content = std::fs::read_to_string(&mdc_path).unwrap();
    assert!(content.starts_with("---"));
    assert!(content.contains("alwaysApply: true"));
    assert!(content.contains("codehud"));
    assert!(content.contains("Tree-sitter"));

    // Idempotent re-install
    let output2 = codehud()
        .args(["install-skill", "cursor"])
        .current_dir(tmp.path())
        .output()
        .expect("failed to run");
    assert!(output2.status.success());

    // Uninstall
    let output3 = codehud()
        .args(["uninstall-skill", "cursor"])
        .current_dir(tmp.path())
        .output()
        .expect("failed to run");
    assert!(output3.status.success(), "uninstall failed: {}", String::from_utf8_lossy(&output3.stderr));
    assert!(!mdc_path.exists(), "codehud.mdc should be removed after uninstall");
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
fn codex_install_creates_agents_md() {
    let dir = tempfile::tempdir().unwrap();
    let output = codehud()
        .args(["install-skill", "codex"])
        .current_dir(dir.path())
        .output()
        .expect("failed to run");
    assert!(output.status.success(), "install failed: {}", String::from_utf8_lossy(&output.stderr));

    let agents = dir.path().join("AGENTS.md");
    assert!(agents.exists(), "AGENTS.md should be created");
    let content = std::fs::read_to_string(&agents).unwrap();
    assert!(content.contains("<!-- codehud:start -->"));
    assert!(content.contains("<!-- codehud:end -->"));
    assert!(content.contains("codehud"));
}

#[test]
fn codex_install_prefers_codex_md() {
    let dir = tempfile::tempdir().unwrap();
    std::fs::write(dir.path().join("codex.md"), "# Codex\n").unwrap();

    let output = codehud()
        .args(["install-skill", "codex"])
        .current_dir(dir.path())
        .output()
        .expect("failed to run");
    assert!(output.status.success());

    // Should write to codex.md, not AGENTS.md
    assert!(!dir.path().join("AGENTS.md").exists());
    let content = std::fs::read_to_string(dir.path().join("codex.md")).unwrap();
    assert!(content.contains("<!-- codehud:start -->"));
    assert!(content.contains("# Codex"));
}

#[test]
fn codex_install_is_idempotent() {
    let dir = tempfile::tempdir().unwrap();

    // Install twice
    for _ in 0..2 {
        let output = codehud()
            .args(["install-skill", "codex"])
            .current_dir(dir.path())
            .output()
            .expect("failed to run");
        assert!(output.status.success());
    }

    let content = std::fs::read_to_string(dir.path().join("AGENTS.md")).unwrap();
    // Should have exactly one start and one end delimiter
    assert_eq!(content.matches("<!-- codehud:start -->").count(), 1);
    assert_eq!(content.matches("<!-- codehud:end -->").count(), 1);
}

#[test]
fn codex_uninstall_removes_block() {
    let dir = tempfile::tempdir().unwrap();

    // Install then uninstall
    codehud()
        .args(["install-skill", "codex"])
        .current_dir(dir.path())
        .output()
        .expect("failed to run");

    let output = codehud()
        .args(["uninstall-skill", "codex"])
        .current_dir(dir.path())
        .output()
        .expect("failed to run");
    assert!(output.status.success());

    // File should be removed (was only codehud content)
    assert!(!dir.path().join("AGENTS.md").exists());
}

#[test]
fn codex_uninstall_preserves_other_content() {
    let dir = tempfile::tempdir().unwrap();
    std::fs::write(dir.path().join("AGENTS.md"), "# My Project\n\nSome instructions.\n").unwrap();

    codehud()
        .args(["install-skill", "codex"])
        .current_dir(dir.path())
        .output()
        .expect("failed to run");

    codehud()
        .args(["uninstall-skill", "codex"])
        .current_dir(dir.path())
        .output()
        .expect("failed to run");

    let content = std::fs::read_to_string(dir.path().join("AGENTS.md")).unwrap();
    assert!(content.contains("# My Project"));
    assert!(!content.contains("<!-- codehud:start -->"));
}

#[test]
fn skill_content_compiled_in() {
    // Verify the content module works
    assert!(!codehud::skill::content::SKILL_CONTENT.is_empty());
    assert!(codehud::skill::content::SKILL_CONTENT.contains("codehud"));
    assert!(codehud::skill::content::SKILL_CONTENT.contains("Tree-sitter"));
}
