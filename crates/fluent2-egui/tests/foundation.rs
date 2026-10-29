//! Foundation helpers paint without panicking inside a kittest frame.

use egui::{CornerRadius, Id, Rect};
use fluent2_egui::Theme;
use fluent2_egui::foundation::elevation::Elevation;
use fluent2_egui::foundation::motion::{self, Curve, Duration};
use fluent2_egui::foundation::{elevation, focus};

#[test]
fn focus_ring_and_shadow_paint_in_a_frame() {
    let theme = Theme::default();
    let mut harness = egui_kittest::Harness::new_ui(move |ui| {
        let rect = Rect::from_min_size(ui.min_rect().min, egui::vec2(40.0, 24.0));
        let radius = CornerRadius::same(4);
        focus::paint_focus_ring(ui, rect, radius);
        elevation::paint_shadow(ui.painter(), rect, radius, Elevation::E16, &theme);
    });
    harness.run();
}

struct MotionRun {
    target: f32,
    samples: Vec<f32>,
}

fn retarget(curve: Curve) -> (f32, f32) {
    let mut harness = egui_kittest::Harness::new_ui_state(
        move |ui, state: &mut MotionRun| {
            let value = motion::animate(
                ui.ctx(),
                Id::unique("retarget"),
                state.target,
                Duration::UltraSlow,
                curve,
            );
            state.samples.push(value);
        },
        MotionRun {
            target: 100.0,
            samples: Vec::new(),
        },
    );
    for _ in 0..4 {
        harness.input_mut().predicted_dt = 0.025;
        harness.step();
    }
    let before = harness.state().samples.last().copied().unwrap_or(f32::NAN);
    harness.state_mut().target = 0.0;
    harness.input_mut().predicted_dt = 0.025;
    harness.step();
    let after = harness.state().samples.last().copied().unwrap_or(f32::NAN);
    (before, after)
}

#[test]
fn accelerate_retarget_stays_on_the_eased_value() {
    let (before, after) = retarget(Curve::AccelerateMid);
    assert!(
        (after - before).abs() < 1.0,
        "accelerate jumped from {before} to {after}"
    );
    assert!(before > 1.0, "never left the start: {before}");
}

#[test]
fn decelerate_retarget_stays_on_the_eased_value() {
    let (before, after) = retarget(Curve::DecelerateMid);
    assert!(
        (after - before).abs() < 1.0,
        "decelerate jumped from {before} to {after}"
    );
    assert!(before > 1.0, "never left the start: {before}");
}
