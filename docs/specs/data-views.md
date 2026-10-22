# Data views spec

Status: draft for WI-1580. Implementation items WI-1581 to WI-1585 cite the section numbers
below; keep the numbering. This document contains no implementation. The only code is public
type and function signatures, written as the contract an implementation must meet.

Conventions are fixed by [`docs/ARCHITECTURE.md`](../ARCHITECTURE.md) (rules 1 to 10). Where this
spec adds a rule, it says so under **Decisions**.

## Decisions

Each decision is reversible by editing this file before the implementation item that depends on
it starts.

| # | Decision | Why |
|---|---|---|
| D1 | The grid is virtualised with one `egui::ScrollArea::both().show_viewport`, not `egui_extras::TableBuilder` and not `ScrollArea::show_rows`. Visible rows are computed from the viewport and a uniform row height (§1.4). | `TableBuilder` keeps column widths in its own egui memory, so `DataGridState` could not own them (item §1); it has no sort, selection, roving focus or AccessKit hooks; and its resize handles cannot autofit from visible rows. `show_rows` scrolls only the rows: a sticky header that also scrolls horizontally needs the viewport rect, which `show_viewport` gives. gridcraft's sheet (`crates/ui-egui/src/grid.rs`) uses the same shape: one allocated rect, its own geometry, only visible cells painted. |
| D2 | Rows have one uniform height per grid, chosen by `DataGridSize` (§10). Variable row heights are not supported in 0.1. Amended by WI-1800: List (only) may opt in to measured heights (§6.1); the DataGrid and TreeView keep D2. | Row index ↔ y is then O(1) both ways, which virtualisation, PageUp/PageDown and scroll-to-focus need. |
| D3 | The app identifies rows with a `RowId(u64)` it supplies (`row_id` closure, default: the row index). Selection and focus are stored as `RowId`s, never as indices. | The app sorts (item §2). After a sort the same index is a different row; ids keep the selection. |
| D4 | One cell closure per grid, `FnMut(CellCtx, &mut Ui) -> R` with `R: CellResponse`, passed to `show`. `R` is `()` for a cell with no interactive child, or the child's `Response` (or `Option` of it). Enter on a cell with a focusable child moves focus into it, and Escape returns (§3.3). Columns hold no closures. | Rows are never copied (item §1) and the app's data is borrowed by one closure, not by one closure per column competing for `&mut`. craft-libs' docking `DockContent` adapter takes the same approach. |
| D5 | State structs (`DataGridState`, `TreeState`, `ListState`, `NavState`, `SplitState`) are plain data with public fields and derive `serde::{Serialize, Deserialize}` behind a new optional `serde` cargo feature of `fluent2-egui`. WI-1581 adds the feature. | The item asks for serde-able state. An optional feature keeps the default dependency set unchanged (office-chrome D3). |
| D6 | Sizes that are not Fluent tokens (row heights, selection column width, nav pane widths) are fields of a `DataMetrics` struct held by `Theme` (`Theme::data`), the same way `Theme::chrome` holds `ChromeMetrics`. WI-1581 adds it. | Rule 7. The precedent is office-chrome D2. |
| D7 | DataGrid, TreeView and List share one crate-private module, `data/rows.rs`: viewport-to-row maths, focus movement, selection maths (anchor, ranges, toggles) and row background painting. It is the "single code path" of item §6. | One keyboard and selection behaviour across the three views. |
| D8 | SplitView adapts craft-libs `crates/ui/src/layout/geometry.rs` and `resize.rs` (§8). craft-libs moves from `role = "reference"` to `role = "adapted"` in that PR. | The code is small, panic-free, already handles NaN and tiny rects, and has keyboard and AccessKit support. Writing it again gains nothing. |
| D9 | **No docking in 0.1** (§9). Amended by WI-1794: docking ships after 0.1 as the optional `docking` feature, on our own split tree (§9.1). | See §9. |
| D10 | Column ids, tree keys of the default type, and nav item keys are `&'static str` or caller-chosen hashable values, never labels. Duplicates do not panic: the first wins. | Rules 4 and 9; office-chrome D4. |

## File layout

```
data/
  mod.rs
  rows.rs            crate-private, D7
  data_grid/
    mod.rs           DataGrid, DataGridResponse, CellCtx       (§1)
    column.rs        Column, ColumnWidth, Align                (§1)
    state.rs         DataGridState, SortSpec, Selection        (§1, §2, §3)
    header.rs        header row, sort, resize, autofit         (§2)
    a11y.rs          AccessKit nodes                           (§4)
  tree_view.rs                                                 (§5)
  list.rs                                                      (§6)
  nav_view.rs                                                  (§7)
  split_view.rs      adapted from craft-libs                   (§8)
```

`data_grid/` is a folder, as ARCHITECTURE's module layout already says. `rows.rs` and
`split_view.rs` are the two files this spec adds to `data/`.

## 1. DataGrid model and API

### 1.1 Columns

