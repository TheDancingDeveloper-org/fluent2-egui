//! NavigationView (§7) and SplitView (§8) behaviour, without the wgpu snapshot feature.

use std::cell::{Cell, RefCell};

use egui::accesskit::Role;
use egui::{Key, Modifiers, Vec2};
use egui_kittest::Harness;
use egui_kittest::kittest::Queryable;
use fluent2_egui::data::nav_view::{NavState, NavigationView, PaneMode};
use fluent2_egui::data::split_view::{SplitAxis, SplitSize, SplitState, SplitView};
use fluent2_egui::{Theme, install};
use fluent2_icons::Icon;

/// What a navigation test reads back from the harness. Each test leaks its own.
struct NavProbe {
    state: RefCell<NavState>,
    selected: RefCell<Vec<&'static str>>,
}

fn nav_probe() -> &'static NavProbe {
    Box::leak(Box::new(NavProbe {
        state: RefCell::new(NavState::default()),
        selected: RefCell::new(Vec::new()),
    }))
}

fn nav_harness(width: f32, probe: &'static NavProbe) -> Harness<'static> {
    let mut harness = Harness::builder()
        .with_size(Vec2::new(width, 600.0))
        .build_ui(move |ui| {
            install(ui.ctx(), &Theme::default());
            let mut state = probe.state.borrow_mut();
            let response = NavigationView::new("nav")
                .title("Library")
                .item("home", Icon::Home, "Home")
                .item("files", Icon::Document, "Files")
                .section("Other")
                .item("trash", Icon::Dismiss, "Trash")
                .footer_item("settings", Icon::Settings, "Settings")
                .show(ui, &mut state, |ui| {
                    ui.label("Content");
                });
            if let Some(key) = response.selected {
                probe.selected.borrow_mut().push(key);
            }
        });
    harness.run();
    harness
}

fn selected_key(probe: &NavProbe) -> Option<String> {
    probe.state.borrow().selected.clone()
}

#[test]
fn mode_follows_the_breakpoints() {
    // Expanded at 1008 pt and up: the pane is 260 pt wide.
    let wide = nav_harness(1200.0, nav_probe());
    let pane = wide.get_by_role(Role::Navigation).rect();
    assert!(
        (pane.width() - 260.0).abs() < 1.0,
        "expanded pane is {} wide",
        pane.width()
    );

    // Compact from 641 to 1007 pt: an icon rail of 48 pt.
    let middle = nav_harness(800.0, nav_probe());
    let rail = middle.get_by_role(Role::Navigation).rect();
    assert!(
        (rail.width() - 48.0).abs() < 1.0,
        "compact rail is {} wide",
        rail.width()
    );

    // Minimal at 640 pt and below: no pane, a hamburger over the content.
    let narrow = nav_harness(400.0, nav_probe());
    assert!(narrow.query_all_by_role(Role::Navigation).next().is_none());
    narrow.get_by_role_and_label(Role::Button, "Open navigation");
}

#[test]
fn a_fixed_mode_overrides_the_breakpoints() {
    let probe = nav_probe();
    probe.state.borrow_mut().mode = PaneMode::Minimal;
    let harness = nav_harness(1200.0, probe);
    assert!(harness.query_all_by_role(Role::Navigation).next().is_none());
}

#[test]
fn a_click_selects_the_item() {
    let probe = nav_probe();
    let mut harness = nav_harness(1200.0, probe);
    harness.get_by_role_and_label(Role::Tab, "Files").click();
    harness.run();
    assert_eq!(selected_key(probe).as_deref(), Some("files"));
    assert_eq!(probe.selected.borrow().as_slice(), &["files"]);
}

#[test]
fn arrow_keys_move_focus_and_enter_selects() {
    let probe = nav_probe();
    let mut harness = nav_harness(1200.0, probe);
    // The hamburger is the first Tab stop; the pane is the second.
    harness.key_press(Key::Tab);
    harness.key_press(Key::Tab);
    harness.key_press(Key::ArrowDown);
    harness.key_press(Key::Enter);
    harness.run();
    assert_eq!(selected_key(probe).as_deref(), Some("files"));
}

#[test]
fn the_minimal_overlay_closes_on_selection_and_escape() {
    let probe = nav_probe();
    probe.state.borrow_mut().mode = PaneMode::Minimal;
    let mut harness = nav_harness(400.0, probe);
    harness
        .get_by_role_and_label(Role::Button, "Open navigation")
        .click();
    harness.run();
    harness.get_by_role_and_label(Role::Tab, "Home");

    harness.get_by_role_and_label(Role::Tab, "Files").click();
    harness.run();
    assert_eq!(selected_key(probe).as_deref(), Some("files"));
    assert!(
        harness.query_all_by_role(Role::Tab).next().is_none(),
        "overlay should close on selection"
    );

    harness
        .get_by_role_and_label(Role::Button, "Open navigation")
        .click();
    harness.run();
    harness.get_by_role_and_label(Role::Tab, "Home");
    harness.key_press(Key::Escape);
    harness.run();
    assert!(
        harness.query_all_by_role(Role::Tab).next().is_none(),
        "Escape should close the overlay"
    );
}

/// What a split test reads back from the harness.
struct SplitProbe {
    state: RefCell<SplitState>,
    /// The width the split was laid out in, read on the first frame.
    width: Cell<f32>,
}

fn split_probe(size: SplitSize) -> &'static SplitProbe {
    Box::leak(Box::new(SplitProbe {
        state: RefCell::new(SplitState { size }),
        width: Cell::new(0.0),
    }))
}

