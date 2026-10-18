//! Ribbon controls (spec §4.2). Each kind is a builder that converts into a [`RibbonControl`]
//! and paints itself in its group's column. It reports through [`RibbonResponse`] by its key.
//!
//! Disabled state is egui's: a control inside a disabled `Ui` paints disabled and reports
//! nothing.

use std::marker::PhantomData;
use std::sync::Arc;

use egui::{
    Galley, Pos2, Rect, Response, Sense, TextStyle, TextWrapMode, Ui, Vec2, WidgetInfo, WidgetText,
    WidgetType, accesskit,
};
use fluent2_tokens::generated::global::{border_radius, spacing_horizontal};

use super::group::ScalingStep;
use super::{RibbonEvent, RibbonEventKind, RibbonResponse, radius};
use crate::controls::{ButtonSize, Checkbox, Combobox, Dropdown, MenuButton, SplitButton, ToggleButton};
use crate::foundation::focus::{self, paint_focus_ring};
use crate::foundation::icon::{IconName, IconSize, IconStyle, paint_icon};
use crate::foundation::text::TypeRamp;
use crate::surfaces::TooltipExt;
use crate::theme::{Theme, c};

/// Text in the `Caption1` step. It wraps to `width` when `wrap` is set.
pub(super) fn caption(ui: &Ui, text: &str, wrap: bool, width: f32) -> Arc<Galley> {
    let mode = if wrap {
        TextWrapMode::Wrap
    } else {
        TextWrapMode::Extend
    };
    WidgetText::from(text).into_galley(
        ui,
        Some(mode),
        width,
        TextStyle::Name(TypeRamp::Caption1.name().into()),
    )
}

/// The tooltip of a control: its label, then the shortcut text the caller gave, if any.
fn tooltip(label: &str, shortcut: Option<&str>) -> String {
    match shortcut {
        Some(shortcut) => format!("{label}\n{shortcut}"),
        None => label.to_owned(),
    }
}

fn report_click(out: &mut RibbonResponse, key: &'static str, response: &Response) {
    if response.clicked() {
        out.events.push(RibbonEvent {
            key,
            kind: RibbonEventKind::Clicked,
        });
    }
}

/// `Changed` when the bound value differs from `before`. Comparing values, not the widget's
/// own `changed` flag, makes this hold for every control that writes its binding.
fn report_change(out: &mut RibbonResponse, key: &'static str, changed: bool) {
    if changed {
        out.events.push(RibbonEvent {
            key,
            kind: RibbonEventKind::Changed,
        });
    }
}

/// Large button: icon above a two-line label. Event: `Clicked`.
///
/// ```
/// # use fluent2_egui::office::ribbon::RibbonLarge;
/// # use fluent2_egui::IconName;
/// let paste = RibbonLarge::new("paste", IconName::ClipboardPaste, "Paste").shortcut("Ctrl+V");
/// ```
pub struct RibbonLarge<'a> {
    key: &'static str,
    icon: IconName,
    label: String,
    shortcut: Option<String>,
    _bound: PhantomData<&'a ()>,
}

impl<'a> RibbonLarge<'a> {
    /// A large button. `key` names it in events and must be unique within the ribbon.
    pub fn new(key: &'static str, icon: IconName, label: &str) -> Self {
        Self {
            key,
            icon,
            label: label.to_owned(),
            shortcut: None,
            _bound: PhantomData,
        }
    }

    /// Shortcut text shown under the label in the tooltip, e.g. `"Ctrl+V"`.
    #[must_use]
    pub fn shortcut(mut self, shortcut: &str) -> Self {
        self.shortcut = Some(shortcut.to_owned());
        self
    }

