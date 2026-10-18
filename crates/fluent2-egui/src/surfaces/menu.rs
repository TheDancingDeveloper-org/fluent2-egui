//! Fluent menu contents on top of egui's popup and menu machinery.
//!
//! Opening, closing, submenus and keyboard movement stay with egui (`egui::Popup`,
//! `Ui::menu_button`, `Response::context_menu`). This module supplies the Fluent frame and
//! the styled contents: items, checkmarks, shortcuts, submenus, dividers and group headers.
//!
//! Metrics follow the WI-1545 description (Fluent UI React menu, the revision pinned there).
//! Colours, radii and spacing come from the installed [`crate::Theme`].

use egui::epaint::RectShape;
use egui::{
    Align, CornerRadius, Frame, Id, InnerResponse, Layout, Popup, Rect, Response, Sense, Stroke,
    TextWrapMode, Ui, UiKind, UiStackInfo, Vec2, Widget, WidgetInfo, WidgetText, WidgetType,
};

use fluent2_icons::{Icon, Size as IconSize, Style as IconStyle};
use fluent2_tokens::generated::global::{border_radius, spacing_horizontal, spacing_vertical, stroke_width};

use crate::foundation::focus;
use crate::foundation::icon::paint_icon;
use crate::foundation::text::{self, TypeRamp};
use crate::theme::{Theme, ThemeKind, c};

/// Height of a menu item, in points.
const ITEM_HEIGHT: f32 = 32.0;
/// Horizontal padding inside an item.
const H_PADDING: f32 = spacing_horizontal::S;
/// Gap between the icon, the label and the shortcut.
const GAP: f32 = spacing_horizontal::S;
/// Width reserved for a leading icon.
const ICON_SLOT: f32 = 20.0;
/// Width reserved for a leading checkmark or a trailing chevron.
const CHECK_SLOT: f32 = 16.0;
/// Vertical margin above and below a divider.
const DIVIDER_MARGIN: f32 = spacing_vertical::XS;
/// Inner padding of the menu frame.
const FRAME_PADDING: f32 = spacing_vertical::XS;

/// Frame for a Fluent menu popup.
///
/// Background `neutral_background_1`, a 1px stroke that is transparent except in high contrast
/// (where it is `neutral_stroke_2`), radius `MEDIUM`, shadow 16, and 4px of inner padding.
pub fn menu_frame(ui: &Ui) -> Frame {
    let theme = Theme::get(ui.ctx());
    let stroke_color = if theme.kind == ThemeKind::HighContrast {
        c(theme.color.neutral_stroke_2)
    } else {
        egui::Color32::TRANSPARENT
    };
    Frame::new()
        .fill(c(theme.color.neutral_background_1))
        .stroke(Stroke::new(stroke_width::THIN, stroke_color))
        .corner_radius(medium_radius())
        .shadow(elevation_shadow(ui, &theme))
        .inner_margin(FRAME_PADDING)
}

/// A row in a Fluent menu.
///
/// ```
/// # use fluent2_egui::surfaces::MenuItem;
/// # use fluent2_icons::Icon;
/// # fn demo(ui: &mut egui::Ui) {
/// if ui.add(MenuItem::new("Save").icon(Icon::Save).shortcut("Ctrl+S")).clicked() {
///     // the app decides what Save does
/// }
/// # }
/// ```
pub struct MenuItem<'a> {
    text: WidgetText,
    icon: Option<Icon>,
    shortcut: Option<&'a str>,
    checked: Option<bool>,
    check_column: bool,
    disabled_reason: Option<&'a str>,
    id_salt: Option<Id>,
    /// Submenu triggers must not dismiss the parent menu.
    closes_menu: bool,
}

impl<'a> MenuItem<'a> {
    /// A menu row labelled `text`. The label is also the accessible name.
    pub fn new(text: impl Into<WidgetText>) -> Self {
        Self {
            text: text.into(),
            icon: None,
            shortcut: None,
            checked: None,
            check_column: false,
            disabled_reason: None,
            id_salt: None,
            closes_menu: true,
        }
    }

