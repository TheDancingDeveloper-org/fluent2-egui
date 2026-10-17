//! Toaster behaviour: expiry, hover pause, and the visible cap.

use std::time::Duration;

use egui_kittest::Harness;
use egui_kittest::kittest::Queryable;
use fluent2_egui::surfaces::{Intent, Toast, ToastPosition, Toaster};
use fluent2_egui::{Theme, install};

fn toast(title: &str, timeout: Duration) -> Toast {
    Toast {
        intent: Intent::Info,
        title: title.to_owned(),
        body: None,
        timeout: Some(timeout),
        action: Some("Undo".to_owned()),
    }
}

#[test]
fn toast_expires_after_its_timeout() {
    let mut harness = Harness::new_ui_state(
        |ui, step: &mut u32| {
            let ctx = ui.ctx().clone();
            install(&ctx, &Theme::default());
            if *step == 0 {
                let _ = Toaster::push(&ctx, toast("Saved", Duration::from_millis(1000)));
            }
            Toaster::show(&ctx, ToastPosition::BottomEnd);
            *step += 1;
        },
        0u32,
    );
    harness.run();
    assert!(harness.query_by_label("Saved").is_some());
    // Advance egui's time by stepping input.
    harness.input_mut().time = harness.input().time.map(|t| t + 1.2).or(Some(1.2));
    harness.run();
    // Exit animation (duration normal, 200 ms) then removal.
    harness.input_mut().time = harness.input().time.map(|t| t + 0.3).or(Some(1.5));
    harness.run();
    assert!(
        harness.query_by_label("Saved").is_none(),
        "toast should have expired"
    );
}

#[test]
fn hover_pauses_expiry() {
    let mut harness = Harness::new_ui_state(
        |ui, step: &mut u32| {
            let ctx = ui.ctx().clone();
            install(&ctx, &Theme::default());
            if *step == 0 {
                let _ = Toaster::push(&ctx, toast("Hover me", Duration::from_millis(500)));
            }
            Toaster::show(&ctx, ToastPosition::BottomEnd);
            *step += 1;
        },
        0u32,
    );
    harness.run();
    // Stay hovered across a span longer than the timeout.
    if let Some(node) = harness.query_by_label("Hover me") {
        node.hover();
    }
    harness.input_mut().time = harness.input().time.map(|t| t + 2.0).or(Some(2.0));
    harness.run();
    assert!(harness.query_by_label("Hover me").is_some(), "hover should pause the timeout");
}

#[test]
fn at_most_three_are_drawn() {
    let mut harness = Harness::new_ui_state(
        |ui, step: &mut u32| {
            let ctx = ui.ctx().clone();
            install(&ctx, &Theme::default());
            if *step == 0 {
                for name in ["One", "Two", "Three", "Four"] {
                    let _ = Toaster::push(&ctx, toast(name, Duration::from_secs(30)));
                }
            }
            Toaster::show(&ctx, ToastPosition::BottomEnd);
            *step += 1;
        },
        0u32,
    );
    harness.run();
    assert!(harness.query_by_label("One").is_none(), "oldest beyond the cap stays undrawn");
    assert!(harness.query_by_label("Two").is_some());
    assert!(harness.query_by_label("Three").is_some());
    assert!(harness.query_by_label("Four").is_some());
}
