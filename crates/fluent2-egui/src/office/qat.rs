//! Quick Access Toolbar: icon-only commands, a trailing customise menu and the position choice
//! (spec §3, WI-1561).
//!
//! The app owns the item order, visibility and position in [`QatState`]. [`Qat::show`] returns
//! which item was clicked and which customise action was picked, and never reorders anything
//! by itself. The app applies each [`QatCustomize`] with [`QatState::apply`], and places the
//! toolbar in the title bar or beside the ribbon according to [`QatState::position`].

use egui::{Id, Layout, Popup, Role, Sense, Ui, WidgetInfo, accesskit};
use fluent2_icons::Icon;

use super::keytips::{self, KeyTipCandidate, KeyTipLevel, Region, TargetKind};
use crate::controls::{Appearance, Button, ButtonSize, ensure_type_ramp};
use crate::foundation::text::{TypeRamp, style};
use crate::surfaces::{MenuItem, TooltipExt, divider, menu_frame};
use crate::theme::{Theme, c};
use fluent2_tokens::generated::global::spacing_horizontal;

/// The toolbar's state. Public fields, no serialisation (spec D3): the app persists it.
#[derive(Clone, Debug, PartialEq, Eq, Default)]
pub struct QatState {
    /// Keys of visible items, in display order.
    pub items: Vec<&'static str>,
    /// Where the app draws the toolbar.
    pub position: QatPosition,
    /// Whether the customise menu is open. The widget opens and closes it.
    pub customize_open: bool,
}

impl QatState {
    /// Apply one customise action from [`QatResponse::customize`].
    ///
    /// `Show(key, true)` appends `key` at the end when it is not already shown. `MoveUp` and
    /// `MoveDown` swap the item with its visible neighbour, and do nothing at the ends or for a
    /// hidden item.
    pub fn apply(&mut self, action: &QatCustomize) {
        match *action {
            QatCustomize::Show(key, true) => {
                if !self.items.contains(&key) {
                    self.items.push(key);
                }
            }
            QatCustomize::Show(key, false) => self.items.retain(|shown| *shown != key),
            QatCustomize::MoveUp(key) => {
                if let Some(index) = self.index_of(key)
                    && index > 0
                {
                    self.items.swap(index - 1, index);
                }
            }
            QatCustomize::MoveDown(key) => {
                if let Some(index) = self.index_of(key)
                    && index + 1 < self.items.len()
                {
                    self.items.swap(index, index + 1);
                }
            }
            QatCustomize::Position(position) => self.position = position,
        }
    }

    fn index_of(&self, key: &str) -> Option<usize> {
        self.items.iter().position(|shown| *shown == key)
    }
}

/// Where the app draws the Quick Access Toolbar.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Default)]
pub enum QatPosition {
    /// Inside the title bar (default).
    #[default]
    TitleBar,
    /// Row above the ribbon tab strip.
    AboveRibbon,
    /// Row below the ribbon.
    BelowRibbon,
}

/// One entry the app offers. `key` is stable across sessions.
#[derive(Clone, Copy, Debug)]
pub struct QatItem<'a> {
    /// Names the item in responses and in [`QatState::items`]. The first offered item with a
    /// given key wins.
    pub key: &'static str,
    /// The icon-only glyph.
    pub icon: Icon,
    /// The accessible name, also the row label in the customise menu.
    pub label: &'a str,
    /// The tooltip shown on hover and focus.
    pub tooltip: &'a str,
    /// Keytip for the KeyTips layer (§6). `None` means auto-assigned.
    pub keytip: Option<&'static str>,
}

/// A Quick Access Toolbar. Build it each frame with [`Qat::new`] and call [`Qat::show`].
///
/// ```
/// # use fluent2_egui::office::qat::{Qat, QatItem, QatState};
/// # use fluent2_icons::Icon;
/// # fn demo(ui: &mut egui::Ui) {
/// let offered = [QatItem { key: "save", icon: Icon::Save, label: "Save", tooltip: "Save", keytip: None }];
/// let mut state = QatState { items: vec!["save"], ..QatState::default() };
/// let response = Qat::new("qat", &mut state, &offered).show(ui);
/// if response.clicked == Some("save") {
///     // the app saves
/// }
/// # }
/// ```
pub struct Qat<'a> {
    id: Id,
    state: &'a mut QatState,
    offered: &'a [QatItem<'a>],
}

