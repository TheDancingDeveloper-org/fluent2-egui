//! Windows appearance via `UISettings` and `AccessibilitySettings`.
//!
//! Every failure is "unknown" (`None`). This module is only compiled for `cfg(windows)` with
//! the `system-preferences` feature.
//!
//! High contrast uses `AccessibilitySettings::HighContrast`. It is the safe WinRT query for
//! the same setting that `SystemParametersInfoW(SPI_GETHIGHCONTRAST)` reports. The workspace
//! forbids `unsafe`, and the `windows` crate exposes the Win32 query only as an unsafe call.

use windows::UI::ViewManagement::{AccessibilitySettings, UIColorType, UISettings};

use super::{Rgba, SystemPreferences};

/// Read accent, contrast, animations and background luminance.
#[must_use]
pub fn read() -> SystemPreferences {
    let settings = UISettings::new().ok();
    SystemPreferences {
        accent: settings.as_ref().and_then(accent),
        high_contrast: high_contrast(),
        reduced_motion: settings.as_ref().and_then(reduced_motion),
        dark: settings.as_ref().and_then(dark),
    }
}

fn accent(settings: &UISettings) -> Option<Rgba> {
    let color = settings.GetColorValue(UIColorType::Accent).ok()?;
    Some(from_ui_color(color))
}

fn dark(settings: &UISettings) -> Option<bool> {
    let color = settings.GetColorValue(UIColorType::Background).ok()?;
    Some(from_ui_color(color).luminance() < 0.5)
}

fn reduced_motion(settings: &UISettings) -> Option<bool> {
    match settings.AnimationsEnabled() {
        Ok(enabled) => Some(!enabled),
        Err(_) => None,
    }
}

fn from_ui_color(color: windows::UI::Color) -> Rgba {
    Rgba::new(
        f32::from(color.R) / 255.0,
        f32::from(color.G) / 255.0,
        f32::from(color.B) / 255.0,
    )
}

fn high_contrast() -> Option<bool> {
    let settings = AccessibilitySettings::new().ok()?;
    settings.HighContrast().ok()
}
