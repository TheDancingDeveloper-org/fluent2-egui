//! Drawer and BottomSheet behaviour, without the wgpu snapshot feature.
//!
//! Widgets are found through the AccessKit tree by role and label.

use std::cell::RefCell;
use std::rc::Rc;

use egui::accesskit::Role;
use egui_kittest::Harness;
use egui_kittest::kittest::Queryable;
use fluent2_egui::controls::Button;
use fluent2_egui::foundation::motion;
use fluent2_egui::surfaces::{
    BottomSheet, CLOSE_LABEL, CloseReason, DialogKind, DialogResult, Drawer, DrawerPosition, DrawerSize,
};
use fluent2_egui::{Theme, install};
use fluent2_tokens::FLUENT;

/// Which surface a harness shows.
#[derive(Clone, Copy)]
enum Surface {
    Drawer(DrawerPosition),
    Sheet,
}

#[derive(Default)]
struct Probe {
    open: bool,
    closed_with: Option<CloseReason>,
}

fn record<R>(probe: &RefCell<Probe>, was_open: bool, open: bool, result: DialogResult<R>) {
    let mut state = probe.borrow_mut();
    state.open = open;
    if let (true, DialogResult::Closed(reason)) = (was_open, result) {
        state.closed_with = Some(reason);
    }
}

fn open_probe() -> Rc<RefCell<Probe>> {
    Rc::new(RefCell::new(Probe {
        open: true,
        closed_with: None,
    }))
}

/// An outside "Open" button (focused while the surface is closed) and one surface titled
/// "Panel title" with an "Inside" button.
fn harness(probe: Rc<RefCell<Probe>>, surface: Surface, kind: DialogKind, reduced: bool) -> Harness<'static> {
    // One 60 Hz frame per step; kittest's default step is as long as the whole slide.
    Harness::builder()
        .with_size(egui::vec2(800.0, 600.0))
        .with_step_dt(1.0 / 60.0)
        .build_ui(move |ui| {
            install(ui.ctx(), &Theme::default());
            motion::set_reduced_motion(ui.ctx(), reduced);
            let opener = ui.add(Button::new("Open").id_salt("opener"));
            let mut open = probe.borrow().open;
            let was_open = open;
            if !open {
                opener.request_focus();
            }
            let body = |ui: &mut egui::Ui| {
                ui.label("Body");
                let _ = ui.add(Button::new("Inside").id_salt("inside"));
            };
            let result = match surface {
                Surface::Drawer(position) => Drawer::new("panel")
                    .title("Panel title")
                    .position(position)
                    .kind(kind)
                    .show(ui.ctx(), &mut open, body),
                Surface::Sheet => {
                    BottomSheet::new("panel")
                        .title("Panel title")
                        .kind(kind)
                        .show(ui.ctx(), &mut open, body)
                }
            };
            record(&probe, was_open, open, result);
        })
}

const SURFACES: [Surface; 3] = [
    Surface::Drawer(DrawerPosition::Start),
    Surface::Drawer(DrawerPosition::End),
    Surface::Sheet,
];

#[test]
fn reports_dialog_role_and_title() {
    for surface in SURFACES {
        let mut harness = harness(open_probe(), surface, DialogKind::Modal, true);
        harness.run();
        let _ = harness.get_by_role_and_label(Role::Dialog, "Panel title");
    }
    let mut harness = harness(open_probe(), Surface::Sheet, DialogKind::Alert, true);
    harness.run();
    let _ = harness.get_by_role_and_label(Role::AlertDialog, "Panel title");
}

#[test]
fn renders_in_every_theme() {
    for theme in [Theme::light(FLUENT), Theme::dark(FLUENT), Theme::high_contrast()] {
        for surface in SURFACES {
            let probe = open_probe();
            let theme = theme.clone();
            let mut harness = Harness::new_ui(move |ui| {
                install(ui.ctx(), &theme);
                let mut open = probe.borrow().open;
                let _ = match surface {
                    Surface::Drawer(position) => Drawer::new("panel")
                        .title("Panel title")
                        .position(position)
                        .show(ui.ctx(), &mut open, |ui| ui.label("Body")),
                    Surface::Sheet => {
                        BottomSheet::new("panel")
                            .title("Panel title")
                            .show(ui.ctx(), &mut open, |ui| ui.label("Body"))
                    }
                };
            });
            harness.run();
        }
    }
}

