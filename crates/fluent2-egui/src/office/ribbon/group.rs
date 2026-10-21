//! Ribbon groups (spec §4.1, §4.2): a labelled run of controls with an optional dialog launcher
//! in the bottom-right corner, drawn at the scale step the layout planner picks (spec §5).

use std::hash::{Hash, Hasher};
use std::sync::Arc;

use egui::{
    Galley, Id, Popup, PopupCloseBehavior, Pos2, Rect, Sense, TextStyle, Ui, UiKind, UiStackInfo, Vec2,
    WidgetText, accesskit,
};
use fluent2_tokens::generated::global::spacing_horizontal;

use super::controls::{RibbonControl, RibbonDialogLauncher, caption, register};
use super::layout::{GroupMeasure, LayoutMetrics, ScalingStep, ScalingSteps, cached_measure, plan_tab};
use super::{RibbonEvent, RibbonEventKind, RibbonResponse, body_node, popup_frame, radius};
use crate::controls::{Appearance, Button, ButtonSize};
use crate::foundation::focus::{self, paint_focus_ring};
use crate::foundation::icon::{IconName, IconSize, IconStyle, paint_icon};
use crate::office::keytips;
use crate::surfaces::{TooltipExt, menu_frame, submenu};
use crate::theme::{Theme, c};

/// Controls stacked in one column of a group, at most (spec §5.1 small rendering).
const SMALL_ROWS: usize = 3;

/// A group under construction. Built by [`super::TabBuilder::group`].
///
/// ```
/// # use fluent2_egui::office::ribbon::{GroupBuilder, RibbonLarge};
/// # use fluent2_egui::IconName;
/// # fn demo(group: &mut GroupBuilder<'_>) {
/// group.add(RibbonLarge::new("paste", IconName::ClipboardPaste, "Paste"));
/// # }
/// ```
pub struct GroupBuilder<'a> {
    key: &'static str,
    label: String,
    controls: Vec<RibbonControl<'a>>,
    launcher: Option<RibbonDialogLauncher>,
    steps: ScalingSteps,
    icon: Option<IconName>,
}

impl<'a> GroupBuilder<'a> {
    pub(super) fn new(key: &'static str, label: &str) -> Self {
        Self {
            key,
            label: label.to_owned(),
            controls: Vec::new(),
            launcher: None,
            steps: ScalingSteps::all(),
            icon: None,
        }
    }

    /// Which scale steps this group may use (spec §5.1). Default: all four.
    pub fn steps(&mut self, steps: ScalingSteps) {
        self.steps = steps;
    }

    /// Icon of the collapsed button (spec §5.1). Default: the first control's icon.
    pub fn icon(&mut self, icon: IconName) {
        self.icon = Some(icon);
    }

    /// Add a control. Controls appear in the order they are added.
    pub fn add(&mut self, control: impl Into<RibbonControl<'a>>) {
        self.controls.push(control.into());
    }

    /// The dialog launcher at the group's bottom-right corner (spec §4.2). A second call
    /// replaces the first.
    pub fn dialog_launcher(&mut self, launcher: RibbonDialogLauncher) {
        self.launcher = Some(launcher);
    }

    pub(super) fn finish(self) -> Group<'a> {
        Group {
            key: self.key,
            label: self.label,
            controls: self.controls,
            launcher: self.launcher,
            steps: self.steps,
            icon: self.icon,
        }
    }
}

/// A built group: what one frame paints.
pub(super) struct Group<'a> {
    key: &'static str,
    label: String,
    controls: Vec<RibbonControl<'a>>,
    launcher: Option<RibbonDialogLauncher>,
    steps: ScalingSteps,
    icon: Option<IconName>,
}

