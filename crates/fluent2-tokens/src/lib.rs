//! Fluent 2 design tokens as plain Rust data.
//!
//! This crate has no UI dependency, so any renderer can consume it. Token values are generated
//! from `@fluentui/tokens` (MIT) at the revision pinned in the repository's `upstream.toml`
//! (`fluentui`); see `UPSTREAM.md`. Generated modules live in [`generated`] and are never edited
//! by hand: run `cargo xtask gen-tokens`.

#![warn(missing_docs)]

pub mod generated;
mod types;

pub use types::{CubicBezier, Rgba, ShadowLayer};

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn web_light_matches_the_published_snapshot() {
        assert_eq!(
            generated::web_light::COLORS.neutral_background_1,
            Rgba::hex(0xFF_FFFF)
        );
        assert_eq!(
            generated::web_light::COLORS.neutral_foreground_1,
            Rgba::hex(0x24_2424)
        );
        assert!((generated::global::font_size::BASE_300 - 14.0).abs() < f32::EPSILON);
        assert!((generated::global::border_radius::MEDIUM - 4.0).abs() < f32::EPSILON);
    }
}
