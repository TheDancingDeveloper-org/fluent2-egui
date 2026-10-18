//! Selection and keyboard focus for the DataGrid (`docs/specs/data-views.md` §3).
//!
//! These functions change [`DataGridState`] and report what happened. They do not draw. The grid
//! calls them from `show`, once per key press or click.

use egui::{Key, Modifiers};

use super::{DataGridState, FocusMode, RowId, SelectionMode};
use crate::data::rows::{
    all_selected, find_row, focus_row, select_all, select_only, select_range, step_column, step_row, toggle,
};

/// Points the grid scrolls for one Left or Right in row focus (§3.3).
pub(super) const HORIZONTAL_SCROLL: f32 = 40.0;
/// Points a Ctrl+Left or Ctrl+Right resizes the focused header column (§3.3).
pub(super) const RESIZE_STEP: f32 = 8.0;

/// What the grid knows about its rows and columns for one frame.
pub(super) struct Layout<'a> {
    /// Data rows, not counting the header.
    pub row_count: usize,
    /// Data columns, not counting the selection checkbox column.
    pub column_count: usize,
    pub mode: SelectionMode,
    pub focus_mode: FocusMode,
    /// Rows moved by PageUp and PageDown: the visible rows less one.
    pub page: usize,
    pub row_id: &'a dyn Fn(usize) -> RowId,
}

/// What one key press asked for, besides the state changes it made.
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub(super) struct KeyOutcome {
    /// Focus moved. The grid scrolls the new focus into view.
    pub moved: bool,
    /// Enter: the grid reports `activated` for the focused row or cell.
    pub activate: bool,
    /// F2: the grid reports `edit_requested`.
    pub edit: bool,
    /// Shift+F10: the grid reports `context_menu`.
    pub context_menu: bool,
    /// Enter or Space on a header cell: sort it (§2.1).
    pub sort: bool,
    /// Ctrl+Left or Ctrl+Right on a header cell: resize it by this many points (§2.2).
    pub resize: f32,
    /// Ctrl+Enter on a header cell: autofit it (§2.3).
    pub autofit: bool,
    /// Left or Right in row focus: scroll the grid by this many points (signed).
    pub scroll_x: f32,
    /// The selection changed.
    pub selection_changed: bool,
}

/// How a move changes the selection, from the modifier keys.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
struct Extend {
    /// Shift: select the range from the anchor.
    shift: bool,
    /// Ctrl: move focus without selecting.
    ctrl: bool,
}

/// Keys pressed this frame, repeats included, with their modifiers.
pub(super) fn pressed_keys(ui: &egui::Ui) -> Vec<(Key, Modifiers)> {
    ui.input(|input| {
        input
            .events
            .iter()
            .filter_map(|event| match event {
                egui::Event::Key {
                    key,
                    pressed: true,
                    modifiers,
                    ..
                } => Some((*key, *modifiers)),
                _ => None,
            })
            .collect()
    })
}

/// Keep the focus on the row it names when the app reorders or shortens the data (§3.4).
pub(super) fn resolve_focus(state: &mut DataGridState, layout: &Layout) {
    let Some(focus) = state.focus.as_mut() else {
        return;
    };
    if focus.header || layout.row_count == 0 {
        return;
    }
    let row = match find_row(focus.row_index_hint, layout.row_count, layout.row_id, focus.row) {
        Some(row) => row,
        None => {
            let row = focus.row_index_hint.min(layout.row_count - 1);
            focus.row = (layout.row_id)(row);
            row
        }
    };
    focus.row_index_hint = row;
}