```rust
/// Width policy of a column. Values are logical points.
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum ColumnWidth {
    /// Exactly this wide.
    Fixed(f32),
    /// Autofit on first show (§2.3), then behaves as `Fixed`.
    Auto,
    /// A share of the width left after fixed and auto columns. Shares are relative:
    /// `Fraction(1.0)` and `Fraction(2.0)` split the remainder 1:2.
    Fraction(f32),
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Default)]
pub enum Align { #[default] Start, Center, End }

pub struct Column {
    /* private */
}

impl Column {
    /// `id` names the column in state and responses (D10). `header` is the visible label.
    pub fn new(id: &'static str, header: impl Into<egui::WidgetText>) -> Self;
    /// Default `ColumnWidth::Fraction(1.0)`.
    pub fn width(self, width: ColumnWidth) -> Self;
    /// Default `min = 44`, `max = f32::INFINITY`. Applied after the policy and after resize.
    pub fn min_width(self, min: f32) -> Self;
    pub fn max_width(self, max: f32) -> Self;
    /// Default true.
    pub fn sortable(self, sortable: bool) -> Self;
    /// Default true.
    pub fn resizable(self, resizable: bool) -> Self;
    /// Alignment of header text and of the cell `Ui` layout. Default `Start`.
    pub fn align(self, align: Align) -> Self;
}
```

Width resolution each frame, in order: (1) a width in `DataGridState::widths` for the column
id wins; (2) `Fixed(w)` gives `w`; (3) `Auto` gives the autofit width, measured once and written
to `state.widths`; (4) `Fraction` columns share `max(0, available − sum of the others)`. Every
result is clamped to `[min, max]`. If `min > max`, `min` wins. Non-finite widths become `min`
(rule 9). When total width exceeds the grid, the grid scrolls horizontally.

### 1.2 Row source and cell closure

```rust
/// What the cell closure is told.
#[derive(Clone, Copy, Debug)]
pub struct CellCtx {
    /// Row index into the app's (already sorted) data, `0..row_count`.
    pub row: usize,
    pub row_id: RowId,
    pub column: &'static str,
    /// Column index in display order.
    pub column_index: usize,
    pub selected: bool,
    /// This cell (cell focus) or its row (row focus) has keyboard focus.
    pub focused: bool,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct RowId(pub u64);

pub struct DataGrid<'a> { /* private */ }

impl<'a> DataGrid<'a> {
    pub fn new(id_salt: impl std::hash::Hash, columns: &'a [Column], row_count: usize) -> Self;
    /// Stable ids for rows (D3). Default `|row| RowId(row as u64)`.
    pub fn row_id(self, f: impl Fn(usize) -> RowId + 'a) -> Self;
    /// Default `DataGridSize::Medium`.
    pub fn size(self, size: DataGridSize) -> Self;
    /// Default `SelectionMode::None`.
    pub fn selection_mode(self, mode: SelectionMode) -> Self;
    /// Default `FocusMode::Row`.
    pub fn focus_mode(self, mode: FocusMode) -> Self;
    /// Accessible name of the grid.
    pub fn label(self, label: impl Into<String>) -> Self;
    pub fn show<R: CellResponse>(
        self,
        ui: &mut egui::Ui,
        state: &mut DataGridState,
        cell: impl FnMut(CellCtx, &mut egui::Ui) -> R, // see D4
    ) -> DataGridResponse;
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Default)]
pub enum DataGridSize { ExtraSmall, Small, #[default] Medium }
```

The cell closure is called only for cells of visible rows (and for autofit, §2.3). Each call
gets a child `Ui` clipped to the cell rect, with horizontal padding `spacingHorizontalS` and the
column's alignment, vertically centred. Text widgets inside it should truncate; the grid does
not wrap. The closure may add interactive widgets; clicks on them do not change row selection
(the grid checks `ui.ctx().is_being_dragged`/the inner response's `clicked` before handling the
row click).

`row_count` larger than `u32::MAX` is clamped for AccessKit indices only. An empty grid
(`row_count == 0`) shows the header and nothing else; the app draws its own empty state.

### 1.3 State

```rust
#[derive(Clone, Debug, Default, PartialEq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct DataGridState {
    /// `None` = unsorted.
    pub sort: Option<SortSpec>,
    /// User-resized or autofitted widths by column id. Owned strings so serde works.
    pub widths: std::collections::BTreeMap<String, f32>,
    pub selection: Selection,
    /// Focused row and, in cell focus mode, column index. `None` before first focus.
    pub focus: Option<GridFocus>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct SortSpec { pub column: String /* id */, pub direction: SortDirection }

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub enum SortDirection { Ascending, Descending }

#[derive(Clone, Debug, Default, PartialEq, Eq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct Selection {
    pub selected: std::collections::BTreeSet<RowId>,
    /// Anchor for Shift ranges.
    pub anchor: Option<RowId>,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct GridFocus {
    pub row: RowId,
    /// Last known index of `row`, used when `row` is no longer found (§3.4).
    pub row_index_hint: usize,
    /// Column index in cell focus; `None` in row focus.
    pub column: Option<usize>,
    /// Focus is on the header row (then `column` is the header cell).
    pub header: bool,
}
```

Column ids are stored as `String` in state so state deserialises without
`'static` data; the API compares them with the `&'static str` ids.

### 1.4 Virtualisation and the sticky header

Layout inside `ScrollArea::both().show_viewport` (D1). Content size is
`(sum of column widths, header_height + row_count × row_height)`. Each frame, with `viewport`
the visible rect in content coordinates:

- first visible row `= floor(max(0, viewport.min.y) / row_height)`, last
  `= ceil((viewport.max.y − header_height) / row_height)`, clamped to `0..row_count`. Only
  these rows are laid out, painted and given AccessKit nodes.
