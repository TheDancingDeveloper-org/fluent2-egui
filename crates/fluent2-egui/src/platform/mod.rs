//! Operating-system appearance, read once at startup or on demand.
//!
//! [`system_preferences`] never panics: every platform error becomes [`None`] ("unknown").
//! Do not call it every frame. On Linux it opens a desktop-portal connection and waits up to
//! 300 ms; on Windows it queries the shell. Cache the result and re-query when the OS reports
//! a settings change, or when the user asks to follow the system again.

use crate::theme::Theme;

#[cfg(feature = "file-dialog")]
pub mod file_dialog;

#[cfg(not(any(
    all(feature = "system-preferences", windows),
    all(feature = "system-preferences", target_os = "linux"),
)))]
mod fallback;
#[cfg(all(feature = "system-preferences", target_os = "linux"))]
mod linux;
#[cfg(all(feature = "system-preferences", windows))]
mod windows;

/// sRGB colour with straight alpha in `0..=1`.
///
/// This is the colour the platform layer hands to a brand ramp. It is not a Fluent token
/// type; the brand-ramp item owns the token colour. Converting to `egui::Color32` rounds
/// each channel.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Rgba {
    /// Red, `0..=1`.
    pub r: f32,
    /// Green, `0..=1`.
    pub g: f32,
    /// Blue, `0..=1`.
    pub b: f32,
    /// Straight alpha, `0..=1`.
    pub a: f32,
}

impl Rgba {
    /// Opaque colour from channels in `0..=1`. Out-of-range channels are clamped.
    #[must_use]
    pub fn new(r: f32, g: f32, b: f32) -> Self {
        Self {
            r: clamp01(r),
            g: clamp01(g),
            b: clamp01(b),
            a: 1.0,
        }
    }

    /// Relative luminance (sRGB, IEC 61966-2-1). Used to decide light versus dark.
    #[must_use]
    pub fn luminance(self) -> f32 {
        fn lin(c: f32) -> f32 {
            if c <= 0.04045 {
                c / 12.92
            } else {
                ((c + 0.055) / 1.055).powf(2.4)
            }
        }
        0.2126 * lin(self.r) + 0.7152 * lin(self.g) + 0.0722 * lin(self.b)
    }

    /// Convert to the token colour type that [`fluent2_tokens::from_seed`] takes. Channels
    /// round to 8 bits; alpha is dropped, because a brand seed is opaque.
    #[must_use]
    pub fn to_token(self) -> fluent2_tokens::Rgba {
        let channel = |v: f32| (v * 255.0).round() as u8;
        fluent2_tokens::Rgba {
            r: channel(self.r),
            g: channel(self.g),
            b: channel(self.b),
            a: 255,
        }
    }

    /// Round to 8-bit channels for egui. The channels are straight (unmultiplied) alpha, so
    /// egui premultiplies them.
    #[must_use]
    pub fn to_color32(self) -> egui::Color32 {
        egui::Color32::from_rgba_unmultiplied(
            (self.r * 255.0).round() as u8,
            (self.g * 255.0).round() as u8,
            (self.b * 255.0).round() as u8,
            (self.a * 255.0).round() as u8,
        )
    }
}

fn clamp01(v: f32) -> f32 {
    if v.is_nan() { 0.0 } else { v.clamp(0.0, 1.0) }
}

/// What the operating system says about appearance.
///
/// Every field is [`None`] when the platform cannot answer (unsupported OS, missing
/// permission, portal timeout, or the `system-preferences` feature is off).
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct SystemPreferences {
    /// OS accent colour, if one was reported.
    pub accent: Option<Rgba>,
    /// `Some(true)` when a high-contrast theme is active.
    pub high_contrast: Option<bool>,
    /// `Some(true)` when the user asked the OS to reduce animation.
    pub reduced_motion: Option<bool>,
    /// `Some(true)` when the OS colour scheme is dark.
    pub dark: Option<bool>,
}

/// Read accent, contrast, motion and colour scheme from the OS.
///
/// Off the `system-preferences` feature, and on every target other than Windows and Linux,
/// this returns all [`None`]. Errors on a supported platform also become [`None`].
///
/// Call this at startup or when the user asks to follow the system. Do not call it every frame.
#[must_use]
pub fn system_preferences() -> SystemPreferences {
    #[cfg(all(feature = "system-preferences", windows))]
    {
        windows::read()
    }
    #[cfg(all(feature = "system-preferences", target_os = "linux"))]
    {
        linux::read()
    }
    #[cfg(not(any(
        all(feature = "system-preferences", windows),
        all(feature = "system-preferences", target_os = "linux"),
    )))]
    {
        fallback::read()
    }
}

/// Build the theme the OS reading implies, and record reduced motion.
///
/// High contrast wins and uses [`Theme::high_contrast`] (no brand seed). Otherwise a dark
/// flag selects [`Theme::dark`], and anything else (including an unknown scheme) selects
/// [`Theme::light`]. A reported accent is passed through [`fluent2_tokens::from_seed`]; a
/// missing accent uses [`fluent2_tokens::FLUENT`]. When `prefs.reduced_motion` is
/// `Some(true)`, this calls [`crate::foundation::motion::set_reduced_motion`].
#[must_use]
pub fn theme_from_system(ctx: &egui::Context, prefs: &SystemPreferences) -> Theme {
    if prefs.reduced_motion == Some(true) {
        crate::foundation::motion::set_reduced_motion(ctx, true);
    }
    if prefs.high_contrast == Some(true) {
        return Theme::high_contrast();
    }
    let brand = match prefs.accent {
        Some(accent) => fluent2_tokens::from_seed(accent.to_token()),
        None => fluent2_tokens::FLUENT,
    };
    if prefs.dark == Some(true) {
        Theme::dark(brand)
    } else {
        Theme::light(brand)
    }
}
