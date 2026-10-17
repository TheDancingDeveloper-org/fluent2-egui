//! The DataGrid header row (§1.4): paint, sort (§2.1), resize (§2.2) and the measurements
//! autofit needs (§2.3).

use egui::{
    Align as EguiAlign, CursorIcon, Id, Label, Layout, Pos2, Rect, Sense, Stroke, Ui, UiBuilder, Vec2,
    WidgetInfo, WidgetType,
};

use super::column::{Column, ColumnWidth};
use super::padded;
use super::state::{DataGridState, SortDirection, SortSpec};
use crate::foundation::icon::{IconName, IconSize, IconStyle, paint_icon};
use crate::theme::{Theme, c};
use fluent2_tokens::generated::global::{spacing_horizontal, stroke_width};

/// Width of a sort arrow, in points. The arrow sits in a slot at the right of a sortable cell.
const SORT_ICON: f32 = 16.0;
/// Hit zone of a resize handle, centred on the column's right border (§2.2).
const RESIZE_HIT_WIDTH: f32 = 8.0;
/// Width offered to a sizing pass, wide enough that nothing truncates while it is measured.
const MEASURE_WIDTH: f32 = 100_000.0;

/// Where the header is drawn and what it lays out. Built once per frame by `DataGrid::show`.
pub(super) struct HeaderLayout<'a> {
    /// Id the grid derives its interactive ids from.
    pub id: Id,
    /// The strip at the top of the viewport.
    pub rect: Rect,
    pub columns: &'a [Column],
    /// Resolved width of each column this frame.
    pub widths: &'a [f32],
    /// Left edge of each column, relative to the grid's left edge.
    pub offsets: &'a [f32],
}

/// What the header did this frame, for `DataGrid::show` to fold into its response.
#[derive(Debug, Default)]
pub(super) struct HeaderEvents {
    /// The sort state changed (§2.1).
    pub sort_changed: bool,
    /// A column was dragged or autofitted this frame.
    pub resized: Option<&'static str>,
    /// Index of a column whose handle was double-clicked. The grid autofits it (§2.3).
    pub autofit: Option<usize>,
}

/// Paint the header and handle its clicks and drags.
///
/// Sort cells are interacted first and resize handles second, so a handle that overlaps the
/// next column's sort cell wins the click.
pub(super) fn show(
    ui: &mut Ui,
    layout: &HeaderLayout,
    state: &mut DataGridState,
    theme: &Theme,
) -> HeaderEvents {
    let mut events = HeaderEvents::default();
    let rect = layout.rect;
    ui.painter()
        .rect_filled(rect, 0.0, c(theme.color.neutral_background_1));
    let text = c(theme.color.neutral_foreground_1);
    let hover = c(theme.color.subtle_background_hover);
    let pressed = c(theme.color.subtle_background_pressed);

    for (index, column) in layout.columns.iter().enumerate() {
        let cell = cell_rect(rect, layout.offsets[index], layout.widths[index]);
        if column.sortable {
            let response = ui.interact(cell, layout.id.with(("sort", column.id)), Sense::click());
            if response.hovered() {
                let fill = if response.is_pointer_button_down_on() {
                    pressed
                } else {
                    hover
                };
                ui.painter().rect_filled(cell, 0.0, fill);
            }
            if response.clicked() {
                state.sort = next_sort(state.sort.as_ref(), column.id);
                events.sort_changed = true;
            }
            let label = column.header.text().to_owned();
            response.widget_info(|| WidgetInfo::labeled(WidgetType::Button, true, label.clone()));
        }
        let sort = state
            .sort
            .as_ref()
            .filter(|spec| spec.column == column.id)
            .map(|spec| spec.direction);
        paint_label(ui, cell, column, sort, text);
    }

    for (index, column) in layout.columns.iter().enumerate() {
        if !column.resizable {
            continue;
        }
        let edge = cell_rect(rect, layout.offsets[index], layout.widths[index]).max.x;
        let handle = Rect::from_min_max(
            Pos2::new(edge - RESIZE_HIT_WIDTH / 2.0, rect.min.y),
            Pos2::new(edge + RESIZE_HIT_WIDTH / 2.0, rect.max.y),
        );
        let response = ui.interact(
            handle,
            layout.id.with(("resize", column.id)),
            Sense::click_and_drag(),
        );
        if response.hovered() || response.dragged() {
            ui.ctx().set_cursor_icon(CursorIcon::ResizeHorizontal);
            let line = Stroke::new(stroke_width::THIN, c(theme.color.neutral_stroke_1));
            ui.painter().vline(edge, rect.y_range(), line);
        }
        if response.drag_started() {
            freeze_fractions_right_of(index, layout, state);
        }
        if response.dragged() {
            let width = layout.widths[index] + response.drag_delta().x;
            state
                .widths
                .insert(column.id.to_owned(), clamp(width, column.min, column.max));
            events.resized = Some(column.id);
        }
        if response.double_clicked() {
            events.autofit = Some(index);
        }
    }
    events
}

