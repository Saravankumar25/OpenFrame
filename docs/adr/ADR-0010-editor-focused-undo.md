# ADR-0010: Editor-focused undo in the screenplay editor vs global undo

- **Status:** Accepted. The foundation part is implemented: the shell ignores Ctrl+Z/Y inside editable targets
  (`apps/desktop/src/app/shell/Shell.tsx`, `isEditable`), and coalesce keys exist in `MutationMeta`. The editor part
  is to be implemented by the screenplay module.
- **Date:** 2026-09-30
- **Related:** FSD §16.1 ("undo/redo … behave like a professional writing app"), §51 (undo across workspaces);
  UX shortcuts (Ctrl/Cmd+Z, Ctrl/Cmd+Shift+Z); ADR-0005

## Context

There are two legitimate expectations of Ctrl+Z:

1. **Inside the screenplay editor** (and any text field), writers expect character- and word-level undo that restores
   the caret and selection instantly, like Word or Final Draft (FSD §16.1).
2. **Everywhere else**, Ctrl+Z undoes the last *project action* (moved card, deleted shot, schedule move) with a human
   label, as recorded by the global, persistent undo stack (ADR-0005).

A single global stack for keystrokes would need a round trip to Rust per Ctrl+Z, would lose caret state, and would
fight the 4 s typing coalescing window. An editor-only stack cannot undo structural actions made outside the editor.

## Decision

1. **Focus decides the scope.** When focus is in an editable element (`input`, `textarea`, `select`,
   `contenteditable`, including the ProseMirror screenplay editor), Ctrl+Z / Ctrl+Y / Ctrl+Shift+Z belong to that
   element and the shell does nothing. Otherwise the shell runs global undo/redo for the current scope (`project`, or
   `global` while the Global Idea Vault is shown).
2. **Screenplay editor = editor-local history.** The editor keeps a `prosemirror-history` stack for the current
   editing session of the open draft. This provides caret-restoring undo and grouping at word boundaries.
3. **Persistence is continuous and coalesced.** Each editor transaction is persisted through screenplay commands
   (e.g. `screenplay.update_element`) with a **coalesce key per element**. The global stack therefore holds one step
   per typing burst (≤ 4 s), labelled in human terms (e.g. "Edited dialogue in Scene 12"). Nothing is lost if the app
   crashes (autosave).
4. **Editor undo is a forward edit globally.** An editor-local undo produces a new persisted change. The global stack
   records it as an edit, not as an "undo". Global history stays linear and consistent, and the stale check
   (ADR-0005) still protects against overwriting.
5. **External changes reset the local stack.** When a `DataChanged` event touches rows the editor shows and did not
   originate from this editor (toolbar/global undo, applied Change Set or package response, restore), the editor reloads
   the affected elements and **clears its local history**. It never replays stale local steps over newer content.
6. **Toolbar Undo/Redo buttons are always global.** Clicking them moves focus out of the editor, so the behavior
   matches rule 1.
7. **Structural actions** (add/delete/reorder scenes, change draft, apply revision) are global commands, undoable
   from the global stack only.

## Consequences

- Writers get native-feeling undo while typing, and project-level undo stays reliable and persistent.
- After the editor loses its local history (rule 5), the global stack can still revert typing bursts, one coalesced
  step at a time.
- Two stacks need clear UX: the toolbar tooltip shows the global label (`Undo: Moved Scene Card …`), and the editor's
  own undo is only reachable by keyboard while typing, as in any text editor.
- Plain form fields (titles, notes) use the browser's native field undo while focused. Their committed values are
  global steps.

## Alternatives rejected

| Alternative | Reason |
|---|---|
| Global stack only (every keystroke/undo via Rust) | Latency per Ctrl+Z, no caret restoration, and conflicts with coalescing |
| Editor stack only for screenplay work | Cannot undo scene deletes or reorders made from menus. Lost on reload. |
| Merging the two stacks (editor steps replayed into global undo) | Bidirectional synchronization of two histories is complex and error-prone, and it risks overwriting later work |
