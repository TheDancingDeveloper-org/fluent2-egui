//! Row drag: drag DataGrid rows onto a drop target outside the grid, such as a sidebar
//! category, with a keyboard alternative.
//!
//! [`DataGrid::drag_source`](crate::data::DataGrid::drag_source) turns a grid into a source.
//! Past egui's drag threshold, a drag on a row carries a [`RowDrag`]: the selected rows when
//! the dragged row is selected, else that row alone (which becomes the selection). A ghost
//! follows the pointer ("3 rows", or the grid's
//! [`drag_label`](crate::data::DataGrid::drag_label)).
//!
//! [`DropTarget`] wraps the [`egui::Response`] of anything the app has drawn (a label, a
//! navigation item, a card). While a row drag hovers it, an accepting target gets a brand
//! highlight and a refusing one shows the not-allowed cursor. Releasing over an accepting
//! target reports a [`RowDrop`]; Escape cancels the drag, and so does a release anywhere else.
//!
//! Keyboard: drag and drop has no keyboard form, so the grid's context menu (secondary click or
//! Shift+F10) offers the same targets through [`drop_menu`], which reports the chosen target.
//!
//! Like [`Reorder`](crate::data::reorder), nothing here changes app data: the app reads the
//! row ids and the target and applies the change.

use egui::{
    Align2, CornerRadius, CursorIcon, Id, LayerId, Order, Response, Stroke, StrokeKind, Ui, WidgetText,
};

use crate::data::data_grid::RowId;
use crate::data::reorder::MoveSource;
use crate::foundation::text::{TypeRamp, font_id};
use crate::surfaces::{MenuItem, submenu};
use crate::theme::{Theme, c};
use fluent2_tokens::generated::global::{border_radius, stroke_width};

/// The payload of a row drag, stored in egui's `DragAndDrop` while the drag lasts.
///
/// ```
/// use fluent2_egui::data::RowId;
/// use fluent2_egui::data::row_drag::RowDrag;
/// let drag = RowDrag { source: egui::Id::unique("grid"), rows: vec![RowId(4), RowId(9)] };
/// assert_eq!(drag.rows.len(), 2);
/// ```
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct RowDrag {
    /// The grid the rows came from (its `id_salt`, hashed).
    pub source: Id,
    /// The dragged rows, in ascending [`RowId`] order.
    pub rows: Vec<RowId>,
}

impl RowDrag {
    /// The row drag in progress, if any.
    #[must_use]
    pub fn current(ctx: &egui::Context) -> Option<Self> {
        egui::DragAndDrop::payload::<Self>(ctx).map(|payload| (*payload).clone())
    }
}

/// Rows dropped on a target, by pointer or through [`drop_menu`].
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct RowDrop {
    /// The grid the rows came from.
    pub source: Id,
    /// The dropped rows, in ascending [`RowId`] order.
    pub rows: Vec<RowId>,
    /// Pointer drop or keyboard (menu) choice.
    pub how: MoveSource,
}

/// What [`DropTarget::show`] reports.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct DropTargetResponse {
    /// A row drag is over the target this frame.
    pub hovered: bool,
    /// The rows were released over the target this frame.
    pub dropped: Option<RowDrop>,
}

/// Turns any [`Response`] into a target for a [`RowDrag`]. Call [`DropTarget::show`] every
/// frame, after the widget is drawn.
///
/// ```
/// # use fluent2_egui::data::row_drag::DropTarget;
/// # fn demo(ui: &mut egui::Ui) {
/// let label = ui.label("Movies");
/// if let Some(drop) = DropTarget::new(&label).show(ui).dropped {
///     // assign drop.rows to the Movies category
///     let _ = drop.rows;
/// }
/// # }
/// ```
pub struct DropTarget<'a> {
    response: &'a Response,
    accepts: bool,
    source: Option<Id>,
}

impl<'a> DropTarget<'a> {
    /// A target over `response`'s rect.
    #[must_use]
    pub fn new(response: &'a Response) -> Self {
        Self {
            response,
            accepts: true,
            source: None,
        }
    }

    /// Whether the target takes the drop. A refusing target shows the not-allowed cursor while
    /// a drag hovers it and never reports a drop. Default `true`. A disabled `Ui` refuses too.
    #[must_use]
    pub fn accepts(mut self, accepts: bool) -> Self {
        self.accepts = accepts;
        self
    }

    /// Take drags only from the grid with this `id_salt`. Default: any grid.
    #[must_use]
    pub fn source(mut self, id_salt: impl std::hash::Hash) -> Self {
        self.source = Some(crate::data::data_grid::salt_id(id_salt));
        self
    }

