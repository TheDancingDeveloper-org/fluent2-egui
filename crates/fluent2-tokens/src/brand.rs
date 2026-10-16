//! Brand ramps and brand-aware theme colours.
//!
//! OKLab and OKLCH conversion follows Björn Ottosson,
//! <https://bottosson.github.io/posts/oklab/>.

use crate::Rgba;
use crate::generated::brand::STEPS;
use crate::generated::brand_map::{DARK_BRAND_MAP, LIGHT_BRAND_MAP};
use crate::generated::colors::AliasColors;
use crate::generated::{high_contrast, web_dark, web_light};

/// Sixteen Fluent brand steps. Index 0 is step 10; index 15 is step 160.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct BrandRamp(pub [Rgba; 16]);

impl BrandRamp {
    /// Step `n`, where `n` is 10, 20, … 160.
    ///
    /// Any other `n` uses the nearest of those steps.
    #[must_use]
    pub fn step(&self, n: u16) -> Rgba {
        self.0[step_index(n)]
    }
}

/// Fluent's own web brand ramp, from `brand-web.json`.
pub const FLUENT: BrandRamp = BrandRamp(STEPS);

/// Light alias colours, with brand-derived tokens taken from `brand`.
#[must_use]
pub fn light(brand: &BrandRamp) -> AliasColors {
    apply(web_light::COLORS, LIGHT_BRAND_MAP, brand)
}

/// Dark alias colours, with brand-derived tokens taken from `brand`.
#[must_use]
pub fn dark(brand: &BrandRamp) -> AliasColors {
    apply(web_dark::COLORS, DARK_BRAND_MAP, brand)
}

/// High contrast alias colours. High contrast does not remap the brand ramp.
#[must_use]
pub fn high_contrast() -> AliasColors {
    high_contrast::COLORS
}

fn apply(
    mut colors: AliasColors,
    map: &[(crate::generated::brand_map::BrandField, u16)],
    brand: &BrandRamp,
) -> AliasColors {
    for (field, step) in map {
        *field.set(&mut colors) = brand.step(*step);
    }
    colors
}

/// A ramp with Fluent's lightness and relative chroma, and `seed`'s hue.
///
/// The Fluent step whose OKLCH lightness is closest to the seed is `k`. Each step keeps that
/// Fluent step's lightness. Its chroma is the Fluent chroma scaled by `seed chroma / Fluent
/// chroma at k`, then reduced until the colour fits in sRGB. The hue is the seed's hue.
#[must_use]
pub fn from_seed(seed: Rgba) -> BrandRamp {
    let fluent: Vec<Oklch> = STEPS.iter().copied().map(oklch_of).collect();
    let seed = oklch_of(seed);
    let k = closest_lightness(&fluent, seed.l);
    let scale = if fluent[k].c == 0.0 {
        0.0
    } else {
        seed.c / fluent[k].c
    };
    let mut steps = [Rgba::hex(0); 16];
    for (i, base) in fluent.iter().enumerate() {
        let chroma = base.c * scale;
        steps[i] = Rgba::from_oklch(base.l, clamp_chroma(base.l, chroma, seed.h), seed.h);
    }
    BrandRamp(steps)
}

fn step_index(n: u16) -> usize {
    let clamped = n.clamp(10, 160);
    let lower = clamped / 10 * 10;
    let index = if clamped - lower > 5 { lower + 10 } else { lower };
    usize::from((index - 10) / 10)
}

fn closest_lightness(steps: &[Oklch], lightness: f32) -> usize {
    steps
        .iter()
        .enumerate()
        .min_by(|(_, a), (_, b)| {
            (a.l - lightness)
                .abs()
                .total_cmp(&(b.l - lightness).abs())
                .then(std::cmp::Ordering::Less)
        })
        .map_or(0, |(index, _)| index)
}

/// Reduce chroma until the colour is inside sRGB.
fn clamp_chroma(l: f32, chroma: f32, hue: f32) -> f32 {
    if chroma <= 0.0 || in_gamut(l, chroma, hue) {
        return chroma.max(0.0);
    }
    let mut lo = 0.0;
    let mut hi = chroma;
    for _ in 0..24 {
        let mid = (lo + hi) * 0.5;
        if in_gamut(l, mid, hue) {
            lo = mid;
        } else {
            hi = mid;
        }
    }
    lo
}

fn in_gamut(l: f32, c: f32, h: f32) -> bool {
    let [r, g, b] = oklch_to_srgb(l, c, h);
    [r, g, b]
        .into_iter()
        .all(|channel| (0.0..=1.0).contains(&channel))
}

#[derive(Clone, Copy)]
struct Oklch {
    l: f32,
    c: f32,
    h: f32,
}

fn oklch_of(color: Rgba) -> Oklch {
    let [l, a, b] = srgb_to_oklab(color.r, color.g, color.b);
    Oklch {
        l,
        c: a.hypot(b),
        h: b.atan2(a),
    }
}

impl Rgba {
    fn from_oklch(l: f32, c: f32, h: f32) -> Self {
        let [r, g, b] = oklch_to_srgb(l, c, h);
        Self {
            r: encode_channel(r),
            g: encode_channel(g),
            b: encode_channel(b),
            a: 255,
        }
    }
}