/// Next sort state for a click on `column` (§2.1): none, then ascending, then descending, then
/// none. Clicking a different column starts it at ascending.
#[must_use]
pub(super) fn next_sort(current: Option<&SortSpec>, column: &str) -> Option<SortSpec> {
    match current {
        Some(spec) if spec.column == column => match spec.direction {
            SortDirection::Ascending => Some(SortSpec {
                column: column.to_owned(),
                direction: SortDirection::Descending,
            }),
            SortDirection::Descending => None,
        },
        _ => Some(SortSpec {
            column: column.to_owned(),
            direction: SortDirection::Ascending,
        }),
    }
}

/// Width after autofit (§2.3): the wider of the header and the widest measured cell, plus
/// `spacingHorizontalS` on each side, clamped to `[min, max]`.
#[must_use]
pub(super) fn autofit_width(header: f32, widest_cell: f32, pad: f32, min: f32, max: f32) -> f32 {
    clamp(header.max(widest_cell) + 2.0 * pad, min, max)
}

/// Width of a column's header in points, including the sort slot for sortable columns.
pub(super) fn header_width(ui: &mut Ui, column: &Column, height: f32) -> f32 {
    let text = natural_width(ui, height, Layout::left_to_right(EguiAlign::Center), |child| {
        child.add(Label::new(column.header.clone()).truncate());
    });
    text + sort_slot(column)
}

/// Width of `add`'s content laid out without a width limit, measured in a sizing pass (§2.3).
/// Nothing is painted and the parent `Ui` gets no space.
pub(super) fn natural_width(ui: &mut Ui, height: f32, layout: Layout, add: impl FnOnce(&mut Ui)) -> f32 {
    let rect = Rect::from_min_size(Pos2::ZERO, Vec2::new(MEASURE_WIDTH, height.max(0.0)));
    let mut child = ui.new_child(
        UiBuilder::new()
            .max_rect(rect)
            .layout(layout)
            .sizing_pass()
            .invisible(),
    );
    add(&mut child);
    child.min_rect().width()
}

/// Space reserved at the right of a sortable header cell for its arrow.
fn sort_slot(column: &Column) -> f32 {
    if column.sortable {
        SORT_ICON + spacing_horizontal::S
    } else {
        0.0
    }
}

/// Draw the header label, and the sort arrow when this column is sorted.
fn paint_label(ui: &mut Ui, cell: Rect, column: &Column, sort: Option<SortDirection>, text: egui::Color32) {
    let slot = sort_slot(column);
    let content = padded(cell);
    let label_rect = Rect::from_min_max(
        content.min,
        Pos2::new((content.max.x - slot).max(content.min.x), content.max.y),
    );
    let mut child = ui.new_child(UiBuilder::new().max_rect(label_rect).layout(column.cell_layout()));
    child.set_clip_rect(cell);
    child.style_mut().visuals.override_text_color = Some(text);
    child.add(Label::new(column.header.clone()).truncate());

    if let Some(direction) = sort {
        let name = match direction {
            SortDirection::Ascending => IconName::ArrowSortUp,
            SortDirection::Descending => IconName::ArrowSortDown,
        };
        let icon = Rect::from_center_size(
            Pos2::new(content.max.x - SORT_ICON / 2.0, cell.center().y),
            Vec2::splat(SORT_ICON),
        );
        paint_icon(ui, icon, name, IconSize::S16, IconStyle::Regular, Some(text));
    }
}

