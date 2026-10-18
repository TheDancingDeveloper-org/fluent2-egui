//! Fluent TitleBar for undecorated windows: app icon, slots, title, caption buttons and drag.
//!
//! The component returns a [`WindowAction`] and never sends a viewport command (spec D1). The
//! app maps the action with [`window_command`]. Spec: `docs/specs/office-chrome.md` §1.

use std::marker::PhantomData;

use egui::{
    Align, CornerRadius, Direction, Id, Layout, Rect, Response, Sense, Ui, UiBuilder, Vec2, ViewportCommand,
    WidgetInfo, WidgetText, WidgetType, accesskit, pos2, vec2,
};

use super::keytips::{self, Region};
use crate::controls::button;
use crate::foundation::focus::paint_focus_ring;
use crate::foundation::icon::{Icon, IconName, IconSize, IconStyle, paint_icon};
use crate::foundation::text::{TypeRamp, style};
use crate::theme::{Theme, c};
use fluent2_tokens::generated::global::spacing_horizontal;

/// The window action the user asked for this frame. The app maps it with [`window_command`].
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum WindowAction {
    /// Minimise the window.
    Minimize,
    /// Toggle between maximised and restored.
    ToggleMaximize,
    /// Begin a window drag (the user pressed on a drag region and moved).
    StartDrag,
    /// Close the window.
    Close,
}

/// A slot closure, held until the bar has been measured and laid out.
type Slot<'s> = Box<dyn FnOnce(&mut Ui) + 's>;

/// The slots a [`TitleBar`] fills in its `add` closure.
///
/// Left to right: app icon, the left slot, the centre slot (or the title), the right slot, then
/// the caption buttons.
pub struct TitleBarSlots<'s> {
    app_icon: Option<Icon>,
    left: Option<Slot<'s>>,
    center: Option<Slot<'s>>,
    right: Option<Slot<'s>>,
}

impl<'s> TitleBarSlots<'s> {
    /// Show `icon` at the far left, in the small icon cell. With no tint of its own it uses
    /// the title foreground.
    pub fn app_icon(&mut self, icon: Icon) {
        self.app_icon = Some(icon);
    }

    /// Content after the app icon, such as the Quick Access Toolbar.
    pub fn left(&mut self, add: impl FnOnce(&mut Ui) + 's) {
        self.left = Some(Box::new(add));
    }

    /// Content in the centre. It replaces the title text.
    pub fn center(&mut self, add: impl FnOnce(&mut Ui) + 's) {
        self.center = Some(Box::new(add));
    }

    /// Content at the right, before the caption buttons.
    pub fn right(&mut self, add: impl FnOnce(&mut Ui) + 's) {
        self.right = Some(Box::new(add));
    }
}

/// What [`TitleBar::show`] returns. `inner` is whatever the slot closure returned.
pub struct TitleBarResponse<R> {
    /// The value returned by the slot closure.
    pub inner: R,
    /// The action requested this frame, if any.
    pub action: Option<WindowAction>,
    /// Whether the drag region was double-clicked.
    pub double_clicked: bool,
    /// The region not covered by slots or caption buttons.
    pub drag_region: Response,
    /// The Minimize caption button.
    pub minimize: Response,
    /// The Maximize or Restore caption button.
    pub maximize: Response,
    /// The Close caption button.
    pub close: Response,
    /// Whether the window is inactive (`ViewportInfo::focused == Some(false)`).
    pub inactive: bool,
}

/// A Fluent title bar for an undecorated window.
///
/// ```
/// # use fluent2_egui::office::TitleBar;
/// # fn demo(ui: &mut egui::Ui) {
/// let response = TitleBar::new("main_title", "Report").maximized(false).focused(Some(true)).show(ui, |slots| {
///     slots.left(|ui| {
///         ui.label("Quick access");
///     });
/// });
/// let _ = response.action;
/// # }
/// ```
pub struct TitleBar<'a> {
    id: Id,
    title: WidgetText,
    maximized: bool,
    focused: Option<bool>,
    with_search: bool,
    marker: PhantomData<&'a ()>,
}

impl<'a> TitleBar<'a> {
    /// A title bar whose id is `id`. `title` is shown in the centre unless a centre slot is set.
    #[must_use]
    pub fn new(id: impl std::hash::Hash, title: impl Into<WidgetText>) -> Self {
        Self {
            id: hashed_id(id),
            title: title.into(),
            maximized: false,
            focused: None,
            with_search: false,
            marker: PhantomData,
        }
    }

    /// Whether the window is maximised. The maximise button shows *restore* when true.
    #[must_use]
    pub fn maximized(mut self, maximized: bool) -> Self {
        self.maximized = maximized;
        self
    }

