//! Skeleton: a list row and a card placeholder, one per animation, and the size ramp (WI-1935).

use fluent2_egui::controls::{Divider, Skeleton, SkeletonAnimation, SkeletonItem};

use super::Page;

/// A contact row: an avatar disc and two text lines.
fn contact_row(animation: SkeletonAnimation) -> Skeleton {
    Skeleton::new()
        .animation(animation)
        .item(SkeletonItem::circle(40.0))
        .item(SkeletonItem::text_line(16.0, 0.5))
        .item(SkeletonItem::text_line(16.0, 0.8))
        .label("Loading contacts")
}

/// Loading states for lists and cards, with each motion and the size ramp.
#[derive(Default)]
pub struct SkeletonPage;

impl Page for SkeletonPage {
    fn name(&self) -> &'static str {
        "Skeleton"
    }

    fn ui(&mut self, ui: &mut egui::Ui) {
        ui.heading("Skeleton");
        ui.add_space(8.0);

        ui.add(Divider::horizontal().text("Animations"));
        ui.columns(3, |columns| {
            columns[0].label("Wave (default)");
            columns[0].add(contact_row(SkeletonAnimation::Wave));
            columns[1].label("Pulse");
            columns[1].add(contact_row(SkeletonAnimation::Pulse));
            columns[2].label("Static (reduced motion)");
            columns[2].add(contact_row(SkeletonAnimation::Static));
        });

        ui.add(Divider::horizontal().text("Card placeholder"));
        ui.add(
            Skeleton::new()
                .item(SkeletonItem::rectangle(128.0))
                .item(SkeletonItem::text_line(24.0, 0.7))
                .item(SkeletonItem::text_line(16.0, 1.0))
                .item(SkeletonItem::text_line(16.0, 0.9))
                .item(SkeletonItem::text_line(16.0, 0.4))
                .label("Loading article"),
        );

        ui.add(Divider::horizontal().text("Size ramp"));
        ui.horizontal_wrapped(|ui| {
            for size in fluent2_egui::controls::SKELETON_SIZES {
                ui.vertical(|ui| {
                    ui.add(Skeleton::new().item(SkeletonItem::circle(size)).label("Loading"));
                    ui.label(format!("{size:.0}"));
                });
            }
        });
    }
}
