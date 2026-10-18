//! Quick Access Toolbar behaviour, without the wgpu snapshot feature.

use std::cell::{Cell, RefCell};
use std::rc::Rc;

use egui::accesskit::Role;
use egui_kittest::Harness;
use egui_kittest::kittest::Queryable;
use fluent2_egui::office::TitleBar;
use fluent2_egui::office::qat::{Qat, QatCustomize, QatItem, QatPosition, QatState};
use fluent2_egui::{Theme, install, install_system_fonts};
use fluent2_icons::Icon;

const ITEMS: [QatItem<'static>; 3] = [
    QatItem {
        key: "open",
        icon: Icon::Open,
        label: "Open",
        tooltip: "Open a file",
        keytip: None,
    },
    QatItem {
        key: "save",
        icon: Icon::Save,
        label: "Save",
        tooltip: "Save the document",
        keytip: None,
    },
    QatItem {
        key: "print",
        icon: Icon::Print,
        label: "Print",
        tooltip: "Print the document",
        keytip: None,
    },
];

/// App state shared with the harness closure, so a test can read it back.
#[derive(Default)]
struct Probe {
    state: RefCell<QatState>,
    clicked: Cell<Option<&'static str>>,
    conflicts: RefCell<Vec<&'static str>>,
}

/// A toolbar in a 480 by 600 window, tall enough for the customise menu, with the app applying customise actions as it would.
fn qat_harness(probe: Rc<Probe>) -> Harness<'static> {
    Harness::builder()
        .with_size(egui::vec2(480.0, 600.0))
        .build_ui(move |ui| {
            install(ui.ctx(), &Theme::default());
            install_system_fonts(ui.ctx());
            let mut state = probe.state.borrow_mut();
            let response = Qat::new("qat_test", &mut state, &ITEMS).show(ui);
            if let Some(key) = response.clicked {
                probe.clicked.set(Some(key));
            }
            probe.conflicts.borrow_mut().extend(response.conflicts);
            if let Some(action) = &response.customize {
                state.apply(action);
            }
        })
}

