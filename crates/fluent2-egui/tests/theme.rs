//! Theme install, read-back and a smoke render per theme.

use fluent2_egui::theme::{self, Theme, ThemeKind};

fn ctx() -> egui::Context {
    egui::Context::default()
}

#[test]
fn install_then_get_returns_kind_and_color() {
    let themes = [
        Theme::light(fluent2_tokens::FLUENT),
        Theme::dark(fluent2_tokens::from_seed(fluent2_tokens::Rgba::hex(0xFF0000))),
        Theme::high_contrast(),
    ];
    for theme in themes {
        let ctx = ctx();
        theme::install(&ctx, &theme);
        let got = Theme::get(&ctx);
        assert_eq!(got.kind, theme.kind);
        assert_eq!(got.color, theme.color);
    }
}

#[test]
fn light_panel_fill_is_neutral_background_2() {
    let ctx = ctx();
    theme::install(&ctx, &Theme::default());
    assert_eq!(
        ctx.global_style().visuals.panel_fill,
        theme::c(fluent2_tokens::light(&fluent2_tokens::FLUENT).neutral_background_2)
    );
}

#[test]
fn button_renders_in_each_theme() {
    for theme in [
        Theme::light(fluent2_tokens::FLUENT),
        Theme::dark(fluent2_tokens::FLUENT),
        Theme::high_contrast(),
    ] {
        let mut harness = egui_kittest::Harness::new_ui(|ui| {
            theme::install(ui.ctx(), &theme);
            ui.add(egui::Button::new("Save"));
        });
        harness.run();
        assert_eq!(Theme::get(&harness.ctx).kind, theme.kind);
    }
}

#[test]
fn high_contrast_follows_its_background() {
    // The generated high contrast background decides the egui theme; this only checks the
    // mapping does not panic and stores the kind.
    let ctx = ctx();
    theme::install(&ctx, &Theme::high_contrast());
    assert_eq!(Theme::get(&ctx).kind, ThemeKind::HighContrast);
}
