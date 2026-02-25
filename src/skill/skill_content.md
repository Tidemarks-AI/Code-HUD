# codehud — Structural Code Intelligence

codehud extracts structural information from code using Tree-sitter. Use it instead of grep/find for understanding codebases.

## Binary
`codehud` — ensure it's on PATH. Install: `cargo install codehud` or via install.sh.

## Workflow

### 1. Orient — understand the project
```bash
codehud --stats .                 # Language breakdown, file counts
codehud --smart-depth .           # Adaptive directory tree
```

### 2. Explore structure — find what's in a file
```bash
codehud --outline src/main.rs     # Functions, structs, impls — signatures only
codehud --list-symbols src/       # All symbols across directory
```

### 3. Drill into specifics
```bash
codehud src/parser.rs             # Full structural view with bodies
codehud src/parser.rs::parse_expr # Single symbol expansion
```

### 4. Search and cross-reference
```bash
codehud --search "Config" .       # Find symbols matching pattern
codehud --xrefs "parse_expr" .    # Where is this symbol used?
```

### 5. Review changes
```bash
codehud --diff                    # Structural diff vs git HEAD
codehud --diff --staged           # Staged changes only
```

## Best Practices
- Start with `--stats` + `--smart-depth` on new codebases — don't read files blindly
- Use `--outline` before reading full files — often the signature is enough
- Prefer `--xrefs` over grep for finding symbol usage
- Use `--diff` to understand what changed structurally (ignores formatting noise)