    fn show(self, ui: &mut Ui, out: &mut RibbonResponse) {
        let theme = Theme::get(ui.ctx());
        let enabled = ui.is_enabled();
        let chrome = &theme.chrome;
        let pad = spacing_horizontal::XS;
        let icon = chrome.icon_large;
        let text = caption(ui, &self.label, true, chrome.large_label_max);
        let width = text.size().x.max(icon) + 2.0 * pad;
        let height = icon + 3.0 * pad + 2.0 * TypeRamp::Caption1.line_height();

        let (rect, response) = ui.allocate_exact_size(Vec2::new(width, height), Sense::click());
        let color = &theme.color;
        let fill = if !enabled {
            None
        } else if response.is_pointer_button_down_on() {
            Some(color.subtle_background_pressed)
        } else if response.hovered() {
            Some(color.subtle_background_hover)
        } else {
            None
        };
        let corner = radius(border_radius::MEDIUM);
        if let Some(fill) = fill {
            ui.painter().rect_filled(rect, corner, c(fill));
        }

        let ink = if enabled {
            color.neutral_foreground_1
        } else {
            color.neutral_foreground_disabled
        };
        // The 32 pt slot holds the 24 pt icon, centred.
        let slot = Rect::from_min_size(
            Pos2::new(rect.center().x - icon / 2.0, rect.top() + pad),
            Vec2::splat(icon),
        );
        paint_icon(
            ui,
            slot,
            self.icon,
            IconSize::S24,
            IconStyle::Regular,
            Some(c(ink)),
        );
        let origin = Pos2::new(rect.center().x - text.size().x / 2.0, slot.bottom() + pad);
        ui.painter().galley(origin, Arc::clone(&text), c(ink));

        if focus::focus_visible(ui, &response) {
            paint_focus_ring(ui, rect, corner);
        }
        response.widget_info(|| WidgetInfo::labeled(WidgetType::Button, enabled, self.label.as_str()));
        let response = response.fluent_tooltip(tooltip(&self.label, self.shortcut.as_deref()));
        report_click(out, self.key, &response);
    }
}

/// Small button: icon beside the label, 24 pt high. Event: `Clicked`.
///
/// ```
/// # use fluent2_egui::office::ribbon::RibbonSmall;
/// # use fluent2_egui::IconName;
/// let cut = RibbonSmall::new("cut", IconName::Cut, "Cut");
/// ```
pub struct RibbonSmall<'a> {
    key: &'static str,
    icon: IconName,
    label: String,
    shortcut: Option<String>,
    _bound: PhantomData<&'a ()>,
}

impl<'a> RibbonSmall<'a> {
    /// A small button. `key` names it in events and must be unique within the ribbon.
    pub fn new(key: &'static str, icon: IconName, label: &str) -> Self {
        Self {
            key,
            icon,
            label: label.to_owned(),
            shortcut: None,
            _bound: PhantomData,
        }
    }

    /// Shortcut text shown in the tooltip.
    #[must_use]
    pub fn shortcut(mut self, shortcut: &str) -> Self {
        self.shortcut = Some(shortcut.to_owned());
        self
    }

    fn show(self, ui: &mut Ui, out: &mut RibbonResponse) {
        let theme = Theme::get(ui.ctx());
        let enabled = ui.is_enabled();
        let chrome = &theme.chrome;
        let pad = spacing_horizontal::S;
        let gap = spacing_horizontal::XS;
        let icon = chrome.icon_small;
        let text = caption(ui, &self.label, false, f32::INFINITY);
        let width = pad + icon + gap + text.size().x + pad;
        let height = icon + 2.0 * gap;

        let (rect, response) = ui.allocate_exact_size(Vec2::new(width, height), Sense::click());
        let color = &theme.color;
        let fill = if !enabled {
            None
        } else if response.is_pointer_button_down_on() {
            Some(color.subtle_background_pressed)
        } else if response.hovered() {
            Some(color.subtle_background_hover)
        } else {
            None
        };
        let corner = radius(border_radius::MEDIUM);
        if let Some(fill) = fill {
            ui.painter().rect_filled(rect, corner, c(fill));
        }

        let ink = if enabled {
            color.neutral_foreground_1
        } else {
            color.neutral_foreground_disabled
        };
        let slot = Rect::from_min_size(rect.min + Vec2::new(pad, gap), Vec2::splat(icon));
        paint_icon(
            ui,
            slot,
            self.icon,
            IconSize::S16,
            IconStyle::Regular,
            Some(c(ink)),
        );
        let origin = Pos2::new(slot.right() + gap, rect.center().y - text.size().y / 2.0);
        ui.painter().galley(origin, Arc::clone(&text), c(ink));

        if focus::focus_visible(ui, &response) {
            paint_focus_ring(ui, rect, corner);
        }
        response.widget_info(|| WidgetInfo::labeled(WidgetType::Button, enabled, self.label.as_str()));
        let response = response.fluent_tooltip(tooltip(&self.label, self.shortcut.as_deref()));
        report_click(out, self.key, &response);
    }
}

