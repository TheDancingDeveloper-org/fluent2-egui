//! Fluent Toast and a toaster host.
//!
//! [`Toaster`] lives in `ctx.data`. The app calls [`Toaster::show`] once per frame;
//! it draws at most three toasts in a foreground [`egui::Area`], newest at the
//! bottom. Enter and exit slide and fade with the motion tokens. A timeout
//! (default 5 s) pauses while the toast is hovered.

use std::time::Duration;

use egui::epaint::RectShape;
use egui::{Area, Color32, CornerRadius, Id, Order, Rect, Sense, Stroke, StrokeKind, Ui, Vec2};

use fluent2_tokens::generated::global::{border_radius, stroke_width};

use crate::controls::{Appearance, Button, ButtonSize};
use crate::foundation::icon::{IconSize, IconStyle, paint_icon};
use crate::foundation::motion::{Curve, Duration as MotionDuration};
use crate::foundation::text::{TypeRamp, style};
use crate::surfaces::message_bar::{Intent, ensure_type_ramp};
use crate::theme::Theme;

/// Where the stack sits. [`BottomEnd`] is the Fluent default.
#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub enum ToastPosition {
    /// Bottom corner on the trailing edge.
    #[default]
    BottomEnd,
    /// Bottom corner on the leading edge.
    BottomStart,
    /// Top corner on the trailing edge.
    TopEnd,
    /// Top corner on the leading edge.
    TopStart,
}

/// One toast the toaster is holding.
#[derive(Clone, Debug)]
pub struct Toast {
    /// Semantic weight, shared with [`super::MessageBar`].
    pub intent: Intent,
    /// Short heading.
    pub title: String,
    /// Optional second line.
    pub body: Option<String>,
    /// How long the toast stays. `None` is 5 seconds. `Some` of zero never expires.
    pub timeout: Option<Duration>,
    /// Optional action label.
    pub action: Option<String>,
}

/// Identity of a toast returned by [`Toaster::push`].
#[derive(Clone, Copy, Debug, Eq, PartialEq, Hash)]
pub struct ToastId(u64);

#[derive(Clone)]
struct Entry {
    id: ToastId,
    toast: Toast,
    born: f64,
    paused_for: f64,
    pause_started: Option<f64>,
    exiting: bool,
    exit_at: f64,
}

#[derive(Clone)]
struct Host {
    next: u64,
    entries: Vec<Entry>,
    action_clicks: Vec<ToastId>,
}

impl Default for Host {
    fn default() -> Self {
        Self {
            next: 1,
            entries: Vec::new(),
            action_clicks: Vec::new(),
        }
    }
}

/// Host for toasts, stored on the egui context.
pub struct Toaster;

impl Toaster {
    /// Queue a toast and return its id.
    pub fn push(ctx: &egui::Context, toast: Toast) -> ToastId {
        // Read the clock before taking the data lock: `ctx.input` inside `data_mut` deadlocks.
        let now = ctx.input(|input| input.time);
        let mut id = ToastId(0);
        ctx.data_mut(|data| {
            let host = data.get_temp_mut_or_default::<Host>(host_id());
            id = ToastId(host.next);
            host.next = host.next.saturating_add(1);
            host.entries.push(Entry {
                id,
                toast,
                born: now,
                paused_for: 0.0,
                pause_started: None,
                exiting: false,
                exit_at: now,
            });
        });
        ctx.request_repaint();
        id
    }

    /// Start dismissing a toast. It finishes its exit animation, then leaves.
    pub fn dismiss(ctx: &egui::Context, id: ToastId) {
        let now = ctx.input(|input| input.time);
        ctx.data_mut(|data| {
            let host = data.get_temp_mut_or_default::<Host>(host_id());
            if let Some(entry) = host
                .entries
                .iter_mut()
                .find(|entry| entry.id == id && !entry.exiting)
            {
                entry.exiting = true;
                entry.exit_at = now;
            }
        });
        ctx.request_repaint();
    }

    /// Ids whose action was clicked since the last call.
    pub fn take_action_clicks(ctx: &egui::Context) -> Vec<ToastId> {
        ctx.data_mut(|data| {
            let host = data.get_temp_mut_or_default::<Host>(host_id());
            std::mem::take(&mut host.action_clicks)
        })
    }

