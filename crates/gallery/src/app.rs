//! The gallery window: a page list, a central page, and a theme bar.

use egui::Color32;
use fluent2_egui::theme::{Theme, ThemeKind};
use fluent2_egui::{system_preferences, theme_from_system};
use fluent2_tokens::Rgba;

use crate::pages::{self, Page};

/// Which theme the bar last installed, and the accent seed it was built from.
pub struct GalleryApp {
    pages: Vec<Box<dyn Page>>,
    selected: usize,
    kind: ThemeKind,
    /// Seed shown in the accent picker. High contrast ignores it.
    accent: Color32,
    /// The theme follows the OS reading taken when the toggle was last turned on.
    follow_system: bool,
}

impl GalleryApp {
    /// Pages in catalogue order, starting on the Fluent light theme.
    pub fn new(ctx: &egui::Context) -> Self {
        let accent = Color32::from_rgb(0x0f, 0x6c, 0xbd);
        apply(ctx, ThemeKind::Light, accent);
        Self {
            pages: pages::pages(),
            selected: 0,
            kind: ThemeKind::Light,
            accent,
            follow_system: false,
        }
    }

    /// Read the OS appearance once and install the theme it implies. Called when the toggle
    /// turns on, not every frame.
    fn follow(&mut self, ctx: &egui::Context) {
        let prefs = system_preferences();
        let theme = theme_from_system(ctx, &prefs);
        self.kind = theme.kind;
        if let Some(accent) = prefs.accent {
            self.accent = accent.to_color32();
        }
        fluent2_egui::install(ctx, &theme);
    }
}

impl eframe::App for GalleryApp {
    fn ui(&mut self, ui: &mut egui::Ui, _frame: &mut eframe::Frame) {
        let ctx = ui.ctx().clone();
        egui::Panel::top("theme_bar").show(ui, |ui| {
            ui.horizontal(|ui| {
                ui.label("Theme");
                for (label, kind) in [
                    ("Light", ThemeKind::Light),
                    ("Dark", ThemeKind::Dark),
                    ("High contrast", ThemeKind::HighContrast),
                ] {
                    if ui
                        .selectable_label(!self.follow_system && self.kind == kind, label)
                        .clicked()
                    {
                        self.follow_system = false;
                        self.kind = kind;
                        apply(&ctx, self.kind, self.accent);
                    }
                }
                if ui.selectable_label(self.follow_system, "Follow system").clicked() {
                    self.follow_system = !self.follow_system;
                    if self.follow_system {
                        self.follow(&ctx);
                    }
                }
                ui.separator();
                ui.label("Accent");
                if egui::color_picker::color_edit_button_srgba(
                    ui,
                    &mut self.accent,
                    egui::color_picker::Alpha::Opaque,
                )
                .changed()
                {
                    self.follow_system = false;
                    apply(&ctx, self.kind, self.accent);
                }
            });
        });

        egui::Panel::left("pages").show(ui, |ui| {
            ui.heading("Gallery");
            ui.add_space(4.0);
            for (index, page) in self.pages.iter().enumerate() {
                if ui.selectable_label(self.selected == index, page.name()).clicked() {
                    self.selected = index;
                }
            }
        });

        egui::CentralPanel::default().show(ui, |ui| {
            egui::ScrollArea::vertical().show(ui, |ui| {
                if let Some(page) = self.pages.get_mut(self.selected) {
                    page.ui(ui);
                }
            });
        });
    }
}

/// Rebuild the theme for `kind` and `accent` and install it.
fn apply(ctx: &egui::Context, kind: ThemeKind, accent: Color32) {
    let brand = fluent2_tokens::from_seed(Rgba {
        r: accent.r(),
        g: accent.g(),
        b: accent.b(),
        a: 255,
    });
    let theme = match kind {
        ThemeKind::Light => Theme::light(brand),
        ThemeKind::Dark => Theme::dark(brand),
        ThemeKind::HighContrast => Theme::high_contrast(),
    };
    fluent2_egui::install(ctx, &theme);
}
