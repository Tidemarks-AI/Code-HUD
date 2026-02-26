# Case Study: VS Code

## A. Repo Profile

**Repository:** [microsoft/vscode](https://github.com/microsoft/vscode)
**Clone path:** `/mnt/data/case-studies/vscode/` (shallow clone)
**Analysis target:** `src/` directory

### Stats (from `codehud --stats`)

| Metric | Value |
|---|---|
| **Files** | 6,033 |
| **Lines** | 1,912,149 |
| **Bytes** | ~82 MB |
| **Tokens** | ~20.5M |

### Language Breakdown

| Language | Files | Lines |
|---|---|---|
| TypeScript | 5,377 | 1,707,564 |
| CSS | 325 | 44,351 |
| JavaScript | 37 | 29,535 |
| txt | 56 | 117,304 |
| HTML | 72 | 2,625 |
| JSON | 142 | 7,265 |
| Other (sh, ps1, fish, zsh, md, less, TSX) | 24 | 3,505 |

### Structure

VS Code is a **monorepo** with a layered architecture under `src/vs/`:

- `src/vs/base/` — shared utilities (common, browser, node, worker)
- `src/vs/editor/` — Monaco editor core
- `src/vs/platform/` — platform services (files, configuration, keybinding, etc.)
- `src/vs/workbench/` — workbench shell and UI
- `src/vs/workbench/contrib/` — feature contributions (terminal, debug, SCM, etc.)
- `src/vs/workbench/services/` — workbench-level services
- `src/vs/server/` — remote server
- `src/vscode-dts/` — proposed API type declarations

The codebase relies heavily on **interfaces** for dependency injection, **contribution patterns** for extensibility, and a strict **layering system** (common → browser/node → electron).

---

## B. Task List for an Agent

### Simple

**1. Get a structural overview of the editor model**
- Command: `codehud src/vs/editor/common/model.ts --outline`
- Features: `--outline`
- Goal: Understand the interfaces defined in the core editor model file

**2. List all exported symbols in the text model**
- Command: `codehud src/vs/editor/common/model.ts --list-symbols`
- Features: `--list-symbols`
- Goal: Get a quick inventory of types and functions

**3. Find all files mentioning `ITextModel`**
- Command: `codehud src/ --search "ITextModel"`
- Features: `--search`
- Goal: Locate every file that uses the core text model interface

**4. Get repo stats for just the editor subsystem**
- Command: `codehud src/vs/editor/ --stats`
- Features: `--stats`
- Goal: Understand the size of the editor vs the whole repo

**5. Show the directory tree of the workbench contributions**
- Command: `codehud src/vs/workbench/contrib/ --tree --depth 2`
- Features: `--tree`, `--depth`
- Goal: Map out all contribution modules at a glance

### Moderate

**6. Find all implementations of `IEditorContribution`**
- Command: `codehud src/vs/editor/ --search "implements IEditorContribution" --max-results 50`
- Features: `--search`, `--max-results`
- Goal: Discover every editor contribution class — core extensibility pattern

**7. Trace cross-file references to `registerEditorCommand`**
- Command: `codehud src/vs/editor/ --xrefs registerEditorCommand`
- Features: `--xrefs`
- Goal: Understand how editor commands are registered across the codebase

**8. Given [issue #292317](https://github.com/microsoft/vscode/issues/292317) (Rename Symbol fails in multi-diff editor), locate the rename controller**
- Command: `codehud src/ --search "RenameController" --max-results 10`
- Features: `--search`
- Goal: Find where rename logic lives to investigate the bug

**9. Outline the diff editor contribution**
- Command: `codehud src/vs/editor/browser/widget/diffEditor/ --outline`
- Features: `--outline`
- Goal: Understand the diff editor's class hierarchy before investigating [issue #277259](https://github.com/microsoft/vscode/issues/277259) (diffEditor border bug)

**10. Find all references to `ICodeEditor` interface**
- Command: `codehud src/vs/editor/ --references ICodeEditor`
- Features: `--references`
- Goal: Map the usage surface of the main editor interface

**11. Search for bracket pair colorization logic**
- Command: `codehud src/ --search "BracketPairColoriz" --max-results 20`
- Features: `--search`
- Goal: Locate the code relevant to [issue #279576](https://github.com/microsoft/vscode/issues/279576) (bracket pair colorization renders `>` incorrectly in C# markdown)

**12. List symbols in the git extension at depth 2**
- Command: `codehud src/vs/workbench/contrib/scm/ --list-symbols --symbol-depth 2`
- Features: `--list-symbols`, `--symbol-depth`
- Goal: Map the SCM contribution's class structure for [issue #280264](https://github.com/microsoft/vscode/issues/280264) (include stashes in graph)

### Hard

**13. Trace the command registration flow for `editor.action.formatDocument`**
- Command: `codehud src/ --search "editor.action.formatDocument" --max-results 30` then `--xrefs` on the handler
- Features: `--search`, `--xrefs`, `--outline`
- Goal: Follow the full registration → handler → execution chain across files

**14. Map the terminal shell integration architecture**
- Command: `codehud src/vs/workbench/contrib/terminal/ --outline --depth 3` then `codehud src/ --search "ShellIntegration" --max-results 30`
- Features: `--outline`, `--search`, `--depth`
- Goal: Understand the shell integration system relevant to [issue #283151](https://github.com/microsoft/vscode/issues/283151) (PowerShell ConstrainedLanguage mode)

**15. Find all `Disposable` subclasses in the platform layer**
- Command: `codehud src/vs/platform/ --search "extends Disposable" --max-results 50`
- Features: `--search`, `--max-results`
- Goal: Audit lifecycle management patterns across platform services

**16. Trace the custom editor file deletion bug path**
- Command: `codehud src/ --search "CustomEditorModel" --max-results 20` then `--xrefs` on relevant symbols
- Features: `--search`, `--xrefs`
- Goal: Investigate [issue #278883](https://github.com/microsoft/vscode/issues/278883) — custom editors fail to open files after deletion via fs API

**17. Analyze the webview contribution for release notes rendering**
- Command: `codehud src/vs/workbench/contrib/webview/ --outline` then `codehud src/ --search "ReleaseNotesWebview" --max-results 10`
- Features: `--outline`, `--search`
- Goal: Find the intersection of webview and release notes relevant to [issue #282039](https://github.com/microsoft/vscode/issues/282039) (chat panel blends into release notes)

**18. Cross-reference `ITextModelService` across all layers**
- Command: `codehud src/ --xrefs ITextModelService`
- Features: `--xrefs`
- Goal: Trace how the text model service is consumed across base, editor, platform, and workbench layers — tests layering enforcement

**19. Diff structural changes in the SCM contribution**
- Command: `codehud src/vs/workbench/contrib/scm/ --diff HEAD~5`
- Features: `--diff`
- Goal: See what symbols changed recently in the SCM module (requires non-shallow clone for real use)

**20. Full architecture scan: outline every workbench service**
- Command: `codehud src/vs/workbench/services/ --outline --smart-depth`
- Features: `--outline`, `--smart-depth`
- Goal: Generate a complete structural map of all workbench services — stress test for large directory traversal

---

## C. Metrics Matrix

_To be filled in after running the benchmark suite._

| # | Task | Feature(s) | Time (ms) | Output Lines | Correct? | Notes |
|---|---|---|---|---|---|---|
| 1 | Outline editor model | `--outline` | | | | |
| 2 | List symbols in model.ts | `--list-symbols` | | | | |
| 3 | Search ITextModel | `--search` | | | | |
| 4 | Stats for editor/ | `--stats` | | | | |
| 5 | Tree of contrib/ | `--tree` | | | | |
| 6 | Search IEditorContribution impls | `--search` | | | | |
| 7 | Xrefs registerEditorCommand | `--xrefs` | | | | |
| 8 | Search RenameController | `--search` | | | | |
| 9 | Outline diff editor | `--outline` | | | | |
| 10 | References ICodeEditor | `--references` | | | | |
| 11 | Search BracketPairColoriz | `--search` | | | | |
| 12 | List symbols SCM depth 2 | `--list-symbols` | | | | |
| 13 | Trace formatDocument flow | `--search` + `--xrefs` | | | | |
| 14 | Terminal shell integration | `--outline` + `--search` | | | | |
| 15 | Disposable subclasses | `--search` | | | | |
| 16 | Custom editor deletion bug | `--search` + `--xrefs` | | | | |
| 17 | Webview + release notes | `--outline` + `--search` | | | | |
| 18 | Xrefs ITextModelService | `--xrefs` | | | | |
| 19 | Diff SCM changes | `--diff` | | | | |
| 20 | Outline all workbench services | `--outline` + `--smart-depth` | | | | |

---

## D. Known Challenges

### Scale
- **6,033 files / 1.9M lines** — one of the largest TypeScript codebases. Directory-wide operations (`--search`, `--xrefs`, `--outline`) will be slow if not parallelized.
- Token count (~20.5M) may cause memory pressure during full-repo analysis.

### Generated / Non-Source Files
- `src/vs/monaco.d.ts` is a **8,781-line generated declaration file** — may pollute search results and symbol listings.
- `src/vscode-dts/` contains ~70+ proposed API `.d.ts` files — useful but noisy for searches.
- `src/typings/` has ambient type declarations that won't follow normal import patterns.
- `.txt` files account for 117K lines (test fixtures, changelogs) — could skew stats.

### TypeScript Patterns That May Trip Up Tree-sitter
- **Decorators** — VS Code uses decorators sparingly but they exist.
- **Const enums** — used throughout; may not be parsed as regular enums.
- **Namespace merging** — some files use `namespace` + `interface` merging patterns.
- **Complex generics** — deeply nested generic types (e.g., `Event<T>`, `IObservable<T>`) may confuse structural extraction.
- **String-based registrations** — commands, contributions, and services are often registered with string IDs (`'editor.action.formatDocument'`), not statically resolvable via AST.

### Architecture-Specific
- **Layering violations** — codehud's `--xrefs` won't know about VS Code's layer rules (`common` can't import `browser`). Results may cross layers without flagging it.
- **Dependency injection** — services are injected via decorators (`@ITextModelService`), so tracing "who uses this service" requires understanding the DI container, not just imports.
- **Contribution point pattern** — features register via `Registry.as<T>(Extensions.Foo).register(...)` which is runtime, not statically analyzable.

### Shallow Clone Limitations
- `--diff` requires git history — shallow clone may only have 1 commit, making diff benchmarks impossible without deepening.
- Some features that depend on git blame or log won't work.
