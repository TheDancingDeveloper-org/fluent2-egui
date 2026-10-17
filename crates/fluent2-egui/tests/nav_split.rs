//! NavigationView (§7) and SplitView (§8) behaviour, without the wgpu snapshot feature.

use std::cell::{Cell, RefCell};

use egui::accesskit::Role;
use egui::{Key, Modifiers, Vec2};
use egui_kittest::Harness;
use egui_kittest::kittest::{NodeT, Queryable};
use fluent2_egui::data::nav_view::{NavState, NavigationView, PaneMode};
use fluent2_egui::data::split_view::{SplitAxis, SplitSize, SplitState, SplitView};
use fluent2_egui::foundation::motion;
use fluent2_egui::{Theme, install};
use fluent2_icons::Icon;

/// What a navigation test reads back from the harness. Each test leaks its own.
struct NavProbe {
    state: RefCell<NavState>,
    selected: RefCell<Vec<&'static str>>,
    /// The width of the `Ui` the view was laid out in, read on the first frame.
    width: Cell<f32>,
}

fn nav_probe() -> &'static NavProbe {
    Box::leak(Box::new(NavProbe {
        state: RefCell::new(NavState::default()),
        selected: RefCell::new(Vec::new()),
        width: Cell::new(0.0),
    }))
}

fn nav_harness(width: f32, probe: &'static NavProbe) -> Harness<'static> {
    build_nav(width, probe, false)
}

fn build_nav(width: f32, probe: &'static NavProbe, reduced_motion: bool) -> Harness<'static> {
    let mut harness = Harness::builder()
        .with_size(Vec2::new(width, 600.0))
        .build_ui(move |ui| {
            install(ui.ctx(), &Theme::default());
            motion::set_reduced_motion(ui.ctx(), reduced_motion);
            probe.width.set(ui.available_width());
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

/// The pane width the harness shows, or `None` when there is no pane (Minimal mode).
fn pane_width(harness: &Harness<'_>) -> Option<f32> {
    harness
        .query_all_by_role(Role::Navigation)
        .next()
        .map(|node| node.rect().width())
}

#[test]
fn breakpoints_are_exact_at_the_boundaries() {
    // The `Ui` the view sits in is narrower than the window by the panel margin. Measure the
    // margin once, then lay the view out at exact `Ui` widths.
    let probe = nav_probe();
    let _ = nav_harness(1000.0, probe);
    let margin = 1000.0 - probe.width.get();

    // (Ui width, expected pane width): Minimal has no pane, Compact a 48 pt rail, Expanded 260.
    let cases = [
        (640.0, None),
        (641.0, Some(48.0)),
        (1007.0, Some(48.0)),
        (1008.0, Some(260.0)),
    ];
    for (ui_width, expected) in cases {
        let probe = nav_probe();
        let harness = nav_harness(ui_width + margin, probe);
        assert!(
            (probe.width.get() - ui_width).abs() < 0.5,
            "laid out at {} not {ui_width}",
            probe.width.get()
        );
        match (pane_width(&harness), expected) {
            (None, None) => {
                harness.get_by_role_and_label(Role::Button, "Open navigation");
            }
            (Some(actual), Some(want)) => {
                assert!(
                    (actual - want).abs() < 1.0,
                    "at {ui_width} pt the pane is {actual} wide, want {want}"
                );
            }
            (actual, want) => panic!("at {ui_width} pt the pane is {actual:?}, want {want:?}"),
        }
    }
}

#[test]
fn the_overlay_slides_in_and_snaps_under_reduced_motion() {
    for reduced in [false, true] {
        let probe = nav_probe();
        probe.state.borrow_mut().mode = PaneMode::Minimal;
        let mut harness = build_nav(400.0, probe, reduced);
        harness
            .get_by_role_and_label(Role::Button, "Open navigation")
            .click();
        // The click opens the overlay on the next frame; the slide starts from there.
        harness.run_steps(2);
        let x = harness.get_by_role(Role::Navigation).rect().min.x;
        harness.run();
        let settled = harness.get_by_role(Role::Navigation).rect().min.x;
        if reduced {
            assert!(
                (x - settled).abs() < 0.5,
                "reduced motion should snap: {x} then {settled}"
            );
        } else {
            assert!(
                x < settled - 1.0,
                "overlay should still be sliding in: {x} before {settled}"
            );
        }
    }
}

#[test]
fn items_are_tabs_in_a_vertical_tablist_and_headings_are_level_two() {
    let harness = nav_harness(1200.0, nav_probe());
    let list = harness.get_by_role(Role::TabList);
    let files = harness.get_by_role_and_label(Role::Tab, "Files");
    let node = files.accesskit_node();
    let data = node.data();
    assert_eq!(data.position_in_set(), Some(2), "Files is item 2");
    assert_eq!(data.size_of_set(), Some(4), "four items in the navigation order");
    assert_eq!(
        list.accesskit_node().data().orientation(),
        Some(egui::accesskit::Orientation::Vertical)
    );
    assert!(
        list.children_recursive()
            .any(|child| child.accesskit_node().id() == node.id()),
        "Files sits inside the TabList"
    );
    let found: Vec<_> = harness
        .query_all_by_role(Role::Heading)
        .map(|n| {
            (
                n.accesskit_node().data().label().map(String::from),
                n.accesskit_node().data().level(),
            )
        })
        .collect();
    assert!(
        found.iter().any(|(label, _)| label.as_deref() == Some("Other")),
        "headings: {found:?}"
    );
    let heading = harness.get_by_role_and_label(Role::Heading, "Other");
    assert_eq!(heading.accesskit_node().data().level(), Some(2));
}

/// Count the 1 pt-high filled rectangles one frame of `build` paints: the divider lines.
fn one_point_lines(build: impl Fn(&mut egui::Ui)) -> usize {
    let ctx = egui::Context::default();
    install(&ctx, &Theme::default());
    let mut lines = 0;
    for _ in 0..2 {
        let raw = egui::RawInput {
            screen_rect: Some(egui::Rect::from_min_size(
                egui::Pos2::ZERO,
                Vec2::new(800.0, 600.0),
            )),
            ..Default::default()
        };
        let mut output = ctx.run_ui(raw, |ui| build(ui));
        lines = output
            .shapes
            .iter()
            .filter(|clipped| matches!(&clipped.shape, egui::Shape::Rect(rect) if rect.rect.height() == 1.0))
            .count();
        // egui panics if texture deltas are dropped unhandled; this test does not upload them.
        output.textures_delta.clear();
    }
    lines
}

#[test]
fn a_compact_section_draws_a_divider_line() {
    let compact = |with_section: bool| {
        move |ui: &mut egui::Ui| {
            let mut state = NavState {
                mode: PaneMode::Compact,
                ..NavState::default()
            };
            let mut nav = NavigationView::new("lines").item("home", Icon::Home, "Home");
            if with_section {
                nav = nav.section("Other");
            }
            nav.item("files", Icon::Document, "Files")
                .show(ui, &mut state, |_| {});
        }
    };
    let without = one_point_lines(compact(false));
    let with = one_point_lines(compact(true));
    assert_eq!(
        with,
        without + 1,
        "a section in the rail becomes one divider line"
    );
}