/// Before a column is dragged wider, pin the fraction columns to its right at their current
/// widths. Otherwise they would shrink, and §2.2 says the grid scrolls instead.
fn freeze_fractions_right_of(index: usize, layout: &HeaderLayout, state: &mut DataGridState) {
    for (later, column) in layout.columns.iter().enumerate().skip(index + 1) {
        let is_fraction = matches!(column.width, ColumnWidth::Fraction(_));
        if is_fraction && !state.widths.contains_key(column.id) {
            state.widths.insert(column.id.to_owned(), layout.widths[later]);
        }
    }
}

/// The header cell of a column: from its left edge, `width` wide, the full header height.
fn cell_rect(rect: Rect, offset: f32, width: f32) -> Rect {
    Rect::from_min_size(
        Pos2::new(rect.min.x + offset, rect.min.y),
        Vec2::new(width, rect.height()),
    )
}

/// Clamp to `[min, max]`. `min` wins when `min > max`, and a non-finite width becomes `min`
/// (ARCHITECTURE rule 9).
fn clamp(width: f32, min: f32, max: f32) -> f32 {
    let min = if min.is_finite() { min.max(0.0) } else { 0.0 };
    let max = if max.is_nan() { f32::INFINITY } else { max.max(min) };
    if width.is_finite() {
        width.clamp(min, max)
    } else {
        min
    }
}

#[cfg(test)]
mod tests {
    use super::{SortDirection, SortSpec, autofit_width, clamp, next_sort};

    fn spec(column: &str, direction: SortDirection) -> Option<SortSpec> {
        Some(SortSpec {
            column: column.to_owned(),
            direction,
        })
    }

    #[test]
    fn a_column_cycles_none_asc_desc_none() {
        let first = next_sort(None, "name");
        assert_eq!(first, spec("name", SortDirection::Ascending));
        let second = next_sort(first.as_ref(), "name");
        assert_eq!(second, spec("name", SortDirection::Descending));
        let third = next_sort(second.as_ref(), "name");
        assert_eq!(third, None);
    }

    #[test]
    fn a_different_column_starts_ascending() {
        let current = spec("name", SortDirection::Descending);
        assert_eq!(
            next_sort(current.as_ref(), "size"),
            spec("size", SortDirection::Ascending)
        );
    }

    #[test]
    fn drag_clamps_to_min_and_max() {
        assert_eq!(clamp(10.0, 44.0, 200.0), 44.0);
        assert_eq!(clamp(300.0, 44.0, 200.0), 200.0);
        assert_eq!(clamp(120.0, 44.0, 200.0), 120.0);
    }

    #[test]
    fn min_wins_when_min_exceeds_max() {
        assert_eq!(clamp(80.0, 60.0, 30.0), 60.0);
    }

    #[test]
    fn non_finite_drag_becomes_min() {
        assert_eq!(clamp(f32::NAN, 44.0, 200.0), 44.0);
        assert_eq!(clamp(f32::INFINITY, 44.0, 200.0), 44.0);
    }

    #[test]
    fn autofit_uses_the_wider_of_header_and_cell_plus_padding() {
        // A 150 pt cell beats a 60 pt header: 150 + 2 × 8.
        assert_eq!(autofit_width(60.0, 150.0, 8.0, 44.0, f32::INFINITY), 166.0);
        // A 90 pt header beats a 40 pt cell.
        assert_eq!(autofit_width(90.0, 40.0, 8.0, 44.0, f32::INFINITY), 106.0);
    }

    #[test]
    fn autofit_is_clamped() {
        assert_eq!(autofit_width(10.0, 10.0, 8.0, 44.0, f32::INFINITY), 44.0);
        assert_eq!(autofit_width(60.0, 400.0, 8.0, 44.0, 200.0), 200.0);
    }
}
