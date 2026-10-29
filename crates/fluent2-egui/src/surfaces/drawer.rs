//! Fluent drawer: a surface that slides in from the start or end edge of the window.
//!
//! The overlay drawer is built on [`egui::Modal`], like [`super::Dialog`], and shares its focus
//! trap, Escape policy and focus return. The inline drawer is a side panel inside a `Ui` that
//! pushes the content beside it and traps nothing. The behaviour is specified in
//! `docs/specs/drawer.md`; the Fluent reference is the React v9 Drawer at
//! <https://github.com/microsoft/fluentui/tree/990e2c65adab04fccc1f9ace3ecae5487ba94348/packages/react-components/react-drawer>
//! (`@fluentui/react-drawer` 9.9.4).

use egui::{
    Align, Align2, Area, Context, CornerRadius, Frame, Id, Layout, Margin, Modal, Order, Panel, Sense,
    Stroke, Ui, UiKind, Vec2, WidgetText,
};

use fluent2_icons::Icon;
use fluent2_tokens::generated::global::{border_radius, spacing_horizontal, spacing_vertical, stroke_width};

use super::dialog::{escape_pressed, finish_close, rounded_u8, shadow64, track_open};
use super::{CloseReason, DialogKind, DialogResult};
use crate::controls::{Appearance, Button, ButtonSize};
use crate::foundation::motion::{self, Curve, Duration};
use crate::foundation::text::{self, TypeRamp};
use crate::theme::{Theme, c};

/// The accessible label of the header's close button, and the label in
/// [`CloseReason::Button`] when it is clicked.
pub const CLOSE_LABEL: &str = "Close";

/// Which edge the drawer opens from.
#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub enum DrawerPosition {
    /// The left edge in a left-to-right layout. The default.
    #[default]
    Start,
    /// The right edge.
    End,
}

/// Drawer width, from Fluent's `size` prop.
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub enum DrawerSize {
    /// 320 points. The default.
    #[default]
    Small,
    /// 592 points.
    Medium,
    /// 940 points.
    Large,
    /// The full window width.
    Full,
    /// A width in points. Non-finite or non-positive values use [`DrawerSize::Small`].
    Custom(f32),
}

impl DrawerSize {
    /// The width in points for a window `available` points wide. Never wider than the window.
    #[must_use]
    pub fn width(self, available: f32) -> f32 {
        let available = if available.is_finite() {
            available.max(0.0)
        } else {
            0.0
        };
        let wanted = match self {
            Self::Small => 320.0,
            Self::Medium => 592.0,
            Self::Large => 940.0,
            Self::Full => available,
            Self::Custom(width) if width.is_finite() && width > 0.0 => width,
            Self::Custom(_) => 320.0,
        };
        wanted.min(available)
    }
}

/// A Fluent drawer.
///
/// [`Drawer::show`] shows the overlay drawer over a dimmed window; [`Drawer::show_inline`] shows
/// it as a panel inside a `Ui`.
///
/// ```
/// # use fluent2_egui::surfaces::{Drawer, DrawerPosition, DrawerSize};
/// # fn demo(ctx: &egui::Context, open: &mut bool) {
/// Drawer::new("filters")
///     .title("Filters")
///     .position(DrawerPosition::End)
///     .size(DrawerSize::Small)
///     .show(ctx, open, |ui| {
///         ui.label("Only open items");
///     });
/// # }
/// ```
pub struct Drawer {
    id_salt: Id,
    title: WidgetText,
    position: DrawerPosition,
    size: DrawerSize,
    kind: DialogKind,
}

impl Drawer {
    /// A drawer whose id comes from `id_salt`, never from the title.
    pub fn new(id_salt: impl std::hash::Hash + std::fmt::Debug) -> Self {
        Self {
            id_salt: Id::unique(id_salt),
            title: WidgetText::default(),
            position: DrawerPosition::Start,
            size: DrawerSize::Small,
            kind: DialogKind::Modal,
        }
    }