- Rows are placed at `y = header_height + row × row_height`.
- The header is painted last, at `y = viewport.min.y` (so it stays at the top), at the same x
  offset as the rows (so it scrolls horizontally with them), on top of rows scrolled under it.
  It is interactive there; row interaction is clipped to below the header.
- Column x positions are a prefix sum of resolved widths, computed once per frame.

`state.focus` changes by keyboard call `ui.scroll_to_rect` on the focused row (and cell) with
`None` alignment, so it scrolls only when needed.

### 1.5 Response

```rust
pub struct DataGridResponse {
    /// The whole grid. `has_focus` while any part of the grid has focus.
    pub response: egui::Response,
    /// The user changed `state.sort` this frame (§2.1). Re-sort before next frame.
    pub sort_changed: bool,
    pub selection_changed: bool,
    /// A column was resized or autofitted this frame.
    pub column_resized: Option<&'static str>,
    /// Enter or double-click on a row (or cell, in cell mode).
    pub activated: Option<CellCtx>,
    /// F2 on a row or cell. The grid does not edit; the app may show an editor.
    pub edit_requested: Option<CellCtx>,
    /// Secondary click or Shift+F10/Menu key; attach a `context_menu` to this.
    pub context_menu: Option<CellCtx>,
}
```

## 2. Sorting and resizing

### 2.1 Sort

Clicking a sortable header cell (or Enter/Space on a focused header cell) cycles that column
none → ascending → descending → none. Choosing a different column starts it at ascending. One
sort column only in 0.1. The grid writes `state.sort`, sets `sort_changed`, and does nothing
else: the app sorts its data and passes rows in the new order (item §2). The header shows
`ArrowSortUp` / `ArrowSortDown` (16 pt, icon catalogue; WI-1581 adds the names if missing via
the `fluent2-icons` generator) after the label for the sorted column.

### 2.2 Resize

Each resizable column has a handle on its right edge: hit width 8 pt centred on the border,
cursor `ResizeHorizontal`, drawn as a 1 pt `neutral_stroke_1` line only on hover or drag. Drag
writes `state.widths[id]`, clamped to `[min, max]`, and sets `column_resized`. Fraction columns
to the right do not shrink to compensate; the grid scrolls instead. Keyboard resizing is in
§3.3. The handle reuses the drag maths of SplitView's `ResizeHandle` (§8), axis horizontal.

### 2.3 Autofit

Double-click on a handle, and first show of an `Auto` column, set the width to
`max(header width, widest visible cell) + 2 × spacingHorizontalS`, clamped. "Visible cell"
means rows in the current viewport only (item §2); rows never shown are not measured. A cell is
measured by calling the cell closure in a sizing pass (`UiBuilder::new().sizing_pass()
.invisible()`) and reading `min_rect().width()`. The header width includes the sort icon slot.

## 3. Selection and keyboard

### 3.1 Modes

```rust
#[derive(Clone, Copy, Debug, PartialEq, Eq, Default)]
pub enum SelectionMode { #[default] None, Single, Multi }

#[derive(Clone, Copy, Debug, PartialEq, Eq, Default)]
pub enum FocusMode { #[default] Row, Cell }
```

- **None:** rows highlight on hover, nothing is selected; Enter still activates.
- **Single:** click or focus move selects exactly that row (selection follows focus).
- **Multi:** a 44 pt checkbox column (`Theme::data.selection_cell_width`) is first, using
  `Checkbox` (WI-1546). Its header is a select-all checkbox showing `CheckState::Mixed` when
  some but not all rows are selected; clicking it selects all (`0..row_count` mapped through
  `row_id`) or clears. A row checkbox toggles that row and sets the anchor. A plain row click
  replaces the selection with that row; Ctrl+click toggles; Shift+click selects the range
  anchor..row (replacing); Ctrl+Shift+click adds that range.

Selection maths lives in `data/rows.rs` (D7) and works on indices, converting to `RowId` at the
boundary. Select-all over very large `row_count` is O(n); the app can avoid it by using
`SelectionMode::Single`.

### 3.2 Focus

The grid is one Tab stop (roving focus). Tab enters at `state.focus`, or the first row, or the
header if `row_count == 0`. Tab again leaves the grid; widgets inside cells are reached with
Enter (cell mode, §3.3), not Tab. In **row** focus the whole row is the focus target and gets
the focus ring; in **cell** focus a single cell is. The header row is always cell-focused.

### 3.3 Keys

| Key | Row focus | Cell focus |
|---|---|---|
| Up / Down | previous / next row; Up from row 0 goes to the header | same, keeping the column |
| Left / Right | no-op (scroll horizontally by 40 pt) | previous / next cell; stops at the ends |
| Home / End | first / last row | first / last cell of the row |
| Ctrl+Home / Ctrl+End | first / last row | first / last row, same column |
| PageUp / PageDown | move by `visible rows − 1` | same, keeping the column |
| Shift + move (Multi) | extend the range from the anchor | same |
| Ctrl + move (Multi) | move focus without selecting | same |
| Space (Multi) | toggle focused row | toggle focused row |
| Ctrl+A (Multi) | select all | select all |
| Enter | `activated` | if the cell has an interactive child, focus moves into it (Escape returns); else `activated` |
| F2 | `edit_requested` | `edit_requested` |
| Shift+F10, Menu | `context_menu` | `context_menu` |
| Escape | clears nothing; releases focus lock | from inside a cell child: back to the cell |

