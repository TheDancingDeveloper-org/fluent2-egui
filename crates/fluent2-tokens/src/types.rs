//! Hand-written value types the generated token modules are built from.

/// An 8-bit colour. Generated tokens use this instead of any UI colour type.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Rgba {
    /// Red.
    pub r: u8,
    /// Green.
    pub g: u8,
    /// Blue.
    pub b: u8,
    /// Alpha. `255` is opaque.
    pub a: u8,
}

impl Rgba {
    /// An opaque colour from `0xRRGGBB`.
    #[must_use]
    pub const fn hex(rgb: u32) -> Self {
        Self {
            r: ((rgb >> 16) & 0xff) as u8,
            g: ((rgb >> 8) & 0xff) as u8,
            b: (rgb & 0xff) as u8,
            a: 255,
        }
    }
}

/// One layer of a Fluent shadow (`x y blur spread color`).
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct ShadowLayer {
    /// Horizontal offset, in pixels.
    pub x: f32,
    /// Vertical offset, in pixels.
    pub y: f32,
    /// Blur radius, in pixels.
    pub blur: f32,
    /// Spread radius, in pixels.
    pub spread: f32,
    /// Layer colour.
    pub color: Rgba,
}

/// A CSS `cubic-bezier(x1, y1, x2, y2)` easing.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct CubicBezier(pub [f32; 4]);
