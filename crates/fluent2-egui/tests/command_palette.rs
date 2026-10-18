//! CommandPalette behaviour, without the wgpu snapshot feature.
//!
//! Widgets are found through the AccessKit tree by role and label.

use std::cell::RefCell;
use std::rc::Rc;

use egui::accesskit::Role;
use egui_kittest::Harness;
use egui_kittest::kittest::{NodeT, Queryable};
use fluent2_egui::surfaces::{CommandPalette, PaletteCommand};
use fluent2_egui::{Theme, install};

#[derive(Default)]
struct Probe {
    open: bool,
    chosen: Vec<String>,
    recents: Vec<String>,
}

fn commands() -> Vec<PaletteCommand> {
    vec![
        PaletteCommand::new("file.open", "Open file")
            .group("File")
            .shortcut("Ctrl+O"),
        PaletteCommand::new("file.save", "Save")
            .group("File")
            .disabled(true),
        PaletteCommand::new("file.close", "Close file").group("File"),
        PaletteCommand::new("view.zoom", "Zoom in")
            .group("View")
            .subtitle("Make text larger"),
    ]
}

fn harness(probe: Rc<RefCell<Probe>>, theme: Theme) -> Harness<'static> {
    let commands = commands();
    Harness::new_ui(move |ui| {
        install(ui.ctx(), &theme);
        let mut state = probe.borrow_mut();
        let mut open = state.open;
        let recents = state.recents.clone();
        if let Some(id) = CommandPalette::new("palette", &commands)
            .recents(&recents)
            .show(ui.ctx(), &mut open)
        {
            state.chosen.push(id);
        }
        state.open = open;
    })
}

fn open_probe() -> Rc<RefCell<Probe>> {
    Rc::new(RefCell::new(Probe {
        open: true,
        ..Probe::default()
    }))
}

fn selected_label(harness: &Harness<'_>) -> Option<String> {
    harness
        .query_all_by_role(Role::ListBoxOption)
        .find(|node| node.accesskit_node().is_selected() == Some(true))
        .and_then(|node| node.accesskit_node().label())
}

#[test]
fn renders_in_every_theme_with_roles() {
    for theme in [
        Theme::default(),
        Theme::dark(fluent2_egui::tokens::FLUENT),
        Theme::high_contrast(),
    ] {
        let probe = open_probe();
        let mut harness = harness(probe, theme);
        harness.run();
        let _ = harness.get_by_role_and_label(Role::Dialog, "Command palette");
        let _ = harness.get_by_role_and_label(Role::ListBox, "Command palette");
        assert_eq!(harness.query_all_by_role(Role::ListBoxOption).count(), 4);
        let save = harness.get_by_role_and_label(Role::ListBoxOption, "Save");
        assert!(save.accesskit_node().is_disabled());
    }
}

#[test]
fn search_box_has_focus_and_first_enabled_row_is_selected() {
    let probe = open_probe();
    let mut harness = harness(probe, Theme::default());
    harness.run();
    harness.run();
    // The search box keeps focus and names the selected row as its active descendant, which
    // AccessKit reports as the focused node.
    let input = harness.get_by_role_and_label(Role::TextInput, "Command palette");
    assert!(input.accesskit_node().active_descendant().is_some());
    let focused = harness
        .query_all_by_role(Role::ListBoxOption)
        .find(|node| node.is_focused());
    assert_eq!(
        focused.and_then(|node| node.accesskit_node().label()).as_deref(),
        Some("Open file")
    );
    assert_eq!(selected_label(&harness).as_deref(), Some("Open file"));
}

#[test]
fn arrows_skip_disabled_rows_and_enter_chooses() {
    let probe = open_probe();
    let mut harness = harness(probe.clone(), Theme::default());
    harness.run();
    harness.key_press(egui::Key::ArrowDown);
    harness.run();
    assert_eq!(selected_label(&harness).as_deref(), Some("Close file"));
    harness.key_press(egui::Key::ArrowDown);
    harness.run();
    assert_eq!(selected_label(&harness).as_deref(), Some("Zoom in"));
    harness.key_press(egui::Key::ArrowDown);
    harness.run();
    assert_eq!(
        selected_label(&harness).as_deref(),
        Some("Zoom in"),
        "Down stops at the end"
    );
    harness.key_press(egui::Key::ArrowUp);
    harness.key_press(egui::Key::ArrowUp);
    harness.run();
    assert_eq!(selected_label(&harness).as_deref(), Some("Open file"));
    harness.key_press(egui::Key::ArrowDown);
    harness.run();
    harness.key_press(egui::Key::Enter);
    harness.run();
    assert_eq!(probe.borrow().chosen, ["file.close"]);
    assert!(!probe.borrow().open, "choosing closes the palette");
}

