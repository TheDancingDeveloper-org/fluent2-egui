# Vogt GUI → fluent2-egui gap analysis

Status: read-only analysis, 2026-10-18. No work items filed yet.

The Vogt GUI today is a SolidJS single-page app (`vogt/web`, vogt-web 0.7.7, Vite, Capacitor for
mobile). The axum server renders no HTML: it serves the API and falls back to `index.html`.
Third-party pieces: xterm.js 6 (fit, search, serialize, web-links), Monaco 0.57 (editor and diff),
and Capacitor (push, speech). Everything else is hand-written: markdown, virtual list, drag and
drop, forms, command palette, and the inline SVG charts.

fluent2-egui state is taken from `origin/main` at `95017ea` plus open PRs. "Merged" means on
main. "Open PR" means a PR exists. "Planned" means a backlog row with no PR. "Specced" means it
is in `docs/specs/*` but has no item.

## 1. Inventory and mapping

### Shell, navigation, layout

| Vogt pattern | Where | fluent2-egui | Status |
|---|---|---|---|
| App shell, surface header per route | `App.tsx`, `SurfaceHeader.tsx` | TitleBar (1559), CommandBar (1569), StatusBar (1569) | Merged |
| Left navigation between surfaces | `routes.ts`, `App.tsx` | NavigationView (1585) | Open PR #39 |
| Resizable split panes | `resizablePane.ts`, `editorSplit.ts` | SplitView (1585) | Open PR #39 |
| Terminal and editor tabs, closable, reorderable | `tabs.ts`, `tabLifecycle.ts`, `terminalDnd.ts` | TabList (1554) has no close, reorder or overflow | Gap: G1 |
| Pane splitting, drag a tab into a pane, saved workspace layouts | `TerminalWorkspace.tsx`, `workspaceLayouts.ts` | Docking deferred past 0.1 (data-views §9) | Gap: G2 |
| Routing, deep links, back button | `routeModel.ts`, HashRouter, `nativeBack.ts` | none (app concern) | Gap: G3 |
| Phone layout, "more" sheet, safe-area insets | `narrow.ts`, `phoneMoreSheet.ts`, `viewportInsets.ts` | NavigationView breakpoints (spec §7.2) only | Gap: G4, G5 |

### Controls, surfaces, feedback

| Vogt pattern | Where | fluent2-egui | Status |
|---|---|---|---|
| Buttons, toggles, menus, context menus | throughout | Button, Toggle, Split/MenuButton, Menu, ContextMenu | Merged |
| Text inputs, search, textarea, checkbox, radio, switch, dropdown, combobox, slider | Settings, SetupWizard, filters | 1546–1549 | Merged |
| Modal dialog | `Dialog.tsx`, `FileWorkflowDialog.tsx` | Dialog (1552) | Merged |
| Tooltip, popover | throughout | 1551 | Merged |
| Badges, session state chips, progress, spinner, link, divider | `sessionRowModel.ts`, `viewAge.tsx` | 1550 | Merged |
| Toasts, inline error bars | Inbox, Board, Terminal and others | MessageBar, Toast (1553) | Open PR #24 |
| Cards | Board cards, Sessions | Card (1554) | Merged |
| Form validation messages | hand-rolled forms | Field (1550) shows the message; validation is the app's job | Merged |
| Multi-step setup wizard | `SetupWizard.tsx`, `SetupSteps.tsx` | none | Gap: G6 |
| Filter bar with saved lenses | `ProgressiveFilters.tsx` | none (built from SearchBox, Dropdown, Menu) | Gap: G7 |
| Freshness "updated N ago", stale or outage tone | `viewAge.tsx` | Badge plus app logic | App, S (G8) |
| Feedback centre | `FeedbackCenter.tsx` | Toast host (1553) plus an app panel | App, S (G9) |
| Avatars, presence | none in Vogt | Avatar/Persona not planned | Not needed |

### Data views

| Vogt pattern | Where | fluent2-egui | Status |
|---|---|---|---|
| Virtualised backlog table | `Backlog.tsx`, `measuredWindow.ts` | DataGrid core (1581) | Merged |
| Sort, column resize | Backlog, Oversight | 1582 | Open PR #35 |
| Row selection, keyboard | Backlog, Oversight | 1583 | Open PR #38 |
| Paged audit table | `AuditBrowser.tsx` | DataGrid plus a pager control | Gap: G10 |
| Rows that are not all the same height (inbox, history) | `measuredWindow.ts` (measured heights) | `rows.rs` assumes a fixed row height | Gap: G11 |
| File tree | `FileTree.tsx` | TreeView (1584) | Planned (branch exists, no PR) |
| Session and inbox lists | `SessionList.tsx`, `Inbox.tsx` | List (1584) | Planned |
| Kanban board with drag between columns | `Board.tsx` (~3.5k lines) | none; egui has drag-and-drop primitives only | Gap: G12 |

