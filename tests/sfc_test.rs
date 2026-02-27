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
    }
}

fn write_file(ext: &str, content: &str) -> NamedTempFile {
    let mut f = tempfile::Builder::new().suffix(ext).tempfile().unwrap();
    f.write_all(content.as_bytes()).unwrap();
    f.flush().unwrap();
    f
}

// --- Vue tests ---

const VUE_SCRIPT_SETUP: &str = r#"<script setup lang="ts">
import { ref, computed } from 'vue'

interface Props {
  title: string
  count?: number
}

const props = defineProps<Props>()

const doubled = computed(() => (props.count ?? 0) * 2)

function increment() {
  console.log('increment')
}
</script>

<template>
  <div>
    <h1>{{ props.title }}</h1>
    <span>{{ doubled }}</span>
  </div>
</template>
"#;

#[test]
fn vue_script_setup_interface_mode() {
    let f = write_file(".vue", VUE_SCRIPT_SETUP);
    let out = process_path(f.path().to_str().unwrap(), opts()).unwrap();
    // Should extract the interface, const, and function
    assert!(out.contains("interface Props"), "Should find Props interface: {out}");
    assert!(out.contains("increment"), "Should find increment function: {out}");
}

#[test]
fn vue_script_setup_expand_symbol() {
    let f = write_file(".vue", VUE_SCRIPT_SETUP);
    let mut o = opts();
    o.symbols = vec!["Props".to_string()];
    let out = process_path(f.path().to_str().unwrap(), o).unwrap();
    assert!(out.contains("title: string"), "Should expand Props: {out}");
}

#[test]
fn vue_line_numbers_offset() {
    // Script starts at line 2 in the .vue file, so items should have offset line numbers
    let f = write_file(".vue", VUE_SCRIPT_SETUP);
    let mut o = opts();
    o.format = OutputFormat::Json;
    let out = process_path(f.path().to_str().unwrap(), o).unwrap();
    let json: serde_json::Value = serde_json::from_str(&out).unwrap();
    let files = json["files"].as_array().unwrap();
    assert!(!files.is_empty());
    let items = files[0]["items"].as_array().unwrap();
    // Props interface starts at line 4 in the .vue (line 3 in script + 1 offset)
    let props_item = items.iter().find(|i| i["name"] == "Props").unwrap();
    assert!(props_item["line_start"].as_u64().unwrap() >= 4, "Props should be at line >= 4: {}", props_item);
}

const VUE_OPTIONS_API: &str = r#"<script lang="ts">
import { defineComponent } from 'vue'

export default defineComponent({
  name: 'MyComponent',
  props: {
    msg: String
  },
  setup() {
    return {}
  }
})
</script>

<template>
  <div>{{ msg }}</div>
</template>
"#;

#[test]
fn vue_options_api() {
    let f = write_file(".vue", VUE_OPTIONS_API);
    let out = process_path(f.path().to_str().unwrap(), opts()).unwrap();
    assert!(out.contains("defineComponent") || out.contains("import"), "Should parse options API: {out}");
}

const VUE_TWO_SCRIPTS: &str = r#"<script lang="ts">
export interface SharedType {
  id: number
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

#[test]
fn vue_two_script_blocks() {
    let f = write_file(".vue", VUE_TWO_SCRIPTS);
    let out = process_path(f.path().to_str().unwrap(), opts()).unwrap();
    assert!(out.contains("SharedType"), "Should find SharedType from first script: {out}");
}

// --- Svelte tests ---

const SVELTE_COMPONENT: &str = r#"<script lang="ts">
  export let name: string = 'world';
  
  interface Item {
    id: number;
    label: string;
  }
  
