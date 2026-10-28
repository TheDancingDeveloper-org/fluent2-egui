//! SplitView page: two panes and a draggable divider.

use fluent2_egui::data::split_view::{SplitAxis, SplitState, SplitView};

use super::Page;

/// Gallery page for SplitView.
#[derive(Default)]
pub struct SplitViewPage {
    horizontal: SplitState,
    vertical: SplitState,
}

impl Page for SplitViewPage {
    fn name(&self) -> &'static str {
        "Split view"
    }

    fn ui(&mut self, ui: &mut egui::Ui) {
        ui.heading("Split view");
        ui.label(
            "Drag a divider to resize the panes, or focus it and use the arrow keys. Shift moves \
             a larger step. Home and End go to the minimum and maximum, and a double-click \
             restores the size from the first frame.",
        );
        ui.add_space(8.0);
        ui.strong("Left and right");
        ui.set_min_height(180.0);
        SplitView::new("gallery_split_horizontal", SplitAxis::Horizontal)
            .min_sizes(80.0, 80.0)
            .label("Resize left and right")
            .show(
                ui,
                &mut self.horizontal,
                |ui| {
                    ui.label("Left pane");
                },
                |ui| {
                    ui.label("Right pane");
                },
            );
        ui.add_space(12.0);
        ui.strong("Top and bottom");
        ui.set_min_height(180.0);
        SplitView::new("gallery_split_vertical", SplitAxis::Vertical)
            .min_sizes(48.0, 48.0)
            .label("Resize top and bottom")
            .show(
                ui,
                &mut self.vertical,
                |ui| {
                    ui.label("Top pane");
                },
                |ui| {
                    ui.label("Bottom pane");
                },
            );
    }
}