    /// Draw the stack. Call once per frame.
    pub fn show(ctx: &egui::Context, position: ToastPosition) {
        let now = ctx.input(|input| input.time);
        // Reduced motion: no enter or exit run, so no slide and no fade.
        let motion = if crate::foundation::motion::reduced_motion(ctx) {
            0.0
        } else {
            MotionDuration::Normal.seconds() as f64
        };
        let mut visible: Vec<ToastId> = Vec::new();
        let mut next_wake: Option<f64> = None;
        ctx.data_mut(|data| {
            let host = data.get_temp_mut_or_default::<Host>(host_id());
            for entry in &mut host.entries {
                let timeout = timeout_secs(&entry.toast);
                if let Some(limit) = timeout.filter(|_| !entry.exiting) {
                    let paused = entry.paused_for
                        + entry
                            .pause_started
                            .map(|started| (now - started).max(0.0))
                            .unwrap_or(0.0);
                    if now - entry.born - paused >= limit {
                        entry.exiting = true;
                        entry.exit_at = now;
                    }
                }
            }
            host.entries
                .retain(|entry| !entry.exiting || now - entry.exit_at < motion);
            // Newest at the bottom: the three newest are the tail, oldest of those first.
            let start = host.entries.len().saturating_sub(MAX_VISIBLE);
            visible = host.entries[start..].iter().map(|entry| entry.id).collect();
            next_wake = next_wake_secs(&host.entries, now, motion);
        });

        // A reactive app only repaints on input, so ask for the frame that ends the next
        // animation or timeout. Without this, an idle window keeps an expired toast.
        if let Some(delay) = next_wake.and_then(|secs| Duration::try_from_secs_f64(secs).ok()) {
            ctx.request_repaint_after(delay);
        }

        if visible.is_empty() {
            return;
        }

        let anchor = match position {
            ToastPosition::BottomEnd => egui::Align2::RIGHT_BOTTOM,
            ToastPosition::BottomStart => egui::Align2::LEFT_BOTTOM,
            ToastPosition::TopEnd => egui::Align2::RIGHT_TOP,
            ToastPosition::TopStart => egui::Align2::LEFT_TOP,
        };
        Area::new(Id::new("fluent2_toaster"))
            .order(Order::Foreground)
            .anchor(anchor, egui::vec2(0.0, 0.0))
            .show(ctx, |ui| {
                ensure_type_ramp(ui);
                ui.set_width(320.0);
                ui.spacing_mut().item_spacing.y = 8.0;
                let theme = Theme::get(ctx);
                for id in visible {
                    paint_one(ui, &theme, id, now, motion);
                }
            });
    }
}

const MAX_VISIBLE: usize = 3;