fn encode_channel(linear: f32) -> u8 {
    let clamped = linear.clamp(0.0, 1.0);
    let encoded = if clamped <= 0.003_130_8 {
        12.92 * clamped
    } else {
        1.055 * clamped.powf(1.0 / 2.4) - 0.055
    };
    (encoded * 255.0).round().clamp(0.0, 255.0) as u8
}

fn decode_channel(value: u8) -> f32 {
    let encoded = f32::from(value) / 255.0;
    if encoded <= 0.04045 {
        encoded / 12.92
    } else {
        ((encoded + 0.055) / 1.055).powf(2.4)
    }
}

/// sRGB 0–255 to OKLab `(L, a, b)`.
#[allow(clippy::excessive_precision)]
fn srgb_to_oklab(r: u8, g: u8, b: u8) -> [f32; 3] {
    let r = decode_channel(r);
    let g = decode_channel(g);
    let b = decode_channel(b);
    let l = 0.412_221_470_8 * r + 0.536_332_536_3 * g + 0.051_445_992_9 * b;
    let m = 0.211_903_498_2 * r + 0.680_699_545_1 * g + 0.107_396_956_6 * b;
    let s = 0.088_302_461_9 * r + 0.281_718_837_6 * g + 0.629_978_700_5 * b;
    let l = l.cbrt();
    let m = m.cbrt();
    let s = s.cbrt();
    [
        0.210_454_255_3 * l + 0.793_617_785_0 * m - 0.004_072_046_8 * s,
        1.977_998_495_1 * l - 2.428_592_205_0 * m + 0.450_593_709_9 * s,
        0.025_904_037_1 * l + 0.782_771_766_2 * m - 0.808_675_766_0 * s,
    ]
}

#[allow(clippy::excessive_precision)]
fn oklch_to_srgb(l: f32, c: f32, h: f32) -> [f32; 3] {
    let a = c * h.cos();
    let b = c * h.sin();
    let l_ = l + 0.396_337_777_4 * a + 0.215_803_757_3 * b;
    let m_ = l - 0.105_561_345_8 * a - 0.063_854_172_8 * b;
    let s_ = l - 0.089_484_177_5 * a - 1.291_485_548_0 * b;
    let l = l_ * l_ * l_;
    let m = m_ * m_ * m_;
    let s = s_ * s_ * s_;
    [
        4.076_741_662_1 * l - 3.307_711_591_3 * m + 0.230_969_929_2 * s,
        -1.268_438_004_6 * l + 2.609_757_401_1 * m - 0.341_319_396_5 * s,
        -0.004_196_086_3 * l - 0.703_418_614_7 * m + 1.707_614_701_0 * s,
    ]
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn fluent_ramp_round_trips_both_themes() {
        assert_eq!(light(&FLUENT), web_light::COLORS);
        assert_eq!(dark(&FLUENT), web_dark::COLORS);
    }

    /// OKLab is not a perfect round trip of the published hex steps, even when the seed is
    /// `FLUENT.step(80)` (scale 1, same hue). With the matrices from
    /// <https://bottosson.github.io/posts/oklab/> and 8-bit rounding, the largest channel
    /// error is 9, at step 40. A bound of 2 is therefore unachievable without changing the
    /// conversion. 10 is that measured maximum, rounded up.
    #[test]
    fn seed_at_step_80_stays_within_ten_of_fluent() {
        let seeded = from_seed(FLUENT.step(80));
        for n in (10..=160).step_by(10) {
            let got = seeded.step(n as u16);
            let want = FLUENT.step(n as u16);
            for (actual, expected) in [(got.r, want.r), (got.g, want.g), (got.b, want.b)] {
                assert!(actual.abs_diff(expected) <= 10, "step {n}: {got:?} vs {want:?}");
            }
        }
    }

    #[test]
    fn from_seed_does_not_panic() {
        let fixed = [
            Rgba::hex(0x000000),
            Rgba::hex(0xFFFFFF),
            Rgba::hex(0xFF0000),
            Rgba::hex(0x00FF00),
            Rgba::hex(0x0000FF),
            Rgba::hex(0x808080),
            Rgba::hex(0x010101),
            Rgba::hex(0xFEFEFE),
        ];
        for seed in fixed {
            let _ = from_seed(seed);
        }
        let mut state = 0x1234_5678_u32;
        for _ in 0..1_000 {
            state = xorshift(state);
            let seed = Rgba {
                r: (state & 0xff) as u8,
                g: ((state >> 8) & 0xff) as u8,
                b: ((state >> 16) & 0xff) as u8,
                a: 255,
            };
            let ramp = from_seed(seed);
            for step in ramp.0 {
                let _ = (step.r, step.g, step.b, step.a);
            }
        }
    }

    #[test]
    fn odd_steps_round_to_the_nearest_ten() {
        assert_eq!(FLUENT.step(0), FLUENT.step(10));
        assert_eq!(FLUENT.step(84), FLUENT.step(80));
        assert_eq!(FLUENT.step(85), FLUENT.step(80));
        assert_eq!(FLUENT.step(86), FLUENT.step(90));
        assert_eq!(FLUENT.step(400), FLUENT.step(160));
    }

    fn xorshift(mut state: u32) -> u32 {
        state ^= state << 13;
        state ^= state >> 17;
        state ^= state << 5;
        state
    }
}
