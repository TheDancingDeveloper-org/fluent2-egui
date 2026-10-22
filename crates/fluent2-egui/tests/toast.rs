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

/// Harness that runs `setup` on the first frame, then draws the stack on every frame.
fn seeded_harness(setup: impl Fn(&egui::Context) + 'static) -> Harness<'static, u32> {
    Harness::builder().with_step_dt(STEP).build_ui_state(
        move |ui, step: &mut u32| {
            let ctx = ui.ctx().clone();
            install(&ctx, &Theme::default());
            if *step == 0 {
                setup(&ctx);
            }
            Toaster::show(&ctx, ToastPosition::BottomEnd);
            *step += 1;
        },
        0u32,
    )
}

#[test]
fn fifty_rapid_pushes_show_the_stack_and_a_count() {
    let mut harness = seeded_harness(|ctx| {
        for n in 1..=50 {
            let _ = Toaster::push(ctx, toast(&format!("Item {n}"), Duration::from_secs(30)));
        }
    });
    harness.step();
    assert!(harness.query_by_label("Item 50").is_some());
    assert!(harness.query_by_label("Item 48").is_some());
    assert!(harness.query_by_label("Item 47").is_none());
    assert!(harness.query_by_label("+47 more").is_some());
}

#[test]
fn keyed_pushes_coalesce_into_one_toast() {
    let mut harness = seeded_harness(|ctx| {
        let first = Toaster::push_keyed(ctx, "pause", toast("Paused 1 torrents", Duration::from_secs(30)));
        for n in 2..=50 {
            let title = format!("Paused {n} torrents");
            let id = Toaster::push_keyed(ctx, "pause", toast(&title, Duration::from_secs(30)));
            assert_eq!(id, first, "a live keyed toast keeps its id");
        }
    });
    harness.step();
    assert!(harness.query_by_label("Paused 50 torrents").is_some());
    assert!(harness.query_by_label("Paused 49 torrents").is_none());
    // Two more plain toasts fill the three slots only if the keyed push was one entry; were
    // it fifty, the coalesced toast would be waiting behind the cap and not drawn.
    let _ = Toaster::push(&harness.ctx, toast("Second", Duration::from_secs(30)));
    let _ = Toaster::push(&harness.ctx, toast("Third", Duration::from_secs(30)));
    harness.step();
    assert!(harness.query_by_label("Paused 50 torrents").is_some());
    assert_eq!(Toaster::history(&harness.ctx).len(), 52);
}

#[test]
fn keyed_push_after_dismiss_starts_a_new_toast() {
    let mut harness = seeded_harness(|ctx| {
        let id = Toaster::push_keyed(ctx, "pause", toast("Paused 3 torrents", Duration::from_secs(30)));
        Toaster::dismiss(ctx, id);
    });
    harness.step();
    advance(&mut harness, 1.0);
    assert!(harness.query_by_label("Paused 3 torrents").is_none());
    let _ = Toaster::push_keyed(
        &harness.ctx,
        "pause",
        toast("Paused 4 torrents", Duration::from_secs(30)),
    );
    harness.step();
    assert!(harness.query_by_label("Paused 4 torrents").is_some());
}

#[test]
fn history_keeps_the_last_hundred_pushes() {
    let mut harness = seeded_harness(|ctx| {
        for n in 1..=150 {
            let _ = Toaster::push(ctx, toast(&format!("Log {n}"), Duration::from_secs(30)));
        }
    });
    harness.step();
    let history = Toaster::history(&harness.ctx);
    assert_eq!(history.len(), 100);
    assert_eq!(history.first().map(|t| t.title.as_str()), Some("Log 51"));
    assert_eq!(history.last().map(|t| t.title.as_str()), Some("Log 150"));

    Toaster::clear_history(&harness.ctx);
    assert!(Toaster::history(&harness.ctx).is_empty());
}

#[test]
fn max_visible_is_configurable() {
    let mut harness = seeded_harness(|ctx| {
        Toaster::set_max_visible(ctx, 1);
        for name in ["One", "Two", "Three"] {
            let _ = Toaster::push(ctx, toast(name, Duration::from_secs(30)));
        }
    });
    harness.step();
    assert!(harness.query_by_label("Three").is_some());
    assert!(harness.query_by_label("Two").is_none());
    assert!(harness.query_by_label("+2 more").is_some());
}
