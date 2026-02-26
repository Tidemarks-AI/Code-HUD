# IEditorContribution Pattern Analysis - VS Code

## Summary

Found **77 classes** implementing the `IEditorContribution` interface across the VS Code codebase. The pattern is a core architectural feature for extending editor functionality with lifecycle management, dependency injection, and performance optimization through lazy instantiation.

---

## Interface Definition

**Location:** `src/vs/editor/common/editorCommon.ts:583`

```typescript
export interface IEditorContribution {
    /**
     * Dispose this contribution.
     */
    dispose(): void;
    /**
     * Store view state.
     */
    saveViewState?(): unknown;
    /**
     * Restore view state.
     */
    restoreViewState?(state: unknown): void;
}
```

---

## Lifecycle Pattern

### 1. Registration

Contributions are registered using `registerEditorContribution()` function:

**Location:** `src/vs/editor/browser/editorExtensions.ts:552`

```typescript
registerEditorContribution<Services extends BrandedService[]>(
    id: string, 
    ctor: { new(editor: ICodeEditor, ...services: Services): IEditorContribution }, 
    instantiation: EditorContributionInstantiation
): void
```

**Example:**
```typescript
registerEditorContribution(
    CommonFindController.ID, 
    FindController, 
    EditorContributionInstantiation.Eager
);
```

### 2. Instantiation Modes

Five instantiation modes control when contributions are created:

| Mode | Count | Description | Timing |
|------|-------|-------------|--------|
| **Eager** | 16 | Created immediately when editor is instantiated | At editor creation |
| **AfterFirstRender** | 25 | Created after first render (50ms max) | Post-render or idle time |
| **BeforeFirstInteraction** | 17 | Created before user interaction | Pre-interaction or idle time |
| **Eventually** | 8 | Created when idle (5000ms max timeout) | Idle time only |
| **Lazy** | 16 | Created only when explicitly requested via `getContribution()` | On-demand |

**Total registered: 82** (5 more than implementing classes due to wrappers/variations)

**Instantiation Logic Location:** `src/vs/editor/browser/widget/codeEditor/codeEditorContributions.ts`

### 3. Construction

Contributions receive the `ICodeEditor` instance and any required services via dependency injection:

```typescript
constructor(
    editor: ICodeEditor,
    @IContextKeyService contextKeyService: IContextKeyService,
    @IStorageService storageService: IStorageService,
    @IClipboardService clipboardService: IClipboardService
) {
    super();
    this._editor = editor;
    // ... initialization
}
```

### 4. Disposal

Contributions must implement `dispose()` to clean up resources:

```typescript
dispose(): void {
    this._toDispose.dispose();
    this._widget.dispose();
    this._model.dispose();
}
```

---

## Common Implementation Patterns

### Pattern 1: Static ID and Accessor

Every contribution defines a unique ID and static getter:

```typescript
export class SuggestController implements IEditorContribution {
    public static readonly ID: string = 'editor.contrib.suggestController';
    
    public static get(editor: ICodeEditor): SuggestController | null {
        return editor.getContribution<SuggestController>(SuggestController.ID);
    }
    // ...
}
```

### Pattern 2: Disposable Base Class

Most contributions extend `Disposable` for lifecycle management:

```typescript
export class CommonFindController extends Disposable implements IEditorContribution {
    constructor(editor: ICodeEditor, ...) {
        super(); // Initialize Disposable
        this._register(disposable); // Register child disposables
    }
}
```

### Pattern 3: View State Persistence (Eager Only)

Eager contributions can save/restore state:

```typescript
saveViewState?(): unknown {
    return { searchString: this._state.searchString };
}

restoreViewState?(state: unknown): void {
    this._state.change(state, false);
}
```

**Note:** Only `Eager` contributions should implement view state methods (enforced with console warning).

---

## All 77 Identified Contributions

### Core Editor Contributions (vs/editor/contrib/)

