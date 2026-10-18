//! Ribbon groups (spec §4.1, §4.2): a labelled run of controls with an optional dialog launcher
//! in the bottom-right corner. Only the `Large` step is drawn here; scaling is WI-1565.

use std::sync::Arc;

use egui::{Galley, Id, Pos2, Rect, Sense, Ui, Vec2, accesskit};
use fluent2_tokens::generated::global::spacing_horizontal;

use super::controls::{RibbonControl, RibbonDialogLauncher, caption, register};
use super::{RibbonEvent, RibbonEventKind, RibbonResponse, body_node};
use crate::controls::{Appearance, Button, ButtonSize};
use crate::foundation::icon::IconName;
use crate::office::keytips;
use crate::theme::{Theme, c};

/// Controls stacked in one column of a group, at most (spec §5.1 small rendering).
const SMALL_ROWS: usize = 3;

/// The scale step a group renders at (spec §5.1). Only `Large` is drawn in this item.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub enum ScalingStep {
    /// Large controls, with labels under icons where the control has them.
    Large,
    /// Small controls, with labels beside icons.
    Medium,
    /// Icon only; labels move into tooltips.
    Small,
    /// One collapsed button that opens the group.
    Collapsed,
}

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
}

impl<'a> GroupBuilder<'a> {
    pub(super) fn new(key: &'static str, label: &str) -> Self {
        Self {
            key,
            label: label.to_owned(),
            controls: Vec::new(),
            launcher: None,
        }
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
        }
    }
}

/// A built group: what one frame paints.
pub(super) struct Group<'a> {
    key: &'static str,
    label: String,
    controls: Vec<RibbonControl<'a>>,
    launcher: Option<RibbonDialogLauncher>,
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

    /// Paint the group: its columns of controls, then its label row with the launcher.
    pub(super) fn show(self, ui: &mut Ui, tab: Option<&'static str>, out: &mut RibbonResponse) {
        let Self {
            key,
            label,
            controls,
            launcher,
        } = self;
        let theme = Theme::get(ui.ctx());
        let pad = spacing_horizontal::XS;

        ui.push_id(key, |ui| {
            let controls_rect = ui
                .horizontal_top(|ui| {
                    ui.spacing_mut().item_spacing.x = pad;
                    for column in columns(controls) {
                        ui.vertical(|ui| {
                            ui.spacing_mut().item_spacing.y = pad;
                            for control in column {
                                control.show(ui, tab, out);
                            }
                        });
                    }
                })
                .response
                .rect;

            let text = caption(ui, &label, false, f32::INFINITY);
            let launch = theme.chrome.icon_small + 2.0 * pad;
            let row_height = text.size().y.max(launcher.as_ref().map_or(0.0, |_| launch));
            let (row, _) = ui.allocate_exact_size(
                Vec2::new(controls_rect.width().max(text.size().x), row_height),
                Sense::hover(),
            );
            paint_label(ui, row, &text, &theme);
            if let Some(launcher) = launcher {
                launcher.show(ui, &label, row, launch, tab, out);
            }

            ui.ctx().accesskit_node_builder(ui.id().with("group"), |node| {
                node.set_role(accesskit::Role::Group);
                node.set_label(label.clone());
            });
        });
    }
}

/// Paint the active tab's body: the groups in a row, at the body's height (spec §4, §10).
///
/// `tab` is the active tab's key: its controls register their KeyTips at that tab's level.
/// Returns the body's rectangle.
pub(super) fn paint_body(
    ui: &mut Ui,
    id: Id,
    label: Option<&str>,
    tab: Option<&'static str>,
    groups: Vec<Group<'_>>,
    out: &mut RibbonResponse,
) -> Rect {
    body_node(ui.ctx(), id, label);
    let height = Theme::get(ui.ctx()).chrome.ribbon_body_height;
    ui.vertical(|ui| {
        ui.set_min_height(height);
        ui.horizontal_top(|ui| {
            ui.spacing_mut().item_spacing.x = spacing_horizontal::S;
            for group in groups {
                group.show(ui, tab, out);
            }
        });
    })
    .response
    .rect
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
fn columns(controls: Vec<RibbonControl<'_>>) -> Vec<Vec<RibbonControl<'_>>> {
    let mut columns: Vec<Vec<RibbonControl<'_>>> = Vec::new();
    for control in controls {
        if control.is_small()
            && let Some(column) = columns.last_mut()
            && column.len() < SMALL_ROWS
            && column.first().is_some_and(RibbonControl::is_small)
        {
            column.push(control);
        } else {
            columns.push(vec![control]);
        }
    }
    columns
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
        register(ui, tab, self.key, self.keytip, &label, response.rect, None);
        if response.clicked() || (ui.is_enabled() && keytips::invoked(ui.ctx(), self.key)) {
            out.events.push(RibbonEvent {
                key: self.key,
                kind: RibbonEventKind::Clicked,
            });
        }
    }
}
