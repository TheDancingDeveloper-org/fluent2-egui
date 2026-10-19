//! Sparkline and MiniBars: degenerate input in every theme, and the AccessKit summary.

use egui::Vec2;
use egui::accesskit::Role;
use egui_kittest::Harness;
use egui_kittest::kittest::{NodeT, Queryable};
use fluent2_egui::controls::{MiniBars, SeriesSummary, Sparkline};
use fluent2_egui::{Theme, install};

const INPUTS: [&[f32]; 7] = [
    &[],
    &[f32::NAN],
    &[f32::INFINITY, f32::NEG_INFINITY],
    &[42.0],
    &[3.0, 3.0, 3.0],
    &[1.0, f32::NAN, 4.0, -2.0, 8.0],
    &[0.0, 0.0],
];

fn themes() -> [Theme; 3] {
    [
        Theme::light(fluent2_tokens::FLUENT),
        Theme::dark(fluent2_tokens::FLUENT),
        Theme::high_contrast(),
    ]
}

#[test]
fn degenerate_inputs_render_in_every_theme() {
    for theme in themes() {
        for values in INPUTS {
            let theme = theme.clone();
            let mut h = Harness::builder()
                .with_size(Vec2::new(300.0, 120.0))
                .build_ui(move |ui| {
                    install(ui.ctx(), &theme);
                    ui.add(Sparkline::new(values).label("Line"));
                    ui.add(
                        Sparkline::new(values)
                            .range(f32::NAN, f32::INFINITY)
                            .size(Vec2::new(f32::NAN, -1.0)),
                    );
                    ui.add(MiniBars::new(values).label("Bars"));
                    ui.add(MiniBars::new(values).max(-5.0).size(Vec2::ZERO));
                });
            h.run();
            assert_eq!(h.query_all_by_role(Role::Image).count(), 4, "{values:?}");
        }
    }
}

#[test]
fn summary_label_reaches_accesskit() {
    let mut h = Harness::builder()
        .with_size(Vec2::new(300.0, 80.0))
        .build_ui(|ui| {
            ui.add(Sparkline::new(&[3.0, f32::NAN, 41.0, 17.5]).label("CPU"));
            ui.add(MiniBars::new(&[]).label("Deploys"));
        });
    h.run();
    let labels: Vec<String> = h
        .query_all_by_role(Role::Image)
        .filter_map(|n| n.accesskit_node().label())
        .collect();
    assert_eq!(
        labels,
        vec![
            "CPU: 3 values, min 3, max 41, last 17.50".to_owned(),
            "Deploys: no data".to_owned()
        ]
    );
}

#[test]
fn default_size_is_one_line_by_five() {
    let mut h = Harness::builder()
        .with_size(Vec2::new(300.0, 80.0))
        .build_ui(|ui| {
            ui.add(Sparkline::new(&[1.0, 2.0]));
        });
    h.run();
    let rect = h.get_by_role(Role::Image).rect();
    assert_eq!((rect.width(), rect.height()), (100.0, 20.0));
}

#[test]
fn summary_of_one_value() {
    let s = SeriesSummary::of(&[f32::NAN, 7.0]);
    assert_eq!(SeriesSummary::label(s, "X"), "X: 1 value, min 7, max 7, last 7");
}
