//! AccessKit nodes of the DataGrid (`docs/specs/data-views.md` §4).
//!
//! Only visible rows and cells get nodes (D1). Indices and counts are absolute, so a screen
//! reader can say "row 4 017 of 10 000" for a virtualised row. The header is row 0.

use egui::accesskit::Role;
use egui::{Context, Id};

/// The grid itself: role `Grid`, its label, the row count (header included) and the column
/// count.
pub(super) fn grid(
    ctx: &Context,
    id: Id,
    label: Option<&str>,
    row_count: usize,
    column_count: usize,
    multiselectable: bool,
    active: Option<Id>,
) {
    ctx.accesskit_node_builder(id, |node| {
        node.set_role(Role::Grid);
        if let Some(active) = active {
            node.set_active_descendant(active.accesskit_id());
        }
        if let Some(label) = label {
            node.set_label(label);
        }
        node.set_row_count(row_count + 1);
        node.set_column_count(column_count);
        if multiselectable {
            node.set_multiselectable();
        }
    });
}

/// A row. `row_index` is absolute (header 0, data rows from 1). `selected` is set only in
/// Single and Multi modes.
pub(super) fn row(ctx: &Context, id: Id, row_index: usize, selected: Option<bool>) {
    ctx.accesskit_node_builder(id, |node| {
        node.set_role(Role::Row);
        node.set_row_index(row_index);
        if let Some(selected) = selected {
            node.set_selected(selected);
        }
    });
}

/// A data cell.
pub(super) fn cell(ctx: &Context, id: Id, row_index: usize, column_index: usize) {
    ctx.accesskit_node_builder(id, |node| {
        node.set_role(Role::GridCell);
        node.set_row_index(row_index);
        node.set_column_index(column_index);
    });
}

/// A header cell, named by its column's header text (§4). The header row is row 0.
pub(super) fn header_cell(ctx: &Context, id: Id, label: &str, column_index: usize) {
    ctx.accesskit_node_builder(id, |node| {
        node.set_role(Role::ColumnHeader);
        node.set_label(label);
        node.set_row_index(0);
        node.set_column_index(column_index);
    });
}
