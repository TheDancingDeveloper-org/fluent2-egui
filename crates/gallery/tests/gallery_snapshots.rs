//! The README's widget gallery (WI-1910): every gallery page, a controls sheet and the editor
//! sample, rendered headlessly to PNG in the light and dark themes (high contrast for the
//! controls sheet and the editor).
//!
//! Needs a wgpu adapter. Run it through `scripts/gallery-screenshots.sh`, which uses the same
//! pinned container and Mesa lavapipe as the `gallery-screenshots` CI job, so the pixels match:
//! `scripts/gallery-screenshots.sh` checks the committed images, `--update` rewrites them.
//! The images live in `docs/images/gallery/`.

#![cfg(feature = "snapshots")]

#[allow(dead_code)]
#[path = "../src/pages/mod.rs"]
mod pages;

#[allow(dead_code)]
#[path = "../examples/editor.rs"]
mod editor;

use egui::Vec2;
use egui_kittest::{Harness, SnapshotOptions, SnapshotResults};
use fluent2_egui::controls::{Appearance, Badge, Button};
use fluent2_egui::surfaces::{Intent, MessageBar};
use fluent2_egui::theme::Theme;
use fluent2_egui::{
    CheckState, Checkbox, Combobox, Dropdown, Input, Link, ProgressBar, RadioGroup, Slider, SpinButton,
    Switch, TabList, Textarea, ToggleButton, install,
};

/// Output directory, relative to this crate (cargo runs tests from the crate directory).
const OUT: &str = "../../docs/images/gallery";

/// Size of every image, in points (one point is one pixel here).
const SIZE: Vec2 = Vec2::new(960.0, 640.0);

/// Frames to run. Some pages animate (spinners, caret blink), so `run()` would never settle;
/// a fixed count keeps every image deterministic.
const FRAMES: usize = 8;

#[derive(Clone, Copy)]
enum Look {
    Light,
    Dark,
    HighContrast,
}

impl Look {
    fn slug(self) -> &'static str {
        match self {
            Self::Light => "light",
            Self::Dark => "dark",
            Self::HighContrast => "hc",
        }
    }

    fn theme(self) -> Theme {
        match self {
            Self::Light => Theme::default(),
            Self::Dark => Theme::dark(fluent2_tokens::FLUENT),
            Self::HighContrast => Theme::high_contrast(),
        }
    }
}

/// Lower-case, hyphenated file stem for a page name ("Data grid" → "data-grid").
fn slug(name: &str) -> String {
    name.chars()
        .map(|c| {
            if c.is_ascii_alphanumeric() {
                c.to_ascii_lowercase()
            } else {
                '-'
            }
        })
        .collect::<String>()
        .split('-')
        .filter(|part| !part.is_empty())
        .collect::<Vec<_>>()
        .join("-")
}

/// Render `content` under `look` into `{name}-{look}.png`, adding the result to `results`.
fn shoot(
    results: &mut SnapshotResults,
    name: &str,
    look: Look,
    framed: bool,
    mut content: impl FnMut(&mut egui::Ui) + 'static,
) {
    let theme = look.theme();
    let mut installed = false;
    let mut harness = Harness::builder()
        .with_size(SIZE)
        .with_pixels_per_point(1.0)
        .build_ui(move |ui| {
            // The theme's text styles apply from the next frame, so the first frame only
            // installs it; pages look up the Fluent type ramp by name.
            if !installed {
                install(ui.ctx(), &theme);
                installed = true;
                ui.ctx().request_repaint();
                return;
            }
            let frame = if framed {
                egui::Frame::central_panel(ui.style())
            } else {
                egui::Frame::NONE
            };
            egui::CentralPanel::default()
                .frame(frame)
                .show(ui, |ui| content(ui));
        });
    harness.run_steps(FRAMES);
    let options = SnapshotOptions::new().output_path(OUT);
    results.add(harness.try_snapshot_options(format!("{name}-{}", look.slug()), &options));
}

