# IEditorContribution Pattern Analysis - VS Code

## Executive Summary

The `IEditorContribution` pattern is a core architectural pattern in VS Code for extending the code editor with additional functionality. I identified **50+ implementations** across the codebase. This pattern follows a lifecycle-based plugin architecture with well-defined registration, instantiation, and disposal phases.

---

## Interface Definition

**Location**: `src/vs/editor/common/editorCommon.ts:583-596`

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

### Interface Contract

1. **Required**: `dispose()` - Clean up resources when contribution is destroyed
2. **Optional**: `saveViewState()` - Persist contribution state (e.g., widget positions, UI state)
3. **Optional**: `restoreViewState(state)` - Restore previously saved state

---

## Lifecycle Pattern

### 1. Registration Phase

Contributions are registered using `registerEditorContribution()` with three parameters:

```typescript
registerEditorContribution<Services extends BrandedService[]>(
    id: string,                              // Unique identifier
    ctor: new(editor, ...services) => IEditorContribution,  // Constructor
    instantiation: EditorContributionInstantiation  // Timing strategy
)
```

**Example**:
```typescript
// src/vs/editor/contrib/bracketMatching/browser/bracketMatching.ts:406
registerEditorContribution(
    BracketMatchingController.ID, 
    BracketMatchingController, 
    EditorContributionInstantiation.AfterFirstRender
);
```

### 2. Instantiation Timing Strategies

**Location**: `src/vs/editor/browser/editorExtensions.ts:34-65`

| Strategy | When Created | Use Case |
|----------|--------------|----------|
| `Eager` | Immediately when editor instantiates | Required for `saveViewState`/`restoreViewState` participation |
| `AfterFirstRender` | ≤50ms after first render (or on-demand) | Most common - balances performance and functionality |
| `BeforeFirstInteraction` | Before first user interaction (or on-demand) | Interactive features (hover, context menu) |
| `Eventually` | When idle, ≤5000ms after editor creation | Non-critical features |
| `Lazy` | Only when explicitly requested via `getContribution()` | Rarely-used features |

All strategies (except `Eager` and explicit requests) respect idle time for better performance.

### 3. Construction Pattern

**Common constructor signature**:
```typescript
constructor(
    private readonly _editor: ICodeEditor,
    @IServiceName private readonly _service: IServiceName,
    // ... additional dependency-injected services
)
```

**Standard initialization pattern**:
1. Call `super()` (if extending `Disposable`)
2. Store editor reference
3. Initialize state
4. Register event listeners using `this._register()`
5. Set up initial state

**Example** (`BracketMatchingController`):
```typescript
constructor(editor: ICodeEditor) {
    super();
    this._editor = editor;
    this._decorations = this._editor.createDecorationsCollection();
    this._updateBracketsSoon = this._register(new RunOnceScheduler(...));
    
    this._register(editor.onDidChangeCursorPosition((e) => { ... }));
    this._register(editor.onDidChangeModelContent((e) => { ... }));
    this._register(editor.onDidChangeModel((e) => { ... }));
}
```

### 4. Static Accessor Pattern

Nearly all contributions follow this pattern:

```typescript
export class MyContribution implements IEditorContribution {
    public static readonly ID = 'editor.contrib.myContribution';
    
    public static get(editor: ICodeEditor): MyContribution | null {
        return editor.getContribution<MyContribution>(MyContribution.ID);
    }
}
```

This allows other code to access the contribution via `MyContribution.get(editor)`.

### 5. Disposal Phase

**Automatic cleanup**:
- All `this._register()` calls are automatically disposed when `dispose()` is called
- Must explicitly dispose any resources not registered via `_register()`

**Example** (`ContentHoverController`):
```typescript
public override dispose(): void {
    super.dispose();  // Disposes all _register() calls
    this._unhookListeners();  // Custom cleanup
    this._listenersStore.dispose();  // Explicit disposal
    this._contentWidget?.dispose();  // Explicit disposal
}
```

### 6. View State Persistence (Optional)

Only used by contributions registered with `EditorContributionInstantiation.Eager`.

**Example** (`FindController`):
```typescript
saveViewState(): any {
    return this._widget?.getViewState();
}

restoreViewState(state: any): void {
    this._widget?.setViewState(state);
}

// Registration comment explicitly notes this:
// registerEditorContribution(..., EditorContributionInstantiation.Eager); 
// eager because it uses `saveViewState`/`restoreViewState`
```

---

## Complete List of Implementations

I identified **51 IEditorContribution implementations** in the VS Code codebase:

### Core Editor Contributions (src/vs/editor/)

