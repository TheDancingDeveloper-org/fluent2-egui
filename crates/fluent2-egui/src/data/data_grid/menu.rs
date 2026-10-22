//! The column chooser menu (§2.4): show and hide columns, resize to fit, reset.

use egui::Ui;

use super::column::Column;
use super::state::DataGridState;
use crate::surfaces::{MenuItem, divider};

/// What [`column_menu`] did.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct ColumnMenuResponse {
    /// `state.layout` or `state.widths` changed. Persist the state if you store it.
    pub changed: bool,
    /// An item was chosen. Close the menu.
    pub close: bool,
}

/// The column chooser: one checkable item per hideable column in display order, then "Resize
/// to fit" for `column` (when given and resizable) and "Reset columns" (§2.4).
///
/// Draw it inside a menu popup, usually opened from
/// [`super::DataGridResponse::header_context_menu`]. Hiding writes `state.layout.hidden`,
/// "Resize to fit" sets `state.autofit_request` for the next `show`, and "Reset columns" clears
/// the layout and the stored widths. [`Column::required`] columns get no item.
///
/// ```
/// # use fluent2_egui::data::data_grid::{Column, DataGridState, column_menu};
/// # fn demo(ui: &mut egui::Ui) {
/// let columns = [Column::new("name", "Name").required(true), Column::new("size", "Size")];
/// let mut state = DataGridState::default();
/// let menu = column_menu(ui, &columns, &mut state, Some("size"));
/// # let _ = menu;
/// # }
/// ```
pub fn column_menu(
    ui: &mut Ui,
    columns: &[Column],
    state: &mut DataGridState,
    column: Option<&str>,
) -> ColumnMenuResponse {
    let mut out = ColumnMenuResponse::default();
    let ordered = state.layout.ordered(columns);
    for item in ordered.into_iter().filter(|item| !item.required) {
        let shown = !state.layout.is_hidden(item.id);
        let response = ui.add(
            MenuItem::new(item.header.clone())
                .checked(shown)
                .check_column(true)
                .id_salt(("column-menu", item.id)),
        );
        if response.clicked() {
            out.changed |= state.layout.set_hidden(item.id, shown);
        }
    }
    divider(ui);
    let fit = column.and_then(|id| columns.iter().find(|item| item.id == id && item.resizable));
    if let Some(fit) = fit
        && ui
            .add(MenuItem::new("Resize to fit").check_column(true))
            .clicked()
    {
        state.autofit_request = Some(fit.id.to_owned());
        out.close = true;
    }
    if ui
        .add(MenuItem::new("Reset columns").check_column(true))
        .clicked()
    {
        state.layout.reset();
        state.widths.clear();
        state.autofit_request = None;
        out.changed = true;
        out.close = true;
    }
    out
}
