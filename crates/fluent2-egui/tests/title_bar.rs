//! TitleBar behaviour, without the wgpu snapshot feature.

use std::cell::Cell;

use egui::ViewportCommand;
use egui::accesskit::Role;
use egui_kittest::Harness;
use egui_kittest::kittest::Queryable;
use fluent2_egui::office::{TitleBar, WindowAction, window_command};
use fluent2_egui::{Icon, IconName, Theme, install};

/// Harness state a test reads back. Each test owns its own cells.
struct Probe {
    /// The last action the bar returned. A frame with no action leaves it unchanged.
    action: Cell<Option<WindowAction>>,
    /// The `inactive` flag from the last frame.
    inactive: Cell<bool>,
}

fn probe() -> &'static Probe {
    Box::leak(Box::new(Probe {
        action: Cell::new(None),
        inactive: Cell::new(false),
    }))
}

fn title_bar_harness(probe: &'static Probe, maximized: bool, focused: Option<bool>) -> Harness<'static> {
    let mut harness = Harness::new_ui(move |ui| {
        install(ui.ctx(), &Theme::default());
        let response = TitleBar::new("title_bar_test", "Document")
            .maximized(maximized)
            .focused(focused)
            .show(ui, |slots| {
                slots.app_icon(Icon::new(IconName::Document));
                slots.left(|ui| {
                    ui.label("Left slot");
                });
                slots.right(|ui| {
                    ui.label("Right slot");
                });
            });
        if let Some(action) = response.action {
            probe.action.set(Some(action));
        }
        probe.inactive.set(response.inactive);
    });
    harness.run();
    harness.run();
    harness
}

#[test]
fn caption_buttons_are_found_by_accessible_label() {
    let harness = title_bar_harness(probe(), false, Some(true));
    harness.get_by_role_and_label(Role::Button, "Minimize");
    harness.get_by_role_and_label(Role::Button, "Maximize");
    harness.get_by_role_and_label(Role::Button, "Close");
}

#[test]
fn maximise_button_reads_restore_when_maximised() {
    let harness = title_bar_harness(probe(), true, Some(true));
    harness.get_by_role_and_label(Role::Button, "Restore");
}

#[test]
fn minimise_click_requests_minimise() {
    let probe = probe();
    let mut harness = title_bar_harness(probe, false, Some(true));
    harness.get_by_role_and_label(Role::Button, "Minimize").click();
    harness.run();
    assert_eq!(probe.action.get(), Some(WindowAction::Minimize));
    assert_eq!(
        window_command(WindowAction::Minimize, false),
        ViewportCommand::Minimized(true)
    );
}

#[test]
fn maximise_click_toggles_maximise() {
    let probe = probe();
    let mut harness = title_bar_harness(probe, false, Some(true));
    harness.get_by_role_and_label(Role::Button, "Maximize").click();
    harness.run();
    assert_eq!(probe.action.get(), Some(WindowAction::ToggleMaximize));
    assert_eq!(
        window_command(WindowAction::ToggleMaximize, false),
        ViewportCommand::Maximized(true)
    );
    assert_eq!(
        window_command(WindowAction::ToggleMaximize, true),
        ViewportCommand::Maximized(false)
    );
}

#[test]
fn close_click_requests_close() {
    let probe = probe();
    let mut harness = title_bar_harness(probe, false, Some(true));
    harness.get_by_role_and_label(Role::Button, "Close").click();
    harness.run();
    assert_eq!(probe.action.get(), Some(WindowAction::Close));
    assert_eq!(window_command(WindowAction::Close, false), ViewportCommand::Close);
}

#[test]
fn drag_maps_to_start_drag() {
    assert_eq!(
        window_command(WindowAction::StartDrag, false),
        ViewportCommand::StartDrag
    );
}

#[test]
fn slots_render_caller_content() {
    let harness = title_bar_harness(probe(), false, Some(true));
    harness.get_by_label("Left slot");
    harness.get_by_label("Right slot");
}

#[test]
fn unfocused_window_is_reported_inactive() {
    let probe = probe();
    let _harness = title_bar_harness(probe, false, Some(false));
    assert!(probe.inactive.get());
}

#[test]
fn focused_window_is_active() {
    let probe = probe();
    let _harness = title_bar_harness(probe, false, Some(true));
    assert!(!probe.inactive.get());
}