fn shown(keys: &[&'static str]) -> Rc<Probe> {
    let probe = Rc::new(Probe::default());
    probe.state.borrow_mut().items = keys.to_vec();
    probe
}

#[test]
fn clicking_an_item_reports_its_key() {
    let probe = shown(&["open", "save", "print"]);
    let mut harness = qat_harness(Rc::clone(&probe));
    harness.run();
    harness.get_by_role_and_label(Role::Button, "Save").click();
    harness.run();
    assert_eq!(probe.clicked.get(), Some("save"));
}

#[test]
fn the_toolbar_is_a_named_toolbar_with_tooltips() {
    let probe = shown(&["open", "save"]);
    let mut harness = qat_harness(probe);
    harness.run();
    assert!(
        harness
            .query_by_role_and_label(Role::Toolbar, "Quick Access Toolbar")
            .is_some()
    );
    assert!(harness.query_by_role_and_label(Role::Button, "Open").is_some());
    assert!(harness.query_by_role_and_label(Role::Button, "Print").is_none());
}

#[test]
fn toggling_an_item_in_the_customise_menu_updates_the_state() {
    let probe = shown(&["open", "save"]);
    let mut harness = qat_harness(Rc::clone(&probe));
    harness.run();

    harness.get_by_role_and_label(Role::Button, "Customize").click();
    harness.run();
    assert!(probe.state.borrow().customize_open, "the customise menu opens");

    // "Print" is not shown yet, so its menu row is the only "Print" in the tree.
    harness
        .get_by_role_and_label(Role::MenuItemCheckBox, "Print")
        .click();
    harness.run();
    harness.run();
    assert_eq!(probe.state.borrow().items, vec!["open", "save", "print"]);
    assert!(
        !probe.state.borrow().customize_open,
        "choosing a row closes the menu"
    );
    assert!(harness.query_by_role_and_label(Role::Button, "Print").is_some());
}

#[test]
fn move_actions_reorder_shown_items_and_stop_at_the_ends() {
    let mut state = QatState {
        items: vec!["open", "save", "print"],
        ..QatState::default()
    };
    state.apply(&QatCustomize::MoveDown("open"));
    assert_eq!(state.items, vec!["save", "open", "print"]);
    state.apply(&QatCustomize::MoveUp("save"));
    assert_eq!(
        state.items,
        vec!["save", "open", "print"],
        "no move above the first item"
    );
    state.apply(&QatCustomize::MoveDown("print"));
    assert_eq!(
        state.items,
        vec!["save", "open", "print"],
        "no move below the last item"
    );
    state.apply(&QatCustomize::MoveUp("print"));
    assert_eq!(state.items, vec!["save", "print", "open"]);
}

#[test]
fn show_actions_hide_and_append_items() {
    let mut state = QatState {
        items: vec!["open"],
        ..QatState::default()
    };
    state.apply(&QatCustomize::Show("save", true));
    state.apply(&QatCustomize::Show("open", true));
    assert_eq!(
        state.items,
        vec!["open", "save"],
        "showing a shown item changes nothing"
    );
    state.apply(&QatCustomize::Show("open", false));
    assert_eq!(state.items, vec!["save"]);
}

#[test]
fn an_item_the_app_stops_offering_is_dropped_and_reported() {
    let probe = shown(&["open", "gone", "save", "open"]);
    let mut harness = qat_harness(Rc::clone(&probe));
    harness.run();
    assert_eq!(probe.state.borrow().items, vec!["open", "save"]);
    assert_eq!(*probe.conflicts.borrow(), vec!["gone", "open"]);
}

#[test]
fn choosing_a_position_in_the_menu_moves_the_toolbar_to_the_other_location() {
    let probe = Rc::new(Probe::default());
    probe.state.borrow_mut().items = vec!["open"];
    // The app places the toolbar: in the title bar, above the ribbon, or below it. The
    // ribbon is a 120 pt block here.
    let mut harness = {
        let probe = Rc::clone(&probe);
        Harness::builder()
            .with_size(egui::vec2(480.0, 600.0))
            .build_ui(move |ui| {
                install(ui.ctx(), &Theme::default());
                install_system_fonts(ui.ctx());
                let mut state = probe.state.borrow_mut();
                let position = state.position;
                let mut response = None;
                match position {
                    QatPosition::TitleBar => {
                        TitleBar::new("qat_position_title", "Document").show(ui, |slots| {
                            slots.left(|ui| {
                                response = Some(Qat::new("qat_test", &mut state, &ITEMS).show(ui));
                            });
                        });
                    }
                    QatPosition::AboveRibbon => {
                        response = Some(Qat::new("qat_test", &mut state, &ITEMS).show(ui));
                        ui.allocate_space(egui::vec2(480.0, 120.0));
                    }
                    QatPosition::BelowRibbon => {
                        ui.allocate_space(egui::vec2(480.0, 120.0));
                        response = Some(Qat::new("qat_test", &mut state, &ITEMS).show(ui));
                    }
                }
                if let Some(customize) = response.and_then(|response| response.customize) {
                    state.apply(&customize);
                }
            })
    };
    harness.run();
    let title_y = harness
        .get_by_role_and_label(Role::Toolbar, "Quick Access Toolbar")
        .rect()
        .min
        .y;

    harness.get_by_role_and_label(Role::Button, "Customize").click();
    harness.run();
    harness
        .get_by_role_and_label(Role::MenuItemCheckBox, "Below the ribbon")
        .click();
    harness.run();
    harness.run();
    assert_eq!(probe.state.borrow().position, QatPosition::BelowRibbon);
    let below_y = harness
        .get_by_role_and_label(Role::Toolbar, "Quick Access Toolbar")
        .rect()
        .min
        .y;
    assert!(
        below_y > title_y + 100.0,
        "below the ribbon the toolbar sits under the 120 pt block: title {title_y}, below {below_y}"
    );
}