On a focused header cell: Left/Right move between header cells, Down goes to row 0, Enter or
Space sorts (§2.1), Ctrl+Left/Ctrl+Right resize the column by 8 pt, Ctrl+Enter autofits.

Arrow, Home/End and PageUp/PageDown keys are taken with a focus lock filter
(`set_focus_lock_filter`) while the grid has focus, as `ResizeHandle` does for arrows.

### 3.4 Focus after the data changes

Each frame the grid looks for `focus.row` near `row_index_hint` (the hint index first, then a
linear search of visible rows). If found, the hint updates. If not found, focus moves to
`min(row_index_hint, row_count − 1)`. Selected ids that no longer exist stay in the set; the
app prunes them if it wants to. This is the only lookup cost of D3.

## 4. Accessibility

AccessKit nodes via `ctx.accesskit_node_builder` (rule 8). Verify the exact names in the pinned
accesskit version in WI-1583, as office-chrome §9.1 does.

| Part | Role | Properties |
|---|---|---|
| Grid | `Grid` | label; `row_count = row_count + 1` (header counts); `column_count`; `multiselectable` when Multi |
| Header row | `Row` | `row_index = 0` |
| Header cell | `ColumnHeader` | label; `column_index`; `sort_direction` on the sorted column; click action |
| Data row | `Row` | `row_index = row + 1`; `selected` when Single/Multi |
| Data cell | `Cell` (`GridCell` if the version has it) | `column_index`; `row_index`; label from the cell's text children |
| Selection checkbox | `CheckBox` | from `Checkbox` |

Only visible rows get nodes (D1). Because `row_count` and every `row_index` are absolute, a
screen reader announces "row 4 017 of 10 000" for a virtualised row. The focused row or cell is
the AccessKit focus. `Focus` and `Click` action requests on a visible row are honoured;
`ScrollIntoView` is honoured for visible nodes only.

## 5. TreeView

### 5.1 Model

The app does not give the tree a node structure. It answers "what are the children of this key"
only for expanded nodes, and the tree flattens the result into rows with depth each frame. That
is the flat-rows-with-depth model; children of collapsed nodes are never asked for, which makes
children lazy.

```rust
/// One child returned by the app.
pub struct TreeNode<K> {
    pub key: K,
    pub label: egui::WidgetText,
    pub icon: Option<Icon>,
    pub children: Children,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Children {
    None,
    /// Has children; ask when expanded.
    Some,
    /// Expanded but the app is still loading; the tree shows a `Spinner` row.
    Loading,
}

#[derive(Clone, Debug, PartialEq, Eq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct TreeState<K: Ord> {
    pub expanded: std::collections::BTreeSet<K>,
    pub selection: std::collections::BTreeSet<K>,
    pub anchor: Option<K>,
    pub focus: Option<K>,
}

pub struct TreeView<'a, K> { /* private */ }

impl<'a, K: Clone + Ord + std::hash::Hash> TreeView<'a, K> {
    pub fn new(id_salt: impl std::hash::Hash) -> Self;
    /// Default `SelectionMode::Single`.
    pub fn selection_mode(self, mode: SelectionMode) -> Self;
    /// Default `TreeSize::Medium`.
    pub fn size(self, size: TreeSize) -> Self;
    pub fn label(self, label: impl Into<String>) -> Self;
    /// `children(None, out)` gives the roots. Called once per expanded node per frame.
    pub fn show(
        self,
        ui: &mut egui::Ui,
        state: &mut TreeState<K>,
        children: impl FnMut(Option<&K>, &mut Vec<TreeNode<K>>),
    ) -> TreeViewResponse<K>;
}

pub struct TreeViewResponse<K> {
    pub response: egui::Response,
    pub selection_changed: bool,
    /// Expanded this frame; an app with lazy data starts loading here.
    pub expanded: Option<K>,
    pub collapsed: Option<K>,
    pub activated: Option<K>,
    pub context_menu: Option<K>,
}
```

Flattening costs O(visible-and-expanded nodes) per frame, and painting is virtualised with the
same viewport maths as the grid (`rows.rs`). Depth is capped at 64; deeper nodes render at depth
64 (rule 9). A key seen twice in one frame is skipped the second time (D10), which also breaks
cycles.

Each row: indentation `(depth) × spacingHorizontalXXL`, then a 24 pt chevron slot
(`ChevronRight`, rotated to `ChevronDown` when expanded, `duration::FAST`; empty for
`Children::None`), optional 20 pt icon, label. Clicking the chevron toggles without selecting.

**Checks and cells (issue #110).** `TreeNode::checked(CheckState)` draws a 16 pt checkbox in a
24 pt slot after the chevron. A click on it, or Space on the focused node, reports
`TreeViewResponse::check_toggled(key)` without selecting; the app sets the new states and does
the tri-state propagation (a folder is `Mixed` when its children are partly checked).
`TreeView::columns(&[TreeColumn], cell)` adds fixed-width trailing columns with a header row
("Name", then each title); `cell(ui, key, column)` is called for visible rows only, so a
5,000-node tree with size, progress and priority cells stays virtualised.

### 5.2 Keyboard

One Tab stop. Up/Down previous/next visible row; Home/End first/last. **Right:** collapsed →
expand; expanded → first child; leaf → nothing. **Left:** expanded → collapse; otherwise →
parent. **`*`:** expand all siblings of the focused node (WAI-ARIA tree pattern), not the whole
subtree, so lazy loading stays bounded. Enter → `activated`; on a parent with
`SelectionMode::None` Enter also toggles. Space, Shift and Ctrl behave as §3.3 (Multi).
Type-ahead is not in 0.1.

### 5.3 Accessibility

`Tree` on the container (`multiselectable` when Multi), `TreeItem` per row with `level =
depth + 1`, `expanded` (absent for leaves), `selected`, `position_in_set` and `size_of_set`
among its siblings, and the label. The Loading row is a `TreeItem` labelled "Loading" with
`busy`.
A checked node's `TreeItem` carries `toggled` (`True`, `False` or `Mixed`, read as "checked",
"not checked", "partially checked"), and its checkbox is a `CheckBox` labelled "Check <label>".

