//! WCAG contrast of every foreground/background token pair the components paint, in light,
//! dark and high contrast (WI-1586). `docs/a11y-report.md` is this table, printed with
//! `cargo test -p fluent2-egui --test a11y_contrast -- --ignored --nocapture`.
//!
//! Text pairs need 4.5:1 (WCAG 1.4.3) and non-text pairs 3:1 (WCAG 1.4.11). Disabled pairs
//! are listed but exempt, as WCAG exempts inactive components. A pair that fails is either
//! in [`KNOWN_FAILURES`] (a Fluent token choice, written up in the report) or a bug.

use fluent2_egui::theme::Theme;
use fluent2_tokens::{AliasColors, FLUENT, Rgba};

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
enum Kind {
    /// Text and icons that carry meaning: 4.5:1.
    Text,
    /// Boundaries and state indicators of controls: 3:1.
    NonText,
    /// Disabled content: reported, never required.
    Disabled,
}

impl Kind {
    fn minimum(self) -> f64 {
        match self {
            Self::Text => 4.5,
            Self::NonText => 3.0,
            Self::Disabled => 0.0,
        }
    }
}

type Token = fn(&AliasColors) -> Rgba;

struct Pair {
    component: &'static str,
    part: &'static str,
    kind: Kind,
    fg: (&'static str, Token),
    bg: (&'static str, Token),
    /// The foreground high contrast paints instead, when it differs.
    hc_fg: Option<(&'static str, Token)>,
}

macro_rules! pair {
    ($component:literal, $part:literal, $kind:ident, $fg:ident on $bg:ident) => {
        Pair {
            component: $component,
            part: $part,
            kind: Kind::$kind,
            fg: (stringify!($fg), |c| c.$fg),
            bg: (stringify!($bg), |c| c.$bg),
            hc_fg: None,
        }
    };
    ($component:literal, $part:literal, $kind:ident, $fg:ident on $bg:ident, hc $hc:ident) => {
        Pair {
            hc_fg: Some((stringify!($hc), |c| c.$hc)),
            ..pair!($component, $part, $kind, $fg on $bg)
        }
    };
}

/// Pairs read off each component's paint code. A translucent background is composed over
/// `neutral_background_1`, the surface every component sits on unless noted.
fn pairs() -> Vec<Pair> {
    vec![
        pair!("Theme", "body text on window", Text, neutral_foreground_1 on neutral_background_1),
        pair!("Theme", "body text on panel", Text, neutral_foreground_1 on neutral_background_2),
        pair!("Button", "label", Text, neutral_foreground_1 on neutral_background_1),
        pair!("Button", "label, hover", Text, neutral_foreground_1_hover on neutral_background_1_hover),
        pair!("Button", "label, pressed", Text, neutral_foreground_1_pressed on neutral_background_1_pressed),
        pair!("Button", "border", NonText, neutral_stroke_1 on neutral_background_1),
        pair!("Button", "primary label", Text, neutral_foreground_on_brand on brand_background),
        pair!("Button", "primary label, hover", Text, neutral_foreground_on_brand on brand_background_hover),
        pair!("Button", "primary label, pressed", Text, neutral_foreground_on_brand on brand_background_pressed),
        pair!("Button", "primary fill vs surface", NonText, brand_background on neutral_background_1),
        pair!("Button", "subtle label", Text, neutral_foreground_2 on subtle_background),
        pair!("Button", "subtle label, hover", Text, neutral_foreground_2_hover on subtle_background_hover),
        pair!("Button", "subtle label, brand hover", Text, neutral_foreground_2_brand_hover on subtle_background_hover),
        pair!("Button", "disabled label", Disabled, neutral_foreground_disabled on neutral_background_disabled),
        pair!("Checkbox", "unchecked box", NonText, neutral_stroke_accessible on neutral_background_1),
        pair!("Checkbox", "checked fill", NonText, compound_brand_background on neutral_background_1),
        pair!("Checkbox", "check mark", NonText, neutral_foreground_inverted on compound_brand_background),
        pair!("Checkbox", "label", Text, neutral_foreground_1 on neutral_background_1),
        pair!("Radio", "ring", NonText, neutral_stroke_accessible on neutral_background_1),
        pair!("Radio", "checked dot", NonText, compound_brand_foreground_1 on neutral_background_1),
        pair!("Switch", "off track", NonText, neutral_stroke_accessible on neutral_background_1),
        pair!("Switch", "on track", NonText, compound_brand_background on neutral_background_1),
        pair!("Switch", "on thumb", NonText, neutral_foreground_inverted on compound_brand_background),
        pair!("Switch", "off thumb", NonText, neutral_foreground_3 on neutral_background_1),
        pair!("Input", "value", Text, neutral_foreground_1 on neutral_background_1),
        pair!("Input", "placeholder", Text, neutral_foreground_4 on neutral_background_1),
        pair!("Input", "underline", NonText, neutral_stroke_accessible on neutral_background_1),
        pair!("Input", "focus underline", NonText, compound_brand_stroke on neutral_background_1),
        pair!("Input", "filled-darker value", Text, neutral_foreground_1 on neutral_background_3),
        pair!("Field", "hint", Text, neutral_foreground_3 on neutral_background_1),
        pair!("Field", "error message", Text, status_danger_foreground_1 on neutral_background_1),
        pair!("Link", "text", Text, brand_foreground_link on neutral_background_1),
        pair!("Link", "text, hover", Text, brand_foreground_link_hover on neutral_background_1),
        pair!("Slider", "rail", NonText, neutral_stroke_accessible on neutral_background_1),
        pair!("Slider", "thumb and fill", NonText, compound_brand_background on neutral_background_1),
        pair!("ProgressBar", "fill vs track", NonText, compound_brand_background on neutral_background_6),
        pair!("Spinner", "arc", NonText, brand_stroke_1 on neutral_background_1),
        pair!("SpinButton", "arrows", Text, neutral_foreground_2 on neutral_background_1),
        pair!("Dropdown / Combobox", "placeholder", Text, neutral_foreground_4 on neutral_background_1),
        pair!("Dropdown / Combobox", "chevron", Text, neutral_foreground_3 on neutral_background_1),
        pair!("Focus ring", "outer ring", NonText, stroke_focus_2 on neutral_background_1),
        pair!("Focus ring", "outer ring on panel", NonText, stroke_focus_2 on neutral_background_2),
        pair!("Focus ring", "inner ring on brand", NonText, stroke_focus_1 on brand_background),
        pair!("Menu", "item", Text, neutral_foreground_1 on neutral_background_1),
        pair!("Menu", "item, hover", Text, neutral_foreground_1_hover on neutral_background_1_hover),
        pair!("Menu", "secondary text", Text, neutral_foreground_3 on neutral_background_1),
        pair!("Menu / Popover", "surface border", NonText, neutral_stroke_2 on neutral_background_1),
        pair!("Tooltip", "text", Text, neutral_foreground_1 on neutral_background_1),
        pair!("Tooltip", "inverted text", Text, neutral_foreground_static_inverted on neutral_background_static),
        pair!("TabList", "selected label", Text, neutral_foreground_1 on neutral_background_1),
        pair!("TabList", "label", Text, neutral_foreground_2 on neutral_background_1),
        pair!("TabList", "subtle selected label", Text, neutral_foreground_1_selected on subtle_background_selected),
        pair!("Dropdown / Combobox", "option, hover", Text, neutral_foreground_1_hover on subtle_background_hover),
        pair!("DataGrid", "sortable header, hover", Text, neutral_foreground_1_hover on subtle_background_hover),
        pair!("DataGrid", "row, hover", Text, neutral_foreground_1_hover on subtle_background_hover),
        pair!("Ribbon gallery", "scroll button, hover", Text, neutral_foreground_2_hover on subtle_background_hover),
        pair!("TabList", "selected indicator", NonText, compound_brand_stroke on neutral_background_1),
        pair!("Card", "text on filled-alternative", Text, neutral_foreground_1 on neutral_background_2),
        pair!("Card", "border", NonText, neutral_stroke_1 on neutral_background_1),
        // WI-1845: an interactive card's content takes the foreground for its fill.
        pair!("Card", "content, hover", Text, neutral_foreground_1_hover on neutral_background_1_hover),
        pair!("Card", "content, pressed", Text, neutral_foreground_1_pressed on neutral_background_1_pressed),
        pair!("Card", "content, selected", Text, neutral_foreground_1_selected on neutral_background_1_selected),
        pair!("Card", "filled-alternative content, hover", Text, neutral_foreground_1_hover on neutral_background_2_hover),
        pair!("Card", "filled-alternative content, selected", Text, neutral_foreground_1_selected on neutral_background_2_selected),
        pair!("Card", "subtle content, hover", Text, neutral_foreground_1_hover on subtle_background_hover),
        pair!("Card", "subtle content, selected", Text, neutral_foreground_1_selected on subtle_background_selected),
        pair!("Ribbon gallery", "tile content, hover", Text, neutral_foreground_1_hover on subtle_background_hover),
        pair!("Ribbon gallery", "tile content, pressed", Text, neutral_foreground_1_pressed on subtle_background_pressed),
        pair!("Ribbon gallery", "tile content, selected", Text, neutral_foreground_1_selected on subtle_background_selected),
        pair!("MessageBar", "info text", Text, neutral_foreground_1 on neutral_background_3),
        pair!("MessageBar", "success text", Text, neutral_foreground_1 on status_success_background_1),
        pair!("MessageBar", "warning text", Text, neutral_foreground_1 on status_warning_background_1),
        pair!("MessageBar", "error text", Text, neutral_foreground_1 on status_danger_background_1),
        pair!("MessageBar", "success icon", NonText, status_success_foreground_1 on status_success_background_1),
        pair!("MessageBar", "warning icon", NonText, status_warning_foreground_1 on status_warning_background_1),
        pair!("MessageBar", "error icon", NonText, status_danger_foreground_1 on status_danger_background_1),
        pair!("MessageBar", "success border", NonText, status_success_border_1 on neutral_background_1),
        pair!("MessageBar", "error border", NonText, status_danger_border_1 on neutral_background_1),
        pair!("TitleBar", "title", Text, neutral_foreground_1 on neutral_background_1),
        pair!("TitleBar", "caption glyph, hover", Text, neutral_foreground_1_hover on subtle_background_hover),
        pair!("TitleBar", "close glyph, hover", Text, neutral_foreground_on_brand on status_danger_background_3),
        pair!("QAT", "overflow glyph", Text, neutral_foreground_3 on neutral_background_1),
        pair!("Ribbon", "tab label", Text, neutral_foreground_2 on neutral_background_1),
        pair!("Ribbon", "selected tab label", Text, brand_foreground_1 on neutral_background_1),
        pair!("Ribbon", "contextual tab, success", Text, status_success_foreground_1 on neutral_background_1),
        pair!("Ribbon", "contextual tab, warning", Text, status_warning_foreground_1 on neutral_background_1),
        pair!("Ribbon", "contextual tab, danger", Text, status_danger_foreground_1 on neutral_background_1),
        pair!("Ribbon", "File tab label", Text, neutral_foreground_on_brand on brand_background),
        pair!("Ribbon", "group label", Text, neutral_foreground_2 on neutral_background_1),
        pair!("Ribbon", "control label, hover", Text, neutral_foreground_1_hover on subtle_background_hover),
        pair!("Ribbon gallery", "item border, selected", NonText, brand_stroke_1 on neutral_background_1),
        pair!("KeyTips", "badge letter", Text, neutral_foreground_1 on neutral_background_1),
        pair!("KeyTips", "typed prefix", Text, brand_foreground_1 on neutral_background_1),
        pair!("KeyTips", "badge border", NonText, neutral_stroke_1 on neutral_background_1),
        pair!("Backstage", "item, selected", Text, neutral_foreground_1_selected on subtle_background_selected),
        pair!("Backstage", "selected bar", NonText, brand_foreground_1 on subtle_background_selected, hc neutral_foreground_1_selected),
        pair!("StatusBar", "text", Text, neutral_foreground_2 on neutral_background_2),
        pair!("DataGrid", "header", Text, neutral_foreground_1 on neutral_background_1),
        pair!("DataGrid", "row, selected", Text, neutral_foreground_1_selected on subtle_background_selected),
        pair!("TreeView / List", "row", Text, neutral_foreground_2 on neutral_background_1),
        pair!("TreeView / List", "row, hover", Text, neutral_foreground_2_hover on subtle_background_hover),
        pair!("TreeView / List", "row, selected", Text, neutral_foreground_1_selected on subtle_background_selected),
        pair!("TreeView / List", "secondary text", Text, neutral_foreground_3 on neutral_background_1),
        pair!("NavigationView", "item", Text, neutral_foreground_2 on neutral_background_4),
        pair!("NavigationView", "item, hover", Text, neutral_foreground_2_hover on neutral_background_4_hover),
        pair!("NavigationView", "item, selected", Text, neutral_foreground_2_brand_selected on neutral_background_4_selected),
        pair!("NavigationView", "header", Text, neutral_foreground_3 on neutral_background_4),
        pair!("SplitView", "splitter", NonText, neutral_stroke_1 on neutral_background_1),
        pair!("SplitView", "splitter, focused", NonText, brand_stroke_1 on neutral_background_1),
    ]
}

/// (theme, component, part) pairs that fail because of Fluent's own token values. Each has a
/// row in `docs/a11y-report.md`.
const KNOWN_FAILURES: &[(&str, &str, &str)] = &[
    // Decorative 1 px edges: Fluent's neutral and status strokes sit below 3:1 in light and
    // dark. Each control is still identified by its text, fill or icon (report §2.1).
    ("light", "Button", "border"),
    ("dark", "Button", "border"),
    ("light", "Menu / Popover", "surface border"),
    ("dark", "Menu / Popover", "surface border"),
    ("light", "Card", "border"),
    ("dark", "Card", "border"),
    ("light", "MessageBar", "success border"),
    ("dark", "MessageBar", "success border"),
    ("light", "MessageBar", "error border"),
    ("dark", "MessageBar", "error border"),
    ("light", "KeyTips", "badge border"),
    ("dark", "KeyTips", "badge border"),
    ("light", "SplitView", "splitter"),
    ("dark", "SplitView", "splitter"),
    // Dark-only token values (report §2.2).
    ("dark", "Button", "primary fill vs surface"),
    ("dark", "Button", "subtle label, brand hover"),
    ("dark", "Field", "error message"),
    ("dark", "Ribbon", "contextual tab, danger"),
];

fn themes() -> [(&'static str, AliasColors); 3] {
    [
        ("light", Theme::light(FLUENT).color),
        ("dark", Theme::dark(FLUENT).color),
        ("hc", Theme::high_contrast().color),
    ]
}

/// `top` over `bottom`, both unmultiplied sRGB.
fn over(top: Rgba, bottom: Rgba) -> Rgba {
    let a = f64::from(top.a) / 255.0;
    let mix = |t: u8, b: u8| (f64::from(t) * a + f64::from(b) * (1.0 - a)).round() as u8;
    Rgba {
        r: mix(top.r, bottom.r),
        g: mix(top.g, bottom.g),
        b: mix(top.b, bottom.b),
        a: 255,
    }
}

fn luminance(c: Rgba) -> f64 {
    let channel = |v: u8| {
        let s = f64::from(v) / 255.0;
        if s <= 0.04045 {
            s / 12.92
        } else {
            ((s + 0.055) / 1.055).powf(2.4)
        }
    };
    0.2126 * channel(c.r) + 0.7152 * channel(c.g) + 0.0722 * channel(c.b)
}

fn ratio(a: Rgba, b: Rgba) -> f64 {
    let (la, lb) = (luminance(a), luminance(b));
    let (hi, lo) = if la > lb { (la, lb) } else { (lb, la) };
    (hi + 0.05) / (lo + 0.05)
}

struct Row {
    theme: &'static str,
    pair: Pair,
    ratio: f64,
}

fn rows() -> Vec<Row> {
    let mut rows = Vec::new();
    for (theme, colors) in themes() {
        for pair in pairs() {
            let surface = over(
                colors.neutral_background_1,
                Rgba {
                    r: 0,
                    g: 0,
                    b: 0,
                    a: 255,
                },
            );
            let bg = over((pair.bg.1)(&colors), surface);
            let fg_token = match pair.hc_fg {
                Some((_, token)) if theme == "hc" => token,
                _ => pair.fg.1,
            };
            let fg = over(fg_token(&colors), bg);
            rows.push(Row {
                theme,
                ratio: ratio(fg, bg),
                pair,
            });
        }
    }
    rows
}

fn fails(row: &Row) -> bool {
    row.ratio + 1e-9 < row.pair.kind.minimum()
}

#[test]
fn every_failing_pair_is_a_known_token_failure() {
    let failing: Vec<(&str, &str, &str)> = rows()
        .iter()
        .filter(|row| fails(row))
        .map(|row| (row.theme, row.pair.component, row.pair.part))
        .collect();
    let unexpected: Vec<_> = failing.iter().filter(|f| !KNOWN_FAILURES.contains(f)).collect();
    let fixed: Vec<_> = KNOWN_FAILURES.iter().filter(|k| !failing.contains(k)).collect();
    assert!(unexpected.is_empty(), "new contrast failures: {unexpected:#?}");
    assert!(
        fixed.is_empty(),
        "no longer failing, drop from KNOWN_FAILURES: {fixed:#?}"
    );
}

#[test]
fn high_contrast_passes_every_pair() {
    let failing: Vec<_> = rows()
        .into_iter()
        .filter(|row| row.theme == "hc" && fails(row))
        .map(|row| format!("{} / {}: {:.2}", row.pair.component, row.pair.part, row.ratio))
        .collect();
    assert!(failing.is_empty(), "high contrast failures: {failing:#?}");
}

#[test]
fn contrast_math_matches_wcag() {
    let black = Rgba {
        r: 0,
        g: 0,
        b: 0,
        a: 255,
    };
    let white = Rgba {
        r: 255,
        g: 255,
        b: 255,
        a: 255,
    };
    assert!((ratio(black, white) - 21.0).abs() < 1e-9);
    assert!((ratio(white, white) - 1.0).abs() < 1e-9);
}

/// Print the report table.
#[test]
#[ignore = "prints docs/a11y-report.md's table"]
fn print_table() {
    let rows = rows();
    println!("| component | part | kind | foreground | background | light | dark | hc |");
    println!("|---|---|---|---|---|---|---|---|");
    let count = pairs().len();
    for index in 0..count {
        let cell = |theme: &str| {
            rows.iter()
                .filter(|row| row.theme == theme)
                .nth(index)
                .map(|row| {
                    if fails(row) {
                        format!("**{:.2} ✗**", row.ratio)
                    } else {
                        format!("{:.2}", row.ratio)
                    }
                })
                .unwrap_or_default()
        };
        let Some(row) = rows.get(index) else { continue };
        let kind = match row.pair.kind {
            Kind::Text => "text 4.5",
            Kind::NonText => "non-text 3",
            Kind::Disabled => "disabled",
        };
        let fg = match row.pair.hc_fg {
            Some((hc, _)) => format!("`{}` (hc: `{hc}`)", row.pair.fg.0),
            None => format!("`{}`", row.pair.fg.0),
        };
        println!(
            "| {} | {} | {} | {} | `{}` | {} | {} | {} |",
            row.pair.component,
            row.pair.part,
            kind,
            fg,
            row.pair.bg.0,
            cell("light"),
            cell("dark"),
            cell("hc"),
        );
    }
}