/// Toggle button bound to `value`. Small. Event: `Changed` (`value` is already updated).
///
/// ```
/// # use fluent2_egui::office::ribbon::RibbonToggle;
/// # use fluent2_egui::IconName;
/// # let mut bold = false;
/// let toggle = RibbonToggle::new("bold", IconName::TextBold, "Bold", &mut bold);
/// ```
pub struct RibbonToggle<'a> {
    key: &'static str,
    icon: IconName,
    label: String,
    shortcut: Option<String>,
    value: &'a mut bool,
}

impl<'a> RibbonToggle<'a> {
    /// A toggle bound to `value`. `key` names it in events and must be unique within the ribbon.
    pub fn new(key: &'static str, icon: IconName, label: &str, value: &'a mut bool) -> Self {
        Self {
            key,
            icon,
            label: label.to_owned(),
            shortcut: None,
            value,
        }
    }

    /// Shortcut text shown in the tooltip.
    #[must_use]
    pub fn shortcut(mut self, shortcut: &str) -> Self {
        self.shortcut = Some(shortcut.to_owned());
        self
    }

    fn show(self, ui: &mut Ui, out: &mut RibbonResponse) {
        let before = *self.value;
        ui.push_id(self.key, |ui| {
            ui.add(
                ToggleButton::new(&mut *self.value, self.label.as_str())
                    .icon(self.icon)
                    .size(ButtonSize::Small),
            )
            .fluent_tooltip(tooltip(&self.label, self.shortcut.as_deref()))
        });
        report_change(out, self.key, *self.value != before);
    }
}

/// Split button: a main action and a chevron that opens `menu`. `Clicked` on the main action
/// only; items in the menu write app state and report nothing (spec §4.2, WI-1544).
///
/// ```
/// # use fluent2_egui::office::ribbon::RibbonSplit;
/// # use fluent2_egui::IconName;
/// let paste = RibbonSplit::new("paste", IconName::ClipboardPaste, "Paste", |ui| {
///     ui.label("Paste special");
/// });
/// ```
pub struct RibbonSplit<'a> {
    key: &'static str,
    icon: IconName,
    label: String,
    shortcut: Option<String>,
    small: bool,
    menu: Box<dyn FnOnce(&mut Ui) + 'a>,
}

impl<'a> RibbonSplit<'a> {
    /// A large split button. `menu` draws the popup.
    pub fn new(key: &'static str, icon: IconName, label: &str, menu: impl FnOnce(&mut Ui) + 'a) -> Self {
        Self {
            key,
            icon,
            label: label.to_owned(),
            shortcut: None,
            small: false,
            menu: Box::new(menu),
        }
    }

    /// Use the small size: 24 pt high, icon beside the label, stacked in a column.
    #[must_use]
    pub fn small(mut self) -> Self {
        self.small = true;
        self
    }

    /// Shortcut text shown in the tooltip of the main action.
    #[must_use]
    pub fn shortcut(mut self, shortcut: &str) -> Self {
        self.shortcut = Some(shortcut.to_owned());
        self
    }