### Content renderers

| Vogt pattern | Where | fluent2-egui | Status |
|---|---|---|---|
| Session terminal: ANSI/VT, scrollback, search, links, OSC52, touch | `Terminal.tsx`, `terminal*.ts` (xterm.js) | none | Gap: G13 |
| Session screen snapshot (read-only) | `/api/sessions/{id}/screen` | none (G13 renderer without input) | Gap: G13 |
| Markdown (chat, work items, inbox) | `markdown.tsx` | none | Gap: G14 |
| Code editor | `Editor.tsx` (Monaco) | none; egui TextEdit only | Gap: G15 |
| Diff view (side by side, inline) | `Git.tsx`, `diffLayout.ts` (Monaco diff) | none | Gap: G16 |
| Syntax highlighting | Monaco only | none | Gap: G15 |
| Search snippet with the match highlighted | `SafeSnippet.tsx` | none (a LayoutJob helper) | Gap: G17 |
| Small charts (sparkline, bars) | inline SVG in App, Sessions, Assistant | none | Gap: G18 |
| History replay timeline | `History.tsx`, `historyReplay.ts` | none (Slider plus app) | App, M (G19) |
| Embedded GUI stream | `Gui.tsx` | none (texture viewer in the app) | App, M (G20) |

### Interaction and platform

| Vogt pattern | Where | fluent2-egui | Status |
|---|---|---|---|
| Command palette with fuzzy match and recents | `CommandPalette.tsx`, `commandPaletteScore.ts` | none | Gap: G21 |
| Keyboard shortcuts, `?` help overlay | `keyboardShortcuts.ts`, `KeyboardShortcuts.tsx` | KeyTips (1566/1567) are Office access keys, not a shortcut registry | Gap: G22 |
| Live updates: SSE over fetch, WebSocket, reconnect, SWR cache | `api.ts`, `transport.ts`, `swr.ts`, `connectionHealth.ts` | none (app; needs a repaint hook) | App, M (G23) |
| Login (password, bearer token, auth profiles) | `App.tsx` login, `authApi.ts`, `authProfiles.ts` | Input, Button, Field cover the form | App, S (G24) |
| Theme: named themes plus System, terminal themes | `appThemes.ts`, `terminalThemes.ts` | Theme, brand ramp, `theme_from_system` (1533, 1535, 1541) | Merged; terminal palette is G13 |
| Push notifications, service worker, offline page | `sw.js`, `push.ts` | none; egui has no service worker | Gap: G25 |
| On-screen modifier keys and touch terminal on phones | `ModKeyRow.tsx`, `terminalTouch.ts` | none | Gap: G26 |
| Voice input and calls | `voice*.ts`, `call*.ts`, audio worklet | none | Gap: G27 |
| File upload | `<input type=file>` in `FileTree.tsx` | none (rfd on native, a browser picker on wasm) | App, S (G28) |
| Clipboard copy and paste | `terminalClipboard.ts` | egui clipboard | Built into egui |
| i18n | none in Vogt | ids never come from labels (ARCHITECTURE rule 4) | Not needed |

## 2. Gaps

Size: S is less than a week, M is one to three weeks, L is more than three weeks or a
research spike. Home: **fluent2** means a generic component in this library. **app** means it
lives in the Vogt Rust GUI crate. **ext** means an outside crate wrapped by the app.

