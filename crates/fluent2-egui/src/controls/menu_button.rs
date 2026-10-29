//! Fluent MenuButton: one button with a trailing chevron that opens a menu.
//!
//! The popup is framed by [`crate::surfaces::menu_frame`] and filled by the caller. Metrics,
//! colours and the focus ring are the same as [`super::SplitButton`].

use egui::{Id, Key, Modifiers, Popup, Response, Role, Sense, Ui, Vec2, WidgetInfo, WidgetText};

use fluent2_icons::Icon;

use super::super::foundation::focus;
use super::button::{Appearance, ButtonSize, Shape};
use super::split_button::{
    Metrics, corner_radius, ensure_type_ramp, face_of, natural_width, paint_glyph, paint_region,
};
use crate::theme::{Theme, c};

/// A button that opens a menu. The whole control is one focus stop.
///
/// ```
/// # use fluent2_egui::controls::MenuButton;
/// # fn demo(ui: &mut egui::Ui) {
/// let (response, picked) = MenuButton::new("Open").show(ui, |ui| {
///     ui.label("Recent");
///     "recent"
/// });
/// let _ = (response.clicked(), picked);
/// # }
/// ```
pub struct MenuButton<'a> {
    text: WidgetText,
    icon: Option<Icon>,
    appearance: Appearance,
    size: ButtonSize,
    shape: Shape,
    id_salt: Option<Id>,
    _lifetime: std::marker::PhantomData<&'a ()>,
}

impl<'a> MenuButton<'a> {
    /// A menu button labelled `text`.
    pub fn new(text: impl Into<WidgetText>) -> Self {
        Self {
            text: text.into(),
            icon: None,
            appearance: Appearance::Secondary,
            size: ButtonSize::Medium,
            shape: Shape::Rounded,
            id_salt: None,
            _lifetime: std::marker::PhantomData,
        }
    }

    /// Place an icon before the label.
    #[must_use]
    pub fn icon(mut self, icon: Icon) -> Self {
        self.icon = Some(icon);
        self
    }

    /// Visual weight. Defaults to [`Appearance::Secondary`].
    #[must_use]
    pub fn appearance(mut self, appearance: Appearance) -> Self {
        self.appearance = appearance;
        self
    }

    /// Size. Defaults to [`ButtonSize::Medium`].
    #[must_use]
    pub fn size(mut self, size: ButtonSize) -> Self {
        self.size = size;
        self
    }

    /// Corner treatment. Defaults to [`Shape::Rounded`].
    #[must_use]
    pub fn shape(mut self, shape: Shape) -> Self {
        self.shape = shape;
        self
    }

    /// Extra salt so two menu buttons with the same label can coexist.
    #[must_use]
    pub fn id_salt(mut self, salt: impl std::hash::Hash + std::fmt::Debug) -> Self {
        self.id_salt = Some(Id::unique(salt));
        self
    }

    /// Show the button and, when open, the menu popup.
    ///
    /// Returns the button response and `Some` of what `menu` returned while the popup is open.
    pub fn show<R>(self, ui: &mut Ui, menu: impl FnOnce(&mut Ui) -> R) -> (Response, Option<R>) {
        let theme = Theme::get(ui.ctx());
        ensure_type_ramp(ui);
        let metrics = Metrics::of(self.size).at(crate::theme::density_of(ui));
        let enabled = ui.is_enabled();
        let label = self.text.text().to_owned();

        let id = self
            .id_salt
            .unwrap_or_else(|| ui.next_auto_id())
            .with("menu_button");
        let width =
            natural_width(ui, &self.text, self.icon.is_some(), metrics) + metrics.icon + metrics.icon_gap;
        let desired = Vec2::new(finite(width), metrics.min_height);
        let rect = ui.allocate_exact_size(desired, Sense::hover()).0;
        let response = ui.interact(rect, id, Sense::click());

        let popup_id = Popup::default_response_id(&response);
        let open_key = response.has_focus()
            && ui.input_mut(|input| {
                input.consume_key(Modifiers::NONE, Key::Enter)
                    || input.consume_key(Modifiers::NONE, Key::Space)
                    || input.consume_key(Modifiers::ALT, Key::ArrowDown)
            });
        if open_key || response.clicked() {
            Popup::toggle_id(ui.ctx(), popup_id);
        }
        let open = Popup::is_id_open(ui.ctx(), popup_id);
        if ui.input(|input| input.key_pressed(Key::Escape)) && open {
            Popup::close_id(ui.ctx(), popup_id);
            response.request_focus();
        }

        if ui.is_rect_visible(rect) {
            let radius = corner_radius(self.size, self.shape, rect);
            let face = face_of(self.appearance, enabled, &response, &theme);
            paint_region(ui, rect, radius, face);
            paint_content(ui, rect, &self, metrics, face);
            if focus::focus_visible(ui, &response) {
                focus::paint_focus_ring(ui, rect, radius);
            }
        }

        response.widget_info(|| WidgetInfo::labeled(Role::Button, enabled, &label));
        ui.ctx().accesskit_node_builder(id, |node| {
            node.set_expanded(open);
        });

        let mut value = None;
        if open
            && enabled
            && let Some(inner) = Popup::menu(&response)
                .open_memory(None)
                .show(|popup| crate::surfaces::menu_frame(popup).show(popup, menu).inner)
        {
            value = Some(inner.inner);
        }
        (response, value)
    }
}

fn paint_content(
    ui: &Ui,
    rect: egui::Rect,
    button: &MenuButton<'_>,
    metrics: Metrics,
    face: super::split_button::Face,
) {
    let foreground = c(face.foreground);
    let (glyph, glyph_px) = metrics.chevron_glyph();
    let icon_px = if button.icon.is_some() { metrics.icon } else { 0.0 };
    let galley = button.text.clone().into_galley(
        ui,
        Some(egui::TextWrapMode::Truncate),
        (rect.width() - metrics.h_padding * 2.0 - icon_px - glyph_px - metrics.icon_gap * 2.0).max(0.0),
        metrics.text_style(),
    );
    let leading_gap = if icon_px > 0.0 { metrics.icon_gap } else { 0.0 };
    let total = icon_px + leading_gap + galley.size().x + metrics.icon_gap + glyph_px;
    let mut x = rect.center().x - total / 2.0;
    if let Some(icon) = button.icon {
        paint_glyph(
            ui,
            icon,
            metrics.icon_size(),
            foreground,
            egui::pos2(x, rect.center().y - icon_px / 2.0),
            icon_px,
        );
        x += icon_px + leading_gap;
    }
    let pos = egui::pos2(x, rect.center().y - galley.size().y / 2.0);
    let text_w = galley.size().x;
    ui.painter().galley(pos, galley, foreground);
    x += text_w + metrics.icon_gap;
    paint_glyph(
        ui,
        Icon::ChevronDown,
        glyph,
        foreground,
        egui::pos2(x, rect.center().y - glyph_px / 2.0),
        glyph_px,
    );
}

fn finite(value: f32) -> f32 {
    if value.is_finite() && value > 0.0 {
        value
    } else {
        0.0
    }
}
