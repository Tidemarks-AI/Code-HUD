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

These are realistic scenarios an agent might face when working in the VS Code codebase. Each task describes a goal — the agent must figure out which tools and approaches to use.

### Orientation (Simple)

**1. Map the workbench contribution modules**
- **Scenario:** A new contributor wants to understand what feature areas VS Code has. Produce a list of all contribution modules under `src/vs/workbench/contrib/` with a one-line description of each based on their contents.
- **Done when:** You've identified all ~60+ contribution directories and can describe what at least 10 of them do.

**2. Understand the editor model's public API surface**
- **Scenario:** You need to write code that interacts with VS Code's text model. Identify the key interfaces and types exported from the core editor model (`src/vs/editor/common/model.ts`).
- **Done when:** You can list the primary interfaces (`ITextModel`, `IModelDecorationOptions`, etc.) and explain what each represents.

**3. Compare the size of the editor vs the workbench**
- **Scenario:** A team lead asks "how much of the codebase is the editor core vs the workbench?" Get concrete numbers (files, lines) for `src/vs/editor/` vs `src/vs/workbench/`.
- **Done when:** You can report file counts and line counts for both subtrees and state the ratio.

**4. Find where VS Code defines its keyboard shortcuts**
- **Scenario:** A user reports a keybinding conflict. You need to find where default keybindings are defined. Locate the files responsible for registering built-in keyboard shortcuts.
- **Done when:** You've identified the keybinding registration files and can explain how a keybinding gets from definition to runtime dispatch.

**5. Inventory the proposed extension APIs**
- **Scenario:** An extension author wants to know what proposed APIs exist. List all proposed API declarations in `src/vscode-dts/` and categorize them by feature area (e.g., chat, terminal, testing).
- **Done when:** You've produced a categorized list of at least 20 proposed APIs with their file names.

### Investigation (Moderate)

**6. Trace how editor commands are registered and executed**
- **Scenario:** A new contributor wants to understand how VS Code's command system works. Map the registration, dispatch, and execution flow for editor commands — starting from `registerEditorCommand`, through the command registry, to actual execution.
- **Done when:** You can describe the flow across files and identify the 3-5 key files/classes involved.

**7. Find all implementations of the editor contribution pattern**
- **Scenario:** You're writing a new editor feature and need to follow VS Code's contribution pattern. Find every class that implements `IEditorContribution` and describe the pattern they follow.
- **Done when:** You've identified at least 20 contribution classes and can describe the lifecycle (registration, instantiation, disposal).

