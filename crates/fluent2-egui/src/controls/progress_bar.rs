//! Fluent ProgressBar.
//!
//! The rail is `neutral_background_6`, the fill is `brand_background`, and the
//! radius is `borderRadiusCircular`. Medium is 2 px (`strokeWidthThick`);
//! Large is 4 px (`strokeWidthThickest`).
//!
//! `None` is indeterminate: the fill sweeps with [`foundation::motion::animate`]
//! over `durationUltraSlow` times four. A visible indeterminate bar requests a
//! repaint; a determinate bar does not. Reduced motion holds the sweep still.
//!
//! Non-finite progress is clamped to `0.0`. Values above `1.0` clamp to `1.0`.

use egui::{CornerRadius, Id, Response, Sense, Ui, Vec2, Widget, WidgetInfo, WidgetType};

use fluent2_tokens::generated::global::{border_radius, stroke_width};

use crate::foundation::motion::{self, Curve, Duration};
use crate::theme::{self, Theme};

/// Thickness of the bar.
#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub enum ProgressBarSize {
    /// 2 px. The default.
    #[default]
    Medium,
    /// 4 px.
    Large,
}

/// A determinate or indeterminate progress bar.
///
/// ```
/// # use fluent2_egui::controls::ProgressBar;
/// # fn demo(ui: &mut egui::Ui) {
/// let _ = ui.add(ProgressBar::new(Some(0.4)));
/// let _ = ui.add(ProgressBar::new(None));
/// # }
/// ```
pub struct ProgressBar {
    value: Option<f32>,
    size: ProgressBarSize,
}

impl ProgressBar {
    /// `Some(fraction)` from 0 to 1, or `None` for an indeterminate sweep.
    #[must_use]
    pub fn new(value: Option<f32>) -> Self {
        Self {
            value,
            size: ProgressBarSize::Medium,
        }
    }

    /// Thickness. Defaults to [`ProgressBarSize::Medium`].
    #[must_use]
    pub fn size(mut self, size: ProgressBarSize) -> Self {
        self.size = size;
        self
    }
}

impl Widget for ProgressBar {
    fn ui(self, ui: &mut Ui) -> Response {
        let theme = Theme::get(ui.ctx());
        let thickness = match self.size {
            ProgressBarSize::Medium => stroke_width::THICK,
            ProgressBarSize::Large => stroke_width::THICKEST,
        };
        let width = finite_nonneg(ui.available_width()).max(thickness);
        let (rect, response) = ui.allocate_exact_size(Vec2::new(width, thickness), Sense::hover());
        let fraction = self.value.map(clamp_unit);
        let indeterminate = fraction.is_none();
        let visible = ui.is_rect_visible(rect);
        if visible {
            let radius = CornerRadius::same(rounded_u8(border_radius::CIRCULAR.min(rect.height())));
            ui.painter()
                .rect_filled(rect, radius, theme::c(theme.color.neutral_background_6));
            let fill = if let Some(fraction) = fraction {
                egui::Rect::from_min_size(rect.min, egui::vec2(rect.width() * fraction, rect.height()))
            } else {
                sweep_rect(ui, response.id, rect)
            };
            if fill.width() > 0.0 {
                ui.painter()
                    .rect_filled(fill, radius, theme::c(theme.color.brand_background));
            }
        }
        if indeterminate && visible {
            ui.ctx().request_repaint();
        }
        let label = match fraction {
            Some(fraction) => format!("{:.0}%", fraction * 100.0),
            None => "Indeterminate".to_owned(),
        };
        response.widget_info(|| {
            let mut info = WidgetInfo::labeled(WidgetType::ProgressIndicator, ui.is_enabled(), label.clone());
            info.value = fraction.map(|fraction| (fraction * 100.0) as f64);
            info
        });
        response
    }
}

/// Sweep phase in `0..=1`, driven by [`motion::animate`].
///
/// `animate` returns its target immediately under reduced motion. The direction
/// is not flipped then, so the fill stays put. The round trip (out and back) is
/// two `durationUltraSlow` runs; the bar keeps requesting repaints, so a full
/// cycle is that pair repeated.
fn sweep_rect(ui: &Ui, id: Id, rail: egui::Rect) -> egui::Rect {
    let going = ui
        .ctx()
        .data(|data| data.get_temp::<bool>(id.with("sweep_dir")).unwrap_or(true));
    let target = if going { 1.0 } else { 0.0 };
    let phase = motion::animate(
        ui.ctx(),
        id.with("sweep"),
        target,
        Duration::UltraSlow,
        Curve::Linear,
    );
    // Reduced motion snaps to `target` on the first call. Flipping then would
    // thrash the target every frame, so only flip once the value has moved.
    let moved = (phase - if going { 0.0 } else { 1.0 }).abs() > 0.01;
    if moved && (phase - target).abs() < 0.01 {
        ui.ctx()
            .data_mut(|data| data.insert_temp(id.with("sweep_dir"), !going));
    }
    let span = rail.width() * 0.35;
    let x = rail.left() + (rail.width() - span) * phase;
    egui::Rect::from_min_size(egui::pos2(x, rail.top()), egui::vec2(span, rail.height()))
}

fn clamp_unit(value: f32) -> f32 {
    if !value.is_finite() {
        0.0
    } else {
        value.clamp(0.0, 1.0)
    }
}

fn finite_nonneg(value: f32) -> f32 {
    if value.is_finite() && value >= 0.0 {
        value
    } else {
        0.0
    }
}

fn rounded_u8(value: f32) -> u8 {
    if !value.is_finite() {
        return 0;
    }
    value.round().clamp(0.0, f32::from(u8::MAX)) as u8
}