fn paint_one(ui: &mut Ui, theme: &Theme, id: ToastId, now: f64, motion: f64) {
    let snapshot = ui.ctx().data(|data| {
        data.get_temp::<Host>(host_id()).and_then(|host| {
            host.entries
                .iter()
                .find(|entry| entry.id == id)
                .map(|entry| (entry.toast.clone(), entry.exiting, entry.exit_at, entry.born))
        })
    });
    let Some((toast, exiting, exit_at, born)) = snapshot else {
        return;
    };
    let elapsed = if exiting { now - exit_at } else { now - born };
    let progress = if motion > 0.0 {
        (elapsed / motion).clamp(0.0, 1.0) as f32
    } else {
        1.0
    };
    let eased = if exiting {
        1.0 - Curve::AccelerateMid.ease(progress)
    } else {
        Curve::DecelerateMid.ease(progress)
    };
    // Opacity goes through `motion::animate` so reduced motion snaps to the target.
    let opacity = crate::foundation::motion::animate(
        ui.ctx(),
        Id::new(("fluent2_toast_opacity", id.0)),
        if exiting { 0.0 } else { 1.0 },
        MotionDuration::Normal,
        if exiting {
            Curve::AccelerateMid
        } else {
            Curve::DecelerateMid
        },
    );
    let slide = (1.0 - eased) * 16.0;

    let palette = crate::surfaces::message_bar::palette_of(toast.intent, theme);
    let (allocated, response) = ui.allocate_exact_size(Vec2::new(320.0, 64.0), Sense::hover());
    // Enter slides up from below; exit slides back down.
    let rect = allocated.translate(Vec2::new(0.0, slide));
    let hovered = response.hovered();
    ui.ctx().data_mut(|data| {
        let host = data.get_temp_mut_or_default::<Host>(host_id());
        if let Some(entry) = host.entries.iter_mut().find(|entry| entry.id == id) {
            match (hovered, entry.pause_started) {
                (true, None) => entry.pause_started = Some(now),
                (false, Some(started)) => {
                    entry.paused_for += (now - started).max(0.0);
                    entry.pause_started = None;
                }
                _ => {}
            }
        }
    });

    // Same roles as the message bar: Alert for Warning and Error, Status otherwise.
    let role = match toast.intent {
        Intent::Warning | Intent::Error => egui::accesskit::Role::Alert,
        Intent::Info | Intent::Success => egui::accesskit::Role::Status,
    };
    let title = toast.title.clone();
    let bounds = egui::accesskit::Rect {
        x0: f64::from(allocated.min.x),
        y0: f64::from(allocated.min.y),
        x1: f64::from(allocated.max.x),
        y1: f64::from(allocated.max.y),
    };
    ui.ctx().accesskit_node_builder(response.id, |node| {
        node.set_role(role);
        node.set_label(title);
        node.set_bounds(bounds);
    });

    if !ui.is_rect_visible(rect) {
        return;
    }
    let painter = ui.painter();
    let radius = CornerRadius::same(rounded_u8(border_radius::MEDIUM));
    painter.add(RectShape::new(
        rect,
        radius,
        mul_alpha(palette.background, opacity),
        Stroke::new(stroke_width::THIN, mul_alpha(palette.stroke, opacity)),
        StrokeKind::Inside,
    ));
    let icon_rect = Rect::from_min_size(
        egui::pos2(rect.left() + 12.0, rect.top() + 12.0),
        Vec2::splat(20.0),
    );
    paint_icon(
        ui,
        icon_rect,
        palette.icon,
        IconSize::S20,
        IconStyle::Regular,
        Some(mul_alpha(palette.foreground, opacity)),
    );
    let title_style = style(TypeRamp::Body1Strong);
    let body_style = style(TypeRamp::Caption1);
    let title = egui::WidgetText::from(toast.title.as_str())
        .color(mul_alpha(palette.foreground, opacity))
        .into_galley(ui, Some(egui::TextWrapMode::Truncate), 200.0, title_style);
    painter.galley(
        egui::pos2(icon_rect.right() + 8.0, rect.top() + 10.0),
        title,
        mul_alpha(palette.foreground, opacity),
    );
    if let Some(body) = &toast.body {
        let galley = egui::WidgetText::from(body.as_str())
            .color(mul_alpha(palette.foreground, opacity))
            .into_galley(ui, Some(egui::TextWrapMode::Truncate), 200.0, body_style);
        painter.galley(
            egui::pos2(icon_rect.right() + 8.0, rect.top() + 32.0),
            galley,
            mul_alpha(palette.foreground, opacity),
        );
    }
    if let Some(action) = toast.action.clone() {
        let slot = Rect::from_min_size(
            egui::pos2(rect.right() - 12.0 - 72.0, rect.center().y - 16.0),
            Vec2::new(72.0, 32.0),
        );
        let clicked = ui
            .scope_builder(egui::UiBuilder::new().max_rect(slot), |ui| {
                ui.add_sized(
                    slot.size(),
                    Button::new(action)
                        .appearance(Appearance::Transparent)
                        .size(ButtonSize::Small),
                )
                .clicked()
            })
            .inner;
        if clicked {
            ui.ctx().data_mut(|data| {
                data.get_temp_mut_or_default::<Host>(host_id())
                    .action_clicks
                    .push(id);
            });
        }
    }
}

/// Seconds until some entry next needs a frame: an animation still running, or a timeout.
/// `None` when nothing is pending (all paused or never expiring).
fn next_wake_secs(entries: &[Entry], now: f64, motion: f64) -> Option<f64> {
    entries
        .iter()
        .filter_map(|entry| {
            let started = if entry.exiting { entry.exit_at } else { entry.born };
            let animating = now - started < motion;
            if animating {
                return Some(0.0);
            }
            if entry.exiting || entry.pause_started.is_some() {
                return None;
            }
            let limit = timeout_secs(&entry.toast)?;
            Some((entry.born + entry.paused_for + limit - now).max(0.0))
        })
        .filter(|secs| secs.is_finite())
        .reduce(f64::min)
}

fn timeout_secs(toast: &Toast) -> Option<f64> {
    match toast.timeout {
        Some(duration) if duration.is_zero() => None,
        Some(duration) => Some(duration.as_secs_f64()),
        None => Some(5.0),
    }
}

fn host_id() -> Id {
    Id::NULL.with("fluent2_toaster_host")
}

fn mul_alpha(color: Color32, opacity: f32) -> Color32 {
    let opacity = if opacity.is_finite() {
        opacity.clamp(0.0, 1.0)
    } else {
        1.0
    };
    Color32::from_rgba_unmultiplied(
        color.r(),
        color.g(),
        color.b(),
        (f32::from(color.a()) * opacity).round().clamp(0.0, 255.0) as u8,
    )
}

fn rounded_u8(value: f32) -> u8 {
    if !value.is_finite() {
        return 0;
    }
    value.clamp(0.0, 255.0).round() as u8
}