1. **MarkerDecorationsContribution** - `src/vs/editor/browser/services/markerDecorations.ts:10`
   - Displays diagnostic markers in editor

2. **SelectionAnchorController** - `src/vs/editor/contrib/anchorSelect/browser/anchorSelect.ts:23`
   - Manages anchor-based selections

3. **BracketMatchingController** - `src/vs/editor/contrib/bracketMatching/browser/bracketMatching.ts:115`
   - Highlights matching brackets

4. **CodeActionController** - `src/vs/editor/contrib/codeAction/browser/codeActionController.ts:54`
   - Quick fixes and refactoring suggestions

5. **CodeLensContribution** - `src/vs/editor/contrib/codelens/browser/codelensController.ts:29`
   - Inline code annotations (references, implementations)

6. **ColorDetector** - `src/vs/editor/contrib/colorPicker/browser/colorDetector.ts:29`
   - Detects color values in code

7. **HoverColorPickerContribution** - `src/vs/editor/contrib/colorPicker/browser/hoverColorPicker/hoverColorPickerContribution.ts:15`
   - Color picker on hover

8. **StandaloneColorPickerController** - `src/vs/editor/contrib/colorPicker/browser/standaloneColorPicker/standaloneColorPickerController.ts:14`
   - Standalone color picker widget

9. **ContextMenuController** - `src/vs/editor/contrib/contextmenu/browser/contextmenu.ts:31`
   - Right-click context menu

10. **CursorUndoRedoController** - `src/vs/editor/contrib/cursorUndo/browser/cursorUndo.ts:46`
    - Undo/redo cursor positions

11. **DragAndDropController** - `src/vs/editor/contrib/dnd/browser/dnd.ts:32`
    - Drag-and-drop text editing

12. **CopyPasteController** - `src/vs/editor/contrib/dropOrPasteInto/browser/copyPasteController.ts:79`
    - Smart copy/paste handling

13. **DropIntoEditorController** - `src/vs/editor/contrib/dropOrPasteInto/browser/dropIntoEditorController.ts:42`
    - Drop files/content into editor

14. **CommonFindController** / **FindController** - `src/vs/editor/contrib/find/browser/findController.ts:94`
    - Find/replace functionality (uses `saveViewState`)

15. **FloatingEditorToolbar** - `src/vs/editor/contrib/floatingMenu/browser/floatingMenu.ts:20`
    - Floating toolbar widget

16. **FoldingController** - `src/vs/editor/contrib/folding/browser/folding.ts:68`
    - Code folding/unfolding

17. **FormatOnType** - `src/vs/editor/contrib/format/browser/formatActions.ts:31`
    - Auto-format as you type

18. **FormatOnPaste** - `src/vs/editor/contrib/format/browser/formatActions.ts:155`
    - Format pasted code

19. **MarkerController** - `src/vs/editor/contrib/gotoError/browser/gotoError.ts:28`
    - Navigate between errors/warnings

20. **MarkerSelectionStatus** - `src/vs/editor/contrib/gotoError/browser/markerSelectionStatus.ts:16`
    - Status bar marker info

21. **GotoDefinitionAtPositionEditorContribution** - `src/vs/editor/contrib/gotoSymbol/browser/link/goToDefinitionAtPosition.ts:35`
    - Ctrl+click to definition

22. **ReferencesController** - `src/vs/editor/contrib/gotoSymbol/browser/peek/referencesController.ts:36`
    - Peek references widget

23. **ContentHoverController** - `src/vs/editor/contrib/hover/browser/contentHoverController.ts:39`
    - Content hover tooltips

24. **GlyphHoverController** - `src/vs/editor/contrib/hover/browser/glyphHoverController.ts:34`
    - Glyph margin hover (breakpoints, etc.)

25. **InPlaceReplaceController** - `src/vs/editor/contrib/inPlaceReplace/browser/inPlaceReplace.ts:24`
    - In-place value replacement

26. **AutoIndentOnPaste** - `src/vs/editor/contrib/indentation/browser/indentation.ts:369`
    - Auto-indent pasted code

27. **InlayHintsController** - `src/vs/editor/contrib/inlayHints/browser/inlayHintsController.ts:129`
    - Inline parameter hints

28. **LinkedEditingContribution** - `src/vs/editor/contrib/linkedEditing/browser/linkedEditing.ts:43`
    - Linked editing (rename in sync)

29. **LinkDetector** - `src/vs/editor/contrib/links/browser/links.ts:34`
    - Clickable links in code