## 6. List

```rust
#[derive(Clone, Debug, Default, PartialEq, Eq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct ListState {
    pub selection: Selection,
    pub focus: Option<RowId>,
    pub focus_hint: usize,
}

pub struct List<'a> { /* private */ }

impl<'a> List<'a> {
    pub fn new(id_salt: impl std::hash::Hash, row_count: usize) -> Self;
    pub fn row_id(self, f: impl Fn(usize) -> RowId + 'a) -> Self;
    /// Default `SelectionMode::Single`.
    pub fn selection_mode(self, mode: SelectionMode) -> Self;
    /// Row height; default `Theme::data.list_row_height`.
    pub fn row_height(self, h: f32) -> Self;
    pub fn label(self, label: impl Into<String>) -> Self;
    pub fn show(self, ui: &mut egui::Ui, state: &mut ListState, row: impl FnMut(CellCtx, &mut egui::Ui)) -> ListResponse;
}

pub struct ListResponse {
    pub response: egui::Response,
    pub selection_changed: bool,
    pub activated: Option<CellCtx>,
    pub context_menu: Option<CellCtx>,
}
```

A List is a one-column grid with no header, row focus only. It uses `rows.rs` for viewport
maths, focus movement, selection and row painting (D7), with the keyboard rows of §3.3 for row
focus. `CellCtx::column` is `""` and `column_index` is 0. Multi mode has no checkbox column;
Ctrl/Shift/Space select. Roles: `ListBox` with `ListBoxOption` children when selection is Single
or Multi; `List` / `ListItem` when None. `size_of_set` and `position_in_set` are absolute, as in
§4.

### 6.1 Variable-height rows (WI-1800, 2026-10-19)

Inbox, history and chat rows differ in height (gap G11). List gets an opt-in mode; the fixed
mode, and DataGrid and TreeView, are unchanged (D2 still holds for them).

```rust
impl<'a> List<'a> {
    /// Measure rows; `estimate` is the height of rows not measured yet. `row_height` becomes
    /// the minimum.
    pub fn variable_height(self, estimate: f32) -> Self;
    /// Forget the measurements of the list made with `id_salt` (font or bulk content change).
    pub fn forget_heights(ctx: &egui::Context, id_salt: impl std::hash::Hash);
}
```

- **Measurement.** A visible row's content is laid out top to bottom in a child `Ui` with no
  height limit; its height plus `spacingVerticalSNudge` above and below, at least `row_height`,
  is stored. A row is drawn with its cached height (or the estimate) and corrected on the next
  frame, which is requested when a height changes by more than half a point.
- **Cache.** `rows::HeightCache` maps `RowId` (D3) to height, so measurements follow rows
  through inserts and sorts. It lives in egui temp memory under the list's id, not in
  `ListState`, so `ListState` stays plain serde data (D5). Heights are clamped to
  `1..=100_000`; non-finite measurements are ignored (rule 9).
- **Geometry.** Each frame builds prefix offsets over all rows, O(n) with one hash lookup per
  row (about 0.1 ms for 10,000 rows); the visible range and the row under a y are binary
  searches. gpui's `ListState` keeps a sum tree for O(log n) updates; a flat prefix array is
  enough at the sizes Vogt shows, and Zed's `sum_tree` (GPL) is not used. Revisit if a list
  needs well over 100,000 rows.
- **Anchoring.** After each frame the list stores a `rows::ScrollAnchor`: the `RowId` at the
  top of the viewport and the distance from its top. On the next frame, if that row's new
  offset plus the distance differs from the last scroll position (rows above changed height,
  or rows were inserted or removed above), the scroll offset is set to it, so the content on
  screen does not move. If the anchor row is gone, nothing is forced.
- **Keyboard reveal.** Arrow, Page, Home and End moves scroll the focused row into view as in
  §3.3. Because the rows they reach are measured only once shown, the reveal is repeated each
  frame, ahead of the anchor, until the row is in view and no height changed, or for at most
  30 frames.
- **Layout.** Row content is top-down (`Align::Min`) rather than the fixed mode's centred
  left-to-right row. Selection, focus ring, roles and `position_in_set` are as in §6.

## 7. NavigationView

### 7.1 Model