/// Apply one key press to the selection and focus.
pub(super) fn handle_key(
    state: &mut DataGridState,
    layout: &Layout,
    key: Key,
    modifiers: Modifiers,
) -> KeyOutcome {
    let before = state.selection.clone();
    let mut out = KeyOutcome::default();
    let extend = Extend {
        shift: modifiers.shift,
        ctrl: modifiers.command || modifiers.ctrl,
    };
    let (row, column) = position(state, layout);
    // The header row is always cell-focused (§3.2).
    let cell_focus = layout.focus_mode == FocusMode::Cell || row.is_none();

    match key {
        Key::ArrowUp | Key::ArrowDown | Key::PageUp | Key::PageDown => {
            let page = layout.page.max(1) as isize;
            let step = match key {
                Key::ArrowUp => -1,
                Key::ArrowDown => 1,
                Key::PageUp => -page,
                _ => page,
            };
            let target = step_row(row, step, layout.row_count);
            move_to(state, layout, row, target, column, extend, &mut out);
        }
        Key::Home | Key::End => {
            let to_end = key == Key::End;
            if cell_focus && !extend.ctrl {
                // First or last cell of the row. The row stays.
                let target = if to_end {
                    layout.column_count.saturating_sub(1)
                } else {
                    0
                };
                set_focus(state, layout, row, target);
                out.moved = true;
            } else {
                let target = match (to_end, layout.row_count) {
                    (_, 0) => None,
                    (true, count) => Some(count - 1),
                    (false, _) => Some(0),
                };
                move_to(state, layout, row, target, column, extend, &mut out);
            }
        }
        Key::ArrowLeft | Key::ArrowRight if row.is_none() && extend.ctrl => {
            out.resize = if key == Key::ArrowLeft {
                -RESIZE_STEP
            } else {
                RESIZE_STEP
            };
        }
        Key::ArrowLeft | Key::ArrowRight => {
            let step = if key == Key::ArrowLeft { -1 } else { 1 };
            if cell_focus {
                let target = step_column(column, step, layout.column_count);
                set_focus(state, layout, row, target);
                out.moved = true;
            } else {
                out.scroll_x = step as f32 * HORIZONTAL_SCROLL;
            }
        }
        // The header cell: Enter or Space sorts, Ctrl+Enter autofits (§3.3).
        Key::Enter | Key::Space if row.is_none() => {
            if key == Key::Enter && extend.ctrl {
                out.autofit = true;
            } else {
                out.sort = true;
            }
        }
        Key::Enter => out.activate = true,
        Key::F2 => out.edit = true,
        Key::F10 if modifiers.shift => out.context_menu = true,
        Key::Space if layout.mode == SelectionMode::Multi => {
            if let Some(row) = row {
                set_focus(state, layout, Some(row), column);
                toggle(&mut state.selection, (layout.row_id)(row));
            }
        }
        Key::A if extend.ctrl && layout.mode == SelectionMode::Multi => {
            select_all(&mut state.selection, layout.row_count, layout.row_id);
        }
        _ => {}
    }

    out.selection_changed = state.selection != before;
    out
}

/// Apply a click on the data cell at `row` and `column` (indices). Returns whether the
/// selection changed.
pub(super) fn click(
    state: &mut DataGridState,
    layout: &Layout,
    row: usize,
    column: usize,
    modifiers: Modifiers,
) -> bool {
    let before = state.selection.clone();
    let shift = modifiers.shift;
    let ctrl = modifiers.command || modifiers.ctrl;
    set_focus(state, layout, Some(row), column);
    let row_id = (layout.row_id)(row);
    match layout.mode {
        SelectionMode::None => {}
        SelectionMode::Single => select_only(&mut state.selection, row_id),
        SelectionMode::Multi if shift => {
            let anchor = anchor_index(state, layout).unwrap_or(row);
            select_range(
                &mut state.selection,
                anchor,
                row,
                layout.row_count,
                layout.row_id,
                ctrl,
            );
        }
        SelectionMode::Multi if ctrl => toggle(&mut state.selection, row_id),
        SelectionMode::Multi => select_only(&mut state.selection, row_id),
    }
    state.selection != before
}

/// The row checkbox of `row` was clicked. Toggles that row and makes it the anchor (§3.1).
pub(super) fn row_checkbox(state: &mut DataGridState, layout: &Layout, row: usize) -> bool {
    let before = state.selection.clone();
    let column = state.focus.and_then(|focus| focus.column).unwrap_or(0);
    set_focus(state, layout, Some(row), column);
    toggle(&mut state.selection, (layout.row_id)(row));
    state.selection != before
}

/// The header checkbox was clicked: select all, or clear when all rows are selected (§3.1).
pub(super) fn header_checkbox(state: &mut DataGridState, layout: &Layout) -> bool {
    let before = state.selection.clone();
    if all_selected(&state.selection, layout.row_count, layout.row_id) {
        state.selection.selected.clear();
        state.selection.anchor = None;
    } else {
        select_all(&mut state.selection, layout.row_count, layout.row_id);
    }
    state.selection != before
}

