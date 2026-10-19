//! Density: compact, medium and touch sizing for every control (`docs/specs/density.md`).
//!
//! Fluent 2 web sizes controls Small 24, Medium 32 and Large 40. Density maps those base
//! heights once, here, and every control asks [`Density::height`] for the height it
//! allocates. Compact steps one size down; Touch makes every interactive target at least
//! [`TOUCH_TARGET`] points. Fonts and icons keep their sizes in every density.

use fluent2_tokens::generated::global::{spacing_horizontal, spacing_vertical};

/// Smallest interactive target in [`Density::Touch`], in points (Apple HIG and Fluent mobile).
pub const TOUCH_TARGET: f32 = 44.0;

/// How tightly controls are sized. Set it with [`super::Theme::with_density`].
///
/// ```
/// use fluent2_egui::theme::Density;
///
/// assert_eq!(Density::Medium.height(32.0), 32.0);
/// assert_eq!(Density::Compact.height(32.0), 24.0);
/// assert_eq!(Density::Touch.height(32.0), 44.0);
/// ```
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub enum Density {
    /// One size down: 40 → 32, 32 → 24. Heights below 32 are kept. For dense desktop tools.
    Compact,
    /// The Fluent 2 sizes as specified. The default.
    #[default]
    Medium,
    /// Every interactive control at least [`TOUCH_TARGET`] tall (icon buttons square), with
    /// wider spacing. For phones and tablets.
    Touch,
}

impl Density {
    /// The height to allocate for a control whose Medium-density height is `base`.
    /// Non-finite or negative `base` is treated as 0 (ARCHITECTURE rule 9).
    #[must_use]
    pub fn height(self, base: f32) -> f32 {
        let base = if base.is_finite() { base.max(0.0) } else { 0.0 };
        match self {
            Self::Compact if base >= 32.0 => base - 8.0,
            Self::Compact | Self::Medium => base,
            Self::Touch => base.max(TOUCH_TARGET),
        }
    }

    /// Row height for a list, tree or menu row whose Medium-density height is `base`. The same
    /// as [`Self::height`]; named for call sites that size rows rather than controls.
    #[must_use]
    pub fn row(self, base: f32) -> f32 {
        self.height(base)
    }

    /// Smallest target width: [`TOUCH_TARGET`] in Touch, 0 otherwise. Controls take
    /// `width.max(density.min_target())` so an unlabelled checkbox is still 44 wide.
    #[must_use]
    pub fn min_target(self) -> f32 {
        if self == Self::Touch { TOUCH_TARGET } else { 0.0 }
    }

    /// egui `item_spacing` for this density: `spacing*XS`, `spacing*S` or `spacing*M`.
    #[must_use]
    pub fn item_spacing(self) -> egui::Vec2 {
        match self {
            Self::Compact => egui::vec2(spacing_horizontal::XS, spacing_vertical::XS),
            Self::Medium => egui::vec2(spacing_horizontal::S, spacing_vertical::S),
            Self::Touch => egui::vec2(spacing_horizontal::M, spacing_vertical::M),
        }
    }
}

/// The density of the installed theme.
pub(crate) fn of(ui: &egui::Ui) -> Density {
    super::Theme::get(ui.ctx()).density
}

#[cfg(test)]
mod tests {
    use super::{Density, TOUCH_TARGET};

    #[test]
    fn compact_steps_one_fluent_size_down() {
        assert_eq!(Density::Compact.height(40.0), 32.0);
        assert_eq!(Density::Compact.height(32.0), 24.0);
        assert_eq!(Density::Compact.height(24.0), 24.0);
        assert_eq!(Density::Compact.height(20.0), 20.0);
    }

    #[test]
    fn touch_targets_are_at_least_44() {
        for base in [0.0, 16.0, 24.0, 32.0, 40.0] {
            assert_eq!(Density::Touch.height(base), TOUCH_TARGET);
        }
        assert_eq!(Density::Touch.height(60.0), 60.0);
    }

    #[test]
    fn medium_is_identity_and_bad_input_is_clamped() {
        assert_eq!(Density::Medium.height(32.0), 32.0);
        assert_eq!(Density::Medium.height(f32::NAN), 0.0);
        assert_eq!(Density::Medium.height(-5.0), 0.0);
        assert_eq!(Density::Touch.height(f32::INFINITY), TOUCH_TARGET);
    }

    #[test]
    fn spacing_grows_with_density() {
        let (c, m, t) = (
            Density::Compact.item_spacing(),
            Density::Medium.item_spacing(),
            Density::Touch.item_spacing(),
        );
        assert!(c.x < m.x && m.x < t.x && c.y < m.y && m.y < t.y);
    }
}
