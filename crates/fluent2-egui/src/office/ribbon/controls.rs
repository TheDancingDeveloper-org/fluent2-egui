//! Ribbon controls (spec §4.2). Each kind is a builder that converts into a [`RibbonControl`]
//! and paints itself in its group's column. It reports through [`RibbonResponse`] by its key.
//!
//! Disabled state is egui's: a control inside a disabled `Ui` paints disabled and reports
//! nothing.

use std::marker::PhantomData;
use std::sync::Arc;

use egui::{
    Galley, Id, Popup, Pos2, Rect, Response, Sense, TextStyle, TextWrapMode, Ui, Vec2, WidgetInfo,
    WidgetText, WidgetType, accesskit,
};
use fluent2_tokens::generated::global::{border_radius, spacing_horizontal};

use fluent2_tokens::Rgba;

use super::layout::ScalingStep;
use super::{RibbonEvent, RibbonEventKind, RibbonResponse, radius};
use crate::controls::button::Metrics as ButtonMetrics;
use crate::controls::split_button::{Metrics as SplitMetrics, natural_width};
use crate::controls::{ButtonSize, Checkbox, Combobox, Dropdown, MenuButton, SplitButton, ToggleButton};
use crate::foundation::focus::{self, paint_focus_ring};
use crate::foundation::icon::{IconName, IconSize, IconStyle, paint_icon};
use crate::foundation::text::TypeRamp;
use crate::office::keytips::{self, KeyTipCandidate, KeyTipLevel, TargetKind};
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

