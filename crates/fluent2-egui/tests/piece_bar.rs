//! PieceBar and PieceMap: degenerate input in every theme, and the AccessKit summary.

use egui::Vec2;
use egui::accesskit::Role;
use egui_kittest::Harness;
use egui_kittest::kittest::{NodeT, Queryable};
use fluent2_egui::controls::{PieceBar, PieceMap, PieceState};
use fluent2_egui::{Theme, install};

fn themes() -> [Theme; 3] {
    [
        Theme::light(fluent2_tokens::FLUENT),
        Theme::dark(fluent2_tokens::FLUENT),
        Theme::high_contrast(),
    ]
}

#[test]
fn degenerate_inputs_render_in_every_theme() {
    let big = vec![0b1010_1010_u8; 25_000];
    for theme in themes() {
        let big = big.clone();
        let mut h = Harness::builder()
            .with_size(Vec2::new(640.0, 240.0))
            .build_ui(move |ui| {
                install(ui.ctx(), &theme);
                ui.add(PieceBar::states(&[]).label("Empty"));
                ui.add(PieceBar::bitfield(&[], 0).label("No bits"));
                ui.add(PieceBar::bitfield(&[0xff], 40).label("Short"));
                ui.add(PieceBar::availability(&[0, 0]).label("Zero"));
                ui.add(PieceBar::bitfield(&big, 200_000).width(600.0).label("Big"));
                ui.add(PieceMap::new(&[]).label("None"));
                ui.add(PieceMap::new(&[PieceState::Downloading]).width(f32::NAN));
            });
        h.run();
        assert_eq!(h.query_all_by_role(Role::Image).count(), 7);
    }
}

#[test]
fn percent_done_reaches_accesskit() {
    let states = [
        PieceState::Have,
        PieceState::Downloading,
        PieceState::Missing,
        PieceState::Have,
    ];
    let mut h = Harness::builder()
        .with_size(Vec2::new(300.0, 160.0))
        .build_ui(move |ui| {
            ui.add(PieceBar::states(&states).label("Pieces"));
            ui.add(PieceBar::bitfield(&[0b1100_0000], 4).label("Bits"));
            ui.add(PieceMap::new(&states).label("Map").legend(false));
            ui.add(PieceBar::availability(&[1, 4, 2]).label("Rarity"));
        });
    h.run();
    let labels: Vec<String> = h
        .query_all_by_role(Role::Image)
        .filter_map(|n| n.accesskit_node().label())
        .collect();
    assert_eq!(
        labels,
        vec![
            "Pieces: 50% done (2 of 4 pieces)".to_owned(),
            "Bits: 50% done (2 of 4 pieces)".to_owned(),
            "Map: 50% done (2 of 4 pieces)".to_owned(),
            "Rarity: 3 pieces, availability 1 to 4".to_owned(),
        ]
    );
}
