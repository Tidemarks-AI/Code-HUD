# Diff Editor Widget Refactoring Plan

## Executive Summary

The diff editor widget consists of 24 TypeScript files totaling 7,849 lines of code, organized into:
- 1 main widget file
- 1 view model file  
- 1 base class (delegating editor)
- 5 feature modules
- 4 component modules (with 4 sub-components for view zones)
- 2 contribution files
- 3 utility files
- 1 embedded variant
- 1 options file
- 1 commands file
- 1 diff provider factory service

## 1. Structural Overview

### Directory Structure

```
diffEditor/
├── diffEditorWidget.ts                    [805 lines] - Main widget
├── diffEditorViewModel.ts                 [771 lines] - Core view model
├── delegatingEditorImpl.ts                [170 lines] - Base class
├── components/
│   ├── accessibleDiffViewer.ts            [737 lines] - Accessibility
│   ├── diffEditorViewZones/
│   │   ├── diffEditorViewZones.ts         [651 lines] - View zone manager
│   │   ├── renderLines.ts                 [316 lines] - Line rendering
│   │   ├── inlineDiffDeletedCodeMargin.ts [180 lines] - Inline margins
│   │   └── copySelection.ts               [77 lines]  - Selection copying
│   ├── diffEditorEditors.ts               [221 lines] - Editor pair wrapper
│   ├── diffEditorDecorations.ts           [129 lines] - Decoration management
│   └── diffEditorSash.ts                  [98 lines]  - Split view sash
├── features/
│   ├── hideUnchangedRegionsFeature.ts     [497 lines] - Collapse unchanged
│   ├── movedBlocksLinesFeature.ts         [377 lines] - Moved code blocks
│   ├── gutterFeature.ts                   [331 lines] - Gutter actions
│   ├── revertButtonsFeature.ts            [179 lines] - Revert UI
│   └── overviewRulerFeature.ts            [171 lines] - Overview ruler
├── utils/
│   └── editorGutter.ts                    [177 lines] - Gutter utilities
├── utils.ts                               [536 lines] - Core utilities
├── commands.ts                            [301 lines] - Commands/actions
├── diffEditorOptions.ts                   [192 lines] - Options management
├── diffProviderFactoryService.ts          [185 lines] - Diff computation
├── diffEditor.contribution.ts             [101 lines] - Registration
├── registrations.contribution.ts          [96 lines]  - Theme/decoration reg
└── embeddedDiffEditorWidget.ts            [55 lines]  - Embedded variant
```

## 2. Class Hierarchy and Relationships

### Core Classes

**DiffEditorWidget** (diffEditorWidget.ts)
- Extends: `DelegatingEditor`
- Implements: `IDiffEditor`
- Role: Main entry point, orchestrates all components and features
- Dependencies: 14 internal modules

**DelegatingEditor** (delegatingEditorImpl.ts)
- Extends: `Disposable`
- Implements: `IEditor`
- Role: Abstract base providing editor delegation pattern
- Used by: `DiffEditorWidget` only

**DiffEditorViewModel** (diffEditorViewModel.ts)
- Extends: `Disposable`
- Implements: `IDiffEditorViewModel`
- Role: Core state and diff computation logic
- Used by: 9 files (all features, components, options)

**EmbeddedDiffEditorWidget** (embeddedDiffEditorWidget.ts)
- Extends: `DiffEditorWidget`
- Role: Variant for embedded scenarios
- Dependencies: 1 internal module

### Component Classes

**DiffEditorEditors** (components/diffEditorEditors.ts)
- Role: Manages the original/modified editor pair
- Used by: `accessibleDiffViewer.ts`, all features

**DiffEditorViewZones** (components/diffEditorViewZones/diffEditorViewZones.ts)
- Role: Manages view zones for inline diff rendering
- Internal dependencies: `renderLines.ts`, `inlineDiffDeletedCodeMargin.ts`
- Used by: `diffEditorWidget.ts`

**AccessibleDiffViewer** (components/accessibleDiffViewer.ts)
- Role: Accessible text-based diff view
- Internal dependencies: `diffEditorEditors.ts`, `utils.ts`
- Used by: `diffEditorWidget.ts`

**DiffEditorDecorations** (components/diffEditorDecorations.ts)
- Role: Manages text decorations for changes
- Dependencies: `diffEditorViewModel.ts`
- Used by: `diffEditorWidget.ts`

**DiffEditorSash** (components/diffEditorSash.ts)
- Role: Split view resize sash
- Used by: `diffEditorWidget.ts`, `gutterFeature.ts`

### Feature Classes