    fn show(self, ui: &mut Ui, out: &mut RibbonResponse) {
        let arrow_label = format!("{} menu", self.label);
        let split = SplitButton::new(self.label.as_str())
            .icon(self.icon)
            .size(sizes(self.small))
            .id_salt(self.key);
        let response = split.show(ui, self.menu);
        // The WI-1544 arrow is labelled "<label> options" and has no popup role. Spec §9.1 says
        // "<label> menu" with `has_popup`, so the ribbon sets both on the arrow's node.
        ui.ctx().accesskit_node_builder(response.menu.id, |node| {
            node.set_label(arrow_label);
            node.set_has_popup(accesskit::HasPopup::Menu);
        });
        let main = response
            .main
            .fluent_tooltip(tooltip(&self.label, self.shortcut.as_deref()));
        report_click(out, self.key, &main);
    }
}

/// Menu button: a button that opens `menu`. `Clicked` when pressed to open (spec §4.2).
///
/// ```
/// # use fluent2_egui::office::ribbon::RibbonMenu;
/// # use fluent2_egui::IconName;
/// let more = RibbonMenu::new("more", IconName::Settings, "More", |ui| {
///     ui.label("Option");
/// });
/// ```
pub struct RibbonMenu<'a> {
    key: &'static str,
    icon: IconName,
    label: String,
    shortcut: Option<String>,
    small: bool,
    menu: Box<dyn FnOnce(&mut Ui) + 'a>,
}

impl<'a> RibbonMenu<'a> {
    /// A large menu button. `menu` draws the popup.
    pub fn new(key: &'static str, icon: IconName, label: &str, menu: impl FnOnce(&mut Ui) + 'a) -> Self {
        Self {
            key,
            icon,
            label: label.to_owned(),
            shortcut: None,
            small: false,
            menu: Box::new(menu),
        }
    }

    /// Use the small size: 24 pt high, icon beside the label, stacked in a column.
    #[must_use]
    pub fn small(mut self) -> Self {
        self.small = true;
        self
    }

    /// Shortcut text shown in the tooltip.
    #[must_use]
    pub fn shortcut(mut self, shortcut: &str) -> Self {
        self.shortcut = Some(shortcut.to_owned());
        self
    }

    fn show(self, ui: &mut Ui, out: &mut RibbonResponse) {
        let (response, _) = MenuButton::new(self.label.as_str())
            .icon(self.icon)
            .size(sizes(self.small))
            .id_salt(self.key)
            .show(ui, self.menu);
        // Spec §9.1: a menu button is a button with `has_popup`.
        ui.ctx().accesskit_node_builder(response.id, |node| {
            node.set_has_popup(accesskit::HasPopup::Menu);
        });
        let response = response.fluent_tooltip(tooltip(&self.label, self.shortcut.as_deref()));
        report_click(out, self.key, &response);
    }
}

/// The button size of a split or menu button.
fn sizes(small: bool) -> ButtonSize {
    if small {
        ButtonSize::Small
    } else {
        ButtonSize::Large
    }
}

/// Checkbox bound to `value`, drawn with its label beside it. Event: `Changed`.
///
/// ```
/// # use fluent2_egui::office::ribbon::RibbonCheckbox;
/// # let mut ruler = false;
/// let ruler = RibbonCheckbox::new("ruler", "Ruler", &mut ruler);
/// ```
pub struct RibbonCheckbox<'a> {
    key: &'static str,
    label: String,
    value: &'a mut bool,
}

impl<'a> RibbonCheckbox<'a> {
    /// A checkbox bound to `value`. `key` names it in events and must be unique within the ribbon.
    pub fn new(key: &'static str, label: &str, value: &'a mut bool) -> Self {
        Self {
            key,
            label: label.to_owned(),
            value,
        }
    }

    fn show(self, ui: &mut Ui, out: &mut RibbonResponse) {
        let before = *self.value;
        ui.push_id(self.key, |ui| {
            ui.add(Checkbox::bool(&mut *self.value, self.label.as_str()))
        });
        report_change(out, self.key, *self.value != before);
    }
}

/// Dropdown bound to the selected index. Event: `Changed`. `label` is the placeholder and the
/// accessible name.
///
/// ```
/// # use fluent2_egui::office::ribbon::RibbonDropdown;
/// # let mut index = 0;
/// let font = RibbonDropdown::new("font", "Font", &["Sans", "Serif"], &mut index);
/// ```
pub struct RibbonDropdown<'a> {
    key: &'static str,
    label: String,
    options: &'a [&'a str],
    selected: &'a mut usize,
}