#[test]
fn gallery_pages() {
    let mut results = SnapshotResults::new();
    for look in [Look::Light, Look::Dark] {
        for mut page in pages::pages() {
            let name = slug(page.name());
            shoot(&mut results, &name, look, true, move |ui| {
                egui::ScrollArea::vertical().show(ui, |ui| page.ui(ui));
            });
        }
    }
    results.unwrap();
}

#[test]
fn controls_sheet() {
    let mut results = SnapshotResults::new();
    for look in [Look::Light, Look::Dark, Look::HighContrast] {
        let mut sheet = Controls::default();
        shoot(&mut results, "controls", look, true, move |ui| sheet.ui(ui));
    }
    results.unwrap();
}

#[test]
fn editor_sample() {
    let mut results = SnapshotResults::new();
    for look in [Look::Light, Look::Dark, Look::HighContrast] {
        let mut sample = editor::Editor::default();
        shoot(&mut results, "editor", look, false, move |ui| sample.ui(ui));
    }
    results.unwrap();
}

/// One of each basic control, with some state set so the image shows checked, filled and
/// selected looks.
struct Controls {
    toggle: bool,
    check: CheckState,
    radio: usize,
    switch: bool,
    name: String,
    notes: String,
    volume: f64,
    copies: f64,
    colour: Option<usize>,
    font: String,
    tab: usize,
}

impl Default for Controls {
    fn default() -> Self {
        Self {
            toggle: true,
            check: CheckState::Checked,
            radio: 1,
            switch: true,
            name: "Ada Lovelace".to_owned(),
            notes: "Notes span\nseveral lines.".to_owned(),
            volume: 6.0,
            copies: 2.0,
            colour: Some(2),
            font: "Calibri".to_owned(),
            tab: 0,
        }
    }
}

impl Controls {
    fn ui(&mut self, ui: &mut egui::Ui) {
        ui.heading("Controls");
        ui.add_space(8.0);
        ui.horizontal(|ui| {
            ui.add(Button::new("Primary").appearance(Appearance::Primary));
            ui.add(Button::new("Secondary"));
            ui.add(Button::new("Outline").appearance(Appearance::Outline));
            ui.add(Button::new("Subtle").appearance(Appearance::Subtle));
            ui.add(ToggleButton::new(&mut self.toggle, "Bold"));
            ui.add(Badge::new("New"));
        });
        ui.add_space(8.0);
        ui.horizontal(|ui| {
            ui.add(Checkbox::new(&mut self.check, "Agree to the terms"));
            ui.add(Switch::new(&mut self.switch, "Wi-Fi"));
            RadioGroup::new(&mut self.radio)
                .option("Small")
                .option("Large")
                .horizontal()
                .show(ui);
        });
        ui.add_space(8.0);
        // Inputs fill the available width, so give them a column.
        ui.scope(|ui| {
            ui.set_max_width(420.0);
            ui.add(Input::new(&mut self.name).placeholder("Name"));
            ui.add_space(4.0);
            ui.add(Textarea::new(&mut self.notes).label("Notes"));
            ui.add_space(4.0);
            ui.add(Slider::new(&mut self.volume, 0.0..=10.0));
            ui.add_space(4.0);
            ui.add(
                SpinButton::new(&mut self.copies)
                    .range(1.0..=99.0)
                    .label("Copies"),
            );
            ui.add_space(4.0);
            ui.add(
                Dropdown::new("colour", &mut self.colour, &["Red", "Green", "Blue"]).placeholder("Colour"),
            );
            ui.add_space(4.0);
            ui.add(Combobox::new("font", &mut self.font, &["Arial", "Calibri"]).placeholder("Font"));
        });
        ui.add_space(8.0);
        TabList::new("tabs", &mut self.tab)
            .tab("Home")
            .tab("Insert")
            .tab("View")
            .show(ui);
        ui.add(ProgressBar::new(Some(0.4)));
        ui.add(Link::new("Learn more"));
        let _ = MessageBar::new(Intent::Warning, "Disk is almost full")
            .title("Storage")
            .show(ui);
    }
}