**HideUnchangedRegionsFeature** (features/hideUnchangedRegionsFeature.ts)
- Role: Collapse/expand unchanged code regions
- Dependencies: `diffEditorViewModel.ts`, `diffEditorEditors.ts`, `diffEditorOptions.ts`, `utils.ts`
- Used by: `diffEditorWidget.ts`

**MovedBlocksLinesFeature** (features/movedBlocksLinesFeature.ts)
- Role: Visual indicators for moved code blocks
- Dependencies: `diffEditorViewModel.ts`, `diffEditorEditors.ts`, `utils.ts`
- Used by: `diffEditorWidget.ts`

**DiffEditorGutter** (features/gutterFeature.ts)
- Role: Gutter actions and toolbar
- Dependencies: `diffEditorViewModel.ts`, `diffEditorEditors.ts`, `diffEditorOptions.ts`, `diffEditorSash.ts`, `utils.ts`, `editorGutter.ts`
- Used by: `diffEditorWidget.ts`

**RevertButtonsFeature** (features/revertButtonsFeature.ts)
- Role: Revert change buttons in glyph margin
- Dependencies: `diffEditorViewModel.ts`, `diffEditorEditors.ts`, `diffEditorOptions.ts`, `diffEditorWidget.ts`
- Used by: `diffEditorWidget.ts`

**OverviewRulerFeature** (features/overviewRulerFeature.ts)
- Role: Overview ruler diff indicators
- Dependencies: `diffEditorViewModel.ts`, `diffEditorEditors.ts`, `utils.ts`
- Used by: `diffEditorWidget.ts`

### Supporting Classes

**DiffEditorOptions** (diffEditorOptions.ts)
- Role: Options management and derived observables
- Dependencies: `diffEditorViewModel.ts`
- Used by: 5 files (widget, features)

**DiffState**, **DiffMapping**, **UnchangedRegion** (diffEditorViewModel.ts)
- Role: Data structures for diff state
- Exported from: `diffEditorViewModel.ts`
- Used by: `diffEditorWidget.ts`, `diffEditorOptions.ts`, `hideUnchangedRegionsFeature.ts`

## 3. Dependency Map

### Internal Module Dependencies (count)

