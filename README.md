# Code HUD

A code context extractor powered by [Tree-sitter](https://tree-sitter.github.io/). Shows the shape of a codebase — signatures, types, structure — without the noise. Supports symbol-aware editing, structural search, cross-file references, git diffs, and integration with AI coding platforms.

## Install

### Quick install (Linux / macOS)

```sh
curl -fsSL https://raw.githubusercontent.com/Tidemarks-AI/Code-HUD/main/install.sh | sh
```

This auto-detects your OS and architecture, downloads the latest release binary, and installs to `/usr/local/bin`. Set `INSTALL_DIR` to change the location, or `VERSION` to pin a specific version:

```sh
INSTALL_DIR=~/.local/bin VERSION=v0.1.0 curl -fsSL https://raw.githubusercontent.com/Tidemarks-AI/Code-HUD/main/install.sh | sh
```

### Download from GitHub Releases

Prebuilt binaries for every release: [GitHub Releases](https://github.com/Tidemarks-AI/Code-HUD/releases)

| Target | Archive |
|--------|---------|
| Linux x86_64 (static/musl) | `codehud-<version>-x86_64-unknown-linux-musl.tar.gz` |
| Linux aarch64 | `codehud-<version>-aarch64-unknown-linux-gnu.tar.gz` |
| macOS x86_64 | `codehud-<version>-x86_64-apple-darwin.tar.gz` |
| macOS Apple Silicon | `codehud-<version>-aarch64-apple-darwin.tar.gz` |

Each release includes a `checksums.sha256` file for verification.

### Build from source

```sh
cargo install --path .
```

## Reading Code

### Interface mode (default)

Shows file structure with function bodies collapsed to `{ ... }`:

```sh
$ codehud src/lib.rs
```

```
src/lib.rs
 1 | use std::collections::HashMap;

 4 | #[derive(Debug, Clone)]
 5 | pub struct User {
 6 |     pub name: String,
 7 |     pub age: u32,
 8 |     email: String,
 9 | }

11 | impl User {
12 |     pub fn new(name: String, age: u32, email: String) -> Self { ... }
16 |     pub fn greeting(&self) -> String { ... }
20 |     fn validate_email(&self) -> bool { ... }
23 | }
```

Line numbers match the original file — collapsed bodies don't shift numbering.

### Expand mode

Pass symbol names to see their full implementation:

```sh
$ codehud src/lib.rs User new
```

```
src/lib.rs::User [4:9]
 4 | #[derive(Debug, Clone)]
 5 | pub struct User {
 6 |     pub name: String,
 7 |     pub age: u32,
 8 |     email: String,
 9 | }

src/lib.rs::new [12:14]
12 | pub fn new(name: String, age: u32, email: String) -> Self {
13 |         Self { name, age, email }
14 |     }
```

### Class signatures mode

Inspect a class with method bodies collapsed — see the shape without the noise:

```sh
$ codehud src/api.ts UserService --signatures
```

```
src/api.ts::UserService [11:30]
11 | export class UserService {
14 |     constructor(db: Database) { ... }
18 |     public getUser(id: UserId): User | undefined { ... }
22 |     public createUser(name: string, age: number): User { ... }
27 |     private validate(user: User): boolean { ... }
30 | }
```

Combine with specific method expansion — signatures for the class, full body for selected methods:

```sh
$ codehud src/api.ts UserService --signatures getUser
```

### Outline mode

Show signatures, docstrings, and types without implementation bodies — great for getting a high-level overview:

```sh
$ codehud src/lib.rs --outline
```

Use `--compact` for even shorter output (minimal signatures with just name + return type, no params or docstrings):

```sh
$ codehud src/ --outline --compact
```

### Bounded expand

Peek at large symbols without dumping the full body:

```sh
$ codehud src/api.ts processData --max-lines 20
```

Truncates after N lines with a `... [truncated: X more lines]` indicator. Works with `--signatures` too.

### Line range extraction

Extract a specific line range with structural context — shows which function/class/module the lines belong to:

```sh
$ codehud src/main.rs --lines 145-155
```

```
// Inside: main
L145:             if let Some(lines_arg) = cli.lines {
L146:                 match codehud::extract_lines(&path, &lines_arg) {
L147:                     Ok(output) => {
L148:                         print!("{}", output);
L149:                     }
L150:                     Err(e) => {
L151:                         eprintln!("Error: {}", e);
L152:                         process::exit(1);
L153:                     }
L154:                 }
L155:                 return;
```

Line numbers are 1-indexed and inclusive. Only works on single files, not directories.

## Searching and References

### Structural search

Grep with AST context — matches are annotated with their enclosing class/method:

```sh
$ codehud src/api.ts --search "validate"
```

```
src/api.ts
  UserService > createUser()
    L24:     if (!this.validate(user)) {
  UserService > validate()
    L27:     private validate(user: User): boolean {
```

Supports regex (`-E`), case-insensitive (`-i`), and directory search:

```sh
$ codehud src/ --search "TODO|FIXME" -E -i
```

Multi-pattern search with OR logic — use `|` to separate patterns:

```sh
$ codehud src/ --search "TextEditor|CodeEditor|DiffEditor"
```

This works in both literal mode (default) and regex mode (`-E`).

Show surrounding context lines (like `grep -C`):

```sh
$ codehud src/ --search "async function" --context 3
```

Cap search results with `--max-results` (or `--limit`):

```sh
$ codehud src/ --search "validate" --limit 5
```

For directory search, results default to 20 unless overridden.

### References

Find all references to a symbol name within a file or directory (AST-aware):

```sh
$ codehud src/lib.rs --references User
$ codehud src/ --references handle_request --context 2
```

Use `--defs-only` to show only definitions, or `--refs-only` to show only usages:

```sh
$ codehud src/ --references Config --defs-only
$ codehud src/ --references Config --refs-only
```

### Cross-file references

Follow imports to find all usages of a symbol across files:

```sh
$ codehud src/ --xrefs UserService
```

## Directory and Tree Views

### Directory mode

Point at a directory to walk all supported files:

```sh
$ codehud src/
$ codehud src/ --depth 0    # target dir only, no subdirs
$ codehud src/ --depth 1    # one level deep
$ codehud src/ --ext rs,ts  # only .rs and .ts files
```

Use `--ext` to filter by file extension (comma-separated, without the dot).

Respects `.gitignore`, `.ignore`, and global gitignore — `target/`, `node_modules/`, etc. are skipped automatically.

Use `--exclude` to exclude paths matching a glob pattern (repeatable):

```sh
$ codehud src/ --exclude dist --exclude "*/migrations/*"
```

In expand mode, directory traversal stops early once all requested symbols have been found.

### Smart depth

For monorepos, `--smart-depth` auto-detects source roots and applies depth relative to them instead of the top-level directory:

```sh
$ codehud . --smart-depth --depth 1
```

### Tree view

Show a directory tree (like `tree` but respecting gitignore):

```sh
$ codehud src/ --tree
```

### Flat file listing

Show one file per line with relative paths:

```sh
$ codehud src/ --files
```

### List symbols

Lightweight symbol enumeration — one line per symbol with kind and line number:

```sh
$ codehud src/lib.rs --list-symbols
```

```
src/lib.rs
  struct User                          L5
  fn     new                           L12
  fn     greeting                      L16
  fn     validate_email                L20
```

Works with directory mode and filters:

```sh
$ codehud src/ --list-symbols --pub --fns
```

Use `--symbol-depth 2` to include class/impl members:

```sh
$ codehud src/ --list-symbols --symbol-depth 2
```

By default, imports are hidden in `--list-symbols` output. Use `--imports` to include them:

```sh
$ codehud src/lib.rs --list-symbols --imports
```

### Stats mode

Show a summary instead of content — useful for context budgeting:

```sh
$ codehud src/ --stats
```

```
Files: 12,453 | Dirs: 2,340 | Lines: 185.4k | Bytes: 5.6M | Tokens: ~1.4M
  Languages: TypeScript (8.2k), JavaScript (2.1k), Rust (1.5k), Python (705)
  Top dirs: src/components (1,230), src/utils (890), src/api (456)

[Use --stats-detailed for full file list]
```

For the full per-file breakdown, use `--stats-detailed`:

```sh
$ codehud src/ --stats-detailed
```

Also works with `--json` for structured output.

## Git Diff

Show structural diffs of changed symbols against a git ref:

```sh
$ codehud src/ --diff            # diff against HEAD
$ codehud src/ --diff main       # diff against main branch
$ codehud src/ --diff --staged   # diff staged changes only
```

## Filters

| Flag | Effect |
|------|--------|
| `--pub` | Only public/exported items |
| `--fns` | Only functions and methods |
| `--types` | Only types (struct/class, enum, trait/interface, type alias) |
| `--no-tests` | Exclude test blocks (`#[cfg(test)]`/`#[test]` in Rust, `*.test.ts`/`describe()`/`it()` in TS/JS, `test_*.py` in Python, `*_test.go` in Go) |
| `--no-imports` | Exclude import/use statements from output |
| `--ext rs,ts` | Filter by file extension (comma-separated) |
| `--exclude <glob>` | Exclude paths matching glob (repeatable) |
| `-d, --depth N` | Limit directory recursion (0 = target dir only) |
| `--smart-depth` | Auto-detect source roots for depth in monorepos |
| `--outline` | Signatures + docstrings without bodies |
| `--compact` | Minimal signatures (name + return type only, use with `--outline`) |
| `--signatures` | Class signatures mode (collapsed method bodies) |
| `--max-lines N` | Truncate expanded output after N lines |
| `--max-output-lines N` | Truncate final output after N lines (any mode) |
| `--search "pat"` | Structural grep (matches with AST context) |
| `-E, --regex` | Treat search pattern as regex |
| `-i` | Case-insensitive search |
| `-C, --context N` | Context lines around search/reference matches (default: 0) |
| `--max-results N` / `--limit N` | Cap search results (default: 20 for dirs) |
| `--lines N-M` | Extract line range with structural context |
| `--list-symbols` | One-line-per-symbol listing (name, kind, line) |
| `--symbol-depth N` | Symbol depth for `--list-symbols` (1=top-level, 2=include members) |
| `--imports` | Include imports in `--list-symbols` (hidden by default) |
| `--references <sym>` | Find all references to a symbol (AST-aware) |
| `--defs-only` | Show only definitions (with `--references`) |
| `--refs-only` | Show only usages (with `--references`) |
| `--xrefs <sym>` | Cross-file reference search (follows imports) |
| `--diff [ref]` | Structural diff against a git ref (default: HEAD) |
| `--staged` | Diff staged changes (use with `--diff`) |
| `--tree` | Directory tree view |
| `--files` | Flat file listing |
| `--stats` | Summary (files, dirs, languages) |
| `--stats-detailed` | Full per-file breakdown |
| `--json` | JSON output |

Filters compose: `--pub --fns` shows only public functions.

## Editing Code

Code HUD can edit files by targeting symbols by name. All edits are **validated** — if the result produces invalid syntax (tree-sitter re-parse), the operation is rejected and the file is left untouched.

Attributes are handled correctly: deleting or replacing a symbol includes its attributes (e.g. `#[derive(...)]`) in the affected range.

### Replace a symbol

Replace the entire symbol (signature + body + attributes):

```sh
$ codehud edit src/lib.rs helper --replace 'fn helper() -> i32 { 42 }'

# Read replacement from stdin (for multi-line edits)
$ cat <<'EOF' | codehud edit src/lib.rs helper --replace --stdin
fn helper(x: i32) -> i32 {
    x * 2
}
EOF
```

### Replace only the body

Keep the existing signature and attributes, replace just the body:

```sh
$ codehud edit src/lib.rs helper --replace-body '{ 42 }'

# From stdin
$ echo '{ x * 2 }' | codehud edit src/lib.rs helper --replace-body --stdin
```

### Delete a symbol

```sh
$ codehud edit src/lib.rs helper --delete
```

### Insert code

Insert new code relative to an existing symbol, or at the edges of a file:

```sh
# Insert after a symbol
$ echo 'fn new_func() {}' | codehud edit src/lib.rs --add-after existing_func --stdin

# Insert before a symbol
$ echo 'use std::io;' | codehud edit src/lib.rs --add-before main --stdin

# Append to end of file
$ echo 'fn last() {}' | codehud edit src/lib.rs --append --stdin

# Prepend to beginning (after leading comments)
$ echo 'use log::info;' | codehud edit src/lib.rs --prepend --stdin
```

### Batch edits

Apply multiple edits to one file atomically via a JSON file:

```sh
$ codehud edit src/lib.rs --batch edits.json
```

```json
[
  { "symbol": "foo", "action": "replace", "content": "fn foo() {}" },
  { "symbol": "bar", "action": "replace-body", "content": "{ 0 }" },
  { "symbol": "baz", "action": "delete" }
]
```

Actions: `replace`, `replace-body`, `delete`. The `content` field is required for replace/replace-body, ignored for delete.

### JSON output and dry run

```sh
# Get structured JSON metadata about what changed
$ codehud edit src/lib.rs helper --replace 'fn helper() {}' --json

# Preview without writing
$ codehud edit src/lib.rs helper --replace 'fn helper() {}' --dry-run
```

## Platform Integration

Code HUD can install itself as a skill (tool) or standalone agent for AI coding platforms.

### Install as a skill

A skill adds codehud as a tool that an existing AI coding agent can use:

```sh
$ codehud install-skill claude-code
$ codehud install-skill cursor
$ codehud install-skill codex
$ codehud install-skill aider
$ codehud install-skill openclaw
```

List available platforms:

```sh
$ codehud install-skill --list
```

Uninstall:

```sh
$ codehud uninstall-skill claude-code
```

### Install as an agent

Register codehud as a standalone agent on a platform:

```sh
$ codehud install-agent openclaw
```

Uninstall (use `--force` to also remove the workspace directory):

```sh
$ codehud uninstall-agent openclaw
$ codehud uninstall-agent openclaw --force
```

## Supported Languages

- Rust (`.rs`)
- TypeScript (`.ts`, `.tsx`)
- Python (`.py`)
- JavaScript (`.js`, `.jsx`)

## Architecture

```
src/
├── main.rs              # CLI entry (clap)
├── lib.rs               # Core orchestration (process_path)
├── dispatch.rs          # Mode dispatch logic
├── pipeline.rs          # Processing pipeline
├── parser.rs            # Tree-sitter parsing
├── error.rs             # Error types (thiserror)
├── walk.rs              # Directory traversal (ignore crate, respects .gitignore)
├── tree.rs              # Tree view (--tree)
├── search.rs            # Structural search (--search, AST-aware grep)
├── references.rs        # Symbol references (--references)
├── xrefs.rs             # Cross-file references (--xrefs, follows imports)
├── diff.rs              # Structural diff engine
├── diff_cli.rs          # Diff CLI integration (--diff, --staged)
├── git.rs               # Git operations (diff, staged changes)
├── sfc.rs               # Single-file component support
├── test_detect.rs       # Test code detection (--no-tests)
├── languages/           # Language detection + grammar queries
│   ├── mod.rs           # Language enum, detection, TS language loader
│   ├── rust.rs          # Rust tree-sitter queries
│   ├── typescript.rs    # TypeScript/TSX tree-sitter queries
│   ├── python.rs        # Python tree-sitter queries
│   └── javascript.rs    # JavaScript/JSX tree-sitter queries
├── extractor/           # Item extraction from AST
│   ├── mod.rs           # Item/ItemKind/Visibility types, LanguageExtractor trait
│   ├── interface.rs     # Interface mode (collapsed bodies)
│   ├── expand.rs        # Expand mode (full source for named symbols)
│   ├── outline.rs       # Outline mode (--outline, --compact)
│   └── collapse.rs      # Body collapsing logic
├── handler/             # Language-specific extraction handlers
│   ├── mod.rs           # Handler trait and registry
│   ├── rust.rs          # Rust-specific extraction (impl blocks, fn signatures)
│   ├── typescript.rs    # TypeScript/TSX-specific extraction
│   ├── python.rs        # Python-specific extraction (classes, decorators)
│   └── javascript.rs    # JavaScript/JSX-specific extraction
├── editor/              # Symbol-aware editing
│   └── mod.rs           # replace, replace_body, delete, insert, batch — with validation
├── output/              # Formatters
│   ├── mod.rs           # OutputFormat enum
│   ├── plain.rs         # Plain text formatter (with line numbers)
│   ├── json.rs          # JSON formatter
│   └── stats.rs         # Stats formatter (file/line/item counts)
├── skill/               # Platform skill installation
│   ├── mod.rs           # Skill install/uninstall dispatch
│   ├── content.rs       # Skill file content generation
│   ├── openclaw.rs      # OpenClaw skill integration
│   ├── claude_code.rs   # Claude Code skill integration
│   ├── codex.rs         # Codex skill integration
│   ├── cursor.rs        # Cursor skill integration
│   └── aider.rs         # Aider skill integration
└── agent/               # Standalone agent installation
    ├── mod.rs           # Agent install/uninstall dispatch
    └── openclaw.rs      # OpenClaw agent integration
```

## License

Dual-licensed under MIT or Apache-2.0.