    /// Paint the hover state and report a drop.
    pub fn show(self, ui: &Ui) -> DropTargetResponse {
        let Some(drag) = RowDrag::current(ui.ctx()) else {
            return DropTargetResponse::default();
        };
        if self.source.is_some_and(|source| source != drag.source) {
            return DropTargetResponse::default();
        }
        let rect = self.response.rect;
        let over = ui
            .ctx()
            .pointer_interact_pos()
            .is_some_and(|pointer| rect.contains(pointer) && self.response.interact_rect.contains(pointer));
        if !over {
            return DropTargetResponse::default();
        }
        let accepts = self.accepts && ui.is_enabled();
        if !accepts {
            ui.ctx().set_cursor_icon(CursorIcon::NotAllowed);
            return DropTargetResponse {
                hovered: true,
                dropped: None,
            };
        }
        ui.ctx().set_cursor_icon(CursorIcon::Grabbing);
        paint_highlight(ui, rect);
        let dropped = ui.input(|input| input.pointer.any_released()).then(|| {
            egui::DragAndDrop::clear_payload(ui.ctx());
            RowDrop {
                source: drag.source,
                rows: drag.rows,
                how: MoveSource::Pointer,
            }
        });
        DropTargetResponse {
            hovered: true,
            dropped,
        }
    }
}

/// The brand fill and stroke of an accepting target under a drag.
fn paint_highlight(ui: &Ui, rect: egui::Rect) {
    let theme = Theme::get(ui.ctx());
    let radius = CornerRadius::same(rounded_u8(border_radius::MEDIUM));
    let painter = ui.ctx().layer_painter(LayerId::new(
        Order::Foreground,
        ui.scope_id().with("row_drop_target"),
    ));
    painter.rect(
        rect,
        radius,
        c(theme.color.brand_background_2),
        Stroke::new(stroke_width::THICK, c(theme.color.compound_brand_stroke)),
        StrokeKind::Inside,
    );
}

/// The ghost that follows the pointer during a row drag.
pub(crate) fn paint_ghost(ui: &Ui, text: &str) {
    let Some(pointer) = ui.ctx().pointer_interact_pos() else {
        return;
    };
    let theme = Theme::get(ui.ctx());
    let painter = ui
        .ctx()
        .layer_painter(LayerId::new(Order::Tooltip, Id::unique("fluent2_row_drag_ghost")));
    let galley = painter.layout_no_wrap(
        text.to_owned(),
        font_id(TypeRamp::Body1),
        c(theme.color.neutral_foreground_on_brand),
    );
    let pad = egui::vec2(8.0, 4.0);
    let rect = Align2::LEFT_TOP.anchor_size(pointer + egui::vec2(12.0, 12.0), galley.size() + pad * 2.0);
    painter.rect_filled(
        rect,
        CornerRadius::same(rounded_u8(border_radius::MEDIUM)),
        c(theme.color.brand_background),
    );
    painter.galley(rect.min + pad, galley, c(theme.color.neutral_foreground_on_brand));
}

/// The default ghost text: "1 row", "3 rows".
#[must_use]
pub fn default_drag_label(count: usize) -> String {
    if count == 1 {
        "1 row".to_owned()
    } else {
        format!("{count} rows")
    }
}

/// One entry of [`drop_menu`]: a target's label and whether it takes the rows.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct DropChoice<'a> {
    /// Menu text, usually the target's own label.
    pub label: &'a str,
    /// A refusing target is shown disabled.
    pub accepts: bool,
}

/// The keyboard alternative to dragging: a submenu titled `title` (for example "Move to")
/// listing `choices`, for the grid's context menu. Returns the index of the chosen entry.
///
/// ```
/// # use fluent2_egui::data::row_drag::{DropChoice, drop_menu};
/// # fn demo(ui: &mut egui::Ui) {
/// let choices = [
///     DropChoice { label: "Movies", accepts: true },
///     DropChoice { label: "Archive", accepts: false },
/// ];
/// if let Some(index) = drop_menu(ui, "Move to", &choices) {
///     let _ = choices[index].label;
/// }
/// # }
/// ```
pub fn drop_menu(ui: &mut Ui, title: impl Into<WidgetText>, choices: &[DropChoice<'_>]) -> Option<usize> {
    submenu(ui, title, None, |ui| drop_choices(ui, choices))
        .inner
        .flatten()
}

/// The entries of [`drop_menu`] without the submenu, for a menu that lists targets directly.
pub fn drop_choices(ui: &mut Ui, choices: &[DropChoice<'_>]) -> Option<usize> {
    let mut chosen = None;
    for (index, choice) in choices.iter().enumerate() {
        let item = MenuItem::new(choice.label).id_salt(("fluent2_drop_choice", index));
        if ui.add_enabled(choice.accepts, item).clicked() {
            chosen = Some(index);
        }
    }
    chosen
}

/// A token radius as egui's `u8`, clamped.
fn rounded_u8(value: f32) -> u8 {
    value.round().clamp(0.0, f32::from(u8::MAX)) as u8
}

#[cfg(test)]
mod tests {
    use super::default_drag_label;

    #[test]
    fn ghost_label_counts_rows() {
        assert_eq!(default_drag_label(1), "1 row");
        assert_eq!(default_drag_label(3), "3 rows");
    }
}