| File | Internal Imports | Internal Dependents |
|------|-----------------|---------------------|
| diffEditorWidget.ts | 14 | 2 (commands, embeddedDiffEditorWidget) |
| diffEditorViewModel.ts | 3 (diffProviderFactoryService, utils, diffEditorOptions) | 9 (all features, components, options, widget) |
| components/accessibleDiffViewer.ts | 1 (diffEditorEditors) | 1 (diffEditorWidget) |
| components/diffEditorViewZones/diffEditorViewZones.ts | 2 (renderLines, inlineDiffDeletedCodeMargin) | 1 (diffEditorWidget) |
| diffEditorOptions.ts | 2 (diffEditorViewModel, diffState) | 5 (widget, 3 features) |
| utils.ts | 0 | 10 (widget, 4 features, 2 components) |
| diffEditorEditors.ts | 0 | 7 (accessibleDiffViewer, all 5 features, diffEditorWidget) |
| features/* | 0 internal (only external + utils/viewModel) | 1 each (diffEditorWidget) |

### Dependency Graph

```
                    ┌─────────────────────────┐
                    │   DiffEditorWidget      │ ◄── commands.ts
                    │   (orchestrates all)    │ ◄── embeddedDiffEditorWidget.ts
                    └───────────┬─────────────┘
                                │
                ┌───────────────┼───────────────┬───────────────┐
                │               │               │               │
        ┌───────▼──────┐  ┌────▼────┐   ┌──────▼──────┐  ┌────▼────┐
        │ViewModel     │  │ Options │   │ Components  │  │Features │
        │ (core state) │  └─────────┘   │             │  │         │
        └──────────────┘                 └─────────────┘  └─────────┘
                │                                │               │
          ┌─────┴─────────┐                     │               │
          │               │                     │               │
     ┌────▼────┐    ┌────▼───────┐      ┌──────▼──────┐  ┌────▼────────┐
     │ DiffProv│    │ utils.ts   │      │ Editors     │  │ (5 features)│
     │ Factory │    │ (helpers)  │      │ ViewZones   │  │             │
     └─────────┘    └────────────┘      │ Decorations │  │             │
                                         │ Sash        │  │             │
                                         │ AccessDiff  │  │             │
                                         └─────────────┘  └─────────────┘
```

### External Dependencies (top imports by file)

- **diffEditorWidget.ts**: 45 external imports (observable, DOM, services, editor core)
- **accessibleDiffViewer.ts**: 30 external imports (DOM, UI components, editor core)
- **diffEditorViewZones.ts**: 28 external imports (observable, editor core, DOM)
- **gutterFeature.ts**: 27 external imports (toolbar, actions, services)
- **hideUnchangedRegionsFeature.ts**: 23 external imports (DOM, observable, languages)

## 4. Public API Surface

### Primary Exports

**DiffEditorWidget** (diffEditorWidget.ts)
- Main class implementing `IDiffEditor` interface
- Public methods: ~50+ (from IEditor + IDiffEditor interfaces)
- Key public APIs:
  - `setModel(model: IDiffEditorModel | null | IDiffEditorViewModel)`
  - `getModel(): IDiffEditorModel | null`
  - `getOriginalEditor(): ICodeEditor`
  - `getModifiedEditor(): ICodeEditor`
  - `updateOptions(newOptions: IDiffEditorOptions)`
  - `layout(dimension?: IDimension)`
  - Navigation methods (goToDiff, revealFirstDiff, etc.)

**EmbeddedDiffEditorWidget** (embeddedDiffEditorWidget.ts)
- Extends `DiffEditorWidget` for embedded scenarios
- Additional: `getParentEditor(): ICodeEditor`

**IDiffProviderFactoryService** (diffProviderFactoryService.ts)
- Service interface for diff computation
- Implementations: `WorkerBasedDiffProviderFactoryService`, `WorkerBasedDocumentDiffProvider`

### Command Exports (commands.ts)

- `ToggleCollapseUnchangedRegions`
- `ToggleShowMovedCodeBlocks`
- `ToggleUseInlineViewWhenSpaceIsLimited`
- `SwitchSide`
- `ExitCompareMove`
- `CollapseAllUnchangedRegions`
- `ShowAllUnchangedRegions`
- `RevertHunkOrSelection`
- `AccessibleDiffViewerNext`
- `AccessibleDiffViewerPrev`
- Helper functions: `findDiffEditor`, `findFocusedDiffEditor`, `findDiffEditorContainingCodeEditor`

### Data Model Exports (diffEditorViewModel.ts)

- `DiffEditorViewModel` - implements `IDiffEditorViewModel`
- `DiffState` - encapsulates diff computation results
- `DiffMapping` - mapping between original and modified
- `UnchangedRegion` - represents collapsed regions

### Utility Exports (utils.ts)

20+ exported functions and classes:
- `joinCombine`, `applyObservableDecorations`, `animatedObservable`
- `PlaceholderViewZone`, `ManagedOverlayWidget`
- `applyViewZones`, `applyStyle`, `translatePosition`
- Numerous helper functions

## 5. File Complexity Analysis

### By Line Count

| Rank | File | Lines | Complexity Factor |
|------|------|-------|-------------------|
| 1 | diffEditorWidget.ts | 805 | **CRITICAL** (orchestrator + 45 imports + 14 internal deps) |
| 2 | diffEditorViewModel.ts | 771 | **CRITICAL** (core logic + 9 dependents) |
| 3 | accessibleDiffViewer.ts | 737 | **HIGH** (complex UI + 30 imports) |
| 4 | diffEditorViewZones.ts | 651 | **HIGH** (view zone logic + 28 imports) |
| 5 | utils.ts | 536 | **MEDIUM** (10 dependents, 0 internal imports) |
| 6 | hideUnchangedRegionsFeature.ts | 497 | **MEDIUM** (complex feature + 23 imports) |
| 7 | movedBlocksLinesFeature.ts | 377 | **MEDIUM** |
| 8 | gutterFeature.ts | 331 | **MEDIUM** (27 imports) |
| 9 | renderLines.ts | 316 | **LOW** (rendering utility) |
| 10 | commands.ts | 301 | **LOW** (command definitions) |

### By Import Count (External)

| File | External Imports | Risk Level |
|------|------------------|------------|
| diffEditorWidget.ts | 45 | **CRITICAL** |
| accessibleDiffViewer.ts | 30 | **HIGH** |
| diffEditorViewZones.ts | 28 | **HIGH** |
| gutterFeature.ts | 27 | **HIGH** |
| hideUnchangedRegionsFeature.ts | 23 | **MEDIUM** |
| diffEditorViewModel.ts | 21 | **MEDIUM** |

### By Internal Dependency Count

**Most dependent upon (high coupling risk):**
- `diffEditorViewModel.ts` - 9 files depend on it
- `utils.ts` - 10 files depend on it
- `diffEditorEditors.ts` - 7 files depend on it

**Most dependencies (high change risk):**
- `diffEditorWidget.ts` - imports from 14 internal modules
- `diffEditorViewModel.ts` - imports from 3 internal modules
- `components/diffEditorViewZones.ts` - imports from 2 internal modules

## 6. The 3 Highest-Risk Files to Change

### 🔴 #1: diffEditorWidget.ts (805 lines)

**Risk Factors:**
- **Central orchestrator**: Imports and instantiates all 5 features + 4 components
- **Highest internal coupling**: 14 internal module imports
- **Highest external coupling**: 45 external imports
- **Public API surface**: Main entry point implementing full IDiffEditor interface
- **Broad responsibility**: Layout, initialization, model management, feature coordination
- **Dependents**: 2 files (commands.ts, embeddedDiffEditorWidget.ts)

**Impact of changes:**
- Affects all features and components
- Breaks public API contracts
- Ripple effect to embedded variant and commands
- High test surface area

**Refactoring challenges:**
- Separating concerns without breaking API
- Managing observable lifecycle and subscriptions
- Coordinating multiple feature lifecycles

---

### 🟠 #2: diffEditorViewModel.ts (771 lines)

**Risk Factors:**
- **Core business logic**: All diff computation and state management
- **Highest dependent count**: 9 files depend on it (all features, components, options)
- **Complex state**: Manages diff state, unchanged regions, move detection
- **Async computation**: Handles cancellation, scheduling, worker coordination
- **Data structure exports**: DiffState, DiffMapping, UnchangedRegion used widely

**Impact of changes:**
- Breaks all features and components that depend on view model
- Changes to DiffState/DiffMapping ripple throughout codebase
- Diff algorithm changes affect all consuming code
- Observable interface changes cascade

**Refactoring challenges:**
- Cannot break exported data structures without major refactor
- Complex observable chains and derived state
- Diff computation performance constraints
- Maintaining backward compatibility

---

### 🟡 #3: components/accessibleDiffViewer.ts (737 lines)

**Risk Factors:**
- **Large and complex**: 737 lines of accessibility logic
- **High external coupling**: 30 external imports (DOM, UI, rendering)
- **Accessibility compliance**: Changes risk breaking screen reader support
- **Complex rendering**: Custom scrollable element, line rendering, state sync
- **Public interface**: AccessibleDiffViewerModelFromEditors used by diffEditorWidget

**Impact of changes:**
- Accessibility regression risk (WCAG compliance)
- Complex DOM manipulation and event handling
- Breaks accessible diff viewer commands
- Testing requires assistive technology

**Refactoring challenges:**
- Maintaining accessibility while refactoring
- Complex DOM and event handling logic
- Synchronization with main diff editor state
- Limited automated testing for accessibility

---

## 7. Refactoring Recommendations Summary

### Structural Issues Identified

1. **God Object Pattern**: `diffEditorWidget.ts` orchestrates too much (805 lines, 14 internal deps)
2. **High Coupling**: `diffEditorViewModel.ts` has 9 dependents (shared mutable state)
3. **Mixed Concerns**: Accessibility viewer (737 lines) mixes UI, state, and rendering
4. **Utility Sprawl**: `utils.ts` (536 lines, 10 dependents) is a catch-all module
5. **Feature Fragmentation**: 5 separate feature files with similar patterns

### Critical Paths for Refactoring

1. **Extract coordinator** from diffEditorWidget.ts (reduce from 805 lines)
2. **Isolate view model state** to reduce coupling from 9 dependents
3. **Split accessibleDiffViewer** into view/model/controller
4. **Consolidate utilities** with clear module boundaries
5. **Unify feature pattern** with base class or interface

### Files to Approach with Caution

- **diffEditorWidget.ts**: Any change affects entire widget
- **diffEditorViewModel.ts**: Changes cascade to 9 dependents
- **accessibleDiffViewer.ts**: Accessibility compliance risk

### Lower-Risk Refactoring Targets

- Individual feature files (isolated by design)
- renderLines.ts, copySelection.ts (pure utilities)
- Registration/contribution files (declarative)
- diffEditorSash.ts (focused responsibility)

---

## 8. Metrics Summary

| Metric | Value |
|--------|-------|
| Total files | 24 |
| Total lines | 7,849 |
| Largest file | diffEditorWidget.ts (805 lines) |
| Most coupled (dependents) | diffEditorViewModel.ts (9) |
| Most dependencies | diffEditorWidget.ts (14 internal) |
| Public API classes | 4 primary (DiffEditorWidget, EmbeddedDiffEditorWidget, DiffEditorViewModel, DiffProviderFactoryService) |
| Feature modules | 5 |
| Component modules | 4 (+4 sub-components) |
| Commands | 10 |
| Utility classes | 20+ |

---

**Analysis completed**: All findings based on static code analysis using grep, find, wc, and pattern matching.