```rust
#[derive(Clone, Copy, Debug, PartialEq, Eq, Default)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub enum PaneMode { #[default] Auto, Expanded, Compact, Minimal }

#[derive(Clone, Debug, PartialEq, Eq, Default)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct NavState {
    pub selected: Option<String>,
    /// The user's hamburger choice: in Expanded/Compact it collapses or expands the pane;
    /// in Minimal it opens the overlay.
    pub pane_open: bool,
    /// `Auto` follows the breakpoints in §7.2.
    pub mode: PaneMode,
}

pub struct NavigationView<'a> { /* private */ }

impl<'a> NavigationView<'a> {
    pub fn new(id_salt: impl std::hash::Hash) -> Self;
    pub fn item(self, key: &'static str, icon: Icon, label: impl Into<String>) -> Self;
    /// A section header above the following items. Hidden in Compact mode, replaced by a divider.
    pub fn section(self, label: impl Into<String>) -> Self;
    pub fn divider(self) -> Self;
    /// Pinned to the bottom of the pane (Settings and the like), in the same order.
    pub fn footer_item(self, key: &'static str, icon: Icon, label: impl Into<String>) -> Self;
    /// Optional header text next to the hamburger in Expanded mode.
    pub fn title(self, title: impl Into<String>) -> Self;
    /// The content area to the right of (or under) the pane.
    pub fn show<R>(self, ui: &mut egui::Ui, state: &mut NavState, content: impl FnOnce(&mut egui::Ui) -> R) -> NavResponse<R>;
}

pub struct NavResponse<R> {
    pub inner: R,
    /// The item the user selected this frame (also written to `state.selected`).
    pub selected: Option<&'static str>,
    pub pane_toggled: bool,
}
```

Sub-items (nested `NavCategoryItem`) are not in 0.1.

### 7.2 Modes and breakpoints

With `PaneMode::Auto`, the mode comes from the width of the `Ui` the view is shown in, matching
WinUI NavigationView: **Expanded** at ≥ 1008 pt, **Compact** at 641–1007 pt, **Minimal** at
≤ 640 pt. Breakpoints are `DataMetrics` fields.

- **Expanded:** pane `nav_pane_width` (260 pt, react-nav `defaultDrawerWidth`), icon + label.
  The hamburger collapses it to Compact width until toggled again (`pane_open = false`).
- **Compact:** pane `nav_compact_width` (48 pt), icons only; labels become tooltips
  (`fluent_tooltip`). The hamburger opens the full pane as an overlay above the content
  (`Order::Foreground`, `shadow16`), closed by Escape, a click outside, or selecting an item.
- **Minimal:** no pane; a hamburger button sits at the top-left of the content. It opens the
  same overlay.

Mode changes snap; the overlay slides in with `duration::NORMAL` and `curve::DECELERATE_MID`
(reduced motion snaps). Content never reflows during the overlay.

### 7.3 Items and indicator

Item height 40 pt (react-nav: `spacingVerticalMNudge` × 2 + 20 pt icon). Padding left
`spacingHorizontalMNudge`, right `spacingHorizontalS`, icon–label gap `spacingHorizontalM`,
radius `border_radius::MEDIUM`. Selected item: label `Body1Strong`, foreground
`neutral_foreground_2_brand_selected`, and the selected indicator is TabList's vertical
indicator: `surfaces::tab_list::paint_indicator` (crate-visible) with the TabList Medium
metrics (3 pt bar), sliding between items with TabList's animation (item §7). react-nav's own
indicator is 4 × 20 pt; we use TabList's so the library has one indicator.

### 7.4 Keyboard and accessibility

The pane is one Tab stop: Up/Down move focus among items (footer included, in order), Home/End,
Enter/Space select. The hamburger is a separate Tab stop before the pane. Roles: pane
`Navigation` with the view's label; items as TabList tabs do: `Tab` with `selected`, inside a
`TabList` with vertical orientation, so screen readers report "1 of 6". Section headers are
`Heading` level 2 (not focusable). Hamburger: `Button`, label "Open navigation" / "Close
navigation" (app-overridable), `expanded`.

## 8. SplitView

### 8.1 Upstream

craft-libs at `457a078809690994b22debdffb703ff36d3129df` (MIT OR Apache-2.0, egui 0.36):

- `crates/ui/src/layout/geometry.rs` (135 lines): `SplitAxis`, `SplitSize {Ratio, FixedFirst,
  FixedSecond}`, `split_rect` and `SplitLayout::resized`. Pure, f64 intermediate maths, returns
  `None` on inverted or non-finite input, relaxes minima to half the span in tiny rects.
- `crates/ui/src/layout/resize.rs` (198 lines): `ResizeHandle`, a focusable divider with drag,
  arrow keys along the axis (focus lock filter), AccessKit `Splitter` role with orientation,
  Increment/Decrement/SetValue actions, and a slider value.

**Adapt both** (D8) into `data/split_view.rs`, in the WI-1585 PR, following `UPSTREAM.md`: file
header naming craft-libs and the rev, an `[[adaptation]]` entry per upstream file, the NOTICE
line, and craft-libs `role = "adapted"`. Changes on adaptation: `ResizeHandleStyle` colours are
replaced by theme tokens (§10); the label becomes `impl Into<String>`; the id comes from the
caller's hash (rule 4); `ResizeHandle` stays `pub(crate)` so the grid's column resize (§2.2)
can use it. No ArtCraft name enters the file.

### 8.2 API

