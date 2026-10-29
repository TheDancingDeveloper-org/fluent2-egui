//! `theme_from_system` is a pure mapping plus the reduced-motion flag. Platform calls are not
//! unit-tested.

use egui::{Context, Id};
use fluent2_egui::foundation::motion::{Curve, Duration, animate};
use fluent2_egui::platform::Rgba;
use fluent2_egui::{SystemPreferences, ThemeKind, theme_from_system};
use fluent2_tokens::FLUENT;

fn accent() -> Rgba {
    Rgba::new(0.0, 0.47, 0.84)
}

/// Reduced motion makes `animate` snap to a new target at once. Otherwise the first frame
/// after a retarget is still at the old value.
fn reduced_motion(ctx: &Context) -> bool {
    let id = Id::unique("probe");
    animate(ctx, id, 0.0, Duration::Slow, Curve::AccelerateMid);
    animate(ctx, id, 1.0, Duration::Slow, Curve::AccelerateMid) == 1.0
}

fn ctx() -> Context {
    Context::default()
}

#[test]
fn high_contrast_wins_and_drops_the_accent() {
    let ctx = ctx();
    let theme = theme_from_system(
        &ctx,
        &SystemPreferences {
            accent: Some(accent()),
            high_contrast: Some(true),
            reduced_motion: Some(false),
            dark: Some(true),
        },
    );
    assert_eq!(theme.kind, ThemeKind::HighContrast);
    assert_eq!(theme.brand, FLUENT);
    assert!(!reduced_motion(&ctx));
}

#[test]
fn dark_keeps_the_accent_seed() {
    let theme = theme_from_system(
        &ctx(),
        &SystemPreferences {
            accent: Some(accent()),
            high_contrast: Some(false),
            reduced_motion: None,
            dark: Some(true),
        },
    );
    assert_eq!(theme.kind, ThemeKind::Dark);
    assert_eq!(theme.brand, fluent2_tokens::from_seed(accent().to_token()));
}

#[test]
fn light_is_the_default_when_the_scheme_is_unknown() {
    let theme = theme_from_system(
        &ctx(),
        &SystemPreferences {
            accent: None,
            high_contrast: None,
            reduced_motion: Some(false),
            dark: None,
        },
    );
    assert_eq!(theme.kind, ThemeKind::Light);
    assert_eq!(theme.brand, FLUENT);
}

#[test]
fn reduced_motion_is_recorded_only_when_the_os_asked() {
    let ctx = ctx();
    let theme = theme_from_system(
        &ctx,
        &SystemPreferences {
            accent: Some(accent()),
            high_contrast: Some(false),
            reduced_motion: Some(true),
            dark: Some(false),
        },
    );
    assert_eq!(theme.kind, ThemeKind::Light);
    assert_eq!(theme.brand, fluent2_tokens::from_seed(accent().to_token()));
    assert!(reduced_motion(&ctx));
}