  function handleClick() {
    console.log('clicked');
  }
</script>

<main>
  <h1>Hello {name}!</h1>
  <button on:click={handleClick}>Click</button>
</main>
"#;

#[test]
fn svelte_script_extraction() {
    let f = write_file(".svelte", SVELTE_COMPONENT);
    let out = process_path(f.path().to_str().unwrap(), opts()).unwrap();
    assert!(out.contains("Item"), "Should find Item interface: {out}");
    assert!(out.contains("handleClick"), "Should find handleClick: {out}");
}

// --- Astro tests ---

const ASTRO_PAGE: &str = r#"---
import Layout from '../layouts/Layout.astro';

interface Props {
  title: string;
}

const { title } = Astro.props;
---

<Layout title={title}>
  <main>
    <h1>{title}</h1>
  </main>
</Layout>
"#;

#[test]
fn astro_frontmatter_extraction() {
    let f = write_file(".astro", ASTRO_PAGE);
    let out = process_path(f.path().to_str().unwrap(), opts()).unwrap();
    assert!(out.contains("Props"), "Should find Props interface: {out}");
}

// --- Directory walk tests ---

#[test]
fn directory_walk_includes_vue_files() {
    let dir = tempfile::TempDir::new().unwrap();
    std::fs::write(dir.path().join("App.vue"), VUE_SCRIPT_SETUP).unwrap();
    std::fs::write(dir.path().join("main.ts"), "export const x = 1;").unwrap();
    let out = process_path(dir.path().to_str().unwrap(), opts()).unwrap();
    // Both files should be processed
    assert!(out.contains("App.vue"), "Should include Vue file: {out}");
    assert!(out.contains("main.ts"), "Should include TS file: {out}");
}

// --- Empty/template-only Vue file ---

#[test]
fn vue_template_only_passthrough() {
    let source = r#"<template>
  <div>Hello</div>
</template>
"#;
    let f = write_file(".vue", source);
    let out = process_path(f.path().to_str().unwrap(), opts()).unwrap();
    // Should still produce output (passthrough of the whole file)
    assert!(out.contains("template") || out.contains("Hello"), "Should passthrough: {out}");
}

// --- List symbols ---

#[test]
fn vue_list_symbols() {
    let f = write_file(".vue", VUE_SCRIPT_SETUP);
    let mut o = opts();
    o.list_symbols = true;
    let out = process_path(f.path().to_str().unwrap(), o).unwrap();
    assert!(out.contains("Props"), "Should list Props symbol: {out}");
    assert!(out.contains("increment"), "Should list increment symbol: {out}");
}

// --- Directory scanning tests for SFC files ---

#[test]
fn sfc_directory_stats_finds_vue_files() {
    let dir = tempfile::TempDir::new().unwrap();
    std::fs::write(dir.path().join("App.vue"), r#"<script setup lang="ts">
const msg = 'hello'
</script>
<template><div>{{ msg }}</div></template>"#).unwrap();
    std::fs::write(dir.path().join("main.ts"), "import App from './App.vue'").unwrap();

    let mut o = opts();
    o.stats = true;
    o.ext = vec!["vue".to_string()];
    let out = process_path(dir.path().to_str().unwrap(), o).unwrap();
    assert!(out.contains("Files: 1"), "Should find 1 vue file: {out}");
    assert!(out.contains("vue"), "Should show vue language: {out}");
}

#[test]
fn sfc_directory_stats_finds_all_sfc_types() {
    let dir = tempfile::TempDir::new().unwrap();
    std::fs::write(dir.path().join("App.vue"), "<script>export default {}</script>").unwrap();
    std::fs::write(dir.path().join("Counter.svelte"), "<script>let count = 0</script>").unwrap();
    std::fs::write(dir.path().join("Index.astro"), "---\nconst x = 1\n---\n<div/>").unwrap();

    let mut o = opts();
    o.stats = true;
    let out = process_path(dir.path().to_str().unwrap(), o).unwrap();
    assert!(out.contains("Files: 3"), "Should find 3 files: {out}");
    assert!(out.contains("vue"), "Should show vue: {out}");
    assert!(out.contains("svelte"), "Should show svelte: {out}");
    assert!(out.contains("astro"), "Should show astro: {out}");
}

#[test]
fn sfc_directory_search_finds_vue_content() {
    let dir = tempfile::TempDir::new().unwrap();
    std::fs::write(dir.path().join("App.vue"), r#"<script setup lang="ts">
import { ref } from 'vue'
const count = ref(0)
function increment() { count.value++ }
</script>
<template><button @click="increment">{{ count }}</button></template>"#).unwrap();

    let out = codehud::search::search_path(
        dir.path().to_str().unwrap(),
        &codehud::search::SearchOptions {
            pattern: "increment".to_string(),
            regex: false,
            case_insensitive: false,
            depth: None,
            ext: vec!["vue".to_string()],
            max_results: None,
            no_tests: false,
            exclude: vec![],
            json: false,
            context: None,
            summary: false,
        },
    ).unwrap();
    assert!(out.contains("increment"), "Should find increment in Vue file: {out}");
    assert!(out.contains("App.vue"), "Should show Vue filename: {out}");
}

#[test]
fn sfc_directory_search_finds_svelte_content() {
    let dir = tempfile::TempDir::new().unwrap();
    std::fs::write(dir.path().join("Counter.svelte"), r#"<script>
let count = 0
function increment() { count++ }
</script>
<button on:click={increment}>{count}</button>"#).unwrap();

    let out = codehud::search::search_path(
        dir.path().to_str().unwrap(),
        &codehud::search::SearchOptions {
            pattern: "increment".to_string(),
            regex: false,
            case_insensitive: false,
            depth: None,
            ext: vec![],
            max_results: None,
            no_tests: false,
            exclude: vec![],
            json: false,
            context: None,
            summary: false,
        },
    ).unwrap();
    assert!(out.contains("increment"), "Should find increment in Svelte: {out}");
    assert!(out.contains("Counter.svelte"), "Should show Svelte filename: {out}");
}