1. **FloatingEditorToolbar** - `floatingMenu.ts`
2. **SuggestController** - `suggest/browser/suggestController.ts`
3. **InlayHintsController** - `inlayHints/browser/inlayHintsController.ts`
4. **CommonFindController** - `find/browser/findController.ts`
5. **SnippetController2** - `snippet/browser/snippetController2.ts`
6. **CodeActionController** - `codeAction/browser/codeActionController.ts`
7. **CopyPasteController** - `dropOrPasteInto/browser/copyPasteController.ts`
8. **DropIntoEditorController** - `dropOrPasteInto/browser/dropIntoEditorController.ts`
9. **ViewportSemanticTokensContribution** - `semanticTokens/browser/viewportSemanticTokens.ts`
10. **CursorUndoRedoController** - `cursorUndo/browser/cursorUndo.ts`
11. **StandaloneColorPickerController** - `colorPicker/browser/standaloneColorPicker/standaloneColorPickerController.ts`
12. **HoverColorPickerContribution** - `colorPicker/browser/hoverColorPicker/hoverColorPickerContribution.ts`
13. **ColorDetector** - `colorPicker/browser/colorDetector.ts`
14. **WordHighlighterContribution** - `wordHighlighter/browser/wordHighlighter.ts`
15. **UnicodeHighlighter** - `unicodeHighlighter/browser/unicodeHighlighter.ts`
16. **LongLinesHelper** - `longLinesHelper/browser/longLinesHelper.ts`
17. **ReadOnlyMessageController** - `readOnlyMessage/browser/contribution.ts`
18. **GlyphHoverController** - `hover/browser/glyphHoverController.ts`
19. **ContentHoverController** - `hover/browser/contentHoverController.ts`
20. **StickyScrollController** - `stickyScroll/browser/stickyScrollController.ts`
21. **InPlaceReplaceController** - `inPlaceReplace/browser/inPlaceReplace.ts`
22. **SelectionAnchorController** - `anchorSelect/browser/anchorSelect.ts`
23. **FoldingController** - `folding/browser/folding.ts`
24. **DragAndDropController** - `dnd/browser/dnd.ts`
25. **PeekContextController** - `peekView/browser/peekView.ts`
26. **AutoIndentOnPaste** - `indentation/browser/indentation.ts`
27. **MessageController** - `message/browser/messageController.ts`
28. **SectionHeaderDetector** - `sectionHeaders/browser/sectionHeaders.ts`
29. **RenameController** - `rename/browser/rename.ts`
30. **SmartSelectController** - `smartSelect/browser/smartSelect.ts`
31. **LinkDetector** - `links/browser/links.ts`
32. **PlaceholderTextContribution** - `placeholderText/browser/placeholderTextContribution.ts`
33. **MarkerController** - `gotoError/browser/gotoError.ts`
34. **MarkerSelectionStatus** - `gotoError/browser/markerSelectionStatus.ts`
35. **ContextMenuController** - `contextmenu/browser/contextmenu.ts`
36. **MultiCursorSelectionController** - `multicursor/browser/multicursor.ts`
37. **SelectionHighlighter** - `multicursor/browser/multicursor.ts`
38. **ReferencesController** - `gotoSymbol/browser/peek/referencesController.ts`
39. **GotoDefinitionAtPositionEditorContribution** - `gotoSymbol/browser/link/goToDefinitionAtPosition.ts`
40. **BracketMatchingController** - `bracketMatching/browser/bracketMatching.ts`
41. **CodeLensContribution** - `codelens/browser/codelensController.ts`
42. **FormatOnType** - `format/browser/formatActions.ts`
43. **FormatOnPaste** - `format/browser/formatActions.ts`
44. **LinkedEditingContribution** - `linkedEditing/browser/linkedEditing.ts`
45. **UnusualLineTerminatorsDetector** - `unusualLineTerminators/browser/unusualLineTerminators.ts`
46. **MiddleScrollController** - `middleScroll/browser/middleScrollController.ts`
47. **ParameterHintsController** - `parameterHints/browser/parameterHints.ts`