    /// Title in the header. It is also the accessible name.
    #[must_use]
    pub fn title(mut self, title: impl Into<WidgetText>) -> Self {
        self.title = title.into();
        self
    }

    /// The edge it opens from. The default is [`DrawerPosition::Start`].
    #[must_use]
    pub fn position(mut self, position: DrawerPosition) -> Self {
        self.position = position;
        self
    }

    /// Width. The default is [`DrawerSize::Small`].
    #[must_use]
    pub fn size(mut self, size: DrawerSize) -> Self {
        self.size = size;
        self
    }

    /// How the overlay drawer reacts to the backdrop and Escape, as for [`super::Dialog`]. The
    /// default is [`DialogKind::Modal`]. The inline drawer ignores it.
    #[must_use]
    pub fn kind(mut self, kind: DialogKind) -> Self {
        self.kind = kind;
        self
    }

    /// Show the overlay drawer while `*open` is `true`.
    ///
    /// On the frame it closes, the body does not run and the result is [`DialogResult::Closed`].
    pub fn show<R>(self, ctx: &Context, open: &mut bool, body: impl FnOnce(&mut Ui) -> R) -> DialogResult<R> {
        let width = self.size.width(ctx.content_rect().width());
        let edge = match self.position {
            DrawerPosition::Start => Edge::Start,
            DrawerPosition::End => Edge::End,
        };
        show_overlay(
            ctx,
            open,
            Overlay {
                id: self.id_salt,
                title: self.title,
                kind: self.kind,
                edge,
                extent: width,
            },
            body,
        )
    }

    /// Show the inline drawer inside `ui` while `*open` is `true`.
    ///
    /// Call it before the content it sits beside, as with [`egui::Panel::show`]. It does
    /// not dim, block or trap: Tab moves between the drawer and the page, and Escape does not
    /// close it. The header's close button clears `*open`.
    pub fn show_inline<R>(
        self,
        ui: &mut Ui,
        open: &mut bool,
        body: impl FnOnce(&mut Ui) -> R,
    ) -> DialogResult<R> {
        let ctx = ui.ctx().clone();
        let id = self.id_salt;
        let seen_key = id.with("seen");
        if !*open {
            ctx.data_mut(|data| data.insert_temp(seen_key, false));
            return DialogResult::Closed(CloseReason::Programmatic);
        }
        let opening = !ctx.data(|data| data.get_temp::<bool>(seen_key).unwrap_or(false));
        ctx.data_mut(|data| data.insert_temp(seen_key, true));
        let theme = Theme::get(&ctx);
        let width = self.size.width(ui.available_width());
        let progress = slide_progress(&ctx, id, opening);
        let shown_width = (width * progress).max(0.0);
        let panel = match self.position {
            DrawerPosition::Start => Panel::left(id.with("panel")),
            DrawerPosition::End => Panel::right(id.with("panel")),
        };
        let frame = Frame::new()
            .fill(c(theme.color.neutral_background_1))
            .inner_margin(margin());
        let shown = panel
            .resizable(false)
            .show_separator_line(false)
            .exact_size(shown_width)
            .frame(frame)
            .show(ui, |ui| {
                // The divider on the side that faces the page.
                let rect = ui.max_rect().expand2(margin_extent());
                let x = match self.position {
                    DrawerPosition::Start => rect.right(),
                    DrawerPosition::End => rect.left(),
                };
                let stroke = Stroke::new(stroke_width::THIN, c(theme.color.neutral_stroke_2));
                ui.painter().vline(x, rect.y_range(), stroke);
                contents(ui, id, &self.title, false, body)
            });
        let (value, header_closed, title_text, title_id) = shown.inner;
        ctx.accesskit_node_builder(shown.response.id, |node| {
            node.set_role(egui::accesskit::Role::Complementary);
            node.set_label(title_text);
            node.push_labelled_by(title_id.accesskit_id());
        });
        if header_closed {
            *open = false;
            ctx.data_mut(|data| data.insert_temp(seen_key, false));
            return DialogResult::Closed(CloseReason::Button(CLOSE_LABEL.to_owned()));
        }
        DialogResult::Open(value)
    }
}

