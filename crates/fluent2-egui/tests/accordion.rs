//! Accordion behaviour: toggling, single and multiple open, reduced motion and AccessKit.

use std::cell::RefCell;

use egui::accesskit::Role;
use egui_kittest::Harness;
use egui_kittest::kittest::{NodeT, Queryable};
use fluent2_egui::foundation::motion;
use fluent2_egui::surfaces::{Accordion, AccordionItem};
use fluent2_egui::{Theme, install};

fn harness(open: &'static RefCell<Vec<usize>>, multiple: bool) -> Harness<'static> {
    let mut harness = Harness::new_ui(move |ui| {
        install(ui.ctx(), &Theme::default());
        motion::set_reduced_motion(ui.ctx(), true);
        let mut state = open.borrow_mut();
        let _ = Accordion::new("acc", &mut state)
            .multiple(multiple)
            .collapsible(true)
            .item(AccordionItem::new("One", |ui| {
                ui.label("Panel one");
            }))
            .item(AccordionItem::new("Two", |ui| {
                ui.label("Panel two");
            }))
            .show(ui);
    });
    harness.run();
    harness
}

fn cell(open: Vec<usize>) -> &'static RefCell<Vec<usize>> {
    Box::leak(Box::new(RefCell::new(open)))
}

#[test]
fn clicking_a_header_toggles_its_panel() {
    let open = cell(Vec::new());
    let mut harness = harness(open, false);
    assert!(harness.query_by_label("Panel one").is_none());
    harness.get_by_label("One").click();
    harness.run();
    assert_eq!(*open.borrow(), vec![0]);
    // Reduced motion snaps: the panel is shown in full on the next frame.
    assert!(harness.query_by_label("Panel one").is_some());
    harness.get_by_label("One").click();
    harness.run();
    assert!(open.borrow().is_empty());
}

#[test]
fn single_mode_keeps_one_open() {
    let open = cell(vec![0]);
    let mut harness = harness(open, false);
    harness.get_by_label("Two").click();
    harness.run();
    assert_eq!(*open.borrow(), vec![1]);
}

#[test]
fn multiple_mode_keeps_both_open() {
    let open = cell(vec![0]);
    let mut harness = harness(open, true);
    harness.get_by_label("Two").click();
    harness.run();
    assert_eq!(*open.borrow(), vec![0, 1]);
}

#[test]
fn keyboard_toggles_the_focused_header() {
    let open = cell(Vec::new());
    let mut harness = harness(open, false);
    harness.get_by_label("One").focus();
    harness.run();
    harness.key_press(egui::Key::Enter);
    harness.run();
    assert_eq!(*open.borrow(), vec![0]);
    harness.key_press(egui::Key::Space);
    harness.run();
    assert!(open.borrow().is_empty());
}

#[test]
fn headers_are_buttons_with_expanded_state() {
    let open = cell(vec![1]);
    let harness = harness(open, false);
    let one = harness.get_by_label("One");
    assert_eq!(one.accesskit_node().role(), Role::Button);
    assert_eq!(one.accesskit_node().data().is_expanded(), Some(false));
    let two = harness.get_by_label("Two");
    assert_eq!(two.accesskit_node().data().is_expanded(), Some(true));
    assert_eq!(two.accesskit_node().data().controls().len(), 1);
}