    /// `ViewportInfo::focused`, passed in so the component stays window-agnostic.
    #[must_use]
    pub fn focused(mut self, focused: Option<bool>) -> Self {
        self.focused = focused;
        self
    }

    /// Adds 8 pt to the height (the extended height) for a search box slot.
    #[must_use]
    pub fn with_search(mut self, enabled: bool) -> Self {
        self.with_search = enabled;
        self
    }

    /// Lay out the bar and run `add` to fill its slots. `add` runs once per frame.
    pub fn show<'s, R>(
        self,
        ui: &mut Ui,
        add: impl FnOnce(&mut TitleBarSlots<'s>) -> R,
    ) -> TitleBarResponse<R> {
        let theme = Theme::get(ui.ctx());
        let t = &theme.color;
        let metrics = theme.chrome;
        button::ensure_type_ramp(ui);
        let inactive = self.focused == Some(false);

        let mut slots = TitleBarSlots {
            app_icon: None,
            left: None,
            center: None,
            right: None,
        };
        let inner = add(&mut slots);

        let height = if self.with_search {
            metrics.title_bar_height_extended
        } else {
            metrics.title_bar_height
        };
        let width = finite_pos(ui.available_width());
        let (rect, _) = ui.allocate_exact_size(vec2(width, height), Sense::hover());
        ui.painter()
            .rect_filled(rect, CornerRadius::ZERO, c(t.neutral_background_1));
        ui.ctx().accesskit_node_builder(self.id, |node| {
            node.set_role(accesskit::Role::Group);
            node.set_label("Title bar");
        });

        // The top resize zone belongs to the window edge (§2, D10), so drag starts below it.
        let drag_rect = Rect::from_min_max(pos2(rect.min.x, rect.min.y + metrics.resize_zone), rect.max);
        let drag_region = ui.interact(drag_rect, self.id.with("drag_region"), Sense::click_and_drag());

        // Caption buttons sit at the right, three cells of `caption_button_width` each.
        let caption_w = metrics.caption_button_width;
        let caption_left = (rect.max.x - caption_w * 3.0).max(rect.min.x);
        let caption_cell = |index: f32| {
            Rect::from_min_size(
                pos2(caption_left + index * caption_w, rect.min.y),
                vec2(caption_w, metrics.caption_button_height),
            )
        };

        // On macOS the traffic lights sit over the left edge of a full-size content view.
        let inset = if cfg!(target_os = "macos") {
            metrics.macos_traffic_light_inset
        } else {
            0.0
        };
        let pad = spacing_horizontal::S;
        let content = Rect::from_min_max(
            pos2(rect.min.x + inset + pad, rect.min.y),
            pos2((caption_left - pad).max(rect.min.x + inset + pad), rect.max.y),
        );

        let TitleBarSlots {
            app_icon,
            left,
            center,
            right,
        } = slots;

        let left_used = ui
            .scope_builder(
                UiBuilder::new()
                    .max_rect(content)
                    .layout(Layout::left_to_right(Align::Center)),
                |ui| {
                    if let Some(icon) = app_icon {
                        let (icon_rect, _) =
                            ui.allocate_exact_size(Vec2::splat(metrics.icon_small), Sense::hover());
                        paint_icon(
                            ui,
                            icon_rect,
                            icon.name,
                            icon.size,
                            icon.style,
                            Some(icon.tint.unwrap_or(c(t.neutral_foreground_1))),
                        );
                    }
                    if let Some(add) = left {
                        add(ui);
                    }
                    ui.min_rect()
                },
            )
            .inner;

        let right_used = ui
            .scope_builder(
                UiBuilder::new()
                    .max_rect(content)
                    .layout(Layout::right_to_left(Align::Center)),
                |ui| {
                    if let Some(add) = right {
                        add(ui);
                    }
                    ui.min_rect()
                },
            )
            .inner;

        let middle = Rect::from_min_max(
            pos2(left_used.max.x.max(content.min.x), content.min.y),
            pos2(
                right_used.min.x.min(content.max.x).max(left_used.max.x),
                content.max.y,
            ),
        );
        let centred = Layout::centered_and_justified(Direction::LeftToRight);
        if let Some(add) = center {
            ui.scope_builder(UiBuilder::new().max_rect(middle).layout(centred), |ui| add(ui));
        } else if !self.with_search {
            let fg = if inactive {
                t.neutral_foreground_3
            } else {
                t.neutral_foreground_1
            };
            ui.scope_builder(UiBuilder::new().max_rect(middle).layout(centred), |ui| {
                ui.scope(|ui| {
                    ui.style_mut().override_text_style = Some(style(TypeRamp::Body1Strong));
                    ui.visuals_mut().override_text_color = Some(c(fg));
                    ui.label(self.title);
                });
            });
        }

        let minimize = caption_button(
            ui,
            self.id.with("minimize"),
            caption_cell(0.0),
            Caption::Minimize,
            self.maximized,
            inactive,
        );
        let maximize = caption_button(
            ui,
            self.id.with("maximize"),
            caption_cell(1.0),
            Caption::Maximize,
            self.maximized,
            inactive,
        );
        let close = caption_button(
            ui,
            self.id.with("close"),
            caption_cell(2.0),
            Caption::Close,
            self.maximized,
            inactive,
        );

        // F6 region (§9.3). A QAT in the title bar marks first, so it keeps the first stop.
        keytips::mark_region(ui.ctx(), Region::TitleBar, rect, Some(minimize.id));

        let double_clicked = drag_region.double_clicked();
        let caption_action = if minimize.clicked() {
            Some(WindowAction::Minimize)
        } else if maximize.clicked() {
            Some(WindowAction::ToggleMaximize)
        } else if close.clicked() {
            Some(WindowAction::Close)
        } else {
            None
        };
        let action = caption_action.or(if double_clicked {
            Some(WindowAction::ToggleMaximize)
        } else if drag_region.drag_started() {
            Some(WindowAction::StartDrag)
        } else {
            None
        });

        TitleBarResponse {
            inner,
            action,
            double_clicked,
            drag_region,
            minimize,
            maximize,
            close,
            inactive,
        }
    }
}