#[test]
fn escape_closes_modal_and_non_modal_but_not_alert() {
    for surface in SURFACES {
        for kind in [DialogKind::Modal, DialogKind::NonModal] {
            let probe = open_probe();
            let mut harness = harness(probe.clone(), surface, kind, true);
            harness.run();
            harness.key_press(egui::Key::Escape);
            harness.run();
            assert!(!probe.borrow().open, "Escape should close {kind:?}");
            assert_eq!(probe.borrow().closed_with, Some(CloseReason::Escape));
        }
        let probe = open_probe();
        let mut harness = harness(probe.clone(), surface, DialogKind::Alert, true);
        harness.run();
        harness.key_press(egui::Key::Escape);
        harness.run();
        assert!(probe.borrow().open, "Escape should not close an alert");
    }
}

#[test]
fn close_button_closes() {
    for surface in SURFACES {
        let probe = open_probe();
        let mut harness = harness(probe.clone(), surface, DialogKind::Alert, true);
        harness.run();
        harness.get_by_label(CLOSE_LABEL).click();
        harness.run();
        assert!(!probe.borrow().open);
        assert_eq!(probe.borrow().closed_with, Some(CloseReason::Button(CLOSE_LABEL)));
    }
}

#[test]
fn backdrop_click_closes_only_modal() {
    // The Start drawer is 320 wide in an 800 wide window: (700, 300) is on the backdrop.
    let backdrop = egui::pos2(700.0, 300.0);
    for (kind, closes) in [(DialogKind::Modal, true), (DialogKind::NonModal, false)] {
        let probe = open_probe();
        let mut harness = harness(probe.clone(), Surface::Drawer(DrawerPosition::Start), kind, true);
        harness.run();
        harness.event(egui::Event::PointerMoved(backdrop));
        harness.event(egui::Event::PointerButton {
            pos: backdrop,
            button: egui::PointerButton::Primary,
            pressed: true,
            modifiers: egui::Modifiers::NONE,
        });
        harness.event(egui::Event::PointerButton {
            pos: backdrop,
            button: egui::PointerButton::Primary,
            pressed: false,
            modifiers: egui::Modifiers::NONE,
        });
        harness.run();
        assert_eq!(!probe.borrow().open, closes, "{kind:?}");
        if closes {
            assert_eq!(probe.borrow().closed_with, Some(CloseReason::Backdrop));
        }
    }
}

/// Which stop inside the surface has focus. Panics if focus escaped to the page.
fn focus_stop(harness: &Harness<'_>) -> &'static str {
    assert!(
        !harness.get_by_label("Open").is_focused(),
        "focus escaped the surface to the page"
    );
    if harness.get_by_label(CLOSE_LABEL).is_focused() {
        "close"
    } else if harness.get_by_label("Inside").is_focused() {
        "Inside"
    } else {
        assert!(
            harness
                .get_by_role_and_label(Role::Dialog, "Panel title")
                .is_focused(),
            "focus is on no stop inside the surface"
        );
        "surface"
    }
}

#[test]
fn tab_and_shift_tab_stay_inside() {
    for surface in SURFACES {
        let probe = open_probe();
        let mut harness = harness(probe.clone(), surface, DialogKind::Modal, true);
        harness.run();
        harness.run();
        assert_eq!(focus_stop(&harness), "surface", "opening focuses the surface");
        let mut forward = Vec::new();
        for _ in 0..3 {
            harness.key_press(egui::Key::Tab);
            harness.run();
            forward.push(focus_stop(&harness));
        }
        assert_eq!(forward, ["close", "Inside", "surface"], "Tab wraps last to first");
        let mut backward = Vec::new();
        for _ in 0..3 {
            harness.key_press_modifiers(egui::Modifiers::SHIFT, egui::Key::Tab);
            harness.run();
            backward.push(focus_stop(&harness));
        }
        assert_eq!(
            backward,
            ["Inside", "close", "surface"],
            "Shift+Tab wraps first to last"
        );
        assert!(probe.borrow().open);
    }
}

#[test]
fn focus_returns_to_opener() {
    for surface in SURFACES {
        let probe = open_probe();
        probe.borrow_mut().open = false;
        let mut harness = harness(probe.clone(), surface, DialogKind::Modal, true);
        harness.run();
        assert!(harness.get_by_label("Open").is_focused());
        probe.borrow_mut().open = true;
        harness.run();
        assert!(
            !harness.get_by_label("Open").is_focused(),
            "focus moved into the surface"
        );
        harness.key_press(egui::Key::Escape);
        harness.run();
        assert!(!probe.borrow().open);
        harness.run();
        assert!(
            harness.get_by_label("Open").is_focused(),
            "focus returns to the opener on close"
        );
    }
}

fn surface_rect(harness: &Harness<'_>) -> egui::Rect {
    harness.get_by_role_and_label(Role::Dialog, "Panel title").rect()
}

#[test]
fn reduced_motion_snaps_into_place() {
    let probe = open_probe();
    probe.borrow_mut().open = false;
    let mut harness = harness(
        probe.clone(),
        Surface::Drawer(DrawerPosition::Start),
        DialogKind::Modal,
        true,
    );
    harness.run();
    probe.borrow_mut().open = true;
    harness.step();
    harness.step();
    let rect = surface_rect(&harness);
    assert!(
        rect.left() > 0.0,
        "reduced motion: in place on the first frame, got {rect:?}"
    );

    let probe = open_probe();
    probe.borrow_mut().open = false;
    let mut harness = harness_sheet(probe.clone(), true);
    harness.run();
    probe.borrow_mut().open = true;
    harness.step();
    harness.step();
    let rect = surface_rect(&harness);
    assert!(
        rect.bottom() < 600.0,
        "reduced motion: sheet in place, got {rect:?}"
    );
}

fn harness_sheet(probe: Rc<RefCell<Probe>>, reduced: bool) -> Harness<'static> {
    harness(probe, Surface::Sheet, DialogKind::Modal, reduced)
}