/// Which window edge an overlay surface is attached to.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(crate) enum Edge {
    Start,
    End,
    Bottom,
}

/// What [`show_overlay`] needs from a Drawer or a BottomSheet.
pub(crate) struct Overlay {
    pub id: Id,
    pub title: WidgetText,
    pub kind: DialogKind,
    pub edge: Edge,
    /// Width for a side drawer, height for a bottom sheet, in points.
    pub extent: f32,
}

/// Show a modal surface attached to a window edge. Shared by [`Drawer`] and
/// [`super::BottomSheet`].
pub(crate) fn show_overlay<R>(
    ctx: &Context,
    open: &mut bool,
    overlay: Overlay,
    body: impl FnOnce(&mut Ui) -> R,
) -> DialogResult<R> {
    let id = overlay.id;
    let Some(opening) = track_open(ctx, id, *open) else {
        return DialogResult::Closed(CloseReason::Programmatic);
    };
    let theme = Theme::get(ctx);
    let surface_id = id.with("surface");
    let content = ctx.content_rect();
    let extent = if overlay.extent.is_finite() {
        overlay.extent.max(0.0)
    } else {
        0.0
    };
    let progress = slide_progress(ctx, id, opening);
    let hidden = extent * (1.0 - progress);

    let (anchor, offset, size, radius) = match overlay.edge {
        Edge::Start => (
            Align2::LEFT_TOP,
            Vec2::new(-hidden, 0.0),
            Vec2::new(extent, content.height()),
            CornerRadius::ZERO,
        ),
        Edge::End => (
            Align2::RIGHT_TOP,
            Vec2::new(hidden, 0.0),
            Vec2::new(extent, content.height()),
            CornerRadius::ZERO,
        ),
        Edge::Bottom => {
            let top = rounded_u8(border_radius::X_LARGE);
            (
                Align2::CENTER_BOTTOM,
                Vec2::new(0.0, hidden),
                Vec2::new(content.width(), extent),
                CornerRadius {
                    nw: top,
                    ne: top,
                    sw: 0,
                    se: 0,
                },
            )
        }
    };
    let inner = (size - margin_extent() * 2.0).max(Vec2::ZERO);
    let frame = Frame::new()
        .fill(c(theme.color.neutral_background_1))
        // Invisible in light and dark; the window-text edge in high contrast.
        .stroke(Stroke::new(stroke_width::THIN, c(theme.color.transparent_stroke)))
        .corner_radius(radius)
        .inner_margin(margin())
        .shadow(shadow64(&theme));
    let area = Area::new(id)
        .kind(UiKind::Modal)
        .sense(Sense::hover())
        .order(Order::Foreground)
        .interactable(true)
        .constrain(false)
        .anchor(anchor, offset);

    let bottom = overlay.edge == Edge::Bottom;
    let shown = Modal::new(id)
        .backdrop_color(c(theme.color.background_overlay))
        .frame(frame)
        .area(area)
        .show(ctx, |ui| {
            ui.set_min_size(inner);
            ui.set_max_size(inner);
            let (value, header_closed, title_text, title_id) = contents(ui, id, &overlay.title, bottom, body);

            // A focusable surface, so Tab has a stop and opening has a target even when the
            // body has no widgets. Allocated last, it is the last stop.
            let surface = ui.interact(ui.min_rect(), surface_id, Sense::focusable_noninteractive());
            if opening {
                surface.request_focus();
            }
            let role = match overlay.kind {
                DialogKind::Alert => egui::accesskit::Role::AlertDialog,
                DialogKind::Modal | DialogKind::Dismissible => egui::accesskit::Role::Dialog,
            };
            ui.ctx().accesskit_node_builder(surface_id, |node| {
                node.set_role(role);
                node.set_modal();
                node.set_label(title_text);
                node.push_labelled_by(title_id.accesskit_id());
            });
            (value, header_closed)
        });

    let (value, header_closed) = shown.inner;
    let mut close = None;
    if header_closed {
        close = Some(CloseReason::Button(CLOSE_LABEL.to_owned()));
    }
    if close.is_none() && shown.backdrop_response.clicked() && overlay.kind == DialogKind::Modal {
        close = Some(CloseReason::Backdrop);
    }
    // A popup open before the body owns Escape, as in Dialog.
    if close.is_none()
        && shown.is_top_modal
        && !shown.any_popup_open
        && overlay.kind != DialogKind::Alert
        && escape_pressed(ctx)
    {
        close = Some(CloseReason::Escape);
    }
    if let Some(reason) = close {
        *open = false;
        finish_close(ctx, id);
        return DialogResult::Closed(reason);
    }
    DialogResult::Open(value)
}