    /// Leading icon, drawn at 20 px.
    #[must_use]
    pub fn icon(mut self, icon: Icon) -> Self {
        self.icon = Some(icon);
        self
    }

    /// Right-aligned shortcut caption, for example `"Ctrl+S"`.
    #[must_use]
    pub fn shortcut(mut self, shortcut: &'a str) -> Self {
        self.shortcut = Some(shortcut);
        self
    }

    /// Check state. Combined with [`Self::check_column`], this reserves the check gutter.
    #[must_use]
    pub fn checked(mut self, checked: bool) -> Self {
        self.checked = Some(checked);
        self
    }

    /// Reserve a 16 px check column even when this particular item is not checked.
    ///
    /// The MVP does not scan sibling items. When any item in the list is checkable, the caller
    /// sets this on every item so the labels line up.
    #[must_use]
    pub fn check_column(mut self, reserve: bool) -> Self {
        self.check_column = reserve;
        self
    }

    /// Tooltip shown when the item is disabled (`ui.add_enabled(false, …)`).
    ///
    /// Does not disable the item by itself.
    #[must_use]
    pub fn disabled_reason(mut self, reason: &'a str) -> Self {
        self.disabled_reason = Some(reason);
        self
    }

    /// Extra salt for the widget id.
    #[must_use]
    pub fn id_salt(mut self, salt: impl std::hash::Hash + std::fmt::Debug) -> Self {
        self.id_salt = Some(Id::new(salt));
        self
    }
}

impl Widget for MenuItem<'_> {
    fn ui(self, ui: &mut Ui) -> Response {
        let theme = Theme::get(ui.ctx());
        let label = self.text.text().to_owned();
        let font = text::font_id(TypeRamp::Body1);
        let shortcut_font = text::font_id(TypeRamp::Caption1);
        let shortcut_color = c(theme.color.neutral_foreground_3);

        let check_w = if self.check_column || self.checked.is_some() {
            CHECK_SLOT + GAP
        } else {
            0.0
        };
        let icon_w = if self.icon.is_some() { ICON_SLOT + GAP } else { 0.0 };
        let shortcut_w = match self.shortcut {
            Some(shortcut) => {
                ui.fonts_mut(|fonts| {
                    fonts
                        .layout_no_wrap(shortcut.to_owned(), shortcut_font.clone(), shortcut_color)
                        .size()
                        .x
                }) + GAP
            }
            None => 0.0,
        };

        let available = ui.available_width().max(1.0);
        let text_width = (available - H_PADDING * 2.0 - check_w - icon_w - shortcut_w).max(1.0);
        let galley = self
            .text
            .clone()
            .into_galley(ui, Some(TextWrapMode::Truncate), text_width, font);
        let width = available;
        let (auto_id, rect) = ui.allocate_space(Vec2::new(width, ITEM_HEIGHT));
        let id = match self.id_salt {
            Some(salt) => ui.make_persistent_id(salt),
            None => auto_id,
        };
        let mut response = ui.interact(rect, id, Sense::click());

        if let Some(reason) = self.disabled_reason {
            // `on_hover_text` only shows on an enabled widget. This reason is for a disabled item.
            response = response.on_disabled_hover_text(reason);
        }
        if response.clicked() && self.closes_menu {
            ui.close();
        }

        paint_row_fill(ui, rect, &response);
        let fg = foreground(ui, &theme, &response);
        let mut x = rect.left() + H_PADDING;
        if check_w > 0.0 {
            if self.checked == Some(true) {
                let slot = centred_slot(rect, x, CHECK_SLOT);
                paint_icon(
                    ui,
                    slot,
                    Icon::Checkmark,
                    IconSize::S16,
                    IconStyle::Regular,
                    Some(fg),
                );
            }
            x += check_w;
        }
        if let Some(icon) = self.icon {
            let slot = centred_slot(rect, x, ICON_SLOT);
            paint_icon(ui, slot, icon, IconSize::S20, IconStyle::Regular, Some(fg));
            x += icon_w;
        }
        let text_pos = egui::pos2(x, rect.center().y - galley.size().y * 0.5);
        ui.painter().galley(text_pos, galley, fg);

        if let Some(shortcut) = self.shortcut {
            let shortcut_galley = ui
                .fonts_mut(|fonts| fonts.layout_no_wrap(shortcut.to_owned(), shortcut_font, shortcut_color));
            let pos = egui::pos2(
                rect.right() - H_PADDING - shortcut_galley.size().x,
                rect.center().y - shortcut_galley.size().y * 0.5,
            );
            ui.painter().galley(pos, shortcut_galley, shortcut_color);
        }

        if focus::focus_visible(ui, &response) {
            focus::paint_focus_ring(ui, rect, medium_radius());
        }

        let selected = self.checked == Some(true);
        let enabled = ui.is_enabled();
        let checkable = self.checked.is_some();
        response.widget_info(|| {
            let mut info = WidgetInfo::labeled(WidgetType::Button, enabled, &label);
            // Only a checkable item has a state. Always set, it made every item a "toggle
            // button, not pressed" in Orca (WI-1586).
            info.selected = checkable.then_some(selected);
            info
        });
        ui.ctx().accesskit_node_builder(response.id, |node| {
            if checkable {
                node.set_role(egui::accesskit::Role::MenuItemCheckBox);
            } else {
                node.set_role(egui::accesskit::Role::MenuItem);
            }
        });
        response
    }
}

