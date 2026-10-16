//! Fluent popover on top of [`egui::Popup`].
//!
//! Positioning (below the anchor, flipping above when there is no room) and click-outside /
//! Escape dismissal stay with egui. This module supplies the Fluent frame, an optional arrow,
//! and moving keyboard focus into the popover on open and back to the anchor on close.
//!
//! Metrics follow the WI-1551 description (Fluent UI React popover, the revision pinned there).

use egui::{
    Align2, Color32, CornerRadius, Frame, Id, InnerResponse, LayerId, Rect, Response, Sense, Shape, Stroke,
    Ui, Vec2,
};

use fluent2_tokens::generated::global::{border_radius, spacing_horizontal, stroke_width};

use crate::theme::{Theme, ThemeKind, c};

/// Inner padding of the popover frame, in points.
const FRAME_PADDING: f32 = spacing_horizontal::L;
/// Side of the rotated square used as an arrow, in points.
const ARROW: f32 = 8.0;
/// How far the arrow overlaps the frame so the join has no gap.
const ARROW_OVERLAP: f32 = 4.0;

/// A Fluent popover anchored to a widget.
///
/// ```
/// # use fluent2_egui::surfaces::Popover;
/// # fn demo(ui: &mut egui::Ui, anchor: &egui::Response, open: &mut bool) {
/// Popover::new("details").show(anchor, open, |ui| {
///     ui.label("More about this");
/// });
/// # }
/// ```
pub struct Popover {
    id_salt: Id,
    arrow: bool,
    label: Option<String>,
}

impl Popover {
    /// A popover whose id is derived from `id_salt` (never from a translated label).
    pub fn new(id_salt: impl std::hash::Hash + std::fmt::Debug) -> Self {
        Self {
            id_salt: Id::new(id_salt),
            arrow: false,
            label: None,
        }
    }

    /// Draw an 8 px rotated square between the anchor and the popover.
    #[must_use]
    pub fn with_arrow(mut self) -> Self {
        self.arrow = true;
        self
    }

    /// Accessible name of the non-modal dialog container.
    #[must_use]
    pub fn label(mut self, label: impl Into<String>) -> Self {
        self.label = Some(label.into());
        self
    }

    /// Show the popover while `open` is true.
    ///
    /// Escape and a click outside set `*open = false`. Focus moves to the popover container
    /// when it opens and back to `anchor` when it closes. Returns `None` while closed.
    pub fn show<R>(self, anchor: &Response, open: &mut bool, add: impl FnOnce(&mut Ui) -> R) -> Option<R> {
        let ctx = anchor.ctx.clone();
        let popup_id = anchor.id.with(self.id_salt).with("popover");
        let was_open = ctx.data(|data| data.get_temp::<bool>(open_key(popup_id)).unwrap_or(false));
        if !*open {
            if was_open {
                ctx.data_mut(|data| data.insert_temp(open_key(popup_id), false));
                anchor.request_focus();
            }
            return None;
        }

        let theme = Theme::get(&ctx);
        let frame = popover_frame(&theme);
        let arrow = self.arrow;
        let label = self.label.clone();
        let anchor_rect = anchor.rect;
        let anchor_id = anchor.id;

        let shown = egui::Popup::from_response(anchor)
            .id(popup_id)
            .frame(frame)
            .gap(if arrow { ARROW - ARROW_OVERLAP } else { 0.0 })
            .open_bool(open)
            .close_behavior(egui::PopupCloseBehavior::CloseOnClickOutside)
            .show(|ui| {
                // A real widget so AccessKit has a rect for the non-modal dialog. It does not
                // take clicks; the caller's contents do.
                let node_id = ui.id().with("dialog");
                let (rect, response) = ui.allocate_at_least(Vec2::ZERO, Sense::hover());
                let accessible = label.clone().unwrap_or_default();
                ui.ctx().accesskit_node_builder(response.id, |node| {
                    node.set_role(egui::accesskit::Role::Dialog);
                    if !accessible.is_empty() {
                        node.set_label(accessible);
                    }
                });
                let _ = (rect, node_id);
                let result = add(ui);
                if !was_open {
                    ui.memory_mut(|memory| memory.request_focus(response.id));
                }
                result
            });

        let now_open = shown.is_some();
        ctx.data_mut(|data| data.insert_temp(open_key(popup_id), now_open));
        if was_open && !now_open {
            ctx.memory_mut(|memory| memory.request_focus(anchor_id));
        }

        if let Some(inner) = &shown
            && arrow
        {
            paint_arrow(
                &ctx,
                inner.response.layer_id,
                anchor_rect,
                inner.response.rect,
                &theme,
            );
        }

        shown.map(|inner| inner.inner)
    }
}

fn open_key(popup_id: Id) -> Id {
    popup_id.with("was_open")
}

fn popover_frame(theme: &Theme) -> Frame {
    let stroke_color = if theme.kind == ThemeKind::HighContrast {
        c(theme.color.neutral_stroke_2)
    } else {
        Color32::TRANSPARENT
    };
    Frame::new()
        .fill(c(theme.color.neutral_background_1))
        .stroke(Stroke::new(stroke_width::THIN, stroke_color))
        .corner_radius(medium_radius())
        .shadow(first_shadow(theme))
        .inner_margin(FRAME_PADDING)
}

fn paint_arrow(ctx: &egui::Context, layer: LayerId, anchor: Rect, popup: Rect, theme: &Theme) {
    let below = popup.top() >= anchor.center().y;
    let x = anchor
        .center()
        .x
        .clamp(popup.left() + ARROW, popup.right() - ARROW);
    let y = if below { popup.top() } else { popup.bottom() };
    let half = ARROW * 0.5;
    // A square rotated 45 degrees is a diamond. Its centre sits on the frame edge so half of
    // it overlaps the fill and half points at the anchor.
    let centre = egui::pos2(x, y);
    let points = vec![
        centre + Vec2::new(0.0, -half),
        centre + Vec2::new(half, 0.0),
        centre + Vec2::new(0.0, half),
        centre + Vec2::new(-half, 0.0),
    ];
    let shape = Shape::convex_polygon(points, c(theme.color.neutral_background_1), Stroke::NONE);
    ctx.layer_painter(layer).add(shape);
    let _ = Align2::CENTER_CENTER;
}

fn medium_radius() -> CornerRadius {
    CornerRadius::same(border_radius::MEDIUM.round().clamp(0.0, 255.0) as u8)
}

fn first_shadow(theme: &Theme) -> egui::Shadow {
    theme
        .shadow
        .shadow_16
        .iter()
        .find_map(|layer| {
            if !layer.x.is_finite()
                || !layer.y.is_finite()
                || !layer.blur.is_finite()
                || !layer.spread.is_finite()
            {
                return None;
            }
            Some(egui::Shadow {
                offset: [
                    layer.x.round().clamp(f32::from(i8::MIN), f32::from(i8::MAX)) as i8,
                    layer.y.round().clamp(f32::from(i8::MIN), f32::from(i8::MAX)) as i8,
                ],
                blur: layer.blur.round().clamp(0.0, f32::from(u8::MAX)) as u8,
                spread: layer.spread.round().clamp(0.0, f32::from(u8::MAX)) as u8,
                color: Color32::from_rgba_unmultiplied(
                    layer.color.r,
                    layer.color.g,
                    layer.color.b,
                    layer.color.a,
                ),
            })
        })
        .unwrap_or(egui::Shadow::NONE)
}

// Keep InnerResponse name available for docs readers via the egui re-export path.
#[allow(dead_code)]
fn _inner_response_name() -> Option<InnerResponse<()>> {
    None
}
