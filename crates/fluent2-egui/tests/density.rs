//! Density (`docs/specs/density.md`): every interactive control's allocated size in Compact,
//! Medium and Touch, the data-view metrics and the installed spacing. Kittest, no wgpu.

use std::cell::RefCell;
use std::rc::Rc;

use egui::{Rect, Vec2};
use egui_kittest::Harness;
use fluent2_egui::controls::{
    Button, CheckState, Checkbox, Combobox, Dropdown, Input, MenuButton, RadioGroup, SearchBox, Slider,
    SpinButton, SplitButton, Switch, ToggleButton,
};
use fluent2_egui::theme::{DataMetrics, Density, TOUCH_TARGET};
use fluent2_egui::{Theme, install};
use fluent2_icons::Icon;

/// Rects of each control, by name, in one frame at `density`.
fn measure(density: Density) -> Vec<(&'static str, Rect)> {
    let out: Rc<RefCell<Vec<(&'static str, Rect)>>> = Rc::default();
    let sink = Rc::clone(&out);
    let mut harness = Harness::builder()
        .with_size(Vec2::new(600.0, 1400.0))
        .build_ui(move |ui| {
            install(ui.ctx(), &Theme::default().with_density(density));
            let mut rects = Vec::new();
            let mut flag = false;
            let mut state = CheckState::Unchecked;
            let mut text = String::new();
            let mut choice = None;
            let mut radio = 0usize;
            let mut value = 0.5f64;
            let b = ui.add(Button::new("Save"));
            rects.push(("button", b.rect));
            let r = ui.add(Button::icon_only(Icon::Dismiss, "Close"));
            rects.push(("icon button", r.rect));
            let r = ui.add(ToggleButton::new(&mut flag, "Bold"));
            rects.push(("toggle", r.rect));
            let r = MenuButton::new("More").show(ui, |_| ()).0;
            rects.push(("menu button", r.rect));
            let split = SplitButton::new("Send").show(ui, |_| ());
            rects.push(("split button", split.main.rect.union(split.menu.rect)));
            rects.push(("split chevron", split.menu.rect));
            rects.push((
                "dropdown",
                ui.add(Dropdown::new("d", &mut choice, &["A", "B"]).width(200.0))
                    .rect,
            ));
            rects.push((
                "combobox",
                ui.add(Combobox::new("c", &mut text, &["A"]).width(200.0)).rect,
            ));
            let mut input = String::new();
            rects.push((
                "input",
                ui.add(Input::new(&mut input).label("Name").desired_width(200.0))
                    .rect,
            ));
            let mut query = String::new();
            rects.push((
                "search box",
                ui.add(SearchBox::new(&mut query).label("Search").desired_width(200.0))
                    .rect,
            ));
            let spin_id = ui.next_auto_id();
            rects.push((
                "spin button",
                ui.add(SpinButton::new(&mut value).label("Count")).rect,
            ));
            // Each stepper is its own hit target (DN3), not just the whole control.
            for (name, part) in [("spin up", "up"), ("spin down", "down")] {
                let rect = ui
                    .ctx()
                    .read_response(spin_id.with(part))
                    .map_or(Rect::NOTHING, |r| r.rect);
                rects.push((name, rect));
            }
            rects.push(("checkbox", ui.add(Checkbox::new(&mut state, "Remember")).rect));
            rects.push(("bare checkbox", ui.add(Checkbox::new(&mut state, "Agree")).rect));
            rects.push((
                "radio",
                RadioGroup::new(&mut radio).option("One").show(ui).response.rect,
            ));
            rects.push(("switch", ui.add(Switch::new(&mut flag, "Wi-Fi")).rect));
            rects.push(("slider", ui.add(Slider::new(&mut value, 0.0..=1.0)).rect));
            *sink.borrow_mut() = rects;
        });
    harness.run();
    out.take()
}

fn height(rects: &[(&'static str, Rect)], name: &str) -> f32 {
    rects
        .iter()
        .find(|(n, _)| *n == name)
        .map_or(f32::NAN, |(_, r)| r.height())
}

#[test]
fn medium_keeps_the_fluent_sizes() {
    let m = measure(Density::Medium);
    for name in [
        "button",
        "icon button",
        "toggle",
        "menu button",
        "split button",
        "dropdown",
        "combobox",
        "input",
        "search box",
        "spin button",
    ] {
        assert_eq!(height(&m, name), 32.0, "{name}");
    }
    assert_eq!(height(&m, "slider"), 20.0);
}

#[test]
fn compact_steps_medium_controls_down_to_24() {
    let c = measure(Density::Compact);
    for name in [
        "button",
        "icon button",
        "toggle",
        "menu button",
        "split button",
        "dropdown",
        "combobox",
        "input",
        "search box",
        "spin button",
    ] {
        assert_eq!(height(&c, name), 24.0, "{name}");
    }
    // Indicators under 32 keep their size.
    let m = measure(Density::Medium);
    for name in ["checkbox", "radio", "switch", "slider"] {
        assert_eq!(height(&c, name), height(&m, name), "{name}");
    }
}

#[test]
fn touch_makes_every_target_at_least_44() {
    let t = measure(Density::Touch);
    assert_eq!(t.len(), 18);
    for (name, rect) in &t {
        assert!(rect.height() >= TOUCH_TARGET, "{name} is {} tall", rect.height());
        assert!(rect.width() >= TOUCH_TARGET, "{name} is {} wide", rect.width());
    }
    let icon = t
        .iter()
        .find(|(n, _)| *n == "icon button")
        .map(|(_, r)| *r)
        .unwrap_or(Rect::NOTHING);
    assert_eq!(icon.size(), Vec2::splat(TOUCH_TARGET), "icon buttons are square");
}

#[test]
fn data_metrics_follow_density() {
    let m = DataMetrics::default();
    assert_eq!(DataMetrics::for_density(Density::Medium), m);
    let c = DataMetrics::for_density(Density::Compact);
    assert_eq!(
        (
            c.list_row_height,
            c.tree_row_height_medium,
            c.grid_row_height_medium
        ),
        (24.0, 24.0, 36.0)
    );
    assert_eq!(c.nav_pane_width_expanded, m.nav_pane_width_expanded);
    let t = DataMetrics::for_density(Density::Touch);
    for h in [
        t.list_row_height,
        t.tree_row_height_small,
        t.grid_row_height_extra_small,
        t.nav_item_height,
        t.grid_header_height,
    ] {
        assert!(h >= TOUCH_TARGET, "{h}");
    }
    assert_eq!(Theme::default().with_density(Density::Touch).data, t);
    assert_eq!(Theme::default().density, Density::Medium);
}

#[test]
fn install_sets_spacing_and_interact_size() {
    for density in [Density::Compact, Density::Medium, Density::Touch] {
        let ctx = egui::Context::default();
        install(&ctx, &Theme::default().with_density(density));
        let spacing = ctx.style_of(egui::Theme::Light).spacing.clone();
        assert_eq!(spacing.item_spacing, density.item_spacing(), "{density:?}");
        assert_eq!(spacing.interact_size.y, density.height(32.0), "{density:?}");
        assert_eq!(Theme::get(&ctx).density, density);
    }
}

#[test]
fn command_palette_rows_follow_density() {
    use egui::accesskit::Role;
    use egui_kittest::kittest::Queryable;
    use fluent2_egui::surfaces::{CommandPalette, PaletteCommand};

    let commands = vec![
        PaletteCommand::new("file.open", "Open file"),
        PaletteCommand::new("view.zoom", "Zoom in").subtitle("Make text larger"),
    ];
    for (density, one_line, two_lines) in [
        (Density::Compact, 24.0, 36.0),
        (Density::Medium, 32.0, 44.0),
        (Density::Touch, TOUCH_TARGET, TOUCH_TARGET),
    ] {
        let commands = commands.clone();
        let mut harness = Harness::builder()
            .with_size(Vec2::new(800.0, 600.0))
            .build_ui(move |ui| {
                install(ui.ctx(), &Theme::default().with_density(density));
                let mut open = true;
                let _ = CommandPalette::new("palette", &commands).show(ui.ctx(), &mut open);
            });
        harness.run();
        let row = |label: &str| {
            harness
                .get_by_role_and_label(Role::ListBoxOption, label)
                .rect()
                .height()
        };
        assert_eq!(row("Open file"), one_line, "{density:?}");
        assert_eq!(row("Zoom in"), two_lines, "{density:?}");
    }
}