impl Group<'_> {
    /// Drop every control and launcher whose key is already in `seen`, reporting it in
    /// `conflicts`. The first occurrence keeps its control (D4). Keys are added to `seen`.
    pub(super) fn claim_keys(&mut self, seen: &mut Vec<&'static str>, conflicts: &mut Vec<&'static str>) {
        self.controls
            .retain(|control| claim(seen, conflicts, control.key()));
        if let Some(launcher) = &self.launcher
            && !claim(seen, conflicts, launcher.key)
        {
            self.launcher = None;
        }
    }

    /// The collapsed button's icon: the group's own, else the first control's (spec §5.1).
    fn collapsed_icon(&self) -> IconName {
        self.icon
            .or_else(|| self.controls.iter().find_map(RibbonControl::icon))
            .unwrap_or(IconName::MoreHorizontal)
    }

    /// Hash of what the widths depend on: keys, labels and allowed steps (spec §5.4).
    fn content_hash(&self) -> u64 {
        let mut hasher = std::hash::DefaultHasher::new();
        (self.key, &self.label, &self.steps, self.launcher.is_some()).hash(&mut hasher);
        for control in &self.controls {
            control.hash_content(&mut hasher);
        }
        hasher.finish()
    }

    /// The group's width at each step (spec §5.1, §5.4).
    fn measure(&self, ui: &Ui) -> GroupMeasure {
        let chrome = &Theme::get(ui.ctx()).chrome;
        let pad = spacing_horizontal::XS;
        let text = caption(ui, &self.label, false, f32::INFINITY).size().x;
        let launch = if self.launcher.is_some() {
            chrome.icon_small + 3.0 * pad
        } else {
            0.0
        };
        let label_row = text + launch;
        let at = |step: ScalingStep| {
            let widths = column_widths(ui, &self.controls, step);
            let controls = widths.iter().sum::<f32>() + pad * widths.len().saturating_sub(1) as f32;
            controls.max(label_row)
        };
        GroupMeasure::new(
            &self.steps,
            [
                at(ScalingStep::Large),
                at(ScalingStep::Medium),
                at(ScalingStep::Small),
                chrome.collapsed_button.max(text),
            ],
        )
    }

    /// Paint the group at `step`: its columns of controls, then its label row with the
    /// launcher. `Collapsed` paints the collapsed button, which opens the group in a popup.
    ///
    /// `tab` is the active tab's key: controls register their KeyTips at that tab's level.
    fn show(self, ui: &mut Ui, tab: Option<&'static str>, out: &mut RibbonResponse, step: ScalingStep) {
        if step == ScalingStep::Collapsed {
            self.show_collapsed(ui, tab, out);
            return;
        }
        let Self {
            key,
            label,
            controls,
            launcher,
            ..
        } = self;
        let theme = Theme::get(ui.ctx());
        let pad = spacing_horizontal::XS;

        ui.push_id(key, |ui| {
            let controls_rect = ui
                .horizontal_top(|ui| {
                    ui.spacing_mut().item_spacing.x = pad;
                    for column in columns(controls, step) {
                        ui.vertical(|ui| {
                            ui.spacing_mut().item_spacing.y = pad;
                            for control in column {
                                control.show(ui, tab, out, step);
                            }
                        });
                    }
                })
                .response
                .rect;
            label_row(ui, &label, controls_rect.width(), launcher, tab, out, &theme);
            group_node(ui, &label);
        });
    }

    /// The collapsed button: the group icon over a chevron, then the label row. A click opens
    /// the whole group at its widest step in a popup (spec §5.1).
    fn show_collapsed(self, ui: &mut Ui, tab: Option<&'static str>, out: &mut RibbonResponse) {
        let theme = Theme::get(ui.ctx());
        let icon = self.collapsed_icon();
        let widest = self
            .steps
            .as_slice()
            .first()
            .copied()
            .filter(|step| *step != ScalingStep::Collapsed)
            .unwrap_or(ScalingStep::Large);
        let key = self.key;
        let label = self.label.clone();
        ui.push_id(key, |ui| {
            let response = collapsed_button(ui, icon, &label, &theme);
            label_row(ui, &label, response.rect.width(), None, tab, out, &theme);
            group_node(ui, &label);
            Popup::from_toggle_button_response(&response)
                .close_behavior(PopupCloseBehavior::CloseOnClickOutside)
                .frame(popup_frame(&theme))
                .show(|ui| {
                    let mut group = self;
                    group.steps = ScalingSteps::new(&[widest]);
                    group.show(ui, tab, out, widest);
                });
        });
    }
}