| # | Gap | Rationale | Size | Home |
|---|---|---|---|---|
| G1 | Closable, reorderable, overflowing tabs | Terminal and editor tabs are the main workspace. Fluent TabList has no close button, drag reorder or overflow menu. | M | fluent2 |
| G2 | Docking and pane splits | The workspace splits panes and lets tabs be dragged between them. The data-views spec defers docking; the POC can use SplitView alone. | L | fluent2 (optional `docking` feature, per §9) |
| G3 | Router and deep links | Maps `#/w/:ref` and similar routes onto egui state, plus the browser and Android back button. | S | app |
| G4 | Phone layout pass | Touch targets, a bottom sheet in place of the "more" menu, safe-area insets. fluent2 has no compact density or touch sizing. | M | fluent2 (density, BottomSheet); app (layout) |
| G5 | Drawer / bottom sheet surface | Phone more-sheet and side panels. Fluent has Drawer; it is not in the backlog. | S | fluent2 |
| G6 | Wizard / stepper | Setup flow. Can be built from Dialog, Buttons and a step header. | S | app |
| G7 | Filter bar and saved views | Used by Backlog, Inbox and Audit. Fluent has no such component; it is composed from existing controls. | S | app |
| G8 | Freshness indicator | Badge plus a timer. | S | app |
| G9 | Feedback centre | A list of past toasts once Toast lands. | S | app |
| G10 | Pager | Audit is paged on purpose. A small generic control. | S | fluent2 |
| G11 | Variable-height virtual rows | Inbox, history and chat rows differ in height. `rows.rs` assumes a fixed row height. | M | fluent2 (List extension; needs a spec note) |
| G12 | Kanban board | Columns of cards with drag between them, a pending-move state and writes switched off when read-only. Fluent has no board. A generic drag/drop list helper would also serve tab reorder (G1). | M | app (board); fluent2 (drag-reorder helper, S) |
| G13 | Terminal emulator widget | The core Vogt surface. Needs a VT parser, grid, scrollback, selection, search, links, OSC52, themes and wasm. Use `alacritty_terminal` or `vt100` with an egui renderer rather than write one. | L | ext + app (keep it out of fluent2: not a Fluent component) |
| G14 | Markdown renderer | Chat, work items and inbox. `egui_commonmark` exists; it needs restyling with Fluent tokens and Vogt's safe-link and file-link rules. | M | ext + app |
| G15 | Code editor with highlighting | Monaco has no egui equivalent. For the POC, use a read-only highlighted view (`egui_extras` syntect) and leave editing out. | L | ext + app |
| G16 | Diff viewer | The Git surface. Built on `similar` plus G15's highlighter, side by side and inline. | M | app |
| G17 | Highlighted snippet | A LayoutJob helper that highlights the matched text in a theme colour. Generic and small. | S | fluent2 (foundation/text) |
| G18 | Charts | Only sparklines and small bars. `egui_plot` would be enough but does not use our theme; a token-styled Sparkline is smaller. | S | fluent2 (Sparkline) or app |
| G19 | History replay timeline | Scrubber plus a replayed terminal. Depends on G13. | M | app |
| G20 | GUI stream viewer | Decode the stream into a texture. | M | app |
| G21 | Command palette | Generic pattern: a centred popover with search, a ranked list, recents and keyboard control. Built from SearchBox, List and Popover. | M | fluent2 |
| G22 | Shortcut registry and help overlay | Bindings, conflict checks and a `?` overlay. Can sit next to KeyTips but is a separate thing. | S | app (fluent2 could later supply a `ShortcutHint` label) |
| G23 | Live data plumbing | SSE, WebSocket, reconnect, SWR cache and repaint-on-event. Must work native and wasm (`ewebsock`, `reqwest`/`gloo`). | M | app |
| G24 | Login screen | A form using existing controls. | S | app |
| G25 | Push and PWA features | An egui wasm canvas still needs a JS shell for the service worker and push. Native needs OS notifications. For the POC: keep the PWA shell, drop push. | M | app (outside the egui canvas) |
| G26 | Mobile terminal input | Modifier key row, touch scroll, soft keyboard. egui's wasm soft-keyboard support is weak; this is the riskiest part of the mobile port. | L | app (+ egui upstream) |
| G27 | Voice | Web Speech and audio worklets are browser APIs. Out of scope for a visual POC. | L | app (defer) |
| G28 | File picker and upload | `rfd` on native, a file input on wasm. | S | app |

## 3. Reading of the result

- **Already enough for the chrome and forms:** shell, navigation (once #39 lands), dialogs,
  inputs, menus, tooltips, badges, cards, theming including System. Toasts arrive with #24 and
  the tables with #35/#38. With those, Settings, Projects, Backlog, Oversight, Audit, Login and
  Work-item detail (except its markdown) can be built on what is planned.
- **The heavy items are not Fluent components:** terminal (G13), editor (G15), diff (G16),
  markdown (G14). None of them belongs in fluent2. They decide whether the POC is credible,
  because Vogt's main screen is the terminal. The cheapest first POC is to show the read-only
  screen snapshot (G13 without input), read-only diff and markdown, and leave the editor out.
- **Candidates for fluent2 itself** (each would need a backlog item and, for G2/G11, a spec
  note): G1 tabs, G5 drawer, G10 pager, G11 variable-height rows, G12 drag-reorder helper,
  G17 highlighted text, G18 sparkline, G21 command palette, G4 compact/touch density. G2
  docking stays deferred unless the operator wants pane splitting in the POC.
- **Mobile** (G4, G25, G26) is the weakest area for egui. A desktop and wasm-desktop POC
  avoids it.