### Standalone Editor Contributions (vs/editor/standalone/)

48. **IPadShowKeyboard** - `standalone/browser/iPadShowKeyboard/iPadShowKeyboard.ts`
49. **InspectTokensController** - `standalone/browser/inspectTokens/inspectTokens.ts`
50. **QuickInputEditorContribution** - `standalone/browser/quickInput/standaloneQuickInputService.ts`

### Editor Services (vs/editor/browser/services/)

51. **MarkerDecorationsContribution** - `browser/services/markerDecorations.ts`

### Workbench Contributions (vs/workbench/)

52. **FloatingEditorClickMenu** - `workbench/browser/codeeditor.ts`
53. **QuickDiffEditorController** - `workbench/contrib/scm/browser/quickDiffWidget.ts`
54. **CallStackEditorContribution** - `workbench/contrib/debug/browser/callStackEditorContribution.ts`
55. **ClickToLocationContribution** - `workbench/contrib/debug/browser/callStackWidget.ts`
56. **InlayHintsAccessibility** - `workbench/contrib/inlayHints/browser/inlayHintsAccessibilty.ts`
57. **FilterController** - `workbench/contrib/output/browser/outputView.ts`
58. **SelectionClipboard** - `workbench/contrib/codeEditor/electron-browser/selectionClipboard.ts`
59. **EditorDictation** - `workbench/contrib/codeEditor/browser/dictation/editorDictation.ts`
60. **InspectEditorTokensController** - `workbench/contrib/codeEditor/browser/inspectEditorTokens/inspectEditorTokens.ts`
61. **LargeFileOptimizationsWarner** - `workbench/contrib/codeEditor/browser/largeFileOptimizations.ts`
62. **EditorLineNumberContextMenu** - `workbench/contrib/codeEditor/browser/editorLineNumberMenu.ts`
63. **EmptyTextEditorHintContribution** - `workbench/contrib/codeEditor/browser/emptyTextEditorHint/emptyTextEditorHint.ts`
64. **MenuPreventer** - `workbench/contrib/codeEditor/browser/menuPreventer.ts`
65. **ToggleWordWrapController** - `workbench/contrib/codeEditor/browser/toggleWordWrap.ts`
66. **InlineChatController** - `workbench/contrib/inlineChat/browser/inlineChatController.ts`
67. **CommentController** - `workbench/contrib/comments/browser/commentsController.ts`
68. **CallHierarchyController** - `workbench/contrib/callHierarchy/browser/callHierarchy.contribution.ts`
69. **TypeHierarchyController** - `workbench/contrib/typeHierarchy/browser/typeHierarchy.contribution.ts`
70. **CodeCoverageDecorations** - `workbench/contrib/testing/browser/codeCoverageDecorations.ts`
71. **TestingDecorations** - `workbench/contrib/testing/browser/testingDecorations.ts`
72. **TestingOutputPeekController** - `workbench/contrib/testing/browser/testingOutputPeek.ts`
73. **TabCompletionController** - `workbench/contrib/snippets/browser/tabCompletion.ts`

### Sessions Contributions (vs/sessions/)

74. **AgentFeedbackEditorInputContribution** - `sessions/contrib/agentFeedback/browser/agentFeedbackEditorInputContribution.ts`
75. **AgentFeedbackOverviewRulerContribution** - `sessions/contrib/agentFeedback/browser/agentFeedbackOverviewRulerContribution.ts`
76. **AgentFeedbackLineDecorationContribution** - `sessions/contrib/agentFeedback/browser/agentFeedbackLineDecorationContribution.ts`
77. **AgentFeedbackEditorWidgetContribution** - `sessions/contrib/agentFeedback/browser/agentFeedbackEditorWidgetContribution.ts`

---

## Instantiation Examples by Mode

### Eager (Performance-Critical or State Management)

