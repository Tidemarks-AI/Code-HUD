//! Single File Component (SFC) preprocessing for `.vue`, `.svelte`, and `.astro` files.
//!
//! Extracts `<script>` blocks from template-based component formats and delegates
//! parsing to the appropriate TypeScript/JavaScript tree-sitter grammar.

use crate::languages::Language;
use regex::Regex;
use std::path::Path;
use std::sync::LazyLock;

/// A script block extracted from an SFC file.
#[derive(Debug, Clone)]
pub struct ScriptBlock {
    /// The script content (without the `<script>` tags).
    pub content: String,
    /// 1-based line number where the script content starts in the original file.
    pub start_line: usize,
    /// The language to parse this block as.
    pub language: Language,
    /// Whether this is a `<script setup>` block (Vue-specific).
    #[cfg_attr(not(test), allow(dead_code))]
    pub is_setup: bool,
}

/// Detected SFC format.
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum SfcKind {
    Vue,
    Svelte,
    Astro,
}

/// Check if a file path is an SFC format.
pub fn detect_sfc(path: &Path) -> Option<SfcKind> {
    let ext = path.extension()?.to_str()?;
    match ext {
        "vue" => Some(SfcKind::Vue),
        "svelte" => Some(SfcKind::Svelte),
        "astro" => Some(SfcKind::Astro),
        _ => None,
    }
}

/// Check if a file is an SFC format (for walk filtering).
pub fn is_sfc_file(path: &Path) -> bool {
    detect_sfc(path).is_some()
}

// Regex for matching <script> opening tags with optional attributes
static SCRIPT_OPEN_RE: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(r#"(?i)<script\b([^>]*)>"#).unwrap()
});

// Regex for matching </script> closing tags
static SCRIPT_CLOSE_RE: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(r#"(?i)</script\s*>"#).unwrap()
});

// Regex for Astro frontmatter delimiters
static ASTRO_FENCE_RE: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(r#"^---\s*$"#).unwrap()
});

/// Detect language from script tag attributes (e.g., `lang="ts"`).
fn detect_script_lang(attrs: &str) -> Language {
    let lower = attrs.to_lowercase();
    if lower.contains("lang=\"ts\"") || lower.contains("lang='ts'")
        || lower.contains("lang=\"typescript\"") || lower.contains("lang='typescript'")
    {
        Language::TypeScript
    } else if lower.contains("lang=\"tsx\"") || lower.contains("lang='tsx'") {
        Language::Tsx
    } else if lower.contains("lang=\"jsx\"") || lower.contains("lang='jsx'") {
        Language::Jsx
    } else {
        // Default: TypeScript is a superset of JS, so it safely parses both
        Language::TypeScript
    }
}

/// Check if script tag has `setup` attribute (Vue `<script setup>`).
fn is_setup_script(attrs: &str) -> bool {
    attrs.contains("setup")
}

/// Extract script blocks from Vue or Svelte SFC content.
fn extract_script_blocks(source: &str, _kind: SfcKind) -> Vec<ScriptBlock> {
    let lines: Vec<&str> = source.lines().collect();
    let mut blocks = Vec::new();
    let mut i = 0;

    while i < lines.len() {
        if let Some(caps) = SCRIPT_OPEN_RE.captures(lines[i]) {
            let attrs = caps.get(1).map_or("", |m| m.as_str());
            let language = detect_script_lang(attrs);
            let is_setup = is_setup_script(attrs);
            let start_line = i + 2; // 1-based, content starts on next line

            // Find closing </script>
            let mut j = i + 1;
            let mut script_lines = Vec::new();
            while j < lines.len() {
                if SCRIPT_CLOSE_RE.is_match(lines[j]) {
                    break;
                }
                script_lines.push(lines[j]);
                j += 1;
            }

            if !script_lines.is_empty() {
                blocks.push(ScriptBlock {
                    content: script_lines.join("\n"),
                    start_line,
                    language,
                    is_setup,
                });
            }

            i = j + 1;
        } else {
            i += 1;
        }
    }

    blocks
}

