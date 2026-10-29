//! Fluent Input: an egui [`TextEdit`] inside a custom-painted Fluent frame.
//!
//! Text editing is egui's. This control paints the frame (`TextEdit` with [`egui::Frame::NONE`])
//! and the focus accent. Metrics follow Fluent UI React `useInputStyles` at the revision pinned
//! on WI-1547: heights 24 / 32 / 40, radius `MEDIUM`, a 1 px rest stroke plus a 1 px accessible
//! bottom border, and a 2 px brand bottom bar that grows from the centre while focused.
//!
//! The accent uses [`foundation::motion::animate`] (`Duration::Normal`, `Curve::DecelerateMid`).
//! Icons and the focus ring come from the foundation helpers.

use egui::{
    Color32, CornerRadius, Id, IdSalt, Rect, Response, Role, Sense, Stroke, StrokeKind, TextEdit, Ui, Vec2,
    Widget, WidgetInfo, epaint::RectShape,
};

use crate::foundation::focus::{focus_visible, paint_focus_ring};
use crate::foundation::icon::{IconName, IconSize, IconStyle, paint_icon};
use crate::foundation::motion::{Curve, Duration, animate};
use crate::theme::{Theme, c};
use fluent2_tokens::generated::global::{border_radius, spacing_horizontal, stroke_width};

use super::ButtonSize;

fn input_height(size: ButtonSize) -> f32 {
    match size {
        ButtonSize::Small => 24.0,
        ButtonSize::Medium => 32.0,
        ButtonSize::Large => 40.0,
    }
}

fn input_icon(size: ButtonSize) -> IconSize {
    if matches!(size, ButtonSize::Large) {
        IconSize::S20
    } else {
        IconSize::S16
    }
}

/// Rest treatment of an input. [`Outline`](Self::Outline) is the Fluent default.
#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub enum InputAppearance {
    /// `neutral_background_1`, a `neutral_stroke_1` stroke, and the accessible bottom border.
    #[default]
    Outline,
    /// No side stroke. Only the accessible bottom border, which becomes the accent bar when focused.
    Underline,
    /// `neutral_background_3` fill and no rest stroke.
    FilledDarker,
    /// `neutral_background_1` fill and no rest stroke, for a darker page surface.
    FilledLighter,
}

/// A single-line Fluent input.
///
/// The returned [`Response`] is the inner [`TextEdit`]'s, so [`Response::changed`] and
/// [`Response::lost_focus`] report the field.
///
/// ```
/// # use fluent2_egui::controls::{Input, InputAppearance};
/// # use fluent2_egui::IconName;
/// # fn demo(ui: &mut egui::Ui, value: &mut String) {
/// let _ = ui.add(
///     Input::new(value)
///         .placeholder("Name")
///         .appearance(InputAppearance::Outline)
///         .content_before(IconName::Person),
/// );
/// # }
/// ```
pub struct Input<'a> {
    text: &'a mut String,
    placeholder: Option<String>,
    label: Option<String>,
    size: ButtonSize,
    appearance: InputAppearance,
    content_before: Option<IconName>,
    content_after: Option<IconName>,
    password: bool,
    desired_width: Option<f32>,
    id_salt: Option<IdSalt>,
    /// SearchBox sets this. Not a public builder: the dismiss control is that control's.
    clear_button: bool,
}

impl<'a> Input<'a> {
    /// An input bound to `text`.
    pub fn new(text: &'a mut String) -> Self {
        Self {
            text,
            placeholder: None,
            label: None,
            size: ButtonSize::Medium,
            appearance: InputAppearance::Outline,
            content_before: None,
            content_after: None,
            password: false,
            desired_width: None,
            id_salt: None,
            clear_button: false,
        }
    }

    /// Grey hint (`neutral_foreground_4`), drawn only while `text` is empty. Also the accessible hint.
    #[must_use]
    pub fn placeholder(mut self, placeholder: impl Into<String>) -> Self {
        self.placeholder = Some(placeholder.into());
        self
    }

    /// Accessible name. Not drawn; a Field control adds the visible label later.
    #[must_use]
    pub fn label(mut self, label: impl Into<String>) -> Self {
        self.label = Some(label.into());
        self
    }

    /// Height. Defaults to [`ButtonSize::Medium`] (32 px).
    #[must_use]
    pub fn size(mut self, size: ButtonSize) -> Self {
        self.size = size;
        self
    }

    /// Rest treatment. Defaults to [`InputAppearance::Outline`].
    #[must_use]
    pub fn appearance(mut self, appearance: InputAppearance) -> Self {
        self.appearance = appearance;
        self
    }

