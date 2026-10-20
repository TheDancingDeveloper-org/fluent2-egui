//! Avatars: sizes, colours, shapes, presence, groups and personas (WI-1932).

use fluent2_egui::controls::{
    AVATAR_SIZES, Avatar, AvatarColor, AvatarGroup, AvatarGroupLayout, AvatarShape, Divider, Persona,
    Presence,
};

use super::Page;

const PEOPLE: [&str; 8] = [
    "Ada Lovelace",
    "Grace Hopper",
    "Alan Turing",
    "Edsger Dijkstra",
    "Barbara Liskov",
    "Donald Knuth",
    "Margaret Hamilton",
    "Ken Thompson",
];

#[derive(Default)]
pub struct AvatarsPage;

impl Page for AvatarsPage {
    fn name(&self) -> &'static str {
        "Avatars"
    }

    fn ui(&mut self, ui: &mut egui::Ui) {
        ui.heading("Avatars");
        ui.add_space(8.0);

        ui.add(Divider::horizontal().text("Sizes"));
        ui.horizontal_wrapped(|ui| {
            for size in AVATAR_SIZES {
                ui.add(
                    Avatar::new("Ada Lovelace")
                        .size(size)
                        .color(AvatarColor::Colorful),
                );
            }
        });

        ui.add(Divider::horizontal().text("Colours and shapes"));
        ui.horizontal_wrapped(|ui| {
            ui.add(Avatar::new("Grace Hopper").size(40.0));
            ui.add(Avatar::new("Grace Hopper").size(40.0).color(AvatarColor::Brand));
            ui.add(Avatar::new("").size(40.0));
            ui.add(
                Avatar::new("Design team")
                    .size(40.0)
                    .shape(AvatarShape::Square)
                    .color(AvatarColor::Brand),
            );
            for person in PEOPLE {
                ui.add(Avatar::new(person).size(40.0).color(AvatarColor::Colorful));
            }
        });

        ui.add(Divider::horizontal().text("Presence"));
        ui.horizontal_wrapped(|ui| {
            for (person, presence) in PEOPLE.iter().zip([
                Presence::Available,
                Presence::Busy,
                Presence::Away,
                Presence::DoNotDisturb,
                Presence::Offline,
            ]) {
                ui.add(
                    Avatar::new(*person)
                        .size(48.0)
                        .color(AvatarColor::Colorful)
                        .presence(presence),
                );
            }
        });

        ui.add(Divider::horizontal().text("Groups"));
        AvatarGroup::new(&PEOPLE)
            .label("Project team")
            .max_visible(5)
            .show(ui);
        ui.add_space(8.0);
        AvatarGroup::new(&PEOPLE)
            .label("Reviewers")
            .layout(AvatarGroupLayout::Stack)
            .max_visible(4)
            .size(40.0)
            .show(ui);

        ui.add(Divider::horizontal().text("Persona"));
        Persona::new(
            Avatar::new("Ada Lovelace")
                .size(48.0)
                .color(AvatarColor::Colorful)
                .presence(Presence::Available),
        )
        .secondary("Analyst, Engine team")
        .tertiary("Available")
        .show(ui);
        ui.add_space(4.0);
        Persona::new(
            Avatar::new("Alan Turing")
                .size(32.0)
                .color(AvatarColor::Colorful)
                .presence(Presence::Away),
        )
        .secondary("Away until 3 pm")
        .show(ui);
    }
}