/// Where keyboard focus is now: `(row, column)`, with `None` for the header row. With no focus
/// yet, focus starts on the first row.
fn position(state: &DataGridState, layout: &Layout) -> (Option<usize>, usize) {
    let Some(focus) = state.focus else {
        return (if layout.row_count > 0 { Some(0) } else { None }, 0);
    };
    let column = focus
        .column
        .unwrap_or(0)
        .min(layout.column_count.saturating_sub(1));
    if focus.header || layout.row_count == 0 {
        return (None, column);
    }
    let row = find_row(focus.row_index_hint, layout.row_count, layout.row_id, focus.row)
        .unwrap_or_else(|| focus.row_index_hint.min(layout.row_count - 1));
    (Some(row), column)
}

/// Index of the anchor row, if it is still in the data.
fn anchor_index(state: &DataGridState, layout: &Layout) -> Option<usize> {
    let anchor = state.selection.anchor?;
    let hint = state.focus.map_or(0, |focus| focus.row_index_hint);
    find_row(hint, layout.row_count, layout.row_id, anchor)
}

/// Move focus from `from` to `target` (`None` is the header) and apply the selection rule for a
/// move. Shift extends a range from the anchor. Ctrl only moves focus. A plain move selects the
/// focused row in Single and Multi.
fn move_to(
    state: &mut DataGridState,
    layout: &Layout,
    from: Option<usize>,
    target: Option<usize>,
    column: usize,
    extend: Extend,
    out: &mut KeyOutcome,
) {
    set_focus(state, layout, target, column);
    out.moved = true;
    match (layout.mode, target) {
        (SelectionMode::None, _) | (_, None) => {}
        (SelectionMode::Multi, Some(to)) if extend.shift => {
            if state.selection.anchor.is_none() {
                state.selection.anchor = from.map(|row| (layout.row_id)(row));
            }
            let anchor = anchor_index(state, layout).or(from).unwrap_or(to);
            select_range(
                &mut state.selection,
                anchor,
                to,
                layout.row_count,
                layout.row_id,
                false,
            );
        }
        (SelectionMode::Multi, Some(_)) if extend.ctrl => {}
        (_, Some(to)) => select_only(&mut state.selection, (layout.row_id)(to)),
    }
}

/// Set focus to `row` (`None` for the header) and `column`.
fn set_focus(state: &mut DataGridState, layout: &Layout, row: Option<usize>, column: usize) {
    let header = row.is_none();
    let cell = header || layout.focus_mode == FocusMode::Cell;
    let row_id = match row {
        Some(row) => (layout.row_id)(row),
        None => state.focus.map_or(RowId(0), |focus| focus.row),
    };
    focus_row(
        &mut state.focus,
        row.unwrap_or(0),
        row_id,
        cell.then_some(column),
        header,
    );
}

#[cfg(test)]
mod tests {
    use egui::{Key, Modifiers};

    use super::{HORIZONTAL_SCROLL, Layout, click, handle_key, header_checkbox, resolve_focus, row_checkbox};
    use crate::data::RowId;
    use crate::data::data_grid::{DataGridState, FocusMode, SelectionMode};

    fn id(row: usize) -> RowId {
        RowId(row as u64)
    }

