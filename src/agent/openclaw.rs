use super::{AgentAdapter, AgentError};
use crate::skill::content::SKILL_CONTENT;
use std::fs;
use std::path::PathBuf;

pub struct OpenClawAdapter;

const AGENT_ID: &str = "codehud";

const SOUL_MD: &str = r#"# SOUL.md — Code HUD

You are a code exploration specialist. You use the `codehud` CLI for all structural code intelligence.

## Core Principles

- **Structure over raw text** — always prefer `codehud` outlines, symbol lists, and references over reading entire files
- **Concise and technical** — no filler, no pleasantries, just precise structural answers
- **Stay focused** — you are not a general-purpose assistant. You explore code. That's it.

## How You Work

1. Use `codehud <path>` for file/directory overviews
2. Use `codehud <path> <symbol>` to expand specific symbols
3. Use `codehud <path> --outline` for signatures and docstrings
4. Use `codehud <path> --search <pattern>` for structural search
5. Use `codehud <path> --references <symbol>` for reference finding
6. Use `codehud <path> --xrefs <symbol>` for cross-file references
7. Use `codehud <path> --diff` for structural diffs

## Vibe

Methodical. Precise. Like a well-indexed codebase — everything in its place, nothing wasted.
"#;

const IDENTITY_MD: &str = r#"# IDENTITY.md

- **Name:** Code HUD
- **Creature:** Code exploration specialist
- **Emoji:** 🔬
- **Vibe:** Precise, structural, focused. A code microscope, not a Swiss army knife.
"#;

const AGENTS_MD: &str = r#"# AGENTS.md — Code HUD

This agent uses the `codehud` CLI for structural code intelligence.

## Available Commands

| Command | Description |
|---|---|
| `codehud <path>` | File/directory overview with collapsed symbols |
| `codehud <path> <symbol>` | Expand a specific symbol |
| `codehud <path> --outline` | Signatures + docstrings without bodies |
| `codehud <path> --list-symbols` | Compact symbol listing |
| `codehud <path> --search <pat>` | Structural search with context |
| `codehud <path> --references <sym>` | Find all references to a symbol |
| `codehud <path> --xrefs <sym>` | Cross-file reference search |
| `codehud <path> --diff` | Structural diff against git HEAD |
| `codehud <path> --tree` | Smart directory tree |
| `codehud edit <file> <sym> --replace <code>` | AST-aware code editing |

## Workflow

1. Start with `--tree` or `--files` to understand project layout
2. Use outlines to understand module structure
3. Expand specific symbols for implementation details
4. Use references/xrefs to trace dependencies
"#;

const SKILL_FRONTMATTER: &str = r#"---
name: codehud
description: "Tree-sitter powered structural code intelligence. Use for code exploration, symbol lookup, cross-references, and structural diff."
metadata:
  openclaw:
    emoji: "🔬"
    requires:
      bins: ["codehud"]
    install:
      - id: cargo
        kind: shell
        command: "cargo install codehud"
        bins: ["codehud"]
        label: "Install codehud (cargo)"
      - id: script
        kind: shell
        command: "curl -fsSL https://raw.githubusercontent.com/Tidemarks-AI/Code-HUD/main/install.sh | sh"
        bins: ["codehud"]
        label: "Install codehud (install script)"
---
"#;

fn home_dir() -> PathBuf {
    dirs::home_dir().expect("could not determine home directory")
}

fn workspace_dir() -> PathBuf {
    home_dir().join(".openclaw/workspace-codehud")
}

fn config_path() -> PathBuf {
    home_dir().join(".openclaw/openclaw.json")
}

fn state_dir() -> PathBuf {
    home_dir().join(".openclaw/agents/codehud/agent")
}

/// Read the openclaw.json config as a serde_json::Value.
fn read_config() -> Result<serde_json::Value, AgentError> {
    let path = config_path();
    if !path.exists() {
        return Err(AgentError::Config(format!(
            "Config file not found: {}. Is OpenClaw installed?",
            path.display()
        )));
    }
    let content = fs::read_to_string(&path)?;
    let config: serde_json::Value = serde_json::from_str(&content)?;
    Ok(config)
}

/// Write the config back to openclaw.json with pretty formatting.
fn write_config(config: &serde_json::Value) -> Result<(), AgentError> {
    let content = serde_json::to_string_pretty(config)?;
    fs::write(config_path(), content)?;
    Ok(())
}

/// Add the codehud agent entry to agents.list[] if not already present.
fn add_agent_to_config(config: &mut serde_json::Value) -> Result<bool, AgentError> {
    let agents_list = config
        .pointer_mut("/agents/list")
        .and_then(|v| v.as_array_mut())
        .ok_or_else(|| AgentError::Config("agents.list not found in config".to_string()))?;

    // Check if already present
    for entry in agents_list.iter() {
        if entry.get("id").and_then(|v| v.as_str()) == Some(AGENT_ID) {
            return Ok(false); // already exists
        }
    }

    let agent_entry = serde_json::json!({
        "id": AGENT_ID,
        "name": "Code HUD",
        "workspace": "~/.openclaw/workspace-codehud",
        "model": "anthropic/claude-sonnet-4-5",
        "skills": ["codehud"],
        "identity": {
            "name": "Code HUD",
            "emoji": "🔬"
        },
        "tools": {
            "allow": ["exec", "read", "write", "edit", "web_search", "web_fetch"]
        }
    });

    agents_list.push(agent_entry);
    Ok(true)
}

