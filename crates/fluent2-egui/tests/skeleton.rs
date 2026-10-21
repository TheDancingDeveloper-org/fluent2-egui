//! Skeleton rendering, accessibility and motion, without the wgpu snapshot feature.

use egui::accesskit::Role;
use egui_kittest::Harness;
use egui_kittest::kittest::{NodeT, Queryable};
use fluent2_egui::controls::{Skeleton, SkeletonAnimation, SkeletonItem};
use fluent2_egui::foundation::motion;
use fluent2_egui::theme::{self, Theme};

fn install(ui: &egui::Ui, theme: Theme) {
    theme::install(ui.ctx(), &theme);
}

fn contact_row(animation: SkeletonAnimation) -> Skeleton {
    Skeleton::new()
        .animation(animation)
        .item(SkeletonItem::circle(40.0))
        .item(SkeletonItem::text_line(16.0, 0.5))
        .item(SkeletonItem::text_line(16.0, 0.8))
}

#[test]
fn renders_in_light_dark_and_high_contrast() {
    for theme in [
        Theme::default(),
        Theme::dark(fluent2_tokens::FLUENT),
        Theme::high_contrast(),
    ] {
        let mut harness = Harness::new_ui(move |ui| {
            install(ui, theme.clone());
            let _ = ui.add(contact_row(SkeletonAnimation::Wave));
            let _ = ui.add(contact_row(SkeletonAnimation::Pulse));
            let _ = ui.add(Skeleton::new());
        });
        // Wave and pulse keep repainting, so step once rather than run to quiescence.
        harness.step();
    }
}

#[test]
fn skeleton_is_one_busy_progress_node_named_loading() {
    let mut harness = Harness::new_ui(|ui| {
        install(ui, Theme::default());
        let _ = ui.add(contact_row(SkeletonAnimation::Static));
    });
    harness.run();
    let node = harness.get_by_role_and_label(Role::ProgressIndicator, "Loading");
    assert!(node.accesskit_node().is_busy());
}

#[test]
fn custom_label_replaces_the_default_name() {
    let mut harness = Harness::new_ui(|ui| {
        install(ui, Theme::default());
        let _ = ui.add(contact_row(SkeletonAnimation::Static).label("Loading contacts"));
    });
    harness.run();
    let _ = harness.get_by_role_and_label(Role::ProgressIndicator, "Loading contacts");
}

#[test]
fn reduced_motion_repaints_nothing() {
    let mut harness = Harness::new_ui(|ui| {
        install(ui, Theme::default());
        motion::set_reduced_motion(ui.ctx(), true);
        let _ = ui.add(contact_row(SkeletonAnimation::Wave));
    });
    harness.run();
    assert!(!harness.ctx.has_requested_repaint());
}

#[test]
fn static_and_high_contrast_skeletons_do_not_animate() {
    let mut static_bar = Harness::builder().build_ui(|ui| {
        install(ui, Theme::default());
        let _ = ui.add(contact_row(SkeletonAnimation::Static));
    });
    static_bar.step();
    assert!(!static_bar.ctx.has_requested_repaint());

    let mut outlined = Harness::builder().build_ui(|ui| {
        install(ui, Theme::high_contrast());
        let _ = ui.add(contact_row(SkeletonAnimation::Wave));
    });
    outlined.step();
    assert!(!outlined.ctx.has_requested_repaint());
}

#[test]
fn wave_and_pulse_request_repaints_while_visible() {
    for animation in [SkeletonAnimation::Wave, SkeletonAnimation::Pulse] {
        let mut harness = Harness::builder().build_ui(move |ui| {
            install(ui, Theme::default());
            let _ = ui.add(contact_row(animation));
        });
        harness.step();
        assert!(
            harness.ctx.has_requested_repaint(),
            "{animation:?} should keep repainting"
        );
    }
}

#[test]
fn empty_and_non_finite_items_do_not_panic() {
    let mut harness = Harness::new_ui(|ui| {
        install(ui, Theme::default());
        let _ = ui.add(Skeleton::new().items([
            SkeletonItem::rectangle(f32::NAN),
            SkeletonItem::circle(f32::INFINITY),
            SkeletonItem::text_line(-4.0, f32::NAN),
        ]));
    });
    harness.step();
}