**8. Investigate: color picker appears on ES6 private fields — [#297799](https://github.com/microsoft/vscode/issues/297799)**
- **Scenario:** Users report that typing `#myField` in a class triggers the color picker incorrectly. Find where the color picker detection logic lives and identify why `#` followed by hex-like characters triggers it.
- **Done when:** You've located the color detection code, identified the regex or heuristic responsible, and can propose where a fix would go.

**9. Understand the dependency injection system**
- **Scenario:** You see `@ITextModelService` decorators everywhere but don't understand how services get wired up. Trace how VS Code's DI container works — from service declaration to injection to instantiation.
- **Done when:** You can explain the flow from `createDecorator()` → service registration → constructor injection, naming the key files involved.

**10. Map the SCM/Git integration architecture**
- **Scenario:** You're preparing to work on [#281005](https://github.com/microsoft/vscode/issues/281005) (bring more stash actions into SCM views). Before writing code, map out how the SCM contribution is structured — what views exist, how they connect to the git extension, and where stash operations are handled.
- **Done when:** You've identified the SCM view files, the git extension's stash-related code, and the connection points between them.

**11. Find the blast radius of removing `IModelDecorationOptions.beforeContentClassName`**
- **Scenario:** A tech debt cleanup proposes removing the `beforeContentClassName` property from `IModelDecorationOptions`. Estimate how many files, features, and extensions would be affected.
- **Done when:** You've listed all direct usages, identified which features depend on it (inline suggestions, git blame decorations, etc.), and given a rough impact assessment.

**12. Investigate: pasting large JSON causes editor freeze — [#265397](https://github.com/microsoft/vscode/issues/265397)**
- **Scenario:** Users report that pasting a large JSON blob into an existing JSON file makes the editor unresponsive. Find where paste handling, tokenization, and bracket matching intersect, and identify likely bottleneck areas.
- **Done when:** You've traced the paste → edit → re-tokenize → bracket-pair-update flow and identified 2-3 specific code areas that could cause the freeze.

### Cross-Cutting Analysis (Hard)

**13. Trace the full lifecycle of a "Format Document" action**
- **Scenario:** A user presses Shift+Alt+F. Trace the full path: keybinding match → command dispatch → formatter selection → edit application → undo stack. Name every file the request passes through.
- **Done when:** You've produced a step-by-step trace with file paths and function/method names for the complete flow.

**14. Audit VS Code's layering rules**
- **Scenario:** VS Code enforces strict import layers: `common` → `browser`/`node` → `electron`. Find 3 examples where the codebase uses patterns to work around these constraints (e.g., service interfaces in `common` with implementations in `browser`). Also check whether any actual layering violations exist in `src/vs/editor/`.
- **Done when:** You've documented the layering pattern with examples and reported whether violations exist.

**15. Map the terminal shell integration system — [#283151](https://github.com/microsoft/vscode/issues/283151)**
- **Scenario:** PowerShell 7.4+ in ConstrainedLanguage mode breaks terminal shell integration. Before investigating, map the full shell integration architecture: how VS Code injects shell integration scripts, how it detects shell type, and how command detection works. Then identify where ConstrainedLanguage mode would cause failures.
- **Done when:** You've produced an architecture diagram (as text) of the shell integration system and identified the specific code paths affected by ConstrainedLanguage mode.

**16. Understand the background agent session system**
- **Scenario:** Multiple recent bugs ([#297975](https://github.com/microsoft/vscode/issues/297975), [#297771](https://github.com/microsoft/vscode/issues/297771), [#297867](https://github.com/microsoft/vscode/issues/297867)) relate to background agent sessions. Find where background sessions are created, managed, and displayed. Identify the shared code paths between these bugs.
- **Done when:** You've identified the key files for background session lifecycle and can explain what state management issues could cause all three bugs.

**17. Investigate: inline suggestion decorations not updating — [#281497](https://github.com/microsoft/vscode/issues/281497)**
- **Scenario:** Accepting an inline suggestion doesn't properly update injected-text decorations. Trace how inline suggestions are rendered (ghost text), how accepting a suggestion triggers decoration cleanup, and where the update could be failing.
- **Done when:** You've identified the inline suggestion rendering pipeline, the decoration lifecycle, and the specific code responsible for cleanup after acceptance.

**18. Map all string-based command registrations for the editor**
- **Scenario:** You want to build a static analysis tool that indexes all VS Code commands. The problem: commands are registered with string IDs at runtime (e.g., `'editor.action.formatDocument'`). Find all command registration patterns in `src/vs/editor/`, categorize them by registration method, and assess how many could be statically discovered vs. requiring runtime analysis.
- **Done when:** You've identified at least 3 different registration patterns, counted commands per pattern, and estimated static discoverability.

**19. Prepare a refactoring plan for the diff editor widget**
- **Scenario:** The diff editor code under `src/vs/editor/browser/widget/diffEditor/` needs refactoring. Before starting, produce a structural analysis: what classes exist, how they relate to each other, what the public API surface is, and which files are the most complex (by size and dependency count). Identify the 3 highest-risk files to change.
- **Done when:** You've produced a dependency map of the diff editor widget, ranked files by complexity, and identified the riskiest refactoring targets.

**20. Cross-layer trace: how `ITextModelService` flows through the architecture**
- **Scenario:** `ITextModelService` is a core service used across all layers. Trace its definition, implementation, registration, and consumption across `base`, `editor`, `platform`, and `workbench` layers. Verify that it respects VS Code's layering rules.
- **Done when:** You've documented where the interface is defined, where it's implemented, how it's registered in the DI container, and listed its consumers per layer.

---

## C. Metrics Matrix

### Agent Benchmark (Code HUD vs Baseline)

Three tasks were benchmarked head-to-head: Code HUD agent (codehud-only) vs Baseline agent (grep/find/cat). Both used `claude-sonnet-4-5`. Full methodology and analysis: **[vscode-benchmark.md](vscode-benchmark.md)**

#### Per-Task Metrics

| Task | Agent | Runtime | Tool Calls | Tokens Out | Prompt/Cache | Files Found |
|------|-------|---------|------------|------------|--------------|-------------|
| **#11** Blast radius | Code HUD | 3m 38s | 14 | 6,700 | 34.7k | 15 |
| | Baseline | 2m 35s | — | 5,900 | 19.3k | 13 |
| **#7** IEditorContribution | Code HUD | 3m 10s | 12 | 8,000 | 35.2k | 93+ |
| | Baseline | 3m 16s | — | 9,600 | 27.1k | 77 |
| **#19** Diff editor refactor | Code HUD | 2m 36s | 11 | 6,700 | 53.2k | 26 |
| | Baseline | 2m 56s | — | 9,300 | 32.8k | 24 |

#### Aggregate Comparison

| Metric | Code HUD | Baseline | Δ |
|--------|----------|----------|---|
| **Total runtime** | 9m 24s | 8m 47s | Baseline 7% faster |
| **Total tokens out** | 21,400 | 24,800 | Code HUD 14% less |
| **Total prompt/cache** | 123.1k | 79.2k | Baseline 36% less |
| **Completeness (avg)** | +10% more files/refs | — | Code HUD wins |
| **Report quality** | ~1,026 lines | ~1,023 lines | Comparable |

**Key takeaway:** Code HUD trades prompt tokens for analytical depth — finds more, with structural context, at the cost of larger intermediate results. For architectural tasks, completeness > prompt cost.

#### Individual Reports

| Task | Code HUD Report | Baseline Report |
|------|----------------|-----------------|
| #7 | [codehud-task-7-report.md](reports/codehud-task-7-report.md) | [baseline-task-7-report.md](reports/baseline-task-7-report.md) |
| #11 | [codehud-task-11-report.md](reports/codehud-task-11-report.md) | [baseline-task-11-report.md](reports/baseline-task-11-report.md) |
| #19 | [codehud-task-19-report.md](reports/codehud-task-19-report.md) | [baseline-task-19-report.md](reports/baseline-task-19-report.md) |

### CLI Tool Benchmark (codehud direct)

Five CLI tasks tested raw `codehud` performance on VS Code. Full results: **[vscode-results-simple.md](vscode-results-simple.md)**

| # | Command | Time | Output Lines | Correct | Useful | Issues |
|---|---------|------|-------------|---------|--------|--------|
| 1 | `--outline` single file | 0.03s | 952 | ✅ | ✅ | Minor formatting |
| 2 | Default single file | 0.04s | 1,393 | ✅ | ✅ | Subtle diff from outline |
| 3 | `--search` across src/ | 7.20s | 57 | ✅ | ✅ | Slow for large trees |
| 4 | `--stats` directory | 0.02s | ~1,130 | ✅ | ✅ | Unsorted file list |
| 5 | `--depth 2` large dir | 7.60s | 112,154 | ⚠️ | ❌ | Output explosion |

### Task Coverage

| # | Task | Difficulty | Completed | Quality | Notes |
|---|---|---|---|---|---|
| 1 | Map workbench contributions | Simple | ⬜ | | |
| 2 | Editor model public API | Simple | ✅ | ✅ | CLI Task 1-2 |
| 3 | Editor vs workbench size | Simple | ✅ | ✅ | CLI Task 4 |
| 4 | Keyboard shortcut definitions | Simple | ⬜ | | |
| 5 | Proposed extension APIs | Simple | ⬜ | | |
| 6 | Command registration flow | Moderate | ⬜ | | |
| 7 | IEditorContribution implementations | Moderate | ✅ | ✅ | Agent benchmark |
| 8 | Color picker on private fields | Moderate | ⬜ | | |
| 9 | Dependency injection system | Moderate | ⬜ | | |
| 10 | SCM/Git architecture | Moderate | ⬜ | | |
| 11 | Blast radius: beforeContentClassName | Moderate | ✅ | ✅ | Agent benchmark |
| 12 | Large JSON paste freeze | Moderate | ⬜ | | |
| 13 | Format Document lifecycle | Hard | ⬜ | | |
| 14 | Layering rules audit | Hard | ⬜ | | |
| 15 | Terminal shell integration | Hard | ⬜ | | |
| 16 | Background agent sessions | Hard | ⬜ | | |
| 17 | Inline suggestion decorations | Hard | ⬜ | | |
| 18 | String-based command registrations | Hard | ⬜ | | |
| 19 | Diff editor refactoring plan | Hard | ✅ | ✅ | Agent benchmark |
| 20 | ITextModelService cross-layer trace | Hard | ⬜ | | |

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