/// A menu row that opens a submenu. Wraps [`Ui::menu_button`].
///
/// The trigger is a Fluent [`MenuItem`] (same height, padding and hover fill) with a trailing
/// ChevronRight, so the label is not a stock egui button. egui's [`egui::containers::menu::SubMenu`]
/// owns hover, keyboard and closing when this is called from inside a menu. From a menu bar it
/// opens through [`egui::Popup::menu`].
pub fn submenu<R>(
    ui: &mut Ui,
    text: impl Into<WidgetText>,
    icon: Option<Icon>,
    add: impl FnOnce(&mut Ui) -> R,
) -> InnerResponse<Option<R>> {
    let text = text.into();
    let in_menu = ui.stack().iter().any(|frame| frame.kind() == Some(UiKind::Menu));
    let mut item = MenuItem::new(text);
    item.closes_menu = false;
    if let Some(icon) = icon {
        item = item.icon(icon);
    }
    // Reserve the trailing column the chevron is painted into, so it does not cover the label.
    item = item.shortcut(" ");
    let response = ui.add(item);
    paint_chevron(ui, &response);

    let popup = if in_menu {
        egui::containers::menu::SubMenu::default().show(ui, &response, |ui| {
            ui.set_min_width(160.0);
            add(ui)
        })
    } else {
        Popup::menu(&response)
            .style(menu_contents_style)
            .frame(menu_frame(ui))
            .info(UiStackInfo::new(UiKind::Menu))
            .show(|ui| {
                ui.set_min_width(160.0);
                add(ui)
            })
    };
    let expanded = popup.is_some();
    ui.ctx().accesskit_node_builder(response.id, |node| {
        node.set_expanded(expanded);
    });
    InnerResponse::new(popup.map(|inner| inner.inner), response)
}

/// A 1 px `neutral_stroke_2` rule with 4 px of vertical margin.
pub fn divider(ui: &mut Ui) {
    let theme = Theme::get(ui.ctx());
    let width = ui.available_width();
    let height = DIVIDER_MARGIN + stroke_width::THIN + DIVIDER_MARGIN;
    let (rect, _) = ui.allocate_exact_size(Vec2::new(width, height), Sense::hover());
    ui.painter().hline(
        rect.x_range(),
        rect.center().y,
        Stroke::new(stroke_width::THIN, c(theme.color.neutral_stroke_2)),
    );
}

/// A non-interactive group caption: caption1, `neutral_foreground_3`.
pub fn group_header(ui: &mut Ui, text: impl Into<WidgetText>) {
    let theme = Theme::get(ui.ctx());
    // caption1 strong: caption1's size in the semibold family.
    let mut font = text::font_id(TypeRamp::Caption1);
    font.family = text::font_id(TypeRamp::Body1Strong).family;
    let color = c(theme.color.neutral_foreground_3);
    let galley = text
        .into()
        .into_galley(ui, Some(TextWrapMode::Truncate), ui.available_width(), font);
    let width = ui.available_width();
    let (rect, _) = ui.allocate_exact_size(Vec2::new(width, 24.0), Sense::hover());
    let pos = egui::pos2(rect.left() + H_PADDING, rect.center().y - galley.size().y * 0.5);
    ui.painter().galley(pos, galley, color);
}