/// Extract frontmatter block from Astro files (content between `---` fences).
fn extract_astro_frontmatter(source: &str) -> Vec<ScriptBlock> {
    let lines: Vec<&str> = source.lines().collect();
    let mut blocks = Vec::new();

    // Find first `---`
    let mut i = 0;
    while i < lines.len() {
        if ASTRO_FENCE_RE.is_match(lines[i].trim()) {
            let start_line = i + 2; // 1-based, content starts on next line
            let mut j = i + 1;
            let mut script_lines = Vec::new();

            while j < lines.len() {
                if ASTRO_FENCE_RE.is_match(lines[j].trim()) {
                    break;
                }
                script_lines.push(lines[j]);
                j += 1;
            }

            if !script_lines.is_empty() {
                blocks.push(ScriptBlock {
                    content: script_lines.join("\n"),
                    start_line,
                    language: Language::TypeScript,
                    is_setup: false,
                });
            }
            break; // Astro only has one frontmatter block
        }
        i += 1;
    }

    // Also check for <script> tags in Astro (client-side scripts)
    blocks.extend(extract_script_blocks(source, SfcKind::Astro));

    blocks
}

/// Extract all script blocks from an SFC file.
pub fn extract_scripts(source: &str, kind: SfcKind) -> Vec<ScriptBlock> {
    match kind {
        SfcKind::Astro => extract_astro_frontmatter(source),
        SfcKind::Vue | SfcKind::Svelte => extract_script_blocks(source, kind),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn vue_script_setup_ts() {
        let source = r#"<script setup lang="ts">
import { ref } from 'vue'

const count = ref(0)

function increment() {
  count.value++
}
</script>

<template>
  <button @click="increment">{{ count }}</button>
</template>
"#;
        let blocks = extract_scripts(source, SfcKind::Vue);
        assert_eq!(blocks.len(), 1);
        assert_eq!(blocks[0].start_line, 2);
        assert_eq!(blocks[0].language, Language::TypeScript);
        assert!(blocks[0].is_setup);
        assert!(blocks[0].content.contains("import { ref }"));
        assert!(blocks[0].content.contains("function increment()"));
    }

    #[test]
    fn vue_two_script_blocks() {
        let source = r#"<script lang="ts">
export default {
  name: 'MyComponent'
}
</script>

<script setup lang="ts">
import { ref } from 'vue'
const count = ref(0)
</script>

<template>
  <div>{{ count }}</div>
</template>
"#;
        let blocks = extract_scripts(source, SfcKind::Vue);
        assert_eq!(blocks.len(), 2);
        assert!(!blocks[0].is_setup);
        assert!(blocks[1].is_setup);
    }

    #[test]
    fn vue_no_lang_defaults_to_ts() {
        let source = r#"<script>
export default {}
</script>
"#;
        let blocks = extract_scripts(source, SfcKind::Vue);
        assert_eq!(blocks.len(), 1);
        assert_eq!(blocks[0].language, Language::TypeScript);
    }

    #[test]
    fn svelte_script() {
        let source = r#"<script lang="ts">
  let count = 0;

  function increment() {
    count += 1;
  }
</script>

<button on:click={increment}>
  {count}
</button>
"#;
        let blocks = extract_scripts(source, SfcKind::Svelte);
        assert_eq!(blocks.len(), 1);
        assert!(blocks[0].content.contains("let count = 0"));
    }

    #[test]
    fn astro_frontmatter() {
        let source = r#"---
import Layout from '../layouts/Layout.astro';
import Card from '../components/Card.astro';

const title = "My Site";
---

<Layout title={title}>
  <main>
    <h1>Welcome</h1>
  </main>
</Layout>
"#;
        let blocks = extract_scripts(source, SfcKind::Astro);
        assert_eq!(blocks.len(), 1);
        assert_eq!(blocks[0].start_line, 2);
        assert!(blocks[0].content.contains("import Layout"));
        assert!(blocks[0].content.contains("const title"));
    }

    #[test]
    fn astro_with_client_script() {
        let source = r#"---
const title = "Hello";
---

<h1>{title}</h1>

<script>
  document.addEventListener('click', () => {
    console.log('clicked');
  });
</script>
"#;
        let blocks = extract_scripts(source, SfcKind::Astro);
        assert_eq!(blocks.len(), 2);
        assert!(blocks[0].content.contains("const title"));
        assert!(blocks[1].content.contains("document.addEventListener"));
    }

    #[test]
    fn detect_sfc_extensions() {
        assert_eq!(detect_sfc(Path::new("App.vue")), Some(SfcKind::Vue));
        assert_eq!(detect_sfc(Path::new("Counter.svelte")), Some(SfcKind::Svelte));
        assert_eq!(detect_sfc(Path::new("index.astro")), Some(SfcKind::Astro));
        assert_eq!(detect_sfc(Path::new("main.ts")), None);
    }

    #[test]
    fn empty_script_block_skipped() {
        let source = r#"<script lang="ts">
</script>
"#;
        let blocks = extract_scripts(source, SfcKind::Vue);
        assert_eq!(blocks.len(), 0);
    }
}
