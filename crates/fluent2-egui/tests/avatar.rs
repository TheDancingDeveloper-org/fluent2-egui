//! Avatar, AvatarGroup and Persona behaviour (WI-1932).

use egui::accesskit::Role;
use egui_kittest::Harness;
use egui_kittest::kittest::Queryable;
use fluent2_egui::controls::{
    Avatar, AvatarColor, AvatarGroup, AvatarGroupLayout, AvatarShape, Persona, Presence,
};
use fluent2_egui::{Theme, install};

#[test]
fn avatar_is_an_image_named_by_its_person_and_presence() {
    let mut harness = Harness::new_ui(|ui| {
        install(ui.ctx(), &Theme::default());
        ui.add(Avatar::new("Ada Lovelace").presence(Presence::Busy));
        ui.add(Avatar::new(""));
    });
    harness.run();
    let _ = harness.get_by_role_and_label(Role::Image, "Ada Lovelace, busy");
    let _ = harness.get_by_role_and_label(Role::Image, "Person");
}

#[test]
fn group_names_itself_and_lists_the_hidden_people() {
    let names = [
        "Ada Lovelace",
        "Grace Hopper",
        "Alan Turing",
        "Edsger Dijkstra",
        "Barbara Liskov",
    ];
    let mut harness = Harness::new_ui(move |ui| {
        install(ui.ctx(), &Theme::default());
        AvatarGroup::new(&names)
            .layout(AvatarGroupLayout::Stack)
            .max_visible(3)
            .show(ui);
    });
    harness.run();
    let _ = harness.get_by_role_and_label(Role::Group, "5 people");
    let _ = harness.get_by_role_and_label(Role::Image, "Ada Lovelace");
    let _ = harness.get_by_role_and_label(Role::Image, "Grace Hopper");
    assert!(
        harness
            .query_by_role_and_label(Role::Image, "Alan Turing")
            .is_none()
    );
    let _ = harness.get_by_role_and_label(
        Role::Image,
        "3 more: Alan Turing, Edsger Dijkstra, Barbara Liskov",
    );
}

#[test]
fn every_variant_renders_in_every_theme() {
    for theme in [
        Theme::default(),
        Theme::dark(fluent2_egui::tokens::FLUENT),
        Theme::high_contrast(),
    ] {
        let mut harness = Harness::new_ui(move |ui| {
            install(ui.ctx(), &theme);
            for size in fluent2_egui::controls::AVATAR_SIZES {
                for color in [AvatarColor::Neutral, AvatarColor::Brand, AvatarColor::Colorful] {
                    ui.add(Avatar::new("Grace Hopper").size(size).color(color));
                }
            }
            for presence in [
                Presence::Available,
                Presence::Busy,
                Presence::Away,
                Presence::DoNotDisturb,
                Presence::Offline,
            ] {
                ui.add(
                    Avatar::new("Alan Turing")
                        .shape(AvatarShape::Square)
                        .presence(presence),
                );
            }
            Persona::new(Avatar::new("Ada Lovelace"))
                .secondary("Analyst")
                .tertiary("London")
                .show(ui);
            AvatarGroup::new(&[]).show(ui);
        });
        harness.run();
        let _ = harness.get_by_label("Analyst");
    }
}
