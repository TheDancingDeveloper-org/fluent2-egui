//! Icon widget: size, full catalogue, disabled.

use fluent2_egui::foundation::icon::{Icon, IconName, IconSize, IconStyle};
use fluent2_egui::theme::{self, Theme};

#[test]
fn add_icon_allocates_twenty_points() {
    let mut harness = egui_kittest::Harness::new_ui(|ui| {
        theme::install(ui.ctx(), &Theme::default());
        let response = ui.add(Icon::new(IconName::Add));
        assert_eq!(response.rect.size(), egui::vec2(20.0, 20.0));
    });
    harness.run();
}

#[test]
fn every_icon_renders_at_every_size_and_style() {
    for name in IconName::ALL.iter().copied() {
        for size in [IconSize::S16, IconSize::S20, IconSize::S24] {
            for style in [IconStyle::Regular, IconStyle::Filled] {
                let mut harness = egui_kittest::Harness::new_ui(move |ui| {
                    theme::install(ui.ctx(), &Theme::default());
                    let mut icon = Icon::new(name).size(size);
                    if style == IconStyle::Filled {
                        icon = icon.filled();
                    }
                    ui.add(icon);
                });
                harness.run();
            }
        }
    }
}

#[test]
fn disabled_icon_does_not_panic() {
    let mut harness = egui_kittest::Harness::new_ui(|ui| {
        theme::install(ui.ctx(), &Theme::default());
        ui.add_enabled_ui(false, |ui| {
            ui.add(Icon::new(IconName::Add));
        });
    });
    harness.run();
}
