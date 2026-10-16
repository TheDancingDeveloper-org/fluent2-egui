//! Fluent Link.
//!
//! Colours follow Fluent UI React `useLinkStyles`: `brand_foreground_link`,
//! `brand_foreground_link_hover` (underlined) and `brand_foreground_link_pressed`.
//! The link does not open a URL. The caller handles [`egui::Response::clicked`].

use egui::{Response, Sense, TextStyle, Ui, Widget, WidgetInfo, WidgetText, WidgetType};

use crate::foundation::focus;
use crate::foundation::text::TypeRamp;
use crate::theme::{self, Theme};

/// A Fluent hyperlink.
///
/// ```
/// # use fluent2_egui::controls::Link;
/// # fn demo(ui: &mut egui::Ui) {
/// if ui.add(Link::new("Details").inline()).clicked() {
///     // the app decides where the link goes
/// }
/// # }
/// ```
pub struct Link<'a> {
    text: WidgetText,
    inline: bool,
    _lifetime: std::marker::PhantomData<&'a ()>,
}

impl<'a> Link<'a> {
    /// A link whose accessible name is `text`.
    pub fn new(text: impl Into<WidgetText>) -> Self {
        Self {
            text: text.into(),
            inline: false,
            _lifetime: std::marker::PhantomData,
        }
    }

    /// Sit in a line of text instead of taking a block.
    #[must_use]
    pub fn inline(mut self) -> Self {
        self.inline = true;
        self
    }
}

impl Widget for Link<'_> {
    fn ui(self, ui: &mut Ui) -> Response {
        ensure_type_ramp(ui);
        let theme = Theme::get(ui.ctx());
        let enabled = ui.is_enabled();
        let style = TextStyle::Name(TypeRamp::Body1.name().into());
        let rest = link_color(enabled, false, false, &theme);
        let galley = self.text.clone().color(rest).into_galley(
            ui,
            Some(egui::TextWrapMode::Truncate),
            ui.available_width().max(1.0),
            style,
        );
        let sense = if enabled { Sense::click() } else { Sense::hover() };
        let (rect, response) = if self.inline {
            ui.allocate_exact_size(galley.size(), sense)
        } else {
            ui.allocate_at_least(galley.size(), sense)
        };
        let hovered = response.hovered();
        let pressed = response.is_pointer_button_down_on();
        let color = link_color(enabled, hovered, pressed, &theme);
        if ui.is_rect_visible(rect) {
            let galley = self.text.clone().color(color).into_galley(
                ui,
                Some(egui::TextWrapMode::Truncate),
                rect.width().max(1.0),
                TextStyle::Name(TypeRamp::Body1.name().into()),
            );
            let pos = rect.left_top();
            ui.painter().galley(pos, galley, color);
            if hovered || pressed {
                let underline = egui::Stroke::new(1.0, color);
                let y = rect.bottom() - 1.0;
                ui.painter().hline(rect.x_range(), y, underline);
            }
            if focus::focus_visible(ui, &response) {
                focus::paint_focus_ring(ui, rect, egui::CornerRadius::ZERO);
            }
        }
        let label = self.text.text().to_owned();
        response.widget_info(|| WidgetInfo::labeled(WidgetType::Link, enabled, label.clone()));
        response
    }
}

fn link_color(enabled: bool, hovered: bool, pressed: bool, theme: &Theme) -> egui::Color32 {
    let color = &theme.color;
    let rgba = if !enabled {
        color.neutral_foreground_disabled
    } else if pressed {
        color.brand_foreground_link_pressed
    } else if hovered {
        color.brand_foreground_link_hover
    } else {
        color.brand_foreground_link
    };
    theme::c(rgba)
}

fn ensure_type_ramp(ui: &mut Ui) {
    use crate::foundation::text::{font_id, style};
    if ui.style().text_styles.contains_key(&style(TypeRamp::Body1)) {
        return;
    }
    let styles = &mut ui.style_mut().text_styles;
    for step in TypeRamp::ALL {
        styles.insert(style(step), font_id(step));
    }
}