    /// Icon drawn inside the leading edge.
    #[must_use]
    pub fn content_before(mut self, icon: IconName) -> Self {
        self.content_before = Some(icon);
        self
    }

    /// Icon drawn inside the trailing edge.
    #[must_use]
    pub fn content_after(mut self, icon: IconName) -> Self {
        self.content_after = Some(icon);
        self
    }

    /// Mask the text. The stored string is unchanged.
    #[must_use]
    pub fn password(mut self) -> Self {
        self.password = true;
        self
    }

    /// Width of the field. Defaults to [`Ui::available_width`].
    #[must_use]
    pub fn desired_width(mut self, width: f32) -> Self {
        self.desired_width = Some(finite(width).max(0.0));
        self
    }

    /// Extra salt for the widget id, so two inputs can coexist.
    #[must_use]
    pub fn id_salt(mut self, salt: impl std::hash::Hash + std::fmt::Debug) -> Self {
        self.id_salt = Some(IdSalt::new(salt));
        self
    }

    /// Trailing dismiss button while `text` is not empty. Used by [`super::SearchBox`].
    pub(crate) fn clear_button(mut self) -> Self {
        self.clear_button = true;
        self
    }
}

impl Widget for Input<'_> {
    fn ui(self, ui: &mut Ui) -> Response {
        show_field(ui, Field::single(self))
    }
}

pub(crate) struct Field<'a> {
    pub(crate) text: &'a mut String,
    pub(crate) placeholder: Option<String>,
    pub(crate) label: Option<String>,
    pub(crate) appearance: InputAppearance,
    pub(crate) content_before: Option<IconName>,
    pub(crate) content_after: Option<IconName>,
    pub(crate) password: bool,
    pub(crate) multiline: bool,
    pub(crate) rows: usize,
    pub(crate) resize_vertical: bool,
    pub(crate) height: f32,
    pub(crate) icon: IconSize,
    pub(crate) desired_width: Option<f32>,
    pub(crate) id_salt: Option<IdSalt>,
    /// SearchBox: a trailing dismiss control clears the string while it is not empty.
    pub(crate) clear_button: bool,
    /// Textarea only. `None` for single-line fields.
    pub(crate) spell: Option<&'a mut dyn super::spell::SpellChecker>,
}

impl<'a> Field<'a> {
    fn single(input: Input<'a>) -> Self {
        Self {
            text: input.text,
            placeholder: input.placeholder,
            label: input.label,
            appearance: input.appearance,
            content_before: input.content_before,
            content_after: input.content_after,
            password: input.password,
            multiline: false,
            rows: 1,
            resize_vertical: false,
            height: input_height(input.size),
            icon: input_icon(input.size),
            desired_width: input.desired_width,
            id_salt: input.id_salt,
            clear_button: input.clear_button,
            spell: None,
        }
    }
}

