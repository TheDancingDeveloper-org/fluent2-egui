//! The DataGrid header row (§1.4). Sorting and resizing arrive with §2.

use egui::{Label, Pos2, Rect, Ui, UiBuilder, Vec2};

use super::column::Column;
use super::padded;
use crate::theme::{Theme, c};

/// Paint the header over `rect`, the top strip of the viewport.
///
/// The fill is `neutral_background_1` so rows scrolling under the header do not show through.
pub(super) fn paint(
    ui: &mut Ui,
    rect: Rect,
    columns: &[Column],
    widths: &[f32],
    offsets: &[f32],
    theme: &Theme,
) {
    ui.painter()
        .rect_filled(rect, 0.0, c(theme.color.neutral_background_1));
    let text = c(theme.color.neutral_foreground_1);
    for (index, column) in columns.iter().enumerate() {
        let cell = Rect::from_min_size(
            Pos2::new(rect.min.x + offsets[index], rect.min.y),
            Vec2::new(widths[index], rect.height()),
        );
        let mut child = ui.new_child(
            UiBuilder::new()
                .max_rect(padded(cell))
                .layout(column.cell_layout()),
        );
        child.set_clip_rect(cell);
        child.style_mut().visuals.override_text_color = Some(text);
        child.add(Label::new(column.header.clone()).truncate());
    }
}