impl<'a> RibbonDropdown<'a> {
    /// A dropdown over `options`, bound to `selected`.
    pub fn new(key: &'static str, label: &str, options: &'a [&'a str], selected: &'a mut usize) -> Self {
        Self {
            key,
            label: label.to_owned(),
            options,
            selected,
        }
    }

    fn show(self, ui: &mut Ui, out: &mut RibbonResponse) {
        let before = *self.selected;
        let mut current = Some(before);
        let width = field_width(ui, &self.label, self.options);
        ui.push_id(self.key, |ui| {
            ui.add(
                Dropdown::new(self.key, &mut current, self.options)
                    .placeholder(self.label.as_str())
                    .size(ButtonSize::Small)
                    .width(width),
            )
        });
        if let Some(index) = current {
            *self.selected = index;
        }
        report_change(out, self.key, *self.selected != before);
    }
}

/// Combobox bound to `text`. Picking an option copies it into `text`. Event: `Changed`.
///
/// ```
/// # use fluent2_egui::office::ribbon::RibbonCombobox;
/// # let mut size = String::new();
/// let sizes = RibbonCombobox::new("size", "Size", &["8", "10", "12"], &mut size);
/// ```
pub struct RibbonCombobox<'a> {
    key: &'static str,
    label: String,
    options: &'a [&'a str],
    text: &'a mut String,
}

impl<'a> RibbonCombobox<'a> {
    /// A combobox over `options`, bound to `text`.
    pub fn new(key: &'static str, label: &str, options: &'a [&'a str], text: &'a mut String) -> Self {
        Self {
            key,
            label: label.to_owned(),
            options,
            text,
        }
    }

    fn show(self, ui: &mut Ui, out: &mut RibbonResponse) {
        let before = self.text.clone();
        let width = field_width(ui, &self.label, self.options);
        ui.push_id(self.key, |ui| {
            ui.add(
                Combobox::new(self.key, &mut *self.text, self.options)
                    .placeholder(self.label.as_str())
                    .size(ButtonSize::Small)
                    .width(width),
            )
        });
        report_change(out, self.key, *self.text != before);
    }
}

/// Width of a dropdown or combobox at the Large step: its widest text (placeholder or option),
/// plus padding and the chevron. Without it a field takes all the width its parent offers.
fn field_width(ui: &Ui, label: &str, options: &[&str]) -> f32 {
    let chrome = &Theme::get(ui.ctx()).chrome;
    let pad = spacing_horizontal::S;
    let widest = std::iter::once(label)
        .chain(options.iter().copied())
        .map(|text| caption(ui, text, false, f32::INFINITY).size().x)
        .fold(0.0_f32, f32::max);
    widest + 2.0 * pad + chrome.icon_small + spacing_horizontal::XS
}

/// Dialog launcher at a group's bottom-right corner. Event: `Clicked`. It is set with
/// [`super::GroupBuilder::dialog_launcher`].
///
/// ```
/// # use fluent2_egui::office::ribbon::RibbonDialogLauncher;
/// let launcher = RibbonDialogLauncher::new("clipboard_dialog");
/// ```
pub struct RibbonDialogLauncher {
    pub(super) key: &'static str,
}

impl RibbonDialogLauncher {
    /// A launcher named `key`, unique within the ribbon.
    pub fn new(key: &'static str) -> Self {
        Self { key }
    }
}

/// The caller's draw closure of a [`RibbonCustom`].
type CustomAdd<'a> = Box<dyn FnMut(&mut Ui, ScalingStep) -> Response + 'a>;

/// Caller-drawn widget. `add` receives the scale step it must draw at and returns its response,
/// which appears in `RibbonResponse::custom`. The width is declared, not measured (spec §4.2).
///
/// ```
/// # use fluent2_egui::office::ribbon::RibbonCustom;
/// let slot = RibbonCustom::new("zoom", |ui, _step| ui.button("Zoom")).widths(96.0, 72.0, 48.0);
/// ```
pub struct RibbonCustom<'a> {
    key: &'static str,
    add: CustomAdd<'a>,
    widths: [f32; 3],
}