/// The 36 pt collapsed button: the group icon, a chevron under it, and the group label as its
/// accessible name and tooltip.
fn collapsed_button(ui: &mut Ui, icon: IconName, label: &str, theme: &Theme) -> egui::Response {
    let chrome = &theme.chrome;
    let pad = spacing_horizontal::XS;
    let size = Vec2::new(
        chrome.collapsed_button,
        chrome.icon_large + 3.0 * pad + 2.0 * crate::foundation::text::TypeRamp::Caption1.line_height(),
    );
    let (rect, response) = ui.allocate_exact_size(size, Sense::click());
    let enabled = ui.is_enabled();
    let color = &theme.color;
    let corner = radius(fluent2_tokens::generated::global::border_radius::MEDIUM);
    let fill = if !enabled {
        None
    } else if response.is_pointer_button_down_on() {
        Some(color.subtle_background_pressed)
    } else if response.hovered() {
        Some(color.subtle_background_hover)
    } else {
        None
    };
    if let Some(fill) = fill {
        ui.painter().rect_filled(rect, corner, c(fill));
    }
    let ink = if !enabled {
        color.neutral_foreground_disabled
    } else if response.is_pointer_button_down_on() {
        color.neutral_foreground_1_pressed
    } else if response.hovered() {
        color.neutral_foreground_1_hover
    } else {
        color.neutral_foreground_1
    };
    let slot = Rect::from_center_size(
        Pos2::new(rect.center().x, rect.top() + pad + chrome.icon_large / 2.0),
        Vec2::splat(chrome.icon_large),
    );
    paint_icon(ui, slot, icon, IconSize::S24, IconStyle::Regular, Some(c(ink)));
    let chevron = Rect::from_center_size(
        Pos2::new(rect.center().x, slot.bottom() + pad + chrome.icon_small / 2.0),
        Vec2::splat(chrome.icon_small),
    );
    paint_icon(
        ui,
        chevron,
        IconName::ChevronDown,
        IconSize::S16,
        IconStyle::Regular,
        Some(c(ink)),
    );
    if focus::focus_visible(ui, &response) {
        paint_focus_ring(ui, rect, corner);
    }
    response.widget_info(|| egui::WidgetInfo::labeled(egui::Role::Button, enabled, label));
    ui.ctx().accesskit_node_builder(response.id, |node| {
        node.set_has_popup(accesskit::HasPopup::Dialog);
    });
    response.fluent_tooltip(label)
}

/// The label row under a group's controls, at least `width` wide, with the launcher.
fn label_row(
    ui: &mut Ui,
    label: &str,
    width: f32,
    launcher: Option<RibbonDialogLauncher>,
    tab: Option<&'static str>,
    out: &mut RibbonResponse,
    theme: &Theme,
) {
    let pad = spacing_horizontal::XS;
    let text = caption(ui, label, false, f32::INFINITY);
    let launch = theme.chrome.icon_small + 2.0 * pad;
    let row_height = text.size().y.max(launcher.as_ref().map_or(0.0, |_| launch));
    let (row, _) = ui.allocate_exact_size(Vec2::new(width.max(text.size().x), row_height), Sense::hover());
    paint_label(ui, row, &text, theme);
    if let Some(launcher) = launcher {
        launcher.show(ui, label, row, launch, tab, out);
    }
}

fn group_node(ui: &Ui, label: &str) {
    let label = label.to_owned();
    ui.ctx().accesskit_node_builder(ui.id().with("group"), |node| {
        node.set_role(accesskit::Role::Group);
        node.set_label(label);
    });
}

/// Paint the active tab's body: the groups in a row at the steps the planner picks for the
/// available width, then the `…` button for any overflowed groups (spec §4, §5, §10).
///
/// `tab` is the active tab's key: its controls register their KeyTips at that tab's level.
/// Returns the body's rectangle.
pub(super) fn paint_body(
    ui: &mut Ui,
    id: Id,
    label: Option<&str>,
    tab: Option<&'static str>,
    groups: Vec<Group<'_>>,
    available: f32,
    out: &mut RibbonResponse,
) -> Rect {
    body_node(ui.ctx(), id, label);
    // Measuring lays out button text, so the type ramp must be bound first, as the buttons do.
    crate::controls::button::ensure_type_ramp(ui);
    let chrome = Theme::get(ui.ctx()).chrome;
    let tab_key = tab.unwrap_or("");
    // Fonts that finish loading change text widths without changing any label, so a probe
    // width in the strong ramp joins the memo's content hash.
    let fonts = WidgetText::from("W")
        .into_galley(
            ui,
            None,
            f32::INFINITY,
            TextStyle::Name(crate::foundation::text::TypeRamp::Subtitle2.name().into()),
        )
        .size()
        .x
        .to_bits();
    let measures: Vec<GroupMeasure> = groups
        .iter()
        .map(|group| {
            cached_measure(
                ui.ctx(),
                id,
                tab_key,
                group.key,
                (group.content_hash(), fonts),
                || group.measure(ui),
            )
        })
        .collect();
    let metrics = LayoutMetrics {
        separator: chrome.group_separator,
        hysteresis: chrome.hysteresis,
        overflow_button: chrome.collapsed_button,
    };
    let plan = plan_tab(ui.ctx(), id, tab_key, &measures, available, metrics);
    let visible = plan.visible();
    let steps = plan.steps().to_vec();

    ui.vertical(|ui| {
        ui.set_min_height(chrome.ribbon_body_height);
        ui.horizontal_top(|ui| {
            ui.spacing_mut().item_spacing.x = chrome.group_separator;
            let mut groups = groups.into_iter();
            for (group, step) in groups.by_ref().take(visible).zip(steps) {
                // Each group stacks its label row under its controls.
                ui.vertical(|ui| group.show(ui, tab, out, step));
            }
            let overflow: Vec<Group<'_>> = groups.collect();
            if !overflow.is_empty() {
                overflow_button(ui, overflow, tab, out);
            }
        });
    })
    .response
    .rect
}