/// Open a context menu on `response`, framed with [`menu_frame`].
pub fn context_menu(response: &Response, add: impl FnOnce(&mut Ui)) {
    Popup::context_menu(response)
        .frame(menu_frame_from_ctx(&response.ctx))
        .style(menu_contents_style)
        .show(|ui| {
            ui.with_layout(Layout::top_down_justified(Align::Min), add);
        });
}

fn menu_frame_from_ctx(ctx: &egui::Context) -> Frame {
    // `menu_frame` needs a `Ui` only to read the theme, which lives on the context.
    let theme = Theme::get(ctx);
    let stroke_color = if theme.kind == ThemeKind::HighContrast {
        c(theme.color.neutral_stroke_2)
    } else {
        egui::Color32::TRANSPARENT
    };
    Frame::new()
        .fill(c(theme.color.neutral_background_1))
        .stroke(Stroke::new(stroke_width::THIN, stroke_color))
        .corner_radius(medium_radius())
        .shadow(first_shadow(&theme))
        .inner_margin(FRAME_PADDING)
}

fn elevation_shadow(ui: &Ui, theme: &Theme) -> egui::Shadow {
    let _ = (ui, theme);
    first_shadow(theme)
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
                color: egui::Color32::from_rgba_unmultiplied(
                    layer.color.r,
                    layer.color.g,
                    layer.color.b,
                    layer.color.a,
                ),
            })
        })
        .unwrap_or(egui::Shadow::NONE)
}

fn menu_contents_style(style: &mut egui::Style) {
    egui::menu::menu_style(style);
    style.spacing.item_spacing = Vec2::ZERO;
    style.spacing.button_padding = Vec2::new(H_PADDING, 0.0);
    style.spacing.menu_margin = egui::Margin::same(FRAME_PADDING as i8);
}

fn medium_radius() -> CornerRadius {
    CornerRadius::same(border_radius::MEDIUM.round().clamp(0.0, 255.0) as u8)
}

/// The label colour for the row state [`paint_row_fill`] paints. High contrast fills a hovered
/// row with the highlight colour, so its text must switch to the matching foreground.
fn foreground(ui: &Ui, theme: &Theme, response: &Response) -> egui::Color32 {
    if !ui.is_enabled() {
        c(theme.color.neutral_foreground_disabled)
    } else if response.is_pointer_button_down_on() {
        c(theme.color.neutral_foreground_1_pressed)
    } else if response.hovered() || response.has_focus() {
        c(theme.color.neutral_foreground_1_hover)
    } else {
        c(theme.color.neutral_foreground_1)
    }
}

fn paint_row_fill(ui: &Ui, rect: Rect, response: &Response) {
    if !ui.is_enabled() {
        return;
    }
    let theme = Theme::get(ui.ctx());
    let fill = if response.is_pointer_button_down_on() {
        c(theme.color.neutral_background_1_pressed)
    } else if response.hovered() || response.has_focus() {
        c(theme.color.neutral_background_1_hover)
    } else {
        return;
    };
    ui.painter().add(RectShape::filled(rect, medium_radius(), fill));
}

fn centred_slot(row: Rect, x: f32, size: f32) -> Rect {
    Rect::from_min_size(egui::pos2(x, row.center().y - size * 0.5), Vec2::splat(size))
}

fn paint_chevron(ui: &Ui, response: &Response) {
    let theme = Theme::get(ui.ctx());
    let fg = foreground(ui, &theme, response);
    let slot = centred_slot(
        response.rect,
        response.rect.right() - H_PADDING - CHECK_SLOT,
        CHECK_SLOT,
    );
    paint_icon(
        ui,
        slot,
        Icon::ChevronRight,
        IconSize::S16,
        IconStyle::Regular,
        Some(fg),
    );
}