fn split_harness(probe: &'static SplitProbe) -> Harness<'static> {
    let mut harness = Harness::builder()
        .with_size(Vec2::new(400.0, 200.0))
        .build_ui(move |ui| {
            install(ui.ctx(), &Theme::default());
            probe.width.set(ui.available_width());
            let mut state = probe.state.borrow_mut();
            SplitView::new("split", SplitAxis::Horizontal)
                .min_sizes(100.0, 100.0)
                .label("Resize editor")
                .show(
                    ui,
                    &mut state,
                    |ui| {
                        ui.label("left");
                    },
                    |ui| {
                        ui.label("right");
                    },
                );
        });
    harness.run();
    harness
}

/// The first pane's width in points. The space for both panes is the laid-out width less the
/// 1 pt divider.
fn first_width(probe: &SplitProbe) -> f32 {
    let space = probe.width.get() - 1.0;
    match probe.state.borrow().size {
        SplitSize::Ratio(ratio) => ratio * space,
        SplitSize::FixedFirst(points) => points,
        SplitSize::FixedSecond(points) => space - points,
    }
}

#[test]
fn keyboard_resizing_stops_at_the_minimum() {
    let probe = split_probe(SplitSize::Ratio(0.5));
    let mut harness = split_harness(probe);
    harness.key_press(Key::Tab);
    for _ in 0..60 {
        harness.key_press(Key::ArrowLeft);
    }
    harness.run();
    assert!(
        (first_width(probe) - 100.0).abs() < 0.5,
        "first pane is {}",
        first_width(probe)
    );
}

#[test]
fn dragging_the_divider_respects_the_minimum() {
    let probe = split_probe(SplitSize::Ratio(0.5));
    let mut harness = split_harness(probe);
    let grip = harness.get_by_role(Role::Splitter).rect().center();
    let press = |pressed| egui::Event::PointerButton {
        pos: grip,
        button: egui::PointerButton::Primary,
        pressed,
        modifiers: Modifiers::NONE,
    };
    harness.event(egui::Event::PointerMoved(grip));
    harness.event(press(true));
    harness.run();
    harness.event(egui::Event::PointerMoved(grip - Vec2::new(500.0, 0.0)));
    harness.run();
    harness.event(press(false));
    harness.run();
    assert!(
        (first_width(probe) - 100.0).abs() < 0.5,
        "drag left the first pane at {}",
        first_width(probe)
    );
}

#[test]
fn shift_moves_ten_steps_and_home_and_end_reach_the_limits() {
    let probe = split_probe(SplitSize::Ratio(0.5));
    let mut harness = split_harness(probe);
    harness.key_press(Key::Tab);
    let start = first_width(probe);
    harness.key_press_modifiers(Modifiers::SHIFT, Key::ArrowRight);
    harness.run();
    assert!(
        (first_width(probe) - (start + 80.0)).abs() < 0.5,
        "shift step moved {}",
        first_width(probe) - start
    );

    harness.key_press(Key::End);
    harness.run();
    let space = probe.width.get() - 1.0;
    assert!(
        (first_width(probe) - (space - 100.0)).abs() < 0.5,
        "End put the first pane at {}",
        first_width(probe)
    );
    harness.key_press(Key::Home);
    harness.run();
    assert!(
        (first_width(probe) - 100.0).abs() < 0.5,
        "Home put the first pane at {}",
        first_width(probe)
    );
}

#[test]
fn non_finite_sizes_do_not_panic_and_reset_to_an_even_split() {
    let probe = split_probe(SplitSize::Ratio(f32::NAN));
    let _harness = split_harness(probe);
    assert_eq!(probe.state.borrow().size, SplitSize::Ratio(0.5));

    let probe = split_probe(SplitSize::FixedFirst(f32::INFINITY));
    let _harness = split_harness(probe);
    assert_eq!(probe.state.borrow().size, SplitSize::Ratio(0.5));
}

#[test]
fn navigation_and_split_render_in_every_theme() {
    let themes = [
        Theme::light(fluent2_tokens::FLUENT),
        Theme::dark(fluent2_tokens::FLUENT),
        Theme::high_contrast(),
    ];
    for theme in themes {
        let nav = NavState::default();
        let theme_copy = theme.clone();
        let mut harness = Harness::builder()
            .with_size(Vec2::new(900.0, 400.0))
            .build_ui(move |ui| {
                install(ui.ctx(), &theme);
                let mut state = nav.clone();
                NavigationView::new("themed")
                    .item("home", Icon::Home, "Home")
                    .show(ui, &mut state, |ui| {
                        ui.label("Content");
                    });
            });
        harness.run();

        let split = SplitState::default();
        let mut harness = Harness::builder()
            .with_size(Vec2::new(400.0, 200.0))
            .build_ui(move |ui| {
                install(ui.ctx(), &theme_copy);
                let mut state = split;
                SplitView::new("themed-split", SplitAxis::Vertical).show(
                    ui,
                    &mut state,
                    |ui| {
                        ui.label("top");
                    },
                    |ui| {
                        ui.label("bottom");
                    },
                );
            });
        harness.run();
    }
}