#[test]
fn typing_filters_and_reranks() {
    let probe = open_probe();
    let mut harness = harness(probe.clone(), Theme::default());
    harness.run();
    harness.run();
    harness.event(egui::Event::Text("zoom".to_owned()));
    harness.run();
    harness.run();
    assert_eq!(harness.query_all_by_role(Role::ListBoxOption).count(), 1);
    assert_eq!(selected_label(&harness).as_deref(), Some("Zoom in"));
    harness.key_press(egui::Key::Enter);
    harness.run();
    assert_eq!(probe.borrow().chosen, ["view.zoom"]);
}

#[test]
fn escape_closes_without_choosing() {
    let probe = open_probe();
    let mut harness = harness(probe.clone(), Theme::default());
    harness.run();
    harness.key_press(egui::Key::Escape);
    harness.run();
    assert!(!probe.borrow().open);
    assert!(probe.borrow().chosen.is_empty());
    assert!(harness.query_by_role(Role::Dialog).is_none());
}

#[test]
fn click_chooses_but_not_on_a_disabled_row() {
    let probe = open_probe();
    let mut harness = harness(probe.clone(), Theme::default());
    harness.run();
    harness.get_by_role_and_label(Role::ListBoxOption, "Save").click();
    harness.run();
    assert!(probe.borrow().open);
    assert!(probe.borrow().chosen.is_empty());
    harness
        .get_by_role_and_label(Role::ListBoxOption, "Zoom in")
        .click();
    harness.run();
    harness.run();
    assert_eq!(probe.borrow().chosen, ["view.zoom"]);
}

#[test]
fn recents_come_first_and_reopening_clears_the_query() {
    let probe = open_probe();
    probe.borrow_mut().recents = vec!["view.zoom".to_owned()];
    let mut harness = harness(probe.clone(), Theme::default());
    harness.run();
    let _ = harness.get_by_label("Recent");
    assert_eq!(selected_label(&harness).as_deref(), Some("Zoom in"));

    harness.run();
    harness.event(egui::Event::Text("close".to_owned()));
    harness.run();
    harness.key_press(egui::Key::Escape);
    harness.run();
    probe.borrow_mut().open = true;
    harness.run();
    harness.run();
    assert_eq!(harness.query_all_by_role(Role::ListBoxOption).count(), 4);
}

#[test]
fn closed_palette_draws_nothing() {
    let probe = Rc::new(RefCell::new(Probe::default()));
    let mut harness = harness(probe, Theme::default());
    harness.run();
    assert!(harness.query_by_role(Role::Dialog).is_none());
}

#[test]
fn rows_are_children_of_the_list_inside_the_dialog() {
    let probe = open_probe();
    let mut harness = harness(probe, Theme::default());
    harness.run();
    for label in ["Open file", "Save", "Close file", "Zoom in"] {
        let row = harness.get_by_role_and_label(Role::ListBoxOption, label);
        let parent = row.parent().map(|node| node.accesskit_node().role());
        assert_eq!(parent, Some(Role::ListBox), "{label} hangs under the list");
    }
    // The scroll area adds plain containers between the list and the dialog; the nearest
    // ancestor with a role is the dialog.
    let list = harness.get_by_role(Role::ListBox);
    let mut ancestor = list.parent();
    while let Some(node) = &ancestor {
        if node.accesskit_node().role() != Role::GenericContainer {
            break;
        }
        ancestor = node.parent();
    }
    assert_eq!(
        ancestor.map(|node| node.accesskit_node().role()),
        Some(Role::Dialog)
    );
    let search = harness.get_by_role_and_label(Role::TextInput, "Command palette");
    let mut ancestor = search.parent();
    while let Some(node) = &ancestor {
        if node.accesskit_node().role() == Role::Dialog {
            break;
        }
        ancestor = node.parent();
    }
    assert!(ancestor.is_some(), "the search box is inside the dialog");
}

#[test]
fn search_text_is_cleared_after_close() {
    let probe = open_probe();
    let mut harness = harness(probe.clone(), Theme::default());
    harness.run();
    harness.run();
    harness.event(egui::Event::Text("zoom".to_owned()));
    harness.run();
    harness.key_press(egui::Key::Enter);
    harness.run();
    assert_eq!(probe.borrow().chosen, ["view.zoom"]);
    probe.borrow_mut().open = true;
    harness.run();
    harness.run();
    let input = harness.get_by_role_and_label(Role::TextInput, "Command palette");
    assert_eq!(input.accesskit_node().value().unwrap_or_default(), "");
    assert_eq!(harness.query_all_by_role(Role::ListBoxOption).count(), 4);
}