/// Pure mapping from a request to the viewport command the app sends (D1).
#[must_use]
pub fn window_command(action: WindowAction, maximized: bool) -> ViewportCommand {
    match action {
        WindowAction::Minimize => ViewportCommand::Minimized(true),
        WindowAction::ToggleMaximize => ViewportCommand::Maximized(!maximized),
        WindowAction::StartDrag => ViewportCommand::StartDrag,
        WindowAction::Close => ViewportCommand::Close,
    }
}

/// Which caption button a cell draws.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Caption {
    Minimize,
    Maximize,
    Close,
}

/// Paint and interact with one caption button in `rect`.
fn caption_button(
    ui: &mut Ui,
    id: Id,
    rect: Rect,
    caption: Caption,
    maximized: bool,
    inactive: bool,
) -> Response {
    let theme = Theme::get(ui.ctx());
    let t = &theme.color;
    let enabled = ui.is_enabled();
    let response = ui.interact(rect, id, Sense::click());
    let hovered = response.hovered();
    let pressed = response.is_pointer_button_down_on();
    let base_fg = if inactive {
        t.neutral_foreground_3
    } else {
        t.neutral_foreground_1
    };

    let (fill, glyph_fg) = if caption == Caption::Close && hovered {
        let fill = if pressed {
            t.status_danger_background_3_pressed
        } else {
            t.status_danger_background_3
        };
        (Some(fill), t.neutral_foreground_on_brand)
    } else if hovered {
        let fill = if pressed {
            t.subtle_background_pressed
        } else {
            t.subtle_background_hover
        };
        (Some(fill), base_fg)
    } else {
        (None, base_fg)
    };

    if let Some(fill) = fill {
        ui.painter().rect_filled(rect, CornerRadius::ZERO, c(fill));
    }
    let (glyph, label) = match caption {
        Caption::Minimize => (IconName::Subtract, "Minimize"),
        Caption::Maximize if maximized => (IconName::SquareMultiple, "Restore"),
        Caption::Maximize => (IconName::Maximize, "Maximize"),
        Caption::Close => (IconName::Dismiss, "Close"),
    };
    let glyph_rect = Rect::from_center_size(rect.center(), Vec2::splat(theme.chrome.icon_small));
    paint_icon(
        ui,
        glyph_rect,
        glyph,
        IconSize::S16,
        IconStyle::Regular,
        Some(c(glyph_fg)),
    );
    if response.has_focus() {
        paint_focus_ring(ui, rect, CornerRadius::ZERO);
    }
    response.widget_info(|| WidgetInfo::labeled(WidgetType::Button, enabled, label));
    response
}

/// An egui id from a caller's `Hash` value. egui 0.36 wants `Debug` as well, which the spec's
/// `impl std::hash::Hash` does not promise, so the value is hashed first.
fn hashed_id(source: impl std::hash::Hash) -> Id {
    use std::hash::Hasher;
    let mut hasher = std::collections::hash_map::DefaultHasher::new();
    source.hash(&mut hasher);
    Id::new(hasher.finish())
}

fn finite_pos(value: f32) -> f32 {
    if value.is_finite() && value > 0.0 {
        value
    } else {
        0.0
    }
}