impl<'a> RibbonCustom<'a> {
    /// A custom widget. `key` names its response in `RibbonResponse::custom`.
    pub fn new(key: &'static str, add: impl FnMut(&mut Ui, ScalingStep) -> Response + 'a) -> Self {
        Self {
            key,
            add: Box::new(add),
            widths: [0.0; 3],
        }
    }

    /// Width in points at each step: Large, Medium, Small.
    #[must_use]
    pub fn widths(mut self, large: f32, medium: f32, small: f32) -> Self {
        self.widths = [large, medium, small];
        self
    }

    fn show(mut self, ui: &mut Ui, out: &mut RibbonResponse) {
        let width = self.widths[0].max(0.0);
        let response = ui
            .allocate_ui(Vec2::new(width, 0.0), |ui| (self.add)(ui, ScalingStep::Large))
            .inner;
        out.custom.push((self.key, response));
    }
}

/// One control of a group, of any kind. Build one with the kind's constructor and
/// [`super::GroupBuilder::add`].
pub struct RibbonControl<'a>(Kind<'a>);

enum Kind<'a> {
    Large(RibbonLarge<'a>),
    Small(RibbonSmall<'a>),
    Toggle(RibbonToggle<'a>),
    Split(RibbonSplit<'a>),
    Menu(RibbonMenu<'a>),
    Checkbox(RibbonCheckbox<'a>),
    Dropdown(RibbonDropdown<'a>),
    Combobox(RibbonCombobox<'a>),
    Custom(RibbonCustom<'a>),
}

macro_rules! into_control {
    ($($ty:ident => $kind:ident),* $(,)?) => {$(
        impl<'a> From<$ty<'a>> for RibbonControl<'a> {
            fn from(control: $ty<'a>) -> Self {
                Self(Kind::$kind(control))
            }
        }
    )*};
}

into_control!(
    RibbonLarge => Large,
    RibbonSmall => Small,
    RibbonToggle => Toggle,
    RibbonSplit => Split,
    RibbonMenu => Menu,
    RibbonCheckbox => Checkbox,
    RibbonDropdown => Dropdown,
    RibbonCombobox => Combobox,
    RibbonCustom => Custom,
);

impl RibbonControl<'_> {
    /// The control's key, for duplicate detection.
    pub(super) fn key(&self) -> &'static str {
        match &self.0 {
            Kind::Large(control) => control.key,
            Kind::Small(control) => control.key,
            Kind::Toggle(control) => control.key,
            Kind::Split(control) => control.key,
            Kind::Menu(control) => control.key,
            Kind::Checkbox(control) => control.key,
            Kind::Dropdown(control) => control.key,
            Kind::Combobox(control) => control.key,
            Kind::Custom(control) => control.key,
        }
    }

    /// Whether the control is small: it stacks in a column with its neighbours (spec §4.2).
    pub(super) fn is_small(&self) -> bool {
        match &self.0 {
            Kind::Small(_) | Kind::Toggle(_) | Kind::Checkbox(_) | Kind::Dropdown(_) | Kind::Combobox(_) => {
                true
            }
            Kind::Split(control) => control.small,
            Kind::Menu(control) => control.small,
            Kind::Large(_) | Kind::Custom(_) => false,
        }
    }

    /// Paint the control and report its events.
    pub(super) fn show(self, ui: &mut Ui, out: &mut RibbonResponse) {
        match self.0 {
            Kind::Large(control) => control.show(ui, out),
            Kind::Small(control) => control.show(ui, out),
            Kind::Toggle(control) => control.show(ui, out),
            Kind::Split(control) => control.show(ui, out),
            Kind::Menu(control) => control.show(ui, out),
            Kind::Checkbox(control) => control.show(ui, out),
            Kind::Dropdown(control) => control.show(ui, out),
            Kind::Combobox(control) => control.show(ui, out),
            Kind::Custom(control) => control.show(ui, out),
        }
    }
}