#[test]
fn full_motion_slides_in_from_the_edge() {
    let probe = open_probe();
    probe.borrow_mut().open = false;
    let mut harness = harness(
        probe.clone(),
        Surface::Drawer(DrawerPosition::Start),
        DialogKind::Modal,
        false,
    );
    harness.run();
    probe.borrow_mut().open = true;
    harness.step();
    harness.step();
    let early = surface_rect(&harness);
    assert!(early.left() < -1.0, "starts off the start edge, got {early:?}");
    harness.run_steps(30);
    let settled = surface_rect(&harness);
    assert!(settled.left() > 0.0, "settles at the edge, got {settled:?}");

    let probe = open_probe();
    probe.borrow_mut().open = false;
    let mut harness = harness_sheet(probe.clone(), false);
    harness.run();
    probe.borrow_mut().open = true;
    harness.step();
    harness.step();
    let early = surface_rect(&harness);
    assert!(
        early.bottom() > 601.0,
        "starts below the bottom edge, got {early:?}"
    );
}

#[test]
fn inline_drawer_does_not_trap_or_dim() {
    let open = Rc::new(RefCell::new(true));
    let mut harness = {
        let open = open.clone();
        Harness::builder()
            .with_size(egui::vec2(800.0, 600.0))
            .build_ui(move |ui| {
                install(ui.ctx(), &Theme::default());
                motion::set_reduced_motion(ui.ctx(), true);
                let mut shown = *open.borrow();
                let _ = Drawer::new("nav")
                    .title("Navigation")
                    .size(DrawerSize::Custom(240.0))
                    .show_inline(ui, &mut shown, |ui| {
                        let _ = ui.add(Button::new("Inside").id_salt("inside"));
                    });
                *open.borrow_mut() = shown;
                let _ = ui.add(Button::new("Page").id_salt("page"));
            })
    };
    harness.run();
    let _ = harness.get_by_role_and_label(Role::Complementary, "Navigation");
    let mut reached_page = false;
    for _ in 0..6 {
        harness.key_press(egui::Key::Tab);
        harness.run();
        reached_page |= harness.get_by_label("Page").is_focused();
    }
    assert!(reached_page, "Tab reaches the page beside an inline drawer");
    harness.key_press(egui::Key::Escape);
    harness.run();
    assert!(*open.borrow(), "Escape does not close an inline drawer");
    harness.get_by_label(CLOSE_LABEL).click();
    harness.run();
    assert!(!*open.borrow(), "the close button closes it");
}

#[test]
fn closed_when_not_open_is_programmatic() {
    let ctx = egui::Context::default();
    let mut open = false;
    let result = Drawer::new("d").show::<()>(&ctx, &mut open, |_| {});
    assert_eq!(result, DialogResult::Closed(CloseReason::Programmatic));
    let result = BottomSheet::new("s").show::<()>(&ctx, &mut open, |_| {});
    assert_eq!(result, DialogResult::Closed(CloseReason::Programmatic));
}

#[test]
fn sizes_clamp_to_the_window() {
    assert_eq!(DrawerSize::Small.width(1000.0), 320.0);
    assert_eq!(DrawerSize::Medium.width(1000.0), 592.0);
    assert_eq!(DrawerSize::Large.width(500.0), 500.0);
    assert_eq!(DrawerSize::Full.width(700.0), 700.0);
    assert_eq!(DrawerSize::Custom(f32::NAN).width(1000.0), 320.0);
    assert_eq!(DrawerSize::Custom(-4.0).width(1000.0), 320.0);
    assert_eq!(DrawerSize::Small.width(f32::INFINITY), 0.0);
}
