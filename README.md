# Code HUD

A code context extractor powered by [Tree-sitter](https://tree-sitter.github.io/). Shows the shape of a codebase — signatures, types, structure — without the noise. Supports symbol-aware editing.

## Install

### Quick install (Linux / macOS)

```sh
curl -fsSL https://raw.githubusercontent.com/Last-but-not-least/codeview/main/install.sh | sh
```

This auto-detects your OS and architecture, downloads the latest release binary, and installs to `/usr/local/bin`. Set `INSTALL_DIR` to change the location, or `VERSION` to pin a specific version:

```sh
INSTALL_DIR=~/.local/bin VERSION=v0.0.1 curl -fsSL https://raw.githubusercontent.com/Last-but-not-least/codeview/main/install.sh | sh
```

### Download from GitHub Releases

Prebuilt binaries for every release: [GitHub Releases](https://github.com/Last-but-not-least/codeview/releases)

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

Supports regex, case-insensitive (`-i`), and directory search:

```sh
$ codehud src/ --search "TODO|FIXME" -i
```

Cap search results with `--max-results`:

```sh
$ codehud src/ --search "validate" --max-results 5
```

For directory search, results default to 20 unless overridden.

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

In expand mode, directory traversal stops early once all requested symbols have been found.

### Stats mode

Show metadata instead of content — useful for context budgeting:

```sh
$ codehud src/ --stats
```

```
files: 16  lines: 1785  bytes: 56493  items: 111
  const: 2  enum: 5  function: 27  impl: 4  mod: 20  struct: 8  trait: 1  use: 44

  src/lib.rs — 166 lines, 5935 bytes, 14 items (2 function, 6 mod, 1 struct, 5 use)
  ...
```

Also works with `--json` for structured output.

### TypeScript support

Works identically with `.ts` and `.tsx` files:

```sh
$ codehud src/api.ts
```

```
src/api.ts
 1 | import { Database } from "./db";
 3 | export interface User {
 4 |     name: string;
 5 |     age: number;
 6 |     email?: string;
 7 | }
 9 | export type UserId = string | number;
11 | export class UserService {
14 |     constructor(db: Database) { ... }
18 |     public getUser(id: UserId): User | undefined { ... }
22 |     public createUser(name: string, age: number): User { ... }
27 |     private validate(user: User): boolean { ... }
30 | }
32 | export function parseUserId(raw: string): UserId { ... }
```

### Python support

Works with `.py` files. The `_private` naming convention maps to private visibility:

```sh
$ codehud app.py
```

```
app.py
 1 | import os
 2 | from dataclasses import dataclass

 4 | @dataclass
 5 | class Config:
 6 |     host: str
 7 |     port: int
 8 |     _secret: str

11 | class App:
12 |     def __init__(self, config: Config): ...
15 |     def run(self): ...
18 |     def handle_request(self, path: str) -> dict: ...
22 |     def _validate(self, data: dict) -> bool: ...

25 | def create_app(env: str = "dev") -> App: ...

28 | def _load_defaults() -> dict: ...
```

Names starting with `_` are treated as private — `--pub` will hide `_validate`, `_load_defaults`, and `_secret`.

### JavaScript support

Works with `.js` and `.jsx` files:

```sh
$ codehud api.js
```

```
api.js
 1 | import express from "express";

 3 | export class Router {
 4 |     constructor(prefix) { ... }
 7 |     get(path, handler) { ... }
10 |     post(path, handler) { ... }
13 | }

15 | export function createApp(config) { ... }

19 | function loadMiddleware(name) { ... }

22 | export default Router;
```

## Filters

| Flag         | Effect                                       |
|--------------|----------------------------------------------|
| `--pub`      | Only public/exported items                   |
| `--fns`      | Only functions and methods                   |
| `--types`    | Only types (struct/class, enum, trait/interface, type alias) |
| `--no-tests` | Exclude test blocks (`#[cfg(test)]` in Rust)  |
| `--depth N`  | Limit directory recursion (0 = target dir only) |
| `--ext rs,ts` | Filter directory walk by file extension (comma-separated) |
| `--signatures` | Class signatures mode (collapsed method bodies) |
| `--max-lines N` | Truncate expanded output after N lines      |
| `--search "pat"` | Structural grep (matches with AST context) |
| `--max-results N` | Cap search output to N results (default: 20 for directories, unlimited for files) |
| `-i`         | Case-insensitive search (with `--search`)    |
| `--lines N-M` | Extract line range with structural context (1-indexed, inclusive) |
| `--list-symbols` | Lightweight symbol listing (name, kind, line number) |
| `--json`     | JSON output                                  |
| `--stats`    | Show file/item counts instead of content     |

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

### JSON output

Add `--json` to any edit command to get structured JSON metadata about what changed:

```sh
$ codehud edit src/lib.rs helper --replace 'fn helper() {}' --json
```

### Dry run

Add `--dry-run` to any edit command to print the result to stdout without writing the file:

```sh
$ codehud edit src/lib.rs helper --replace 'fn helper() {}' --dry-run
```

## Architecture

```
src/
├── main.rs              # CLI entry (clap)
├── lib.rs               # Core orchestration (process_path)
├── parser.rs            # Tree-sitter parsing
├── error.rs             # Error types (thiserror)
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
│   ├── collapse.rs      # Body collapsing logic
│   ├── rust.rs          # Rust-specific extraction (impl blocks, fn signatures)
│   ├── typescript.rs    # TypeScript/TSX-specific extraction
│   ├── python.rs        # Python-specific extraction (classes, decorators)
│   └── javascript.rs    # JavaScript/JSX-specific extraction
├── search.rs            # Structural search (--search, AST-aware grep)
├── editor/              # Symbol-aware editing
│   └── mod.rs           # replace, replace_body, delete, batch — with validation
├── output/              # Formatters
│   ├── mod.rs           # OutputFormat enum
│   ├── plain.rs         # Plain text formatter (with line numbers)
│   ├── json.rs          # JSON formatter
│   └── stats.rs         # Stats formatter (file/line/item counts)
└── walk.rs              # Directory traversal (ignore crate, respects .gitignore)
```

## Supported Languages

- Rust (`.rs`)
- TypeScript (`.ts`, `.tsx`)
- Python (`.py`)
- JavaScript (`.js`, `.jsx`)

## License

Dual-licensed under MIT or Apache-2.0.
