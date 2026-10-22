//! LineChart, Sparkline and MiniBars: a 6 h speed graph, live and degenerate series. Switch
//! the theme in the header to see them in light, dark and high contrast.

use fluent2_egui::controls::{
    Button, ChartSeries, LineChart, MiniBars, Sparkline, TimeRange, bytes_per_second,
};

use super::Page;

pub struct ChartsPage {
    series: Vec<f32>,
    tick: u32,
    speed: [ChartSeries; 2],
    range: TimeRange,
}

/// Six hours at 1 Hz.
const SPEED_SAMPLES: u32 = 21_600;

/// Deterministic download and upload speeds (bytes per second) for second `t`.
fn speed_at(t: u32) -> (f32, f32) {
    let x = t as f32;
    let wave = (x / 900.0).sin() * 0.5 + 0.5;
    let ripple = (x / 37.0).sin() * 0.15 + 0.85;
    let down = 4.0e6 * wave * ripple;
    // A stall of two minutes every hour.
    let down = if t % 3600 < 120 { 0.0 } else { down };
    (down, 2.5e5 + 3.0e5 * (x / 410.0).cos().abs())
}

impl Default for ChartsPage {
    fn default() -> Self {
        let mut down = ChartSeries::new("Download", SPEED_SAMPLES as usize).fill(true);
        let mut up = ChartSeries::new("Upload", SPEED_SAMPLES as usize);
        for t in 0..SPEED_SAMPLES {
            let (d, u) = speed_at(t);
            down.push(f64::from(t), d);
            up.push(f64::from(t), u);
        }
        let mut page = Self {
            series: Vec::new(),
            tick: 0,
            speed: [down, up],
            range: TimeRange::SixHours,
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
        ui.heading("LineChart");
        ui.label("Download and upload at 1 Hz over 6 h. Hover for values; the chips pick the range.");
        ui.add_space(8.0);
        LineChart::new("speed", &self.speed)
            .label("Speed")
            .value_format(bytes_per_second)
            .show(ui, &mut self.range);
        ui.add_space(16.0);
        ui.heading("Sparkline and MiniBars");
        ui.label("Small charts for cells and tiles, without axes.");
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