fn report_click(out: &mut RibbonResponse, key: &'static str, clicked: bool) {
    if clicked {
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

/// Register a control's KeyTip at the level of tab `tab` (spec §6.2). A control that opens a
/// popup passes its popup id and registers as a popup control. A disabled control, or one
/// painted outside a tab, registers nothing.
pub(super) fn register(
    ui: &Ui,
    tab: Option<&'static str>,
    key: &'static str,
    keytip: Option<&'static str>,
    label: &str,
    rect: Rect,
    popup: Option<Id>,
) {
    let Some(tab) = tab else {
        return;
    };
    if !ui.is_enabled() {
        return;
    }
    let kind = if popup.is_some() {
        TargetKind::PopupControl
    } else {
        TargetKind::Control
    };
    keytips::register_with_popup(
        ui.ctx(),
        KeyTipCandidate {
            target: key,
            level: KeyTipLevel::Tab(tab),
            kind,
            requested: keytip,
            label: label.to_owned(),
        },
        rect,
        popup,
    );
}

/// Whether KeyTips invoked `key` this frame. A disabled control is never invoked.
fn invoked(ui: &Ui, key: &str) -> bool {
    ui.is_enabled() && keytips::invoked(ui.ctx(), key)
}

/// Adds `.keytip(key)` to each control builder (spec §6.2).
macro_rules! keytip_method {
    ($($ty:ty),* $(,)?) => {$(
        impl $ty {
            /// The control's KeyTip (spec §6.2): one or two ASCII letters, or one digit. Without
            /// one, a key is assigned from the label.
            #[must_use]
            pub fn keytip(mut self, key: &'static str) -> Self {
                self.keytip = Some(key);
                self
            }
        }
    )*};
}

keytip_method!(
    RibbonLarge<'_>,
    RibbonSmall<'_>,
    RibbonToggle<'_>,
    RibbonSplit<'_>,
    RibbonMenu<'_>,
    RibbonCheckbox<'_>,
    RibbonDropdown<'_>,
    RibbonCombobox<'_>,
    RibbonCustom<'_>,
    RibbonDialogLauncher,
);

/// Large button: icon above a two-line label. Event: `Clicked`.
///
/// ```
/// # use fluent2_egui::office::ribbon::RibbonLarge;
/// # use fluent2_egui::IconName;
/// let paste = RibbonLarge::new("paste", IconName::ClipboardPaste, "Paste").shortcut("Ctrl+V");
/// ```
pub struct RibbonLarge<'a> {
    key: &'static str,
    keytip: Option<&'static str>,
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
            keytip: None,
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

    fn show(self, ui: &mut Ui, tab: Option<&'static str>, out: &mut RibbonResponse, step: ScalingStep) {
        let tip = tooltip(&self.label, self.shortcut.as_deref());
        let response = match step {
            ScalingStep::Large | ScalingStep::Collapsed => paint_large(ui, self.icon, &self.label),
            ScalingStep::Medium => paint_small(ui, self.icon, &self.label, true),
            ScalingStep::Small => paint_small(ui, self.icon, &self.label, false),
        };
        let response = response.fluent_tooltip(tip);
        register(ui, tab, self.key, self.keytip, &self.label, response.rect, None);
        report_click(out, self.key, response.clicked() || invoked(ui, self.key));
    }

    fn width(&self, ui: &Ui, step: ScalingStep) -> f32 {
        match step {
            ScalingStep::Large | ScalingStep::Collapsed => large_width(ui, &self.label),
            ScalingStep::Medium => small_width(ui, &self.label, true),
            ScalingStep::Small => small_width(ui, &self.label, false),
        }
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
    keytip: Option<&'static str>,
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
            keytip: None,
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

    fn show(self, ui: &mut Ui, tab: Option<&'static str>, out: &mut RibbonResponse, step: ScalingStep) {
        let tip = tooltip(&self.label, self.shortcut.as_deref());
        let response = paint_small(ui, self.icon, &self.label, step != ScalingStep::Small);
        let response = response.fluent_tooltip(tip);
        register(ui, tab, self.key, self.keytip, &self.label, response.rect, None);
        report_click(out, self.key, response.clicked() || invoked(ui, self.key));
    }

    fn width(&self, ui: &Ui, step: ScalingStep) -> f32 {
        small_width(ui, &self.label, step != ScalingStep::Small)
    }
}

/// Width of a large button: the wrapped label or the 32 pt icon slot, plus padding (§5.4).
fn large_width(ui: &Ui, label: &str) -> f32 {
    let chrome = &Theme::get(ui.ctx()).chrome;
    let text = caption(ui, label, true, chrome.large_label_max);
    text.size().x.max(chrome.icon_large) + 2.0 * spacing_horizontal::XS
}

/// Width of a small button, with or without its label (§5.4).
fn small_width(ui: &Ui, label: &str, with_label: bool) -> f32 {
    let icon = Theme::get(ui.ctx()).chrome.icon_small;
    let text = if with_label {
        spacing_horizontal::XS + caption(ui, label, false, f32::INFINITY).size().x
    } else {
        0.0
    };
    2.0 * spacing_horizontal::S + icon + text
}

/// The fill of a subtle ribbon button in its current state.
fn subtle_fill(theme: &Theme, enabled: bool, response: &Response) -> Option<Rgba> {
    let color = &theme.color;
    if !enabled {
        None
    } else if response.is_pointer_button_down_on() {
        Some(color.subtle_background_pressed)
    } else if response.hovered() {
        Some(color.subtle_background_hover)
    } else {
        None
    }
}

/// Paint a large button: a 32 pt icon slot above a label of up to two lines (§5.1).
fn paint_large(ui: &mut Ui, icon_name: IconName, label: &str) -> Response {
    let theme = Theme::get(ui.ctx());
    let enabled = ui.is_enabled();
    let chrome = &theme.chrome;
    let pad = spacing_horizontal::XS;
    let icon = chrome.icon_large;
    let text = caption(ui, label, true, chrome.large_label_max);
    let width = large_width(ui, label);
    let height = icon + 3.0 * pad + 2.0 * TypeRamp::Caption1.line_height();

    let (rect, response) = ui.allocate_exact_size(Vec2::new(width, height), Sense::click());
    let corner = radius(border_radius::MEDIUM);
    if let Some(fill) = subtle_fill(&theme, enabled, &response) {
        ui.painter().rect_filled(rect, corner, c(fill));
    }
    let ink = ink(&theme, enabled);
    // The 32 pt slot holds the 24 pt icon, centred.
    let slot = Rect::from_min_size(
        Pos2::new(rect.center().x - icon / 2.0, rect.top() + pad),
        Vec2::splat(icon),
    );
    paint_icon(
        ui,
        slot,
        icon_name,
        IconSize::S24,
        IconStyle::Regular,
        Some(c(ink)),
    );
    let origin = Pos2::new(rect.center().x - text.size().x / 2.0, slot.bottom() + pad);
    ui.painter().galley(origin, Arc::clone(&text), c(ink));
    finish_button(ui, &response, rect, label, enabled);
    response
}

/// Paint a small button, 24 pt high: a 16 pt icon and, when `with_label`, the label beside it.
/// Without the label it stays in the accessible name and the tooltip (§5.1 `Small`).
fn paint_small(ui: &mut Ui, icon_name: IconName, label: &str, with_label: bool) -> Response {
    let theme = Theme::get(ui.ctx());
    let enabled = ui.is_enabled();
    let pad = spacing_horizontal::S;
    let gap = spacing_horizontal::XS;
    let icon = theme.chrome.icon_small;
    let width = small_width(ui, label, with_label);
    let height = icon + 2.0 * gap;

    let (rect, response) = ui.allocate_exact_size(Vec2::new(width, height), Sense::click());
    let corner = radius(border_radius::MEDIUM);
    if let Some(fill) = subtle_fill(&theme, enabled, &response) {
        ui.painter().rect_filled(rect, corner, c(fill));
    }
    let ink = ink(&theme, enabled);
    let slot = Rect::from_min_size(rect.min + Vec2::new(pad, gap), Vec2::splat(icon));
    paint_icon(
        ui,
        slot,
        icon_name,
        IconSize::S16,
        IconStyle::Regular,
        Some(c(ink)),
    );
    if with_label {
        let text = caption(ui, label, false, f32::INFINITY);
        let origin = Pos2::new(slot.right() + gap, rect.center().y - text.size().y / 2.0);
        ui.painter().galley(origin, text, c(ink));
    }
    finish_button(ui, &response, rect, label, enabled);
    response
}

fn ink(theme: &Theme, enabled: bool) -> Rgba {
    if enabled {
        theme.color.neutral_foreground_1
    } else {
        theme.color.neutral_foreground_disabled
    }
}

/// Focus ring and accessible name of a painted button.
fn finish_button(ui: &Ui, response: &Response, rect: Rect, label: &str, enabled: bool) {
    if focus::focus_visible(ui, response) {
        paint_focus_ring(ui, rect, radius(border_radius::MEDIUM));
    }
    response.widget_info(|| WidgetInfo::labeled(WidgetType::Button, enabled, label));
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
    keytip: Option<&'static str>,
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
            keytip: None,
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

    fn show(self, ui: &mut Ui, tab: Option<&'static str>, out: &mut RibbonResponse, step: ScalingStep) {
        let before = *self.value;
        let tip = tooltip(&self.label, self.shortcut.as_deref());
        // A KeyTip presses the toggle, as a click does.
        if invoked(ui, self.key) {
            *self.value = !*self.value;
        }
        let response = ui.push_id(self.key, |ui| {
            let toggle = if step == ScalingStep::Small {
                ToggleButton::icon_only(&mut *self.value, self.icon, self.label.as_str())
            } else {
                ToggleButton::new(&mut *self.value, self.label.as_str()).icon(self.icon)
            };
            ui.add(toggle.size(ButtonSize::Small)).fluent_tooltip(tip)
        });
        register(
            ui,
            tab,
            self.key,
            self.keytip,
            &self.label,
            response.inner.rect,
            None,
        );
        report_change(out, self.key, *self.value != before);
    }

    /// The `ToggleButton` small width: padding, icon, gap and label, at least `min_width`.
    fn width(&self, ui: &Ui, step: ScalingStep) -> f32 {
        let metrics = ButtonMetrics::of(ButtonSize::Small);
        if step == ScalingStep::Small {
            return (metrics.icon + 2.0 * metrics.h_padding).max(metrics.min_height);
        }
        let text = caption(ui, &self.label, false, f32::INFINITY).size().x;
        (text + metrics.icon + metrics.icon_gap + 2.0 * metrics.h_padding).max(metrics.min_width)
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
    keytip: Option<&'static str>,
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
            keytip: None,
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

    fn show(self, ui: &mut Ui, tab: Option<&'static str>, out: &mut RibbonResponse, step: ScalingStep) {
        let arrow_label = format!("{} menu", self.label);
        let text = if step == ScalingStep::Small {
            ""
        } else {
            self.label.as_str()
        };
        let split = SplitButton::new(text)
            .icon(self.icon)
            .size(sizes(self.small, step))
            .id_salt(self.key);
        let response = split.show(ui, self.menu);
        if step == ScalingStep::Small {
            // The label moved into the tooltip; the main action keeps it as its name (§5.1).
            ui.ctx().accesskit_node_builder(response.main.id, |node| {
                node.set_label(self.label.clone());
            });
        }
        // The WI-1544 arrow is labelled "<label> options" and has no popup role. Spec §9.1 says
        // "<label> menu" with `has_popup`, so the ribbon sets both on the arrow's node.
        ui.ctx().accesskit_node_builder(response.menu.id, |node| {
            node.set_label(arrow_label);
            node.set_has_popup(accesskit::HasPopup::Menu);
        });
        // The KeyTip opens the menu, as the chevron does (§6.3 `OpenPopup`).
        let popup = Popup::default_response_id(&response.menu);
        if ui.is_enabled() && keytips::popup_opened(ui.ctx(), self.key) {
            Popup::open_id(ui.ctx(), popup);
            ui.ctx().request_repaint();
        }
        let rect = response.main.rect.union(response.menu.rect);
        register(ui, tab, self.key, self.keytip, &self.label, rect, Some(popup));
        let main = response
            .main
            .fluent_tooltip(tooltip(&self.label, self.shortcut.as_deref()));
        report_click(out, self.key, main.clicked());
    }
}

impl RibbonSplit<'_> {
    fn width(&self, ui: &Ui, step: ScalingStep) -> f32 {
        let size = sizes(self.small, step);
        let label = if step == ScalingStep::Small {
            ""
        } else {
            self.label.as_str()
        };
        natural_width(ui, &WidgetText::from(label), true, SplitMetrics::of(size)) + chevron_box(size)
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
    keytip: Option<&'static str>,
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
            keytip: None,
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

    fn show(self, ui: &mut Ui, tab: Option<&'static str>, out: &mut RibbonResponse, step: ScalingStep) {
        let text = if step == ScalingStep::Small {
            ""
        } else {
            self.label.as_str()
        };
        let (response, _) = MenuButton::new(text)
            .icon(self.icon)
            .size(sizes(self.small, step))
            .id_salt(self.key)
            .show(ui, self.menu);
        // Spec §9.1: a menu button is a button with `has_popup`. At `Small` the label moves
        // into the tooltip and stays the accessible name.
        let small = step == ScalingStep::Small;
        ui.ctx().accesskit_node_builder(response.id, |node| {
            node.set_has_popup(accesskit::HasPopup::Menu);
            if small {
                node.set_label(self.label.clone());
            }
        });
        // The KeyTip opens the menu and reports `Clicked`, as pressing the button does.
        let popup = Popup::default_response_id(&response);
        let keyed = ui.is_enabled() && keytips::popup_opened(ui.ctx(), self.key);
        if keyed {
            Popup::open_id(ui.ctx(), popup);
            ui.ctx().request_repaint();
        }
        register(
            ui,
            tab,
            self.key,
            self.keytip,
            &self.label,
            response.rect,
            Some(popup),
        );
        let response = response.fluent_tooltip(tooltip(&self.label, self.shortcut.as_deref()));
        report_click(out, self.key, response.clicked() || keyed);
    }
}

impl RibbonMenu<'_> {
    fn width(&self, ui: &Ui, step: ScalingStep) -> f32 {
        let size = sizes(self.small, step);
        let label = if step == ScalingStep::Small {
            ""
        } else {
            self.label.as_str()
        };
        let metrics = SplitMetrics::of(size);
        natural_width(ui, &WidgetText::from(label), true, metrics) + metrics.icon + metrics.icon_gap
    }
}

/// The button size of a split or menu button: large only at the `Large` step (§5.1).
fn sizes(small: bool, step: ScalingStep) -> ButtonSize {
    if small || !matches!(step, ScalingStep::Large | ScalingStep::Collapsed) {
        ButtonSize::Small
    } else {
        ButtonSize::Large
    }
}

/// The chevron region of a split button: the icon box of its size (`SplitButton` metrics).
fn chevron_box(size: ButtonSize) -> f32 {
    match size {
        ButtonSize::Small => 24.0,
        ButtonSize::Medium => 32.0,
        ButtonSize::Large => 40.0,
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
    keytip: Option<&'static str>,
    label: String,
    value: &'a mut bool,
}

impl<'a> RibbonCheckbox<'a> {
    /// A checkbox bound to `value`. `key` names it in events and must be unique within the ribbon.
    pub fn new(key: &'static str, label: &str, value: &'a mut bool) -> Self {
        Self {
            key,
            keytip: None,
            label: label.to_owned(),
            value,
        }
    }

    fn show(self, ui: &mut Ui, tab: Option<&'static str>, out: &mut RibbonResponse, step: ScalingStep) {
        let before = *self.value;
        if invoked(ui, self.key) {
            *self.value = !*self.value;
        }
        let label = self.label.clone();
        let response = ui.push_id(self.key, |ui| {
            if step == ScalingStep::Small {
                // Checkbox only; the label moves into the tooltip and the accessible name.
                let response = ui
                    .add(Checkbox::bool(&mut *self.value, ""))
                    .fluent_tooltip(label.as_str());
                ui.ctx()
                    .accesskit_node_builder(response.id, |node| node.set_label(label));
                response
            } else {
                ui.add(Checkbox::bool(&mut *self.value, label.as_str()))
            }
        });
        register(
            ui,
            tab,
            self.key,
            self.keytip,
            &self.label,
            response.inner.rect,
            None,
        );
        report_change(out, self.key, *self.value != before);
    }

    /// Box, gap and `Body1` label, as `Checkbox` lays them out.
    fn width(&self, ui: &Ui, step: ScalingStep) -> f32 {
        let label = if step == ScalingStep::Small {
            0.0
        } else {
            WidgetText::from(self.label.as_str())
                .into_galley(
                    ui,
                    Some(TextWrapMode::Extend),
                    f32::INFINITY,
                    TextStyle::Name(TypeRamp::Body1.name().into()),
                )
                .size()
                .x
        };
        16.0 + spacing_horizontal::S + label
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
    keytip: Option<&'static str>,
    label: String,
    options: &'a [&'a str],
    selected: &'a mut usize,
}

impl<'a> RibbonDropdown<'a> {
    /// A dropdown over `options`, bound to `selected`.
    pub fn new(key: &'static str, label: &str, options: &'a [&'a str], selected: &'a mut usize) -> Self {
        Self {
            key,
            keytip: None,
            label: label.to_owned(),
            options,
            selected,
        }
    }

    fn show(self, ui: &mut Ui, tab: Option<&'static str>, out: &mut RibbonResponse, step: ScalingStep) {
        let before = *self.selected;
        let mut current = Some(before);
        let width = field_width(step);
        let response = ui.push_id(self.key, |ui| {
            ui.add(
                Dropdown::new(self.key, &mut current, self.options)
                    .placeholder(self.label.as_str())
                    .size(ButtonSize::Small)
                    .width(width),
            )
        });
        // A KeyTip moves focus to the field, where typing or the arrow keys pick a value.
        if invoked(ui, self.key) {
            response.inner.request_focus();
        }
        register(
            ui,
            tab,
            self.key,
            self.keytip,
            &self.label,
            response.inner.rect,
            None,
        );
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
    keytip: Option<&'static str>,
    label: String,
    options: &'a [&'a str],
    text: &'a mut String,
}

impl<'a> RibbonCombobox<'a> {
    /// A combobox over `options`, bound to `text`.
    pub fn new(key: &'static str, label: &str, options: &'a [&'a str], text: &'a mut String) -> Self {
        Self {
            key,
            keytip: None,
            label: label.to_owned(),
            options,
            text,
        }
    }

    fn show(self, ui: &mut Ui, tab: Option<&'static str>, out: &mut RibbonResponse, step: ScalingStep) {
        let before = self.text.clone();
        let width = field_width(step);
        let response = ui.push_id(self.key, |ui| {
            ui.add(
                Combobox::new(self.key, &mut *self.text, self.options)
                    .placeholder(self.label.as_str())
                    .size(ButtonSize::Small)
                    .width(width),
            )
        });
        // A KeyTip moves focus to the field, where typing or the arrow keys pick a value.
        if invoked(ui, self.key) {
            response.inner.request_focus();
        }
        register(
            ui,
            tab,
            self.key,
            self.keytip,
            &self.label,
            response.inner.rect,
            None,
        );
        report_change(out, self.key, *self.text != before);
    }
}

/// Width of a dropdown or combobox: 120, 96 or 64 pt by step (spec §5.1).
fn field_width(step: ScalingStep) -> f32 {
    match step {
        ScalingStep::Large | ScalingStep::Collapsed => 120.0,
        ScalingStep::Medium => 96.0,
        ScalingStep::Small => 64.0,
    }
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
    pub(super) keytip: Option<&'static str>,
}

impl RibbonDialogLauncher {
    /// A launcher named `key`, unique within the ribbon.
    pub fn new(key: &'static str) -> Self {
        Self { key, keytip: None }
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
    keytip: Option<&'static str>,
    add: CustomAdd<'a>,
    widths: [f32; 3],
}

impl RibbonCustom<'_> {
    /// The declared width at `step`, never measured. Non-finite or negative is 0.
    fn width(&self, step: ScalingStep) -> f32 {
        let width = match step {
            ScalingStep::Large | ScalingStep::Collapsed => self.widths[0],
            ScalingStep::Medium => self.widths[1],
            ScalingStep::Small => self.widths[2],
        };
        if width.is_finite() { width.max(0.0) } else { 0.0 }
    }
}

impl<'a> RibbonCustom<'a> {
    /// A custom widget. `key` names its response in `RibbonResponse::custom`.
    pub fn new(key: &'static str, add: impl FnMut(&mut Ui, ScalingStep) -> Response + 'a) -> Self {
        Self {
            key,
            keytip: None,
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

    fn show(mut self, ui: &mut Ui, tab: Option<&'static str>, out: &mut RibbonResponse, step: ScalingStep) {
        let width = self.width(step);
        let response = ui
            .allocate_ui(Vec2::new(width, 0.0), |ui| {
                ui.set_min_width(width);
                (self.add)(ui, step)
            })
            .inner;
        // A KeyTip focuses the caller's widget; the caller handles its keys.
        if invoked(ui, self.key) {
            response.request_focus();
        }
        register(ui, tab, self.key, self.keytip, self.key, response.rect, None);
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

    /// Whether the control stacks in a column with its neighbours at `step` (spec §4.2, §5.1).
    /// Below `Large` every control but a custom one renders small.
    pub(super) fn is_small(&self, step: ScalingStep) -> bool {
        if matches!(step, ScalingStep::Medium | ScalingStep::Small) {
            return !matches!(self.0, Kind::Custom(_));
        }
        match &self.0 {
            Kind::Small(_) | Kind::Toggle(_) | Kind::Checkbox(_) | Kind::Dropdown(_) | Kind::Combobox(_) => {
                true
            }
            Kind::Split(control) => control.small,
            Kind::Menu(control) => control.small,
            Kind::Large(_) | Kind::Custom(_) => false,
        }
    }

    /// The control's icon, if it has one: the default icon of a collapsed group (spec §5.1).
    pub(super) fn icon(&self) -> Option<IconName> {
        match &self.0 {
            Kind::Large(control) => Some(control.icon),
            Kind::Small(control) => Some(control.icon),
            Kind::Toggle(control) => Some(control.icon),
            Kind::Split(control) => Some(control.icon),
            Kind::Menu(control) => Some(control.icon),
            Kind::Checkbox(_) | Kind::Dropdown(_) | Kind::Combobox(_) | Kind::Custom(_) => None,
        }
    }

    /// Width in points at `step` (spec §5.1, §5.4). `Collapsed` measures as `Large`.
    pub(super) fn width(&self, ui: &Ui, step: ScalingStep) -> f32 {
        match &self.0 {
            Kind::Large(control) => control.width(ui, step),
            Kind::Small(control) => control.width(ui, step),
            Kind::Toggle(control) => control.width(ui, step),
            Kind::Split(control) => control.width(ui, step),
            Kind::Menu(control) => control.width(ui, step),
            Kind::Checkbox(control) => control.width(ui, step),
            Kind::Dropdown(_) | Kind::Combobox(_) => field_width(step),
            Kind::Custom(control) => control.width(step),
        }
    }

    /// Feed what the control's widths depend on into `state`, for the width memo (§5.4).
    pub(super) fn hash_content(&self, state: &mut impl std::hash::Hasher) {
        use std::hash::Hash;
        self.key().hash(state);
        self.is_small(ScalingStep::Large).hash(state);
        match &self.0 {
            Kind::Large(control) => (0u8, &control.label).hash(state),
            Kind::Small(control) => (1u8, &control.label).hash(state),
            Kind::Toggle(control) => (2u8, &control.label).hash(state),
            Kind::Split(control) => (3u8, &control.label).hash(state),
            Kind::Menu(control) => (4u8, &control.label).hash(state),
            Kind::Checkbox(control) => (5u8, &control.label).hash(state),
            Kind::Dropdown(_) => 6u8.hash(state),
            Kind::Combobox(_) => 7u8.hash(state),
            Kind::Custom(control) => {
                8u8.hash(state);
                for width in control.widths {
                    width.to_bits().hash(state);
                }
            }
        }
    }

    /// Paint the control at `step` and report its events.
    pub(super) fn show(
        self,
        ui: &mut Ui,
        tab: Option<&'static str>,
        out: &mut RibbonResponse,
        step: ScalingStep,
    ) {
        match self.0 {
            Kind::Large(control) => control.show(ui, tab, out, step),
            Kind::Small(control) => control.show(ui, tab, out, step),
            Kind::Toggle(control) => control.show(ui, tab, out, step),
            Kind::Split(control) => control.show(ui, tab, out, step),
            Kind::Menu(control) => control.show(ui, tab, out, step),
            Kind::Checkbox(control) => control.show(ui, tab, out, step),
            Kind::Dropdown(control) => control.show(ui, tab, out, step),
            Kind::Combobox(control) => control.show(ui, tab, out, step),
            Kind::Custom(control) => control.show(ui, tab, out, step),
        }
    }
}
