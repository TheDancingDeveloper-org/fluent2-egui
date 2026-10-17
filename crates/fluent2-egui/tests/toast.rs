//! Toaster behaviour: expiry, hover pause, dismissal and the visible cap.
//!
//! Time comes from the harness step: every frame advances egui's clock by [`STEP`] seconds, so
//! a test advances time by running frames. The margins below are wide enough that the first
//! frames, when the Area is still settling, do not change the outcome.

use std::time::Duration;

use egui_kittest::Harness;
use egui_kittest::kittest::Queryable;
use fluent2_egui::surfaces::{Intent, Toast, ToastId, ToastPosition, Toaster};
use fluent2_egui::{Theme, install};

#[path = "../../../gallery/src/pages/toasts.rs"]
mod gallery_toasts;

/// Seconds of egui time per frame.
const STEP: f32 = 0.1;

fn toast(title: &str, timeout: Duration) -> Toast {
    Toast {
        intent: Intent::Info,
        title: title.to_owned(),
        body: None,
        timeout: Some(timeout),
        action: Some("Undo".to_owned()),
    }
}

fn stepped_harness(toasts: Vec<Toast>) -> Harness<'static, u32> {
    Harness::builder().with_step_dt(STEP).build_ui_state(
        move |ui, step: &mut u32| {
            let ctx = ui.ctx().clone();
            install(&ctx, &Theme::default());
            if *step == 0 {
                for toast in &toasts {
                    let _ = Toaster::push(&ctx, toast.clone());
                }
            }
            Toaster::show(&ctx, ToastPosition::BottomEnd);
            *step += 1;
        },
        0u32,
    )
}

/// Run enough frames to move egui's clock forward by `seconds`.
fn advance<State>(harness: &mut Harness<'_, State>, seconds: f32) {
    let frames = (seconds / STEP).round() as usize;
    for _ in 0..frames {
        harness.step();
    }
}

#[test]
fn toast_expires_after_its_timeout() {
    let mut harness = stepped_harness(vec![toast("Saved", Duration::from_millis(1000))]);
    harness.step();
    assert!(harness.query_by_label("Saved").is_some());
    // Well short of the timeout: still shown.
    advance(&mut harness, 0.5);
    assert!(harness.query_by_label("Saved").is_some());
    // Well past the timeout and its exit animation: gone.
    advance(&mut harness, 2.0);
    assert!(
        harness.query_by_label("Saved").is_none(),
        "toast should have expired"
    );
}

#[test]
fn hover_pauses_expiry() {
    let mut harness = stepped_harness(vec![toast("Hover me", Duration::from_millis(1000))]);
    harness.step();
    // Let the entrance settle so the toast is where it will stay, then hover it.
    advance(&mut harness, 0.5);
    let centre = harness.get_by_label("Hover me").rect().center();
    harness.hover_at(centre);
    // Three seconds hovered, well past the one-second timeout.
    advance(&mut harness, 3.0);
    assert!(
        harness.query_by_label("Hover me").is_some(),
        "hover should pause the timeout"
    );
}

#[derive(Default)]
struct DismissState {
    step: u32,
    id: Option<ToastId>,
    dismiss: bool,
}

#[test]
fn dismiss_removes_the_toast() {
    let mut harness = Harness::builder().with_step_dt(STEP).build_ui_state(
        |ui, state: &mut DismissState| {
            let ctx = ui.ctx().clone();
            install(&ctx, &Theme::default());
            if state.step == 0 {
                state.id = Some(Toaster::push(&ctx, toast("Dismiss me", Duration::from_secs(30))));
            }
            if let (true, Some(id)) = (state.dismiss, state.id) {
                Toaster::dismiss(&ctx, id);
            }
            Toaster::show(&ctx, ToastPosition::BottomEnd);
            state.step += 1;
        },
        DismissState::default(),
    );
    harness.step();
    assert!(harness.query_by_label("Dismiss me").is_some());
    harness.state_mut().dismiss = true;
    harness.step();
    // Past the exit animation: the toast is gone.
    advance(&mut harness, 1.0);
    assert!(harness.query_by_label("Dismiss me").is_none());
}

#[test]
fn at_most_three_are_drawn() {
    let mut harness = stepped_harness(
        ["One", "Two", "Three", "Four"]
            .into_iter()
            .map(|name| toast(name, Duration::from_secs(30)))
            .collect(),
    );
    harness.step();
    assert!(
        harness.query_by_label("One").is_none(),
        "oldest beyond the cap stays undrawn"
    );
    assert!(harness.query_by_label("Two").is_some());
    assert!(harness.query_by_label("Three").is_some());
    assert!(harness.query_by_label("Four").is_some());
}

#[test]
fn gallery_page_pushes_toasts() {
    let mut harness = Harness::new_ui(|ui| {
        install(ui.ctx(), &Theme::default());
        gallery_toasts::page(ui);
        Toaster::show(ui.ctx(), ToastPosition::BottomEnd);
    });
    harness.run();
    let _ = harness.get_by_label("Info toast");
}