pub(crate) fn show_field(ui: &mut Ui, mut field: Field<'_>) -> Response {
    let theme = Theme::get(ui.ctx());
    let enabled = ui.is_enabled();
    let icon_px = match field.icon {
        IconSize::S16 => 16.0,
        IconSize::S20 => 20.0,
        IconSize::S24 => 24.0,
    };
    let pad_x = spacing_horizontal::S_NUDGE + spacing_horizontal::XXS;
    let slot = icon_px + spacing_horizontal::XXS;
    let leading = if field.content_before.is_some() { slot } else { 0.0 };
    let show_clear = field.clear_button && !field.text.is_empty();
    let trailing_slots = u8::from(field.content_after.is_some()) + u8::from(show_clear);

    let width = field
        .desired_width
        .unwrap_or_else(|| ui.available_width())
        .max(icon_px + pad_x * 2.0);
    let line = ui.text_style_height(&egui::TextStyle::Body);
    let height = if field.multiline {
        line * field.rows.max(1) as f32 + pad_x
    } else {
        crate::theme::density_of(ui).height(field.height)
    };
    let (rect, _) = ui.allocate_exact_size(Vec2::new(finite(width), finite(height)), Sense::hover());

    // Salt, not a root id. `Id::unique` ignores the parent, so two empty inputs
    // (no label, no placeholder) would share one id and egui paints an id-clash warning.
    // The leading icon and the clear button are part of the salt: a plain Input and a
    // SearchBox can otherwise share label, placeholder and line count.
    let id = match field.id_salt {
        Some(salt) => ui.scope_id().with_salt(salt),
        None => ui.make_persistent_id((
            "fluent2_input",
            field.label.clone(),
            field.placeholder.clone(),
            field.multiline,
            field.content_before,
            field.clear_button,
        )),
    };
    // Escape clears a SearchBox. Focus this frame is assigned while TextEdit runs, so the
    // check uses last frame's focus, which is set for the frame the key arrives.
    if field.clear_button
        && enabled
        && !field.text.is_empty()
        && ui.memory(|memory| memory.had_focus_last_frame(id))
        && ui.input_mut(|input| input.consume_key(egui::Modifiers::NONE, egui::Key::Escape))
    {
        field.text.clear();
    }
    let text_rect = Rect::from_min_max(
        egui::pos2(rect.left() + pad_x + leading, rect.top()),
        egui::pos2(
            rect.right() - pad_x - slot * f32::from(trailing_slots),
            rect.bottom(),
        ),
    );

    let hint = field.placeholder.clone().unwrap_or_default();
    let text_color = if enabled {
        c(theme.color.neutral_foreground_1)
    } else {
        c(theme.color.neutral_foreground_disabled)
    };
    let hint_color = c(theme.color.neutral_foreground_4);
    let hint_text = egui::WidgetText::from(hint.clone()).color(hint_color);
    let edit = if field.multiline {
        let mut multi = TextEdit::multiline(field.text)
            .frame(egui::Frame::NONE)
            .id(id)
            .interactive(enabled)
            .text_color(text_color)
            .hint_text(hint_text.clone())
            .desired_rows(field.rows.max(1));
        if !field.resize_vertical {
            multi = multi.min_size(text_rect.size());
        }
        multi
    } else {
        let mut single = TextEdit::singleline(field.text)
            .frame(egui::Frame::NONE)
            .id(id)
            .interactive(enabled)
            .text_color(text_color)
            .hint_text(hint_text)
            .desired_width(text_rect.width().max(0.0));
        if field.password {
            single = single.password(true);
        }
        single
    };
    // The frame fill goes under the text: reserve its slot before the TextEdit paints
    // (WI-1913), and fill it in `paint_frame` once focus and hover are known.
    let background = ui.painter().add(egui::Shape::Noop);
    let spell = field.spell.take();
    let mut response = if let Some(checker) = spell {
        // `show` needs the `Ui` that owns the text rect, which `put` builds for a `Widget`.
        let mut output = ui
            .scope_builder(
                egui::UiBuilder::new()
                    .max_rect(text_rect)
                    .layout(egui::Layout::centered_and_justified(egui::Direction::TopDown)),
                |ui| edit.show(ui),
            )
            .inner;
        let mut session = super::spell::SpellSession::new(id, field.text, checker);
        session.paint(ui, &output.galley, output.galley_pos);
        let action = session.menu(ui, &output.response.response, &output.galley, output.galley_pos);
        if let Some((range, with)) = action.replace
            && super::spell::apply_replace(field.text, range, &with).is_some()
        {
            output.response.response.mark_changed();
        }
        output.response.response
    } else {
        ui.put(text_rect, edit)
    };

    paint_frame(
        ui,
        rect,
        &theme,
        FramePaint {
            appearance: field.appearance,
            focused: response.has_focus() && enabled,
            hovered: response.hovered() && enabled,
            enabled,
            accent_rect: response.rect,
            background,
        },
    );

    let icon_color = if enabled {
        c(theme.color.neutral_foreground_3)
    } else {
        c(theme.color.neutral_foreground_disabled)
    };
    if let Some(icon) = field.content_before {
        let icon_rect = Rect::from_center_size(
            egui::pos2(rect.left() + pad_x + icon_px / 2.0, rect.center().y),
            Vec2::splat(icon_px),
        );
        paint_icon(
            ui,
            icon_rect,
            icon,
            field.icon,
            IconStyle::Regular,
            Some(icon_color),
        );
    }
    if let Some(icon) = field.content_after {
        let icon_rect = Rect::from_center_size(
            egui::pos2(
                rect.right() - pad_x - icon_px / 2.0 - if show_clear { slot } else { 0.0 },
                rect.center().y,
            ),
            Vec2::splat(icon_px),
        );
        paint_icon(
            ui,
            icon_rect,
            icon,
            field.icon,
            IconStyle::Regular,
            Some(icon_color),
        );
    }

    if show_clear && enabled {
        let clear_rect = Rect::from_center_size(
            egui::pos2(rect.right() - pad_x - icon_px / 2.0, rect.center().y),
            Vec2::splat(icon_px),
        );
        let clear = ui.interact(clear_rect, id.with("clear"), Sense::click());
        paint_icon(
            ui,
            clear_rect,
            IconName::Dismiss,
            field.icon,
            IconStyle::Regular,
            Some(icon_color),
        );
        if clear.clicked() {
            field.text.clear();
            response.request_focus();
            response.mark_changed();
        }
        clear.widget_info(|| WidgetInfo::labeled(Role::Button, true, "Clear"));
    }

    if focus_visible(ui, &response) {
        paint_focus_ring(ui, rect, CornerRadius::same(rounded_u8(border_radius::MEDIUM)));
    }

    let accessible = field
        .label
        .clone()
        .filter(|label| !label.is_empty())
        .or_else(|| super::field::enclosing_field_label(ui.ctx()).filter(|label| !label.is_empty()))
        .unwrap_or_default();
    // The TextEdit node lives at `id`, which we passed in. `response.id` is the outer
    // allocation, so a later `accessible_name` would name the wrong node. The placeholder
    // already went in as `hint_text`.
    if !accessible.is_empty() {
        // The TextEdit node is `id` (passed to TextEdit::id). `response.id` is the outer
        // allocation, so a later `accessible_name` would name the wrong node.
        ui.ctx().accesskit_node_builder(id, |builder| {
            builder.set_label(accessible);
        });
    }
    response
}

