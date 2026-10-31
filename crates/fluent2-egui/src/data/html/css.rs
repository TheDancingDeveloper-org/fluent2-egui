//! The little CSS this view reads: colours, and a handful of inline `style` properties.
//!
//! There is no cascade, no selectors and no box model. A `style` attribute is split on `;`
//! into `name: value` pairs, and only the properties below are kept.

/// An sRGB colour from the document, before any theme remapping.
#[derive(Clone, Copy, Debug, Eq, PartialEq, Hash)]
pub struct Rgb(pub u8, pub u8, pub u8);

/// What an element's `style` attribute (and presentational attributes) said.
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub(crate) struct Declared {
    pub color: Option<Rgb>,
    pub background: Option<Rgb>,
    pub bold: Option<bool>,
    pub italic: Option<bool>,
    pub underline: Option<bool>,
    pub strike: Option<bool>,
    /// Font size in points, or a factor of the parent size.
    pub size: Option<Size>,
    pub align: Option<TextAlign>,
    /// `display: none` and the other ways mail hides preheader text.
    pub hidden: bool,
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub(crate) enum Size {
    Points(f32),
    Factor(f32),
}

/// Horizontal alignment of a block's text.
#[derive(Clone, Copy, Debug, Default, Eq, PartialEq, Hash)]
pub enum TextAlign {
    /// Start edge. The default.
    #[default]
    Left,
    /// Centred.
    Center,
    /// End edge.
    Right,
}

/// Parse a `style` attribute.
pub(crate) fn parse_style(style: &str) -> Declared {
    let mut out = Declared::default();
    let mut zero_height = false;
    let mut overflow_hidden = false;
    for declaration in style.split(';') {
        let Some((name, value)) = declaration.split_once(':') else {
            continue;
        };
        let name = name.trim().to_ascii_lowercase();
        let value = value.trim().trim_end_matches("!important").trim();
        let lower = value.to_ascii_lowercase();
        match name.as_str() {
            "color" => out.color = parse_color(value),
            "background-color" => out.background = parse_color(value),
            // `background: #fff url(...) no-repeat`: take the first colour-looking word.
            "background" => out.background = lower.split_whitespace().find_map(parse_color),
            "font-weight" => {
                out.bold = Some(match lower.as_str() {
                    "bold" | "bolder" => true,
                    "normal" | "lighter" => false,
                    number => number.parse::<u16>().is_ok_and(|n| n >= 600),
                });
            }
            "font-style" => out.italic = Some(lower == "italic" || lower == "oblique"),
            "text-decoration" | "text-decoration-line" => {
                out.underline = Some(lower.contains("underline"));
                out.strike = Some(lower.contains("line-through"));
            }
            "font-size" => {
                out.size = parse_size(&lower);
                if matches!(out.size, Some(Size::Points(p)) if p < 1.0) {
                    out.hidden = true;
                }
            }
            "text-align" => out.align = parse_align(&lower),
            "display" if lower == "none" => out.hidden = true,
            "visibility" if lower == "hidden" => out.hidden = true,
            "mso-hide" if lower == "all" => out.hidden = true,
            "opacity" if lower.parse::<f32>().is_ok_and(|o| o == 0.0) => out.hidden = true,
            "max-height" | "height" if parse_size(&lower).is_some_and(|s| s == Size::Points(0.0)) => {
                zero_height = true;
            }
            "overflow" if lower == "hidden" => overflow_hidden = true,
            _ => {}
        }
    }
    if zero_height && overflow_hidden {
        out.hidden = true;
    }
    out
}

/// `left`, `center`, `right` (and `justify` as left).
pub(crate) fn parse_align(value: &str) -> Option<TextAlign> {
    match value.trim().to_ascii_lowercase().as_str() {
        "left" | "start" | "justify" => Some(TextAlign::Left),
        "center" | "middle" => Some(TextAlign::Center),
        "right" | "end" => Some(TextAlign::Right),
        _ => None,
    }
}

/// A CSS length as points (`px` and `pt` both map to points; egui points are CSS pixels), or a
/// factor for `em`, `%` and the size keywords.
pub(crate) fn parse_size(value: &str) -> Option<Size> {
    let value = value.trim();
    let keyword = match value {
        "xx-small" => Some(9.0),
        "x-small" => Some(10.0),
        "small" => Some(13.0),
        "medium" => Some(14.0),
        "large" => Some(18.0),
        "x-large" => Some(24.0),
        "xx-large" => Some(32.0),
        _ => None,
    };
    if let Some(points) = keyword {
        return Some(Size::Points(points));
    }
    let number = |suffix: &str| {
        value
            .strip_suffix(suffix)
            .and_then(|n| n.trim().parse::<f32>().ok())
    };
    if let Some(px) = number("px") {
        Some(Size::Points(px))
    } else if let Some(pt) = number("pt") {
        Some(Size::Points(pt * 4.0 / 3.0))
    } else if let Some(em) = number("em").or_else(|| number("rem")) {
        Some(Size::Factor(em))
    } else if let Some(pct) = number("%") {
        Some(Size::Factor(pct / 100.0))
    } else {
        value.parse::<f32>().ok().map(Size::Points)
    }
    .filter(|size| match size {
        Size::Points(p) | Size::Factor(p) => p.is_finite() && *p >= 0.0,
    })
}

/// The size of `<font size="1..7">`, in points.
pub(crate) fn font_tag_size(value: &str) -> Option<Size> {
    let value = value.trim();
    let base: i32 = 3;
    let step = if let Some(rest) = value.strip_prefix('+') {
        base + rest.parse::<i32>().ok()?
    } else if let Some(rest) = value.strip_prefix('-') {
        base - rest.parse::<i32>().ok()?
    } else {
        value.parse::<i32>().ok()?
    };
    let points = match step.clamp(1, 7) {
        1 => 10.0,
        2 => 13.0,
        3 => 16.0,
        4 => 18.0,
        5 => 24.0,
        6 => 32.0,
        _ => 48.0,
    };
    Some(Size::Points(points))
}

/// `#rgb`, `#rrggbb`, `rgb(r, g, b)`, `rgba(...)` (alpha ignored unless zero) or a common
/// named colour. `transparent` and anything else is `None`.
#[must_use]
pub fn parse_color(value: &str) -> Option<Rgb> {
    let value = value.trim().to_ascii_lowercase();
    if let Some(hex) = value.strip_prefix('#') {
        let digits: Vec<u8> = hex
            .chars()
            .map(|ch| ch.to_digit(16).and_then(|d| u8::try_from(d).ok()))
            .collect::<Option<_>>()?;
        return match digits.as_slice() {
            [r, g, b] | [r, g, b, _] => Some(Rgb(r * 17, g * 17, b * 17)),
            [r1, r2, g1, g2, b1, b2] | [r1, r2, g1, g2, b1, b2, _, _] => {
                Some(Rgb(r1 * 16 + r2, g1 * 16 + g2, b1 * 16 + b2))
            }
            _ => None,
        };
    }
    if let Some(inner) = value
        .strip_prefix("rgba(")
        .or_else(|| value.strip_prefix("rgb("))
        .and_then(|rest| rest.strip_suffix(')'))
    {
        let parts: Vec<&str> = inner.split([',', ' ', '/']).filter(|p| !p.is_empty()).collect();
        if parts.len() >= 4 && parts[3].trim().parse::<f32>().is_ok_and(|a| a == 0.0) {
            return None;
        }
        let channel = |part: &str| -> Option<u8> {
            let part = part.trim();
            let number = if let Some(pct) = part.strip_suffix('%') {
                pct.parse::<f32>().ok()? * 2.55
            } else {
                part.parse::<f32>().ok()?
            };
            Some(number.round().clamp(0.0, 255.0) as u8)
        };
        return Some(Rgb(
            channel(parts.first()?)?,
            channel(parts.get(1)?)?,
            channel(parts.get(2)?)?,
        ));
    }
    NAMED.iter().find(|(name, _)| *name == value).map(|(_, rgb)| *rgb)
}

const NAMED: [(&str, Rgb); 24] = [
    ("black", Rgb(0, 0, 0)),
    ("white", Rgb(255, 255, 255)),
    ("red", Rgb(255, 0, 0)),
    ("green", Rgb(0, 128, 0)),
    ("blue", Rgb(0, 0, 255)),
    ("navy", Rgb(0, 0, 128)),
    ("gray", Rgb(128, 128, 128)),
    ("grey", Rgb(128, 128, 128)),
    ("silver", Rgb(192, 192, 192)),
    ("maroon", Rgb(128, 0, 0)),
    ("purple", Rgb(128, 0, 128)),
    ("teal", Rgb(0, 128, 128)),
    ("olive", Rgb(128, 128, 0)),
    ("orange", Rgb(255, 165, 0)),
    ("yellow", Rgb(255, 255, 0)),
    ("lime", Rgb(0, 255, 0)),
    ("aqua", Rgb(0, 255, 255)),
    ("fuchsia", Rgb(255, 0, 255)),
    ("darkgray", Rgb(169, 169, 169)),
    ("darkgrey", Rgb(169, 169, 169)),
    ("lightgray", Rgb(211, 211, 211)),
    ("lightgrey", Rgb(211, 211, 211)),
    ("whitesmoke", Rgb(245, 245, 245)),
    ("darkblue", Rgb(0, 0, 139)),
];

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn colours() {
        assert_eq!(parse_color("#FFF"), Some(Rgb(255, 255, 255)));
        assert_eq!(parse_color("#1a2B3c"), Some(Rgb(0x1a, 0x2b, 0x3c)));
        assert_eq!(parse_color("rgb(10, 20, 30)"), Some(Rgb(10, 20, 30)));
        assert_eq!(parse_color("rgba(10,20,30,0)"), None);
        assert_eq!(parse_color("rgb(100%, 0%, 0%)"), Some(Rgb(255, 0, 0)));
        assert_eq!(parse_color("Navy"), Some(Rgb(0, 0, 128)));
        assert_eq!(parse_color("transparent"), None);
        assert_eq!(parse_color("#12"), None);
        assert_eq!(parse_color("url(x)"), None);
    }

    #[test]
    fn style_attribute() {
        let d = parse_style(
            "color:#333; font-weight:700 ; font-size: 12pt; text-align:center;BACKGROUND: #eee url(a.png)",
        );
        assert_eq!(d.color, Some(Rgb(0x33, 0x33, 0x33)));
        assert_eq!(d.bold, Some(true));
        assert_eq!(d.size, Some(Size::Points(16.0)));
        assert_eq!(d.align, Some(TextAlign::Center));
        assert_eq!(d.background, Some(Rgb(0xee, 0xee, 0xee)));
        assert!(!d.hidden);
    }

    #[test]
    fn hidden_preheaders() {
        assert!(parse_style("display:none").hidden);
        assert!(parse_style("display: none !important").hidden);
        assert!(parse_style("mso-hide:all").hidden);
        assert!(parse_style("font-size:0px").hidden);
        assert!(parse_style("max-height:0; overflow:hidden").hidden);
        assert!(!parse_style("max-height:0").hidden);
    }

    #[test]
    fn sizes() {
        assert_eq!(parse_size("14px"), Some(Size::Points(14.0)));
        assert_eq!(parse_size("1.5em"), Some(Size::Factor(1.5)));
        assert_eq!(parse_size("80%"), Some(Size::Factor(0.8)));
        assert_eq!(parse_size("-3px"), None);
        assert_eq!(font_tag_size("+1"), Some(Size::Points(18.0)));
        assert_eq!(font_tag_size("9"), Some(Size::Points(48.0)));
    }

    fn prop_config() -> proptest::test_runner::Config {
        proptest::test_runner::Config {
            cases: 256,
            failure_persistence: None,
            rng_seed: proptest::test_runner::RngSeed::Fixed(0xF2E6_C055),
            ..proptest::test_runner::Config::default()
        }
    }

    proptest::proptest! {
        #![proptest_config(prop_config())]

        /// Arbitrary `style` text never panics, and any size it keeps is finite and non-negative.
        #[test]
        fn parse_style_never_panics_and_sizes_are_sane(style in ".{0,200}") {
            if let Some(Size::Points(v) | Size::Factor(v)) = parse_style(&style).size {
                proptest::prop_assert!(v.is_finite() && v >= 0.0, "size {v}");
            }
        }

        /// Any accepted length is finite and non-negative, whatever the number.
        #[test]
        fn parse_size_rejects_non_finite_and_negative(
            number in proptest::num::f32::ANY,
            unit in proptest::sample::select(vec!["", "px", "pt", "em", "rem", "%"]),
        ) {
            let text = format!("{number}{unit}");
            if let Some(Size::Points(v) | Size::Factor(v)) = parse_size(&text) {
                proptest::prop_assert!(v.is_finite() && v >= 0.0, "{text} gave {v}");
            } else {
                proptest::prop_assert!(!number.is_finite() || number < 0.0 || unit == "pt" && !(number * 4.0 / 3.0).is_finite(), "{text} was rejected");
            }
        }

        /// `<font size>` always lands on one of the seven HTML steps.
        #[test]
        fn font_tag_size_is_one_of_seven_steps(value in "[+-]?[0-9]{0,12}") {
            if let Some(size) = font_tag_size(&value) {
                let steps = [10.0, 13.0, 16.0, 18.0, 24.0, 32.0, 48.0].map(Size::Points);
                proptest::prop_assert!(steps.contains(&size), "{value} gave {size:?}");
            }
        }

        /// Hex colours round-trip in long form and short form.
        #[test]
        fn hex_colours_round_trip(r: u8, g: u8, b: u8, upper: bool) {
            let long = format!("#{r:02x}{g:02x}{b:02x}");
            let long = if upper { long.to_ascii_uppercase() } else { long };
            proptest::prop_assert_eq!(parse_color(&long), Some(Rgb(r, g, b)));
            let (r, g, b) = (r & 0xf, g & 0xf, b & 0xf);
            proptest::prop_assert_eq!(
                parse_color(&format!("#{r:x}{g:x}{b:x}")),
                Some(Rgb(r * 17, g * 17, b * 17))
            );
        }

        /// `rgb()` with in-range channels gives those channels back.
        #[test]
        fn rgb_function_round_trips(r: u8, g: u8, b: u8) {
            proptest::prop_assert_eq!(parse_color(&format!("rgb({r}, {g}, {b})")), Some(Rgb(r, g, b)));
        }

        /// Arbitrary colour text never panics.
        #[test]
        fn parse_color_never_panics(value in ".{0,64}") {
            let _ = parse_color(&value);
        }
    }
}