```rust
#[derive(Clone, Copy, Debug, PartialEq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct SplitState { pub size: SplitSize }

pub struct SplitView { /* private */ }

impl SplitView {
    pub fn new(id_salt: impl std::hash::Hash, axis: SplitAxis) -> Self;
    /// Per-pane minimum, default 48 pt. When both cannot fit each pane gets half.
    pub fn min_sizes(self, first: f32, second: f32) -> Self;
    /// Keyboard step, default 8 pt; Shift+arrow uses 10× this.
    pub fn step(self, step: f32) -> Self;
    /// Accessible name of the divider, e.g. "Resize navigation".
    pub fn label(self, label: impl Into<String>) -> Self;
    pub fn show<A, B>(
        self,
        ui: &mut egui::Ui,
        state: &mut SplitState,
        first: impl FnOnce(&mut egui::Ui) -> A,
        second: impl FnOnce(&mut egui::Ui) -> B,
    ) -> SplitResponse<A, B>;
}

pub struct SplitResponse<A, B> {
    pub first: A,
    pub second: B,
    pub divider: egui::Response,
    /// `state.size` changed this frame.
    pub resized: bool,
}
```

Upstream takes one minimum; the adaptation extends `clamp_first` to two (`first_min`,
`second_min`), keeping the half-span relaxation. The divider is 1 pt visible
(`neutral_stroke_1`), 8 pt hit area (upstream `hit_margin` 3.5 each side). Double-click on the
divider restores the size passed to `SplitState`'s `Default` (the app's initial value), which
the view keeps in `ctx.data`. Home/End on the focused divider move to the minimum / maximum.

## 9. Docking decision

Checked 2026-10-17 against egui 0.36:

| Option | Version | egui | Licence | Notes |
|---|---|---|---|---|
| `egui_dock` | 0.21.1 (2026-08-06) | `^0.36` | MIT | Tabbed dock tree, floating windows, `TabViewer` trait, a large `Style` struct that would mirror our theme. |
| `egui_tiles` | 0.17.1 (2026-08-18) | `^0.36.0` | MIT OR Apache-2.0 | rerun's tile tree; `Behavior` trait paints tabs and dividers, so our TabList and tokens could paint them directly. serde optional. |
| craft-libs `crates/ui/src/docking` | rev `457a0788` (craft-ui, egui 0.36) | 0.36 | MIT OR Apache-2.0 | ~1 800 lines (`model.rs` 792, `surface.rs` 917, `targets.rs` 99): transactional layout model, permissions, drop targets. Reference only today. |

**Recommendation: no docking in 0.1.** Reasons:

1. The target is Office-style apps (ribbon, backstage, a document, side panes). NavigationView
   and SplitView cover their layout; none of the M1–M3 samples needs drag-to-dock.
2. Each crate pins one egui minor. egui 0.37 shipped on 2026-10-15 and neither crate has a 0.37
   release yet. A docking dependency would make our egui upgrade wait on a third party, against
   the "one egui minor per release line" rule's intent.
3. Adapting craft-libs' docking is ~1 800 lines to review and test, larger than all of M3 here.

After 0.1, if an app needs docking, the recommended route is an optional `docking` feature with
an adapter over **`egui_tiles`**: its `Behavior` trait lets us paint tabs with TabList and the
theme without a parallel style struct, and its licence matches ours. This needs its own work
item and an `upstream.toml` dependency entry.

### 9.1 Amendment (WI-1794, 2026-10-19): our own split tree

The Vogt port (gap G2 in `docs/vogt-port-gap-analysis.md`) needs pane splits, side docks and
dragging a tab into a pane. That reopens the 0.1 decision above. Docking ships as the optional,
off-by-default `docking` cargo feature, on **our own split tree**, not an adapter over
`egui_tiles` or `egui_dock`:

- **No third-party dependency.** Reason 2 above still holds: a docking crate would make every
  egui minor upgrade wait on its release. The feature adds no dependency at all.
- **It is small.** The parts we need are a binary split tree and drop zones. Dividers reuse
  SplitView (§8) and each pane is a TabStrip (WI-1793), so the new code is the tree model,
  its painter and the drop logic.
- **The model is plain data.** `DockTree<T>` holds the app's tab values, so restoring a saved
  layout needs no id-to-widget registry; with `serde` it serialises as is.

Design references only, nothing copied: fluent-gpui `fluent-layout` `dock.rs`, `pane_group.rs`,
`pane.rs`, `workspace.rs` (a two-pane splitter and fixed dock panels; no split tree), Zed's
workspace panes (GPL, design only), and `egui_tiles`.