struct FramePaint {
    appearance: InputAppearance,
    focused: bool,
    hovered: bool,
    enabled: bool,
    accent_rect: Rect,
    /// The slot reserved before the text was painted, so the fill lands beneath it.
    background: egui::layers::ShapeIdx,
}

fn paint_frame(ui: &Ui, rect: Rect, theme: &Theme, paint: FramePaint) {
    let radius = if matches!(paint.appearance, InputAppearance::Underline) {
        CornerRadius::ZERO
    } else {
        CornerRadius::same(rounded_u8(border_radius::MEDIUM))
    };
    let fill = match paint.appearance {
        InputAppearance::FilledDarker => c(theme.color.neutral_background_3),
        _ => c(theme.color.neutral_background_1),
    };
    let rest = if !paint.enabled && matches!(paint.appearance, InputAppearance::Outline) {
        Stroke::new(stroke_width::THIN, c(theme.color.neutral_stroke_disabled))
    } else if matches!(paint.appearance, InputAppearance::Outline) {
        let color = if paint.hovered {
            theme.color.neutral_stroke_1_hover
        } else {
            theme.color.neutral_stroke_1
        };
        Stroke::new(stroke_width::THIN, c(color))
    } else {
        Stroke::NONE
    };
    ui.painter().set(
        paint.background,
        RectShape::new(rect, radius, fill, rest, StrokeKind::Inside),
    );

    let target = if paint.focused { 1.0 } else { 0.0 };
    let t = animate(
        ui.ctx(),
        accent_id(paint.accent_rect),
        target,
        Duration::Normal,
        Curve::DecelerateMid,
    );
    if t > 0.0 {
        let half = rect.width() * 0.5 * t;
        let bar = Rect::from_min_max(
            egui::pos2(rect.center().x - half, rect.bottom() - stroke_width::THICK),
            egui::pos2(rect.center().x + half, rect.bottom()),
        );
        ui.painter()
            .rect_filled(bar, CornerRadius::ZERO, c(theme.color.compound_brand_stroke));
    }
    if t < 1.0 {
        let color = if !paint.enabled {
            theme.color.neutral_stroke_disabled
        } else if paint.hovered {
            theme.color.neutral_stroke_accessible_hover
        } else {
            theme.color.neutral_stroke_accessible
        };
        ui.painter().line_segment(
            [rect.left_bottom(), rect.right_bottom()],
            Stroke::new(stroke_width::THIN, c(color)),
        );
    }
}

/// Whether the focus accent for `rect` is currently aimed at fully open.
///
/// The motion helper stores the animation target. A focused field stores `1.0`; an unfocused
/// field stores `0.0` only after it has been painted. Absence means the field was not painted.
pub fn accent_open_for_test(ctx: &egui::Context, rect: Rect) -> Option<bool> {
    let id = accent_id(rect).with("fluent2_motion_to");
    ctx.data(|data| data.get_temp::<f32>(id).map(|value| value > 0.5))
}

fn accent_id(rect: Rect) -> Id {
    Id::unique((
        "fluent2_input_accent",
        rect.min.x.to_bits(),
        rect.min.y.to_bits(),
        rect.width().to_bits(),
    ))
}

fn rounded_u8(value: f32) -> u8 {
    if !value.is_finite() || value <= 0.0 {
        0
    } else if value >= f32::from(u8::MAX) {
        u8::MAX
    } else {
        value.round() as u8
    }
}

fn finite(value: f32) -> f32 {
    if value.is_finite() { value } else { 0.0 }
}

// Silence a doctest-only import name clash: Color32 is used by text colours.
const _: Option<Color32> = None;
