//! Inter-pane drag: a typed payload moved between two panes, a refusing target, Escape, and
//! the keyboard menu. No wgpu feature.

use std::cell::RefCell;

use egui::accesskit::{Live, Role};
use egui::{Event, Key, Modifiers, PointerButton, Pos2, Sense, Vec2};
use egui_kittest::Harness;
use egui_kittest::kittest::Queryable;
use fluent2_egui::data::MoveSource;
use fluent2_egui::data::pane_drag::{
    DragSource, DropTarget, MoveChoice, PaneDrop, announce, end_frame, live_region_id, move_menu,
};
use fluent2_egui::surfaces::menu_frame;
use fluent2_egui::{Theme, install};

struct Probe {
    inbox: RefCell<Vec<String>>,
    archive_open: bool,
    drops: RefCell<Vec<PaneDrop<String>>>,
}

fn probe(archive_open: bool) -> &'static Probe {
    Box::leak(Box::new(Probe {
        inbox: RefCell::new(Vec::new()),
        archive_open,
        drops: RefCell::new(Vec::new()),
    }))
}

fn harness(probe: &'static Probe, theme: Theme) -> Harness<'static> {
    Harness::builder()
        .with_size(Vec2::new(520.0, 280.0))
        .build_ui(move |ui| {
            install(ui.ctx(), &theme);
            ui.horizontal_top(|ui| {
                ui.vertical(|ui| {
                    ui.set_width(200.0);
                    ui.strong("Messages");
                    let card = ui.add(egui::Label::new("Invoice 12").sense(Sense::click_and_drag()));
                    DragSource::new(&card, "invoice-12".to_owned(), "Invoice 12").show(ui);
                    let open = card
                        .secondary_clicked()
                        .then_some(egui::SetOpenCommand::Bool(true));
                    let choices = [
                        MoveChoice {
                            label: "Inbox",
                            accepts: true,
                        },
                        MoveChoice {
                            label: "Archive",
                            accepts: probe.archive_open,
                        },
                    ];
                    let chosen = egui::Popup::menu(&card)
                        .open_memory(open)
                        .frame(menu_frame(ui))
                        .show(|ui| move_menu(ui, "Move to", &choices))
                        .and_then(|inner| inner.inner);
                    if let Some(index) = chosen {
                        let drop = PaneDrop {
                            source: card.id,
                            payload: "invoice-12".to_owned(),
                            target: choices[index].label.to_owned(),
                            how: MoveSource::Keyboard,
                        };
                        probe.drops.borrow_mut().push(drop.clone());
                        if index == 0 {
                            probe.inbox.borrow_mut().push(drop.payload);
                        }
                    }
                });
                ui.vertical(|ui| {
                    ui.set_width(160.0);
                    ui.strong("Folders");
                    for (name, accepts) in [("Inbox", true), ("Archive", probe.archive_open)] {
                        let label = ui.label(name);
                        if let Some(drop) = DropTarget::<String>::new(&label, name)
                            .accepts(accepts)
                            .show(ui)
                            .dropped
                        {
                            probe.drops.borrow_mut().push(drop.clone());
                            if name == "Inbox" {
                                probe.inbox.borrow_mut().push(drop.payload);
                            }
                        }
                    }
                });
            });
            end_frame::<String>(ui);
            announce(ui);
        })
}

fn centre(harness: &Harness<'_>, label: &str) -> Pos2 {
    harness.get_by_label(label).rect().center()
}

fn button(harness: &mut Harness<'_>, pos: Pos2, pressed: bool, which: PointerButton) {
    harness.event(Event::PointerButton {
        pos,
        button: which,
        pressed,
        modifiers: Modifiers::NONE,
    });
    harness.run_steps(1);
}

fn drag_to(harness: &mut Harness<'_>, from: Pos2, to: Pos2) {
    harness.event(Event::PointerMoved(from));
    harness.run_steps(2);
    button(harness, from, true, PointerButton::Primary);
    for step in 1..=6 {
        let t = step as f32 / 6.0;
        harness.event(Event::PointerMoved(from + (to - from) * t));
        harness.run_steps(2);
    }
}

fn light() -> Theme {
    Theme::light(fluent2_tokens::FLUENT)
}

#[test]
fn dragging_onto_inbox_moves_the_payload() {
    let probe = probe(true);
    let mut harness = harness(probe, light());
    harness.run();
    let from = centre(&harness, "Invoice 12");
    let to = centre(&harness, "Inbox");
    drag_to(&mut harness, from, to);
    button(&mut harness, to, false, PointerButton::Primary);
    harness.run();
    assert_eq!(probe.inbox.borrow().as_slice(), ["invoice-12"]);
    let drop = &probe.drops.borrow()[0];
    assert_eq!(drop.how, MoveSource::Pointer);
    assert_eq!(drop.target, "Inbox");
}

#[test]
fn a_locked_folder_refuses_the_drop() {
    let probe = probe(false);
    let mut harness = harness(probe, light());
    harness.run();
    let from = centre(&harness, "Invoice 12");
    let to = centre(&harness, "Archive");
    drag_to(&mut harness, from, to);
    assert!(
        probe.drops.borrow().is_empty(),
        "a locked folder reports no drop while the pointer is still down"
    );
    button(&mut harness, to, false, PointerButton::Primary);
    harness.run();
    assert!(probe.drops.borrow().is_empty());
    assert!(probe.inbox.borrow().is_empty());
}

#[test]
fn escape_cancels_without_a_drop() {
    let probe = probe(true);
    let mut harness = harness(probe, light());
    harness.run();
    let from = centre(&harness, "Invoice 12");
    let to = centre(&harness, "Inbox");
    drag_to(&mut harness, from, from + Vec2::new(40.0, 0.0));
    harness.event(Event::Key {
        key: Key::Escape,
        physical_key: None,
        pressed: true,
        repeat: false,
        modifiers: Modifiers::NONE,
    });
    harness.run_steps(1);
    button(&mut harness, to, false, PointerButton::Primary);
    harness.run();
    assert!(probe.drops.borrow().is_empty());
}

#[test]
fn the_menu_moves_without_a_pointer_drag() {
    let probe = probe(true);
    let mut harness = harness(probe, light());
    harness.run();
    let from = centre(&harness, "Invoice 12");
    button(&mut harness, from, true, PointerButton::Secondary);
    button(&mut harness, from, false, PointerButton::Secondary);
    harness.run();
    let move_to = centre(&harness, "Move to");
    button(&mut harness, move_to, true, PointerButton::Primary);
    button(&mut harness, move_to, false, PointerButton::Primary);
    harness.run();
    let inbox = harness
        .get_by_role_and_label(Role::MenuItem, "Inbox")
        .rect()
        .center();
    button(&mut harness, inbox, true, PointerButton::Primary);
    button(&mut harness, inbox, false, PointerButton::Primary);
    harness.run();
    assert_eq!(probe.inbox.borrow().as_slice(), ["invoice-12"]);
    assert_eq!(probe.drops.borrow()[0].how, MoveSource::Keyboard);
}

#[test]
fn themes_render_and_the_live_region_exists_after_a_drag() {
    for theme in [
        Theme::light(fluent2_tokens::FLUENT),
        Theme::dark(fluent2_tokens::FLUENT),
        Theme::high_contrast(),
    ] {
        let probe = probe(true);
        let mut harness = harness(probe, theme);
        harness.run();
        let from = centre(&harness, "Invoice 12");
        drag_to(&mut harness, from, from + Vec2::new(30.0, 8.0));
        harness.run();
        let region = harness.get_by_role(Role::Status);
        assert_eq!(
            egui_kittest::kittest::NodeT::accesskit_node(&region).live(),
            Live::Polite
        );
        let _ = live_region_id();
    }
}
