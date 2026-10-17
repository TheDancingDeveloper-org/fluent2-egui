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
    // The pointer takes a few frames to register as hovering the Area, so the timeout leaves
    // a second of slack after the entrance settles.
    let mut harness = stepped_harness(vec![toast("Hover me", Duration::from_secs(2))]);
    harness.step();
    // Let the entrance settle so the toast is where it will stay, then hover it.
    advance(&mut harness, 0.5);
    let centre = harness.get_by_label("Hover me").rect().center();
    harness.hover_at(centre);
    // Five seconds hovered, well past the two-second timeout.
    advance(&mut harness, 5.0);
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

#[derive(Default)]
struct ActionState {
    step: u32,
    id: Option<ToastId>,
    clicks: Vec<ToastId>,
}

#[test]
fn action_click_is_reported() {
    let mut harness = Harness::builder().with_step_dt(STEP).build_ui_state(
        |ui, state: &mut ActionState| {
            let ctx = ui.ctx().clone();
            install(&ctx, &Theme::default());
            if state.step == 0 {
                state.id = Some(Toaster::push(&ctx, toast("Moved", Duration::from_secs(30))));
            }
            Toaster::show(&ctx, ToastPosition::BottomEnd);
            state.clicks.extend(Toaster::take_action_clicks(&ctx));
            state.step += 1;
        },
        ActionState::default(),
    );
    harness.step();
    // Let the entrance settle so the button is where it will stay.
    advance(&mut harness, 0.5);
    assert!(harness.state().clicks.is_empty());
    harness.get_by_label("Undo").click();
    advance(&mut harness, 0.3);
    let id = harness.state().id;
    assert_eq!(harness.state().clicks, id.into_iter().collect::<Vec<_>>());
}

#[test]
fn idle_toaster_schedules_the_repaint_that_expires_it() {
    use std::sync::{Arc, Mutex};

    let mut harness = stepped_harness(vec![toast("Idle", Duration::from_secs(2))]);
    let delays: Arc<Mutex<Vec<Duration>>> = Arc::default();
    let sink = Arc::clone(&delays);
    harness.ctx.set_request_repaint_callback(move |info| {
        if let Ok(mut delays) = sink.lock() {
            delays.push(info.delay);
        }
    });
    harness.step();
    // Past the entrance animation, nothing is moving: only the timeout is pending.
    advance(&mut harness, 0.5);
    if let Ok(mut delays) = delays.lock() {
        delays.clear();
    }
    harness.step();
    let delays = delays.lock().map(|d| d.clone()).unwrap_or_default();
    assert!(
        delays
            .iter()
            .any(|delay| *delay > Duration::ZERO && *delay <= Duration::from_secs(2)),
        "show should ask for a repaint at the timeout, got {delays:?}"
    );
}