```typescript
// FindController - needs to preserve search state
registerEditorContribution(
    CommonFindController.ID, 
    FindController, 
    EditorContributionInstantiation.Eager
); // eager because it uses `saveViewState`/`restoreViewState`

// CursorUndoRedoController - needs immediate event listening
registerEditorContribution(
    CursorUndoRedoController.ID, 
    CursorUndoRedoController, 
    EditorContributionInstantiation.Eager
); // eager because it needs to listen to record cursor state ASAP
```

### AfterFirstRender (Visual Features)

```typescript
// InlayHintsController - UI decoration after content rendered
registerEditorContribution(
    InlayHintsController.ID, 
    InlayHintsController, 
    EditorContributionInstantiation.AfterFirstRender
);

// StandaloneColorPickerController - color picker UI
registerEditorContribution(
    StandaloneColorPickerController.ID, 
    StandaloneColorPickerController, 
    EditorContributionInstantiation.AfterFirstRender
);
```

### BeforeFirstInteraction (Input Handling)

```typescript
// SuggestController - autocomplete on typing
registerEditorContribution(
    SuggestController.ID, 
    SuggestController, 
    EditorContributionInstantiation.BeforeFirstInteraction
);

// ContextMenuController - right-click handling
registerEditorContribution(
    ContextMenuController.ID, 
    ContextMenuController, 
    EditorContributionInstantiation.BeforeFirstInteraction
);
```

### Eventually (Non-Essential Features)

```typescript
// CodeActionController - code fixes/refactorings
registerEditorContribution(
    CodeActionController.ID, 
    CodeActionController, 
    EditorContributionInstantiation.Eventually
);

// IPadShowKeyboard - platform-specific feature
registerEditorContribution(
    IPadShowKeyboard.ID, 
    IPadShowKeyboard, 
    EditorContributionInstantiation.Eventually
);
```

### Lazy (On-Demand Only)

```typescript
// SnippetController2 - only when snippets are used
registerEditorContribution(
    SnippetController2.ID, 
    SnippetController2, 
    EditorContributionInstantiation.Lazy
);

// InspectTokensController - debug feature
registerEditorContribution(
    InspectTokensController.ID, 
    InspectTokensController, 
    EditorContributionInstantiation.Lazy
);
```

---

## Key Implementation Details

### Dependency Injection

Contributions receive dependencies via constructor parameters decorated with `@IServiceName`:

```typescript
constructor(
    private readonly _editor: ICodeEditor,
    @IContextKeyService contextKeyService: IContextKeyService,
    @IStorageService storageService: IStorageService
) { }
```

### Access from Editor Instance

Contributions are accessed via static getter methods:

```typescript
const suggestController = SuggestController.get(editor);
if (suggestController) {
    suggestController.triggerSuggest();
}
```

### Resource Management

Contributions extending `Disposable` use `this._register()` to track child disposables:

```typescript
this._register(editor.onDidChangeModel(() => { /* ... */ }));
this._register(new DisposableStore());
```

### Performance Optimization

The instantiation strategy optimizes editor startup:

1. **Eager** (16): Only critical contributions load immediately
2. **Deferred** (66): Most contributions load lazily based on need/idle time
3. **Result**: Faster initial editor load, smooth user experience

---

## Architectural Benefits

1. **Modularity**: Features are isolated, testable units
2. **Extensibility**: New contributions follow consistent pattern
3. **Performance**: Lazy loading reduces startup time
4. **Lifecycle Management**: Automatic disposal prevents memory leaks
5. **Dependency Injection**: Clean service access without global state
6. **State Persistence**: Built-in view state save/restore for Eager contributions

---

## Files Analyzed

- Interface: `src/vs/editor/common/editorCommon.ts`
- Registration: `src/vs/editor/browser/editorExtensions.ts`
- Lifecycle: `src/vs/editor/browser/widget/codeEditor/codeEditorContributions.ts`
- Implementations: 77 files across `src/vs/editor/`, `src/vs/workbench/`, `src/vs/sessions/`

**Analysis complete. Pattern documented.**
