//! Link, field and progress bar behaviour, without the wgpu snapshot feature.

use std::cell::Cell;

use egui::accesskit::Role;
use egui_kittest::Harness;
use egui_kittest::kittest::Queryable;
use fluent2_egui::controls::{Field, Link, ProgressBar, ValidationState};
use fluent2_egui::theme::{self, Theme};

fn install(ui: &egui::Ui) {
    theme::install(ui.ctx(), &Theme::default());
}

#[test]
fn link_click_sets_clicked() {
    let clicked = Cell::new(false);
    let mut harness = Harness::new_ui(|ui| {
        install(ui);
        if ui.add(Link::new("Details")).clicked() {
            clicked.set(true);
        }
    });
    harness.run();
    harness.get_by_role_and_label(Role::Link, "Details").click();
    harness.run();
    assert!(clicked.get());
}

#[test]
fn field_shows_required_marker_and_validation() {
    let mut harness = Harness::new_ui(|ui| {
        install(ui);
        Field::new("Name")
            .required(true)
            .validation(ValidationState::Error("Required".into()))
            .show(ui, |ui| {
                ui.label("child");
            });
    });
    harness.run();
    let _ = harness.get_by_label("Required");
    let _ = harness.get_by_label("Name *");
}

#[test]
fn indeterminate_progress_requests_repaint_determinate_does_not() {
    let mut indeterminate = Harness::builder().build_ui(|ui| {
        install(ui);
        let _ = ui.add(ProgressBar::new(None));
    });
    indeterminate.step();
    assert!(indeterminate.ctx.has_requested_repaint());

    let mut determinate = Harness::new_ui(|ui| {
        install(ui);
        let _ = ui.add(ProgressBar::new(Some(0.5)));
    });
    determinate.run();
    assert!(!determinate.ctx.has_requested_repaint());
}

#[test]
fn non_finite_progress_does_not_panic() {
    let mut harness = Harness::new_ui(|ui| {
        install(ui);
        let _ = ui.add(ProgressBar::new(Some(f32::NAN)));
        let _ = ui.add(ProgressBar::new(Some(f32::INFINITY)));
        let _ = ui.add(ProgressBar::new(Some(f32::NEG_INFINITY)));
        let _ = ui.add(ProgressBar::new(Some(2.0)));
    });
    harness.run();
}

#[test]
fn reduced_motion_holds_the_indeterminate_bar_still() {
    let mut harness = Harness::builder().build_ui(|ui| {
        install(ui);
        fluent2_egui::foundation::motion::set_reduced_motion(ui.ctx(), true);
        let value = fluent2_egui::foundation::motion::animate(
            ui.ctx(),
            egui::Id::unique("sweep-check"),
            1.0,
            fluent2_egui::foundation::motion::Duration::UltraSlow,
            fluent2_egui::foundation::motion::Curve::Linear,
        );
        assert_eq!(value, 1.0, "reduced motion snaps to the target");
        let _ = ui.add(ProgressBar::new(None));
    });
    harness.step();
    harness.step();
    // The still segment needs no further frames (WI-1586).
    assert!(!harness.ctx.has_requested_repaint());
}