    fn layout(mode: SelectionMode, focus_mode: FocusMode, rows: usize) -> Layout<'static> {
        Layout {
            row_count: rows,
            column_count: 3,
            mode,
            focus_mode,
            page: 5,
            row_id: &id,
        }
    }

    fn selected(state: &DataGridState) -> Vec<u64> {
        state.selection.selected.iter().map(|id| id.0).collect()
    }

    fn focus_row_of(state: &DataGridState) -> Option<usize> {
        state.focus.map(|focus| focus.row_index_hint)
    }

    const NONE: Modifiers = Modifiers::NONE;

    #[test]
    fn plain_click_selects_only_that_row() {
        let mut state = DataGridState::default();
        let layout = layout(SelectionMode::Multi, FocusMode::Row, 10);
        click(&mut state, &layout, 2, 0, NONE);
        click(&mut state, &layout, 4, 0, NONE);
        assert_eq!(selected(&state), vec![4]);
    }

    #[test]
    fn ctrl_click_toggles_and_shift_click_selects_the_range() {
        let mut state = DataGridState::default();
        let layout = layout(SelectionMode::Multi, FocusMode::Row, 10);
        click(&mut state, &layout, 1, 0, NONE);
        click(&mut state, &layout, 3, 0, Modifiers::CTRL);
        assert_eq!(selected(&state), vec![1, 3]);
        // Ctrl+click made row 3 the anchor, so Shift+click ranges from 3.
        click(&mut state, &layout, 6, 0, Modifiers::SHIFT);
        assert_eq!(selected(&state), vec![3, 4, 5, 6]);
    }

    #[test]
    fn shift_click_replaces_and_ctrl_shift_adds() {
        let mut state = DataGridState::default();
        let layout = layout(SelectionMode::Multi, FocusMode::Row, 10);
        click(&mut state, &layout, 0, 0, NONE);
        click(&mut state, &layout, 5, 0, Modifiers::SHIFT);
        assert_eq!(selected(&state), vec![0, 1, 2, 3, 4, 5]);
        click(&mut state, &layout, 8, 0, Modifiers::SHIFT);
        assert_eq!(selected(&state), vec![0, 1, 2, 3, 4, 5, 6, 7, 8]);
        click(&mut state, &layout, 9, 0, Modifiers::SHIFT | Modifiers::CTRL);
        assert_eq!(selected(&state).len(), 10);
    }

    #[test]
    fn single_mode_selection_follows_focus() {
        let mut state = DataGridState::default();
        let layout = layout(SelectionMode::Single, FocusMode::Row, 10);
        click(&mut state, &layout, 2, 0, Modifiers::CTRL);
        assert_eq!(selected(&state), vec![2]);
        handle_key(&mut state, &layout, Key::ArrowDown, NONE);
        assert_eq!(selected(&state), vec![3]);
    }

    #[test]
    fn none_mode_never_selects() {
        let mut state = DataGridState::default();
        let layout = layout(SelectionMode::None, FocusMode::Row, 10);
        click(&mut state, &layout, 2, 0, NONE);
        handle_key(&mut state, &layout, Key::ArrowDown, NONE);
        assert!(state.selection.selected.is_empty());
        assert_eq!(focus_row_of(&state), Some(3));
    }

    #[test]
    fn arrow_up_reaches_the_header_and_down_returns_to_row_zero() {
        let mut state = DataGridState::default();
        let layout = layout(SelectionMode::Single, FocusMode::Row, 3);
        handle_key(&mut state, &layout, Key::ArrowUp, NONE);
        assert!(state.focus.is_some_and(|focus| focus.header));
        handle_key(&mut state, &layout, Key::ArrowDown, NONE);
        assert_eq!(selected(&state), vec![0]);
    }

    #[test]
    fn home_end_and_ctrl_variants_move_to_the_ends() {
        let mut state = DataGridState::default();
        let layout = layout(SelectionMode::Single, FocusMode::Row, 20);
        click(&mut state, &layout, 7, 1, NONE);
        handle_key(&mut state, &layout, Key::End, NONE);
        assert_eq!(selected(&state), vec![19]);
        handle_key(&mut state, &layout, Key::Home, NONE);
        assert_eq!(selected(&state), vec![0]);
        handle_key(&mut state, &layout, Key::End, Modifiers::CTRL);
        assert_eq!(selected(&state), vec![19]);
    }

    #[test]
    fn cell_home_and_end_move_within_the_row() {
        let mut state = DataGridState::default();
        let layout = layout(SelectionMode::None, FocusMode::Cell, 20);
        click(&mut state, &layout, 4, 1, NONE);
        handle_key(&mut state, &layout, Key::End, NONE);
        let focus = state.focus.unwrap_or_else(|| panic!("no focus"));
        assert_eq!((focus.row_index_hint, focus.column), (4, Some(2)));
        handle_key(&mut state, &layout, Key::Home, NONE);
        assert_eq!(state.focus.and_then(|focus| focus.column), Some(0));
        assert_eq!(focus_row_of(&state), Some(4));
    }

    #[test]
    fn page_down_moves_by_a_page_and_clamps_at_the_end() {
        let mut state = DataGridState::default();
        let layout = layout(SelectionMode::Single, FocusMode::Row, 100);
        handle_key(&mut state, &layout, Key::PageDown, NONE);
        assert_eq!(selected(&state), vec![5]);
        for _ in 0..40 {
            handle_key(&mut state, &layout, Key::PageDown, NONE);
        }
        assert_eq!(selected(&state), vec![99]);
    }

    #[test]
    fn shift_arrow_extends_from_the_anchor() {
        let mut state = DataGridState::default();
        let layout = layout(SelectionMode::Multi, FocusMode::Row, 10);
        click(&mut state, &layout, 2, 0, NONE);
        handle_key(&mut state, &layout, Key::ArrowDown, Modifiers::SHIFT);
        handle_key(&mut state, &layout, Key::ArrowDown, Modifiers::SHIFT);
        assert_eq!(selected(&state), vec![2, 3, 4]);
    }

    #[test]
    fn ctrl_arrow_moves_focus_without_selecting() {
        let mut state = DataGridState::default();
        let layout = layout(SelectionMode::Multi, FocusMode::Row, 10);
        click(&mut state, &layout, 2, 0, NONE);
        handle_key(&mut state, &layout, Key::ArrowDown, Modifiers::CTRL);
        assert_eq!(selected(&state), vec![2]);
        assert_eq!(focus_row_of(&state), Some(3));
    }

    #[test]
    fn space_toggles_and_ctrl_a_selects_all_in_multi_only() {
        let mut state = DataGridState::default();
        let rows = layout(SelectionMode::Multi, FocusMode::Row, 4);
        click(&mut state, &rows, 1, 0, NONE);
        handle_key(&mut state, &rows, Key::Space, NONE);
        assert!(state.selection.selected.is_empty());
        handle_key(&mut state, &rows, Key::A, Modifiers::CTRL);
        assert_eq!(selected(&state).len(), 4);

        let mut single = DataGridState::default();
        let rows = layout(SelectionMode::Single, FocusMode::Row, 4);
        handle_key(&mut single, &rows, Key::A, Modifiers::CTRL);
        assert!(single.selection.selected.is_empty());
    }

    #[test]
    fn enter_f2_and_shift_f10_are_reported() {
        let mut state = DataGridState::default();
        let rows = layout(SelectionMode::None, FocusMode::Row, 4);
        assert!(handle_key(&mut state, &rows, Key::Enter, NONE).activate);
        assert!(handle_key(&mut state, &rows, Key::F2, NONE).edit);
        assert!(handle_key(&mut state, &rows, Key::F10, Modifiers::SHIFT).context_menu);
        assert!(!handle_key(&mut state, &rows, Key::F10, NONE).context_menu);
    }

    #[test]
    fn left_and_right_scroll_in_row_focus_and_move_in_cell_focus() {
        let mut state = DataGridState::default();
        let rows = layout(SelectionMode::None, FocusMode::Row, 4);
        let out = handle_key(&mut state, &rows, Key::ArrowRight, NONE);
        assert_eq!(out.scroll_x, HORIZONTAL_SCROLL);
        assert!(!out.moved);

        let mut cells = DataGridState::default();
        let rows = layout(SelectionMode::None, FocusMode::Cell, 4);
        for _ in 0..3 {
            handle_key(&mut cells, &rows, Key::ArrowRight, NONE);
        }
        assert_eq!(cells.focus.and_then(|focus| focus.column), Some(2));
    }

    #[test]
    fn row_checkbox_toggles_and_sets_the_anchor() {
        let mut state = DataGridState::default();
        let layout = layout(SelectionMode::Multi, FocusMode::Row, 5);
        row_checkbox(&mut state, &layout, 3);
        assert_eq!(selected(&state), vec![3]);
        assert_eq!(state.selection.anchor, Some(id(3)));
        row_checkbox(&mut state, &layout, 3);
        assert!(state.selection.selected.is_empty());
    }

    #[test]
    fn header_checkbox_selects_all_then_clears() {
        let mut state = DataGridState::default();
        let layout = layout(SelectionMode::Multi, FocusMode::Row, 3);
        assert!(header_checkbox(&mut state, &layout));
        assert_eq!(selected(&state).len(), 3);
        assert!(header_checkbox(&mut state, &layout));
        assert!(state.selection.selected.is_empty());
    }

    #[test]
    fn focus_follows_a_row_that_moved_under_it() {
        let mut state = DataGridState::default();
        let layout = layout(SelectionMode::None, FocusMode::Row, 10);
        click(&mut state, &layout, 6, 0, NONE);
        // The app swapped rows 2 and 6: the focused id (6) is now at index 2.
        let reordered = |row: usize| match row {
            2 => RowId(6),
            6 => RowId(2),
            other => RowId(other as u64),
        };
        let moved = Layout {
            row_id: &reordered,
            ..layout
        };
        resolve_focus(&mut state, &moved);
        assert_eq!(focus_row_of(&state), Some(2));
    }
}