**Model** (`surfaces::DockTree<T>`). A tree of `DockNode::Pane(Pane { id, tabs, selected })`
and `DockNode::Split { axis, state: SplitState, first, second }`. `PaneId`s are never reused.
Operations: `add_tab`, `split(pane, side, tab)`, `close_tab`, `move_tab(from, index,
DropTarget::{Tabs, Split})`, `reorder_tab`. Invariants after every operation: no empty pane,
no split with one child (an emptied pane is removed and its sibling takes the parent's place),
and a pane's `selected` is in range. A tab cannot split its own pane when it is that pane's
only tab. Invalid ids or indices change nothing.

**View** (`surfaces::DockArea`). The app implements `DockViewer<T>` (`tab(&T) -> StripTab`,
`ui(&mut Ui, &mut T)`). Each split is a SplitView keyed by its first pane's id. Each pane is a
TabStrip above the selected tab's body. A tab dragged out of its strip shows a drop preview
on the pane under the pointer: the outer 25 % of each side splits that side, the middle joins
the pane's tabs. `DockArea::show` applies reorders, moves and closes to the tree and returns
the closed tab values in `DockResponse`. TabStrip gains `dragging` and `dropped_outside` on its
response for this.

**Side docks** (`surfaces::SideDock`). One collapsible panel on the left, right or bottom of
some content, with a title, a close button and a SplitView divider. `SideDockState { open,
size }` is the app's (serde). The app reopens a closed dock.

Out of scope: floating windows, tabs torn out to new OS windows, and docks that hold a
`DockArea` of their own panes (nest a `DockArea` in a dock body instead).

## 10. Tokens

`color.` is `Theme::color`. Sizes marked *DM* are `DataMetrics` fields (D6); others are
`fluent2_tokens::generated::global`.

**DataGrid** (react-table at fluentui `93e324f7`).

| Part | Token / value |
|---|---|
| Row height Medium / Small / ExtraSmall | 44 / 34 / 24 pt *DM* (`grid_row_height_*`) |
| Header row height | 32 pt *DM* (`grid_header_height`) |
| Selection column width | 44 pt *DM* (`selection_cell_width`) |
| Cell padding | `spacing_horizontal::S` |
| Cell text | `Body1`; ExtraSmall uses `Caption1` (`fontSizeBase200`) |
| Text | `color.neutral_foreground_1` |
| Header text | `Body1`, regular weight; `color.neutral_foreground_1` |
| Header hover / pressed | `color.subtle_background_hover` / `color.subtle_background_pressed` |
| Row separator (Medium, Small) | `stroke_width::THIN` `color.neutral_stroke_2` |
| Row hover / pressed | `color.subtle_background_hover` / `color.subtle_background_pressed` |
| Row selected | `color.subtle_background_selected`; hover on selected stays selected |
| Resize line | `color.neutral_stroke_1`, 1 pt, on hover/drag |
| Sort icon | 16 pt, `color.neutral_foreground_1` |
| Focus ring | `paint_focus_ring`, `color.stroke_focus_2`, `border_radius::MEDIUM`, inset 2 pt |

**TreeView** (react-tree).

| Part | Token / value |
|---|---|
| Row height Medium / Small | 32 / 24 pt *DM* (`tree_row_height_*`) |
| Indent per level | `spacing_horizontal::XXL` |
| Row padding | `spacing_horizontal::S`; chevron slot 24 pt; gap `spacing_horizontal::XS` |
| Text / icon | `color.neutral_foreground_2` / `color.neutral_foreground_3` |
| Hover | `color.subtle_background_hover`, text `color.neutral_foreground_2_hover` |
| Pressed | `color.subtle_background_pressed` |
| Selected | `color.subtle_background_selected`, text `color.neutral_foreground_1` |
| Radius, focus | `border_radius::MEDIUM`, `color.stroke_focus_2` |

**List**: row height 32 pt *DM* (`list_row_height`); colours and focus as DataGrid rows, no
separators.

**NavigationView** (react-nav).

| Part | Token / value |
|---|---|
| Pane background | `color.neutral_background_4` |
| Item hover / pressed | `color.neutral_background_4_hover` / `color.neutral_background_4_pressed` |
| Item text, icon | `color.neutral_foreground_2`, `Body1`; icon 20 pt |
| Selected text, icon | `color.neutral_foreground_2_brand_selected`, `Body1Strong` |
| Indicator | TabList indicator: `color.compound_brand_foreground_1`, 3 pt (§7.3) |
| Pane width expanded / compact | 260 / 48 pt *DM* |
| Breakpoints | 1008 / 641 pt *DM* |
| Item height | 40 pt *DM* |
| Section header | `Caption1` (the ramp has no strong caption), `color.neutral_foreground_3`, margin 8 pt vertical |
| Overlay shadow | `shadow16`; overlay motion `duration::NORMAL` |
| Divider | `color.neutral_stroke_2` |

**SplitView**.

| Part | Token / value |
|---|---|
| Divider idle | `color.neutral_stroke_1`, 1 pt |
| Divider hover / drag | `color.neutral_stroke_1_hover` / `color.brand_stroke_1`, 2 pt |
| Divider focus | `paint_focus_ring` around the hit rect |

Token names were checked against `crates/fluent2-tokens/src/generated/colors.rs` at `495741d`
(all present, including the `neutral_background_4_*` and `compound_brand_foreground_1` names).

## 11. Item breakdown check

Against the Vogt titles of WI-1581 to WI-1585 (read 2026-10-17).

| Section | Covered by |
|---|---|
| §1, D5, D6, `rows.rs` viewport maths | WI-1581 |
| §2, `ResizeHandle` reuse | WI-1582 (needs `ResizeHandle` from WI-1585 or writes it first; see below) |
| §3, §4 | WI-1583 |
| §5, §6, `rows.rs` selection reuse | WI-1584 |
| §7, §8, UPSTREAM changes | WI-1585 |
| §9 | none needed (no docking in 0.1); §9.1 docking: WI-1794 |
| §10 | every item, for its part |

**Correction:** §2.2 reuses SplitView's adapted `ResizeHandle`, but WI-1585 can land after
WI-1582. Whichever of the two lands first does the adaptation (§8.1) into
`data/split_view.rs`; the other uses it. Item authors should add §8.1 to WI-1582's references.

## Acceptance

An implementation item author can implement each section without asking a design question.
Signatures follow ARCHITECTURE rules 1 to 10: builders, closures for content, named response
structs, caller ids, no dispatch, no literal colours (sizes go through `DataMetrics`).

Not verified here, checked by the item that cites it: exact AccessKit role and property names
in the pinned accesskit (§4, §5.3, §7.4), and whether `ArrowSortUp`/`ArrowSortDown`/`ChevronDown`
are already in the icon catalogue (§2.1, §5.1).