30. **LongLinesHelper** - `src/vs/editor/contrib/longLinesHelper/browser/longLinesHelper.ts:12`
    - Performance optimization for long lines

31. **MessageController** - `src/vs/editor/contrib/message/browser/messageController.ts:26`
    - In-editor messages

32. **MiddleScrollController** - `src/vs/editor/contrib/middleScroll/browser/middleScrollController.ts:18`
    - Middle-click scrolling

33. **MultiCursorSelectionController** - `src/vs/editor/contrib/multicursor/browser/multicursor.ts:455`
    - Multi-cursor editing

34. **SelectionHighlighter** - `src/vs/editor/contrib/multicursor/browser/multicursor.ts:842`
    - Highlight matching selections

35. **ParameterHintsController** - `src/vs/editor/contrib/parameterHints/browser/parameterHints.ts:23`
    - Function parameter hints

36. **PeekContextController** - `src/vs/editor/contrib/peekView/browser/peekView.ts:64`
    - Peek view context management

37. **PlaceholderTextContribution** - `src/vs/editor/contrib/placeholderText/browser/placeholderTextContribution.ts:18`
    - Placeholder text in empty editors

38. **ReadOnlyMessageController** - `src/vs/editor/contrib/readOnlyMessage/browser/contribution.ts:15`
    - Read-only editor notifications

39. **RenameController** - `src/vs/editor/contrib/rename/browser/rename.ts:148`
    - Symbol renaming

40. **SectionHeaderDetector** - `src/vs/editor/contrib/sectionHeaders/browser/sectionHeaders.ts:19`
    - Section header detection

41. **ViewportSemanticTokensContribution** - `src/vs/editor/contrib/semanticTokens/browser/viewportSemanticTokens.ts:25`
    - Semantic syntax highlighting

42. **SmartSelectController** - `src/vs/editor/contrib/smartSelect/browser/smartSelect.ts:54`
    - Expand/shrink selection

43. **SnippetController2** - `src/vs/editor/contrib/snippet/browser/snippetController2.ts:51`
    - Snippet insertion/navigation

44. **StickyScrollController** - `src/vs/editor/contrib/stickyScroll/browser/stickyScrollController.ts:50`
    - Sticky scroll headers

45. **SuggestController** - `src/vs/editor/contrib/suggest/browser/suggestController.ts:114`
    - IntelliSense/autocomplete

46. **UnicodeHighlighter** - `src/vs/editor/contrib/unicodeHighlighter/browser/unicodeHighlighter.ts:39`
    - Highlight confusable Unicode characters

47. **UnusualLineTerminatorsDetector** - `src/vs/editor/contrib/unusualLineTerminators/browser/unusualLineTerminators.ts:27`
    - Detect unusual line endings

48. **WordHighlighterContribution** - `src/vs/editor/contrib/wordHighlighter/browser/wordHighlighter.ts:826`
    - Highlight word under cursor

### Standalone Editor Contributions

49. **IPadShowKeyboard** - `src/vs/editor/standalone/browser/iPadShowKeyboard/iPadShowKeyboard.ts:15`
    - iPad keyboard button

50. **InspectTokensController** - `src/vs/editor/standalone/browser/inspectTokens/inspectTokens.ts:25`
    - Token inspection tool

51. **QuickInputEditorContribution** - `src/vs/editor/standalone/browser/quickInput/standaloneQuickInputService.ts:173`
    - Quick input widget integration

### Workbench Editor Contributions (Additional 20+)

The workbench layer adds many more contributions:

52. **AgentFeedbackEditorInputContribution** - `src/vs/sessions/contrib/agentFeedback/browser/agentFeedbackEditorInputContribution.ts:192`
53. **AgentFeedbackEditorWidgetContribution** - `src/vs/sessions/contrib/agentFeedback/browser/agentFeedbackEditorWidgetContribution.ts:421`
54. **AgentFeedbackLineDecorationContribution** - `src/vs/sessions/contrib/agentFeedback/browser/agentFeedbackLineDecorationContribution.ts:29`
55. **AgentFeedbackOverviewRulerContribution** - `src/vs/sessions/contrib/agentFeedback/browser/agentFeedbackOverviewRulerContribution.ts:27`
56. **FloatingEditorClickMenu** - `src/vs/workbench/browser/codeeditor.ts:167`
57. **CallHierarchyController** - `src/vs/workbench/contrib/callHierarchy/browser/callHierarchy.contribution.ts:40`
58. **EditorDictation** - `src/vs/workbench/contrib/codeEditor/browser/dictation/editorDictation.ts:183`
59. **EditorLineNumberContextMenu** - `src/vs/workbench/contrib/codeEditor/browser/editorLineNumberMenu.ts:49`
60. **EmptyTextEditorHintContribution** - `src/vs/workbench/contrib/codeEditor/browser/emptyTextEditorHint/emptyTextEditorHint.ts:37`
61. **InspectEditorTokensController** - `src/vs/workbench/contrib/codeEditor/browser/inspectEditorTokens/inspectEditorTokens.ts:41`
62. **LargeFileOptimizationsWarner** - `src/vs/workbench/contrib/codeEditor/browser/largeFileOptimizations.ts:18`
63. **MenuPreventer** - `src/vs/workbench/contrib/codeEditor/browser/menuPreventer.ts:15`
64. **ToggleWordWrapController** - `src/vs/workbench/contrib/codeEditor/browser/toggleWordWrap.ts:112`
65. **SelectionClipboard** - `src/vs/workbench/contrib/codeEditor/electron-browser/selectionClipboard.ts:26`
66. **CommentController** - `src/vs/workbench/contrib/comments/browser/commentsController.ts:452`
67. **CommentsInputContentProvider** - `src/vs/workbench/contrib/comments/browser/commentsInputContentProvider.ts:20`
68. **CallStackEditorContribution** - `src/vs/workbench/contrib/debug/browser/callStackEditorContribution.ts:118`
69. **ClickToLocationContribution** - `src/vs/workbench/contrib/debug/browser/callStackWidget.ts:662`
70. **InlayHintsAccessibility** - `src/vs/workbench/contrib/inlayHints/browser/inlayHintsAccessibilty.ts:25`
71. **InlineChatController** - `src/vs/workbench/contrib/inlineChat/browser/inlineChatController.ts:103`
72. **FilterController** - `src/vs/workbench/contrib/output/browser/outputView.ts:358`
73. **QuickDiffEditorController** - `src/vs/workbench/contrib/scm/browser/quickDiffWidget.ts:481`
74. **TabCompletionController** - `src/vs/workbench/contrib/snippets/browser/tabCompletion.ts:27`
75. **CodeCoverageDecorations** - `src/vs/workbench/contrib/testing/browser/codeCoverageDecorations.ts:64`
76. **TestingDecorations** - `src/vs/workbench/contrib/testing/browser/testingDecorations.ts:359`
77. **TestingOutputPeekController** - `src/vs/workbench/contrib/testing/browser/testingOutputPeek.ts:430`
78. **TypeHierarchyController** - `src/vs/workbench/contrib/typeHierarchy/browser/typeHierarchy.contribution.ts:39`

---

## Pattern Summary

### Key Characteristics

1. **Decoupled Extension**: Contributions extend editor functionality without modifying core editor code
2. **Lifecycle Management**: Well-defined creation and disposal with automatic cleanup
3. **Performance-Aware**: Multiple instantiation strategies optimize startup time
4. **Service Injection**: Dependency injection for accessing VS Code services
5. **Event-Driven**: React to editor events (cursor, model, configuration changes)
6. **State Management**: Optional view state persistence for UI elements
7. **Singleton per Editor**: Each editor instance gets its own contribution instances

### Common Implementation Patterns

1. **Extend `Disposable`**: Most contributions extend `Disposable` for automatic cleanup
2. **Store Editor Reference**: Keep `_editor: ICodeEditor` field
3. **Static ID + Accessor**: Public static `ID` string and `get(editor)` method
4. **Register Event Handlers**: Use `this._register()` for automatic disposal
5. **Lazy Widget Creation**: Create UI widgets only when needed
6. **Decoration Collections**: Use `IEditorDecorationsCollection` for visual additions

### Registration Best Practices

- **Eager**: Only for contributions needing `saveViewState`/`restoreViewState`
- **AfterFirstRender**: Most common - good balance for visible features
- **BeforeFirstInteraction**: User-facing interactive features (hover, menus)
- **Eventually**: Non-critical enhancements
- **Lazy**: Rarely-used debug/diagnostic tools

---

## Conclusion

The `IEditorContribution` pattern is a well-designed plugin architecture that:

- Enables **modular feature development** without core editor changes
- Provides **performance optimization** through lazy instantiation strategies
- Ensures **proper resource management** via lifecycle hooks
- Supports **dependency injection** for service access
- Allows **state persistence** for UI elements

With **78+ implementations** found, this pattern is the primary mechanism for extending VS Code's code editor functionality, covering everything from basic features (bracket matching, folding) to advanced capabilities (IntelliSense, debugging decorations, AI-assisted coding).
