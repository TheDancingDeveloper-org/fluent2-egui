//! Office status bar: a bottom strip with left and right slots and a zoom control (spec §8.1,
//! WI-1569).

use egui::{Align, Id, Layout, Sense, Ui, UiBuilder, WidgetInfo, WidgetType, accesskit, vec2};

use super::keytips::{self, Region};
use crate::controls::{Appearance, Button, ButtonSize, Slider};
use crate::foundation::text::{TypeRamp, font_id};
use crate::theme::{Theme, c};

/// Zoom range in percent (spec §8.1).
const ZOOM_MIN: f32 = 10.0;
const ZOOM_MAX: f32 = 400.0;
const ZOOM_RESET: f32 = 100.0;
/// Change per press of the − and + buttons, in percentage points.
const ZOOM_STEP: f32 = 10.0;

/// A caller's slot content, run once per frame.
type Slot<'a> = Box<dyn FnOnce(&mut Ui) + 'a>;

/// The status bar: 24 pt tall, `neutral_background_2`, `Caption1` text in
/// `neutral_foreground_2`.
///
/// ```
/// # use fluent2_egui::office::status_bar::StatusBar;
/// # fn demo(ui: &mut egui::Ui) {
/// let mut zoom = 100.0_f32;
/// let response = StatusBar::new("status")
///     .left(|ui| {
///         ui.label("Ready");
///     })
///     .zoom(&mut zoom)
///     .show(ui);
/// let _ = response.zoom_reset;
/// # }
/// ```
pub struct StatusBar<'a> {
    id: Id,
    left: Option<Slot<'a>>,
    right: Option<Slot<'a>>,
    zoom: Option<&'a mut f32>,
}

/// What a [`StatusBar`] reports for one frame.
#[derive(Debug)]
pub struct StatusBarResponse {
    /// The zoom slider's response, if a zoom was set.
    pub zoom: Option<egui::Response>,
    /// The zoom reset button was clicked.
    pub zoom_reset: bool,
}

impl<'a> StatusBar<'a> {
    /// An empty status bar. `id` keeps its child layouts apart from other bars.
    #[must_use]
    pub fn new(id: impl std::hash::Hash) -> Self {
        Self {
            id: super::chrome_id(id),
            left: None,
            right: None,
            zoom: None,
        }
    }

    /// Items at the left, laid out from the left edge.
    #[must_use]
    pub fn left(mut self, add: impl FnOnce(&mut Ui) + 'a) -> Self {
        self.left = Some(Box::new(add));
        self
    }

    /// Items at the right, placed before the zoom control.
    #[must_use]
    pub fn right(mut self, add: impl FnOnce(&mut Ui) + 'a) -> Self {
        self.right = Some(Box::new(add));
        self
    }

    /// Shows the zoom control bound to `percent`. The value is clamped to 10..=400 when drawn,
    /// and a NaN becomes 100.
    #[must_use]
    pub fn zoom(mut self, percent: &'a mut f32) -> Self {
        self.zoom = Some(percent);
        self
    }

    /// Draws the bar across the available width.
    pub fn show(self, ui: &mut Ui) -> StatusBarResponse {
        let theme = Theme::get(ui.ctx());
        let height = theme.chrome.status_bar_height;
        let (rect, bar) = ui.allocate_exact_size(vec2(ui.available_width(), height), Sense::hover());
        ui.painter()
            .rect_filled(rect, 0.0, c(theme.color.neutral_background_2));
        ui.ctx().accesskit_node_builder(bar.id, |node| {
            node.set_role(accesskit::Role::Group);
            node.set_label("Status bar");
        });

        let StatusBar {
            id,
            left,
            right,
            zoom,
        } = self;
        let text_color = c(theme.color.neutral_foreground_2);
        let caption = font_id(TypeRamp::Caption1);

        let mut left_ui = ui.new_child(
            UiBuilder::new()
                .id_salt(id.with("left"))
                .max_rect(rect)
                .layout(Layout::left_to_right(Align::Center)),
        );
        left_ui.visuals_mut().override_text_color = Some(text_color);
        left_ui.style_mut().override_font_id = Some(caption.clone());
        if let Some(add) = left {
            add(&mut left_ui);
        }

        // Right to left: the first widget added sits at the far right, so the zoom control is
        // added before the caller's right items.
        let mut right_ui = ui.new_child(
            UiBuilder::new()
                .id_salt(id.with("right"))
                .max_rect(rect)
                .layout(Layout::right_to_left(Align::Center)),
        );
        right_ui.visuals_mut().override_text_color = Some(text_color);
        right_ui.style_mut().override_font_id = Some(caption);

        let mut out = StatusBarResponse {
            zoom: None,
            zoom_reset: false,
        };
        if let Some(percent) = zoom {
            let value = clean_zoom(*percent);
            let mut slider_value = f64::from(value);
            let enabled = right_ui.is_enabled();
            // Right to left: the percentage label sits at the far right, then +, the slider and −.
            let label = format!("{value:.0}%");
            if right_ui
                .add(
                    Button::new(label)
                        .appearance(Appearance::Subtle)
                        .size(ButtonSize::Small),
                )
                .clicked()
            {
                out.zoom_reset = true;
            }
            let zoom_in = right_ui.add(
                Button::new("+")
                    .appearance(Appearance::Subtle)
                    .size(ButtonSize::Small),
            );
            zoom_in.widget_info(|| WidgetInfo::labeled(WidgetType::Button, enabled, "Zoom in"));
            let mut step = if zoom_in.clicked() { ZOOM_STEP } else { 0.0 };
            let slider = right_ui.add(Slider::new(&mut slider_value, 10.0..=400.0).step(Some(1.0)));
            slider.widget_info(|| WidgetInfo::slider(enabled, slider_value, "Zoom"));
            let zoom_out = right_ui.add(
                Button::new("\u{2212}")
                    .appearance(Appearance::Subtle)
                    .size(ButtonSize::Small),
            );
            zoom_out.widget_info(|| WidgetInfo::labeled(WidgetType::Button, enabled, "Zoom out"));
            if zoom_out.clicked() {
                step -= ZOOM_STEP;
            }
            *percent = if out.zoom_reset {
                ZOOM_RESET
            } else {
                clean_zoom(slider_value as f32 + step)
            };
            out.zoom = Some(slider);
        }
        if let Some(add) = right {
            right_ui.horizontal(|ui| add(ui));
        }

        // F6 region (§9.3): focus lands on the zoom slider. Without zoom the bar has no
        // focusable control of its own; the app can mark one with `keytips::mark_region`.
        keytips::mark_region(
            ui.ctx(),
            Region::StatusBar,
            rect,
            out.zoom.as_ref().map(|slider| slider.id),
        );

        out
    }
}

/// Clamps a zoom percentage to the drawable range. A NaN is replaced with 100 (rule 9).
fn clean_zoom(percent: f32) -> f32 {
    if percent.is_finite() {
        percent.clamp(ZOOM_MIN, ZOOM_MAX)
    } else {
        ZOOM_RESET
    }
}
