//! Sparkline and MiniBars: live and degenerate series. Switch the theme in the header to see
//! them in light, dark and high contrast.

use fluent2_egui::controls::{Button, MiniBars, Sparkline};

use super::Page;

pub struct ChartsPage {
    series: Vec<f32>,
    tick: u32,
}

impl Default for ChartsPage {
    fn default() -> Self {
        let mut page = Self {
            series: Vec::new(),
            tick: 0,
        };
        for _ in 0..24 {
            page.step();
        }
        page
    }
}

impl ChartsPage {
    /// Append a deterministic pseudo-random value, keeping the last 24.
    fn step(&mut self) {
        self.tick = self.tick.wrapping_mul(1_103_515_245).wrapping_add(12_345);
        let v = 20.0 + (self.tick >> 16) as f32 % 60.0;
        self.series.push(v);
        if self.series.len() > 24 {
            self.series.remove(0);
        }
    }
}

const CASES: [(&str, &[f32]); 6] = [
    ("Empty", &[]),
    ("NaN only", &[f32::NAN]),
    ("One point", &[42.0]),
    ("Flat", &[5.0, 5.0, 5.0, 5.0]),
    ("Gap and negatives", &[3.0, 6.0, f32::NAN, -4.0, 2.0, 8.0]),
    ("All zero", &[0.0, 0.0, 0.0]),
];

impl Page for ChartsPage {
    fn name(&self) -> &'static str {
        "Charts"
    }

    fn ui(&mut self, ui: &mut egui::Ui) {
        ui.heading("Sparkline and MiniBars");
        ui.label("Small charts for cells and tiles. Larger charts belong to egui_plot in the app.");
        ui.add_space(8.0);
        ui.horizontal(|ui| {
            ui.label("Requests per minute");
            ui.add(Sparkline::new(&self.series).label("Requests per minute"));
            ui.add(MiniBars::new(&self.series).label("Requests per minute"));
            if ui.add(Button::new("Add value")).clicked() {
                self.step();
            }
        });
        ui.horizontal(|ui| {
            ui.label("CPU, 0 to 100 %");
            ui.add(
                Sparkline::new(&self.series)
                    .range(0.0, 100.0)
                    .size(egui::vec2(200.0, 32.0))
                    .label("CPU"),
            );
            ui.add(
                MiniBars::new(&self.series)
                    .max(100.0)
                    .size(egui::vec2(200.0, 32.0))
                    .label("CPU"),
            );
        });
        ui.add_space(12.0);
        ui.strong("Edge cases");
        egui::Grid::new("chart_cases")
            .spacing([16.0, 8.0])
            .show(ui, |ui| {
                for (name, values) in CASES {
                    ui.label(name);
                    ui.add(Sparkline::new(values).label(name));
                    ui.add(MiniBars::new(values).label(name));
                    ui.end_row();
                }
            });
    }
}