/// What [`Qat::show`] reports for one frame.
#[derive(Clone, Debug, PartialEq, Eq, Default)]
pub struct QatResponse {
    /// The key of the item clicked this frame.
    pub clicked: Option<&'static str>,
    /// The customise action picked this frame, if any.
    pub customize: Option<QatCustomize>,
    /// Keys that were in `items` but are not offered, or repeat an earlier key. They are
    /// dropped from `items` this frame.
    pub conflicts: Vec<&'static str>,
}

/// One change picked in the customise menu. The app applies it with [`QatState::apply`].
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum QatCustomize {
    /// Show (`true`) or hide (`false`) an item.
    Show(&'static str, bool),
    /// Move a shown item one place earlier.
    MoveUp(&'static str),
    /// Move a shown item one place later.
    MoveDown(&'static str),
    /// Place the toolbar here.
    Position(QatPosition),
}

impl<'a> Qat<'a> {
    /// A toolbar over `state`, offering `offered`. `id` is caller-supplied.
    #[must_use]
    pub fn new(id: impl std::hash::Hash, state: &'a mut QatState, offered: &'a [QatItem<'a>]) -> Self {
        Self {
            id: super::chrome_id(id),
            state,
            offered,
        }
    }

    /// Draw the toolbar and its customise menu into `ui`.
    pub fn show(self, ui: &mut Ui) -> QatResponse {
        let Self { id, state, offered } = self;
        ensure_type_ramp(ui);

        // Drop keys that are not offered, and repeats of an earlier key (spec D4).
        let mut conflicts = Vec::new();
        let mut kept: Vec<&'static str> = Vec::with_capacity(state.items.len());
        for key in state.items.iter().copied() {
            if offered.iter().any(|item| item.key == key) && !kept.contains(&key) {
                kept.push(key);
            } else {
                conflicts.push(key);
            }
        }
        if !conflicts.is_empty() {
            state.items = kept;
        }

        keytips::install(ui.ctx());
        let mut clicked = None;
        let mut customize = None;
        let mut first = None;
        let row = ui.horizontal(|ui| {
            ui.spacing_mut().item_spacing.x = spacing_horizontal::XS;
            for key in &state.items {
                let Some(item) = offered.iter().find(|item| item.key == *key) else {
                    continue;
                };
                let button = Button::icon_only(item.icon, item.label)
                    .appearance(Appearance::Transparent)
                    .size(ButtonSize::Small)
                    .id_salt((id, item.key));
                let response = ui.add(button).fluent_tooltip(item.tooltip);
                first = first.or(Some(response.id));
                if ui.is_enabled() {
                    keytips::register_for(
                        ui.ctx(),
                        KeyTipCandidate {
                            target: item.key,
                            level: KeyTipLevel::Root,
                            kind: TargetKind::QatItem,
                            requested: item.keytip,
                            label: item.label.to_owned(),
                        },
                        response.rect,
                        response.id,
                    );
                }
                // A KeyTip runs the item, as a click does (§6.3).
                if response.clicked() || (ui.is_enabled() && keytips::invoked(ui.ctx(), item.key)) {
                    clicked = Some(item.key);
                }
            }

            let trigger = Button::icon_only(Icon::MoreHorizontal, "Customize")
                .appearance(Appearance::Transparent)
                .size(ButtonSize::Small)
                .id_salt((id, "customize"));
            let trigger = ui.add(trigger).fluent_tooltip("Customize Quick Access Toolbar");
            if trigger.clicked() {
                state.customize_open = !state.customize_open;
            }
            let mut open = state.customize_open;
            let popup = Popup::menu(&trigger).open_bool(&mut open).show(|popup| {
                menu_frame(popup)
                    .show(popup, |popup| {
                        customize_menu(popup, &state.items, state.position, offered)
                    })
                    .inner
            });
            state.customize_open = open;
            customize = popup.and_then(|inner| inner.inner);
            trigger.id
        });

        // The toolbar has no widget of its own, so an interact over its rect gives the AccessKit
        // node bounds. It senses hover only, so clicks still reach the buttons.
        let toolbar = ui.interact(row.response.rect, id, Sense::hover());
        toolbar.widget_info(|| WidgetInfo::labeled(Role::Unknown, true, "Quick Access Toolbar"));
        ui.ctx().accesskit_node_builder(id, |node| {
            node.set_role(accesskit::Role::Toolbar);
        });

        // F6: a QAT in the title bar is part of that region; above or below the ribbon it is
        // its own (§9.3). The customise button is its last stop when no item is shown.
        let region = match state.position {
            QatPosition::TitleBar => Region::TitleBar,
            QatPosition::AboveRibbon | QatPosition::BelowRibbon => Region::Qat,
        };
        keytips::mark_region(ui.ctx(), region, row.response.rect, first.or(Some(row.inner)));

        QatResponse {
            clicked,
            customize,
            conflicts,
        }
    }
}

/// The customise menu body: one checkable row per offered item with move buttons, then the
/// position choice.
fn customize_menu(
    ui: &mut Ui,
    shown: &[&'static str],
    position: QatPosition,
    offered: &[QatItem<'_>],
) -> Option<QatCustomize> {
    let mut action = None;
    for item in offered {
        let index = shown.iter().position(|key| *key == item.key);
        let is_shown = index.is_some();
        let can_up = index.is_some_and(|index| index > 0);
        let can_down = index.is_some_and(|index| index + 1 < shown.len());
        ui.horizontal(|ui| {
            ui.with_layout(Layout::right_to_left(egui::Align::Center), |ui| {
                // Right to left: the first widget added sits at the far right.
                let move_button = |glyph: Icon, label: String, salt: &'static str| {
                    Button::icon_only(glyph, label)
                        .appearance(Appearance::Transparent)
                        .size(ButtonSize::Small)
                        .id_salt((item.key, salt))
                };
                if ui
                    .add_enabled(
                        can_down,
                        move_button(Icon::ChevronDown, format!("Move {} down", item.label), "down"),
                    )
                    .clicked()
                {
                    action = Some(QatCustomize::MoveDown(item.key));
                }
                if ui
                    .add_enabled(
                        can_up,
                        move_button(Icon::ChevronUp, format!("Move {} up", item.label), "up"),
                    )
                    .clicked()
                {
                    action = Some(QatCustomize::MoveUp(item.key));
                }
                let row = MenuItem::new(item.label)
                    .icon(item.icon)
                    .checked(is_shown)
                    .check_column(true)
                    .id_salt(item.key);
                if ui.add(row).clicked() {
                    action = Some(QatCustomize::Show(item.key, !is_shown));
                }
            })
        });
    }

    divider(ui);
    let caption = egui::RichText::new("Position")
        .text_style(style(TypeRamp::Caption1))
        .color(c(Theme::get(ui.ctx()).color.neutral_foreground_3));
    ui.label(caption);
    for choice in [
        QatPosition::TitleBar,
        QatPosition::AboveRibbon,
        QatPosition::BelowRibbon,
    ] {
        let row = MenuItem::new(position_label(choice))
            .checked(choice == position)
            .check_column(true)
            .id_salt(position_label(choice));
        if ui.add(row).clicked() {
            action = Some(QatCustomize::Position(choice));
        }
    }
    action
}

fn position_label(position: QatPosition) -> &'static str {
    match position {
        QatPosition::TitleBar => "In title bar",
        QatPosition::AboveRibbon => "Above the ribbon",
        QatPosition::BelowRibbon => "Below the ribbon",
    }
}