/// Remove the codehud agent entry from agents.list[].
fn remove_agent_from_config(config: &mut serde_json::Value) -> Result<bool, AgentError> {
    let agents_list = config
        .pointer_mut("/agents/list")
        .and_then(|v| v.as_array_mut())
        .ok_or_else(|| AgentError::Config("agents.list not found in config".to_string()))?;

    let before = agents_list.len();
    agents_list.retain(|entry| {
        entry.get("id").and_then(|v| v.as_str()) != Some(AGENT_ID)
    });
    Ok(agents_list.len() < before)
}

/// Add "codehud" to the main agent's subagents.allowAgents array.
fn add_to_spawn_allowlist(config: &mut serde_json::Value) -> Result<bool, AgentError> {
    let agents_list = config
        .pointer_mut("/agents/list")
        .and_then(|v| v.as_array_mut())
        .ok_or_else(|| AgentError::Config("agents.list not found in config".to_string()))?;

    // Find the main/default agent
    for entry in agents_list.iter_mut() {
        let is_main = entry.get("default").and_then(|v| v.as_bool()).unwrap_or(false)
            || entry.get("id").and_then(|v| v.as_str()) == Some("main");

        if is_main {
            // Navigate to subagents.allowAgents, creating if needed
            let subagents = entry
                .as_object_mut()
                .unwrap()
                .entry("subagents")
                .or_insert_with(|| serde_json::json!({}));
            let allow = subagents
                .as_object_mut()
                .unwrap()
                .entry("allowAgents")
                .or_insert_with(|| serde_json::json!([]));
            let arr = allow.as_array_mut().unwrap();

            if arr.iter().any(|v| v.as_str() == Some(AGENT_ID)) {
                return Ok(false); // already present
            }
            arr.push(serde_json::Value::String(AGENT_ID.to_string()));
            return Ok(true);
        }
    }
    Ok(false) // no main agent found
}

/// Remove "codehud" from the main agent's subagents.allowAgents array.
fn remove_from_spawn_allowlist(config: &mut serde_json::Value) -> Result<bool, AgentError> {
    let agents_list = config
        .pointer_mut("/agents/list")
        .and_then(|v| v.as_array_mut())
        .ok_or_else(|| AgentError::Config("agents.list not found in config".to_string()))?;

    for entry in agents_list.iter_mut() {
        let is_main = entry.get("default").and_then(|v| v.as_bool()).unwrap_or(false)
            || entry.get("id").and_then(|v| v.as_str()) == Some("main");

        if is_main {
            if let Some(arr) = entry
                .pointer_mut("/subagents/allowAgents")
                .and_then(|v| v.as_array_mut())
            {
                let before = arr.len();
                arr.retain(|v| v.as_str() != Some(AGENT_ID));
                return Ok(arr.len() < before);
            }
            return Ok(false);
        }
    }
    Ok(false)
}

impl AgentAdapter for OpenClawAdapter {
    fn install(&self) -> Result<(), AgentError> {
        let ws = workspace_dir();

        // 1. Create workspace and write files
        fs::create_dir_all(&ws)?;
        fs::write(ws.join("SOUL.md"), SOUL_MD.trim())?;
        fs::write(ws.join("IDENTITY.md"), IDENTITY_MD.trim())?;
        fs::write(ws.join("AGENTS.md"), AGENTS_MD.trim())?;
        println!("✓ Created workspace at {}", ws.display());

        // 2. Install codehud skill into agent workspace
        let skill_dir = ws.join("skills/codehud");
        fs::create_dir_all(&skill_dir)?;
        let skill_content = format!("{}\n{}\n", SKILL_FRONTMATTER.trim(), SKILL_CONTENT.trim());
        fs::write(skill_dir.join("SKILL.md"), skill_content)?;
        println!("✓ Installed codehud skill to {}", skill_dir.display());

        // 3. Create agent state directory
        let state = state_dir();
        fs::create_dir_all(&state)?;
        println!("✓ Created agent state dir at {}", state.display());

        // 4. Register in openclaw.json
        let mut config = read_config()?;
        let added = add_agent_to_config(&mut config)?;
        if added {
            println!("✓ Registered codehud agent in openclaw.json");
        } else {
            println!("  codehud agent already registered in openclaw.json");
        }

        let allowlisted = add_to_spawn_allowlist(&mut config)?;
        if allowlisted {
            println!("✓ Added codehud to main agent spawn allowlist");
        } else {
            println!("  codehud already in main agent spawn allowlist");
        }

        write_config(&config)?;

        println!();
        println!("Run 'openclaw gateway restart' to activate the agent");
        Ok(())
    }

    fn uninstall(&self, force: bool) -> Result<(), AgentError> {
        // 1. Update openclaw.json
        let mut config = read_config()?;
        let removed = remove_agent_from_config(&mut config)?;
        if removed {
            println!("✓ Removed codehud agent from openclaw.json");
        } else {
            println!("  codehud agent not found in openclaw.json");
        }

        let unlisted = remove_from_spawn_allowlist(&mut config)?;
        if unlisted {
            println!("✓ Removed codehud from main agent spawn allowlist");
        } else {
            println!("  codehud not in main agent spawn allowlist");
        }

        write_config(&config)?;

        // 2. Remove workspace (only with --force)
        let ws = workspace_dir();
        if ws.exists() {
            if force {
                fs::remove_dir_all(&ws)?;
                println!("✓ Removed workspace at {}", ws.display());
            } else {
                println!("  Workspace preserved at {} (use --force to remove)", ws.display());
            }
        }

        // 3. Remove state dir
        let state = state_dir();
        if state.exists() {
            fs::remove_dir_all(&state)?;
            println!("✓ Removed agent state dir");
        }

        println!();
        println!("Run 'openclaw gateway restart' to apply changes");
        Ok(())
    }

    fn name(&self) -> &'static str {
        "OpenClaw"
    }
}