/// Slide-in progress from 0 (hidden) to 1 (in place). `durationGentle` on
/// `curveDecelerateMid`, as Fluent's drawer enters. Reduced motion is always 1.
fn slide_progress(ctx: &Context, id: Id, opening: bool) -> f32 {
    let slide = id.with("slide");
    if opening {
        // Every opening starts a fresh run off-screen, not where the last one ended.
        motion::reset(ctx, slide, 0.0);
    }
    motion::animate(ctx, slide, 1.0, Duration::Gentle, Curve::DecelerateMid).clamp(0.0, 1.0)
}

fn margin() -> Margin {
    Margin::symmetric(
        rounded_i8(spacing_horizontal::XXL),
        rounded_i8(spacing_vertical::XXL),
    )
}

fn margin_extent() -> Vec2 {
    Vec2::new(spacing_horizontal::XXL, spacing_vertical::XXL)
}

fn rounded_i8(value: f32) -> i8 {
    if value.is_finite() {
        value.round().clamp(0.0, f32::from(i8::MAX)) as i8
    } else {
        0
    }
}

/// Header (title and close button), then the body in a scroll area. Returns the body's value,
/// whether the close button was clicked, the title text and the title's widget id.
fn contents<R>(
    ui: &mut Ui,
    id: Id,
    title: &WidgetText,
    handle: bool,
    body: impl FnOnce(&mut Ui) -> R,
) -> (R, bool, String, Id) {
    let theme = Theme::get(ui.ctx());
    crate::controls::ensure_type_ramp(ui);
    if handle {
        // The grab handle Fluent draws on a bottom sheet. It is decoration, not a control.
        let (rect, _) = ui.allocate_exact_size(
            Vec2::new(ui.available_width(), spacing_vertical::S),
            Sense::hover(),
        );
        let bar = egui::Rect::from_center_size(rect.center(), Vec2::new(32.0, 4.0));
        ui.painter()
            .rect_filled(bar, CornerRadius::same(2), c(theme.color.neutral_stroke_1));
    }
    let title_text = title.text().to_owned();
    let mut title_id = id.with("title");
    let mut closed = false;
    ui.horizontal(|ui| {
        title_id = ui
            .push_id(id.with("title"), |ui| {
                ui.label(text::rich(title_text.clone(), TypeRamp::Subtitle1))
            })
            .inner
            .id;
        ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
            closed = ui
                .add(
                    Button::icon_only(Icon::Dismiss, CLOSE_LABEL)
                        .appearance(Appearance::Subtle)
                        .size(ButtonSize::Medium)
                        .id_salt(id.with("close")),
                )
                .clicked();
        });
    });
    ui.add_space(spacing_vertical::M);
    let value = egui::ScrollArea::vertical()
        .id_salt(id.with("body"))
        .auto_shrink([false, true])
        .show(ui, body)
        .inner;
    (value, closed, title_text, title_id)
}