/// The trailing `…` button. Its menu lists the overflowed groups as sub-menus, each holding
/// that group at its widest step (spec §5.2).
fn overflow_button(ui: &mut Ui, groups: Vec<Group<'_>>, tab: Option<&'static str>, out: &mut RibbonResponse) {
    let theme = Theme::get(ui.ctx());
    let response = collapsed_button(ui, IconName::MoreHorizontal, "More groups", &theme);
    ui.ctx().accesskit_node_builder(response.id, |node| {
        node.set_has_popup(accesskit::HasPopup::Menu);
    });
    Popup::menu(&response)
        .close_behavior(PopupCloseBehavior::CloseOnClickOutside)
        .frame(menu_frame(ui))
        .info(UiStackInfo::new(UiKind::Menu))
        .show(|ui| {
            for group in groups {
                let label = group.label.clone();
                let icon = group.collapsed_icon();
                submenu(ui, label, Some(icon), |ui| {
                    ui.vertical(|ui| group.show(ui, tab, out, ScalingStep::Large));
                });
            }
        });
}

/// Keep `key` if it is new, otherwise report it and return false.
fn claim(seen: &mut Vec<&'static str>, conflicts: &mut Vec<&'static str>, key: &'static str) -> bool {
    if seen.contains(&key) {
        conflicts.push(key);
        false
    } else {
        seen.push(key);
        true
    }
}

/// Split controls into columns. Small controls stack up to [`SMALL_ROWS`] to a column; every
/// other control takes a column of its own (spec §4.2).
fn columns(controls: Vec<RibbonControl<'_>>, step: ScalingStep) -> Vec<Vec<RibbonControl<'_>>> {
    let mut columns: Vec<Vec<RibbonControl<'_>>> = Vec::new();
    for control in controls {
        if control.is_small(step)
            && let Some(column) = columns.last_mut()
            && column.len() < SMALL_ROWS
            && column.first().is_some_and(|first| first.is_small(step))
        {
            column.push(control);
        } else {
            columns.push(vec![control]);
        }
    }
    columns
}

/// The width of each column at `step`, using the same stacking as [`columns`].
fn column_widths(ui: &Ui, controls: &[RibbonControl<'_>], step: ScalingStep) -> Vec<f32> {
    let mut widths: Vec<(f32, usize, bool)> = Vec::new();
    for control in controls {
        let small = control.is_small(step);
        let width = control.width(ui, step);
        if small
            && let Some((column, rows, true)) = widths.last_mut()
            && *rows < SMALL_ROWS
        {
            *column = column.max(width);
            *rows += 1;
        } else {
            widths.push((width, 1, small));
        }
    }
    widths.into_iter().map(|(width, _, _)| width).collect()
}

/// The group label, centred in `row` (spec §10: `Caption1`, secondary text).
fn paint_label(ui: &Ui, row: Rect, text: &Arc<Galley>, theme: &Theme) {
    let origin = Pos2::new(
        row.center().x - text.size().x / 2.0,
        row.center().y - text.size().y / 2.0,
    );
    ui.painter()
        .galley(origin, Arc::clone(text), c(theme.color.neutral_foreground_2));
}

impl RibbonDialogLauncher {
    /// The launcher in the group's corner. `row` is the label row, `size` the square it takes.
    pub(super) fn show(
        self,
        ui: &mut Ui,
        group_label: &str,
        row: Rect,
        size: f32,
        tab: Option<&'static str>,
        out: &mut RibbonResponse,
    ) {
        let corner = Rect::from_min_size(
            Pos2::new(row.right() - size, row.bottom() - size),
            Vec2::splat(size),
        );
        let response = ui.put(
            corner,
            Button::icon_only(IconName::Open, format!("{group_label} dialog"))
                .appearance(Appearance::Transparent)
                .size(ButtonSize::Small)
                .id_salt(self.key),
        );
        let label = format!("{group_label} dialog");
        register(
            ui,
            tab,
            self.key,
            self.keytip,
            &label,
            (response.rect, Some(response.id)),
            None,
        );
        if response.clicked() || (ui.is_enabled() && keytips::invoked(ui.ctx(), self.key)) {
            out.events.push(RibbonEvent {
                key: self.key,
                kind: RibbonEventKind::Clicked,
            });
        }
    }
}
