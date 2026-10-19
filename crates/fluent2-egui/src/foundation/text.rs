//! Fluent 2 type ramp as egui [`TextStyle`]s, and the OS UI font when available.
//!
//! Sizes and line heights come from [`fluent2_tokens::generated::global`]. egui's [`FontId`] has
//! no line-height field, so each step's line height is recorded beside its size for callers that
//! lay text out themselves.
//!
//! Fonts, in order of preference: on Windows the system UI face is read from the OS at runtime
//! (Segoe UI, never shipped). Everywhere else, and on Windows without Segoe UI, the bundled Inter
//! Regular and SemiBold faces are used (OFL-1.1, see `fonts/`; cargo feature `bundled-font`).
//! With that feature off, egui's own defaults stay in place. A font file that cannot be read is
//! skipped and logged at debug.

use std::ops::Range;

use egui::text::{LayoutJob, TextFormat};
use egui::{Color32, FontDefinitions, FontFamily, FontId, RichText, Stroke, TextStyle};

/// Name of the family used for semibold steps of the ramp.
///
/// egui has no font-weight axis on a [`FontId`], so semibold is a second family. It points at a
/// semibold face when one was found, and at the same face as [`FontFamily::Proportional`]
/// otherwise.
pub const SEMIBOLD_FAMILY: &str = "fluent2-semibold";

/// One step of the Fluent type ramp, in Fluent's own order from smallest to largest.
#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
pub enum TypeRamp {
    /// `caption2`: `fontSizeBase100`, regular.
    Caption2,
    /// `caption1`: `fontSizeBase200`, regular.
    Caption1,
    /// `body1`: `fontSizeBase300`, regular.
    Body1,
    /// `body1Strong`: `fontSizeBase300`, semibold.
    Body1Strong,
    /// `subtitle2`: `fontSizeBase400`, semibold.
    Subtitle2,
    /// `subtitle1`: `fontSizeBase500`, semibold.
    Subtitle1,
    /// `title3`: `fontSizeBase600`, semibold.
    Title3,
    /// `title2`: `fontSizeHero700`, semibold.
    Title2,
    /// `title1`: `fontSizeHero800`, semibold.
    Title1,
    /// `largeTitle`: `fontSizeHero900`, semibold.
    LargeTitle,
    /// `display`: `fontSizeHero1000`, semibold.
    Display,
}

impl TypeRamp {
    /// Every step, smallest first.
    pub const ALL: [Self; 11] = [
        Self::Caption2,
        Self::Caption1,
        Self::Body1,
        Self::Body1Strong,
        Self::Subtitle2,
        Self::Subtitle1,
        Self::Title3,
        Self::Title2,
        Self::Title1,
        Self::LargeTitle,
        Self::Display,
    ];

    /// The Fluent token name this step is installed under (`TextStyle::Name`).
    pub const fn name(self) -> &'static str {
        match self {
            Self::Caption2 => "caption2",
            Self::Caption1 => "caption1",
            Self::Body1 => "body1",
            Self::Body1Strong => "body1_strong",
            Self::Subtitle2 => "subtitle2",
            Self::Subtitle1 => "subtitle1",
            Self::Title3 => "title3",
            Self::Title2 => "title2",
            Self::Title1 => "title1",
            Self::LargeTitle => "large_title",
            Self::Display => "display",
        }
    }

    /// Font size in points (`fontSizeBase*` / `fontSizeHero*`).
    pub fn size(self) -> f32 {
        use fluent2_tokens::generated::global::font_size;
        match self {
            Self::Caption2 => font_size::BASE_100,
            Self::Caption1 => font_size::BASE_200,
            Self::Body1 | Self::Body1Strong => font_size::BASE_300,
            Self::Subtitle2 => font_size::BASE_400,
            Self::Subtitle1 => font_size::BASE_500,
            Self::Title3 => font_size::BASE_600,
            Self::Title2 => font_size::HERO_700,
            Self::Title1 => font_size::HERO_800,
            Self::LargeTitle => font_size::HERO_900,
            Self::Display => font_size::HERO_1000,
        }
    }

    /// Line height in points (`lineHeightBase*` / `lineHeightHero*`).
    ///
    /// egui does not take a line height on a [`FontId`]; this is exposed for callers that need
    /// the token value.
    pub fn line_height(self) -> f32 {
        use fluent2_tokens::generated::global::line_height;
        match self {
            Self::Caption2 => line_height::BASE_100,
            Self::Caption1 => line_height::BASE_200,
            Self::Body1 | Self::Body1Strong => line_height::BASE_300,
            Self::Subtitle2 => line_height::BASE_400,
            Self::Subtitle1 => line_height::BASE_500,
            Self::Title3 => line_height::BASE_600,
            Self::Title2 => line_height::HERO_700,
            Self::Title1 => line_height::HERO_800,
            Self::LargeTitle => line_height::HERO_900,
            Self::Display => line_height::HERO_1000,
        }
    }

    /// Whether the step uses the semibold family.
    pub const fn semibold(self) -> bool {
        !matches!(self, Self::Caption2 | Self::Caption1 | Self::Body1)
    }

    /// The family this step is drawn in.
    pub fn family(self) -> FontFamily {
        if self.semibold() {
            FontFamily::Name(SEMIBOLD_FAMILY.into())
        } else {
            FontFamily::Proportional
        }
    }
}

/// The [`TextStyle`] a ramp step is stored under.
pub fn style(name: TypeRamp) -> TextStyle {
    TextStyle::Name(name.name().into())
}

/// The [`FontId`] a ramp step resolves to.
pub fn font_id(name: TypeRamp) -> FontId {
    FontId::new(name.size(), name.family())
}

/// Text drawn in a ramp step.
///
/// ```
/// # use fluent2_egui::foundation::text::{rich, TypeRamp};
/// let heading = rich("Save", TypeRamp::Subtitle1);
/// let _ = heading;
/// ```
pub fn rich(text: impl Into<String>, name: TypeRamp) -> RichText {
    RichText::new(text).text_style(style(name))
}

/// `text` as a [`LayoutJob`] with the byte `ranges` drawn in the theme's highlight colour.
///
/// Plain text uses `neutralForeground1`; matches use `brandForeground1` and are underlined, so
/// the highlight does not rely on colour alone. See [`highlighted_with`] for the rules on
/// `ranges` and for custom colours.
///
/// ```
/// # use fluent2_egui::foundation::text::{font_id, highlighted, TypeRamp};
/// let theme = fluent2_egui::Theme::default();
/// let job = highlighted("Save as", &[0..1, 5..7], font_id(TypeRamp::Body1), &theme);
/// assert_eq!(job.text, "Save as");
/// ```
pub fn highlighted(text: &str, ranges: &[Range<usize>], font: FontId, theme: &crate::Theme) -> LayoutJob {
    highlighted_with(
        text,
        ranges,
        font,
        crate::theme::c(theme.color.neutral_foreground_1),
        crate::theme::c(theme.color.brand_foreground_1),
    )
}

/// `text` as a [`LayoutJob`] in `color`, with the byte `ranges` in `accent` and underlined.
///
/// The ranges come from the caller (a fuzzy matcher, a search). They may be in any order,
/// overlap, be empty or reach past the end: they are clamped to the text, widened to the
/// enclosing `char` boundaries and merged. The job has one section per run, and never panics.
pub fn highlighted_with(
    text: &str,
    ranges: &[Range<usize>],
    font: FontId,
    color: Color32,
    accent: Color32,
) -> LayoutJob {
    let plain = TextFormat::simple(font.clone(), color);
    let marked = TextFormat {
        underline: Stroke::new(fluent2_tokens::generated::global::stroke_width::THIN, accent),
        ..TextFormat::simple(font, accent)
    };
    let mut job = LayoutJob::default();
    let mut at = 0;
    for range in highlight_runs(text, ranges) {
        if let Some(before) = text.get(at..range.start).filter(|s| !s.is_empty()) {
            job.append(before, 0.0, plain.clone());
        }
        if let Some(matched) = text.get(range.clone()) {
            job.append(matched, 0.0, marked.clone());
        }
        at = range.end;
    }
    if let Some(rest) = text.get(at..).filter(|s| !s.is_empty()) {
        job.append(rest, 0.0, plain);
    }
    job
}

/// Byte ranges for the `char` positions in `indices`, for [`highlighted`].
///
/// Fuzzy matchers (the command palette's scorer, nucleo) report matched `char` positions,
/// while substring search reports byte ranges; this bridges the first to the second. Adjacent
/// positions become one range. Positions past the end are dropped, and order and duplicates do
/// not matter.
///
/// ```
/// # use fluent2_egui::foundation::text::char_ranges;
/// assert_eq!(char_ranges("é日x", &[0, 1, 9]), [0..5]);
/// ```
pub fn char_ranges(text: &str, indices: &[usize]) -> Vec<Range<usize>> {
    let mut wanted = indices.to_vec();
    wanted.sort_unstable();
    wanted.dedup();
    let mut ranges: Vec<Range<usize>> = Vec::new();
    let mut next = wanted.iter().peekable();
    for (index, (byte, ch)) in text.char_indices().enumerate() {
        if next.peek().is_none() {
            break;
        }
        if next.next_if(|&&want| want == index).is_none() {
            continue;
        }
        let end = byte + ch.len_utf8();
        match ranges.last_mut() {
            Some(last) if last.end == byte => last.end = end,
            _ => ranges.push(byte..end),
        }
    }
    ranges
}

/// The non-empty, sorted, disjoint byte ranges of `text` to highlight, on `char` boundaries.
fn highlight_runs(text: &str, ranges: &[Range<usize>]) -> Vec<Range<usize>> {
    let floor = |mut i: usize| {
        i = i.min(text.len());
        while !text.is_char_boundary(i) {
            i -= 1;
        }
        i
    };
    let ceil = |mut i: usize| {
        i = i.min(text.len());
        while !text.is_char_boundary(i) {
            i += 1;
        }
        i
    };
    let mut runs: Vec<Range<usize>> = ranges
        .iter()
        .filter(|range| range.start < range.end)
        .map(|range| floor(range.start)..ceil(range.end))
        .filter(|range| range.start < range.end)
        .collect();
    runs.sort_by_key(|range| range.start);
    let mut merged: Vec<Range<usize>> = Vec::with_capacity(runs.len());
    for range in runs {
        match merged.last_mut() {
            Some(last) if range.start <= last.end => last.end = last.end.max(range.end),
            _ => merged.push(range),
        }
    }
    merged
}

/// Write the type ramp into both of egui's styles (light and dark).
///
/// Each [`TypeRamp`] step becomes `TextStyle::Name("<fluent name>")`. egui's built-ins are
/// pointed at the matching step: `Body` and `Button` at `body1`, `Small` at `caption1`,
/// `Heading` at `subtitle1`, and `Monospace` at the `body1` size in the monospace family.
pub fn install_text_styles(ctx: &egui::Context) {
    ctx.all_styles_mut(|style_map| {
        for step in TypeRamp::ALL {
            style_map.text_styles.insert(style(step), font_id(step));
        }
        style_map
            .text_styles
            .insert(TextStyle::Body, font_id(TypeRamp::Body1));
        style_map
            .text_styles
            .insert(TextStyle::Button, font_id(TypeRamp::Body1));
        style_map
            .text_styles
            .insert(TextStyle::Small, font_id(TypeRamp::Caption1));
        style_map
            .text_styles
            .insert(TextStyle::Heading, font_id(TypeRamp::Subtitle1));
        style_map.text_styles.insert(
            TextStyle::Monospace,
            FontId::new(TypeRamp::Body1.size(), FontFamily::Monospace),
        );
    });
}

/// Load the UI font and register the `fluent2-semibold` family.
///
/// On Windows this tries `%WINDIR%\Fonts\SegUIVar.ttf` first (Segoe UI Variable, used for both
/// the regular and the semibold family), then `segoeui.ttf` with `seguisb.ttf`. When neither is
/// readable, and on every other platform, the bundled Inter faces are used if the
/// `bundled-font` feature is on. Otherwise egui's defaults remain.
///
/// Calling this twice does not add the font a second time: a face already present under the
/// same name is left as it is.
pub fn install_system_fonts(ctx: &egui::Context) {
    #[cfg(all(target_os = "windows", not(target_arch = "wasm32")))]
    let os_faces = windows_faces();
    #[cfg(not(all(target_os = "windows", not(target_arch = "wasm32"))))]
    let os_faces: Option<Vec<Face>> = None;

    if let Some(faces) = os_faces.or_else(bundled_faces) {
        install_faces(ctx, faces);
        return;
    }

    // No face to install. The semibold family still has to resolve, so point it at whatever
    // egui already uses for proportional text. Fonts exist only after the first pass, so a call
    // before that queues the fallback and a later call sees it already registered.
    install_semibold_fallback(ctx);
}

/// A face ready to go first in a family.
struct Face {
    name: String,
    family: FontFamily,
    data: egui::FontData,
}

/// Regular and SemiBold Inter (OFL-1.1). Static faces, so the semibold step draws a real weight;
/// egui cannot select a weight on a variable font.
#[cfg(feature = "bundled-font")]
const INTER_REGULAR: &[u8] = include_bytes!("../../fonts/Inter-Regular.ttf");
#[cfg(feature = "bundled-font")]
const INTER_SEMIBOLD: &[u8] = include_bytes!("../../fonts/Inter-SemiBold.ttf");

/// The bundled Inter faces: `Inter` for the proportional family, `Inter SemiBold` for the
/// semibold family.
#[cfg(feature = "bundled-font")]
fn bundled_faces() -> Option<Vec<Face>> {
    Some(vec![
        Face {
            name: "Inter".to_owned(),
            family: FontFamily::Proportional,
            data: egui::FontData::from_static(INTER_REGULAR),
        },
        Face {
            name: "Inter SemiBold".to_owned(),
            family: FontFamily::Name(SEMIBOLD_FAMILY.into()),
            data: egui::FontData::from_static(INTER_SEMIBOLD),
        },
    ])
}

/// Without the `bundled-font` feature there is nothing to install.
#[cfg(not(feature = "bundled-font"))]
fn bundled_faces() -> Option<Vec<Face>> {
    None
}

/// Segoe UI Variable if present, otherwise Segoe UI regular plus its semibold face.
#[cfg(all(target_os = "windows", not(target_arch = "wasm32")))]
fn windows_faces() -> Option<Vec<Face>> {
    let Some(dir) = std::env::var_os("WINDIR").map(std::path::PathBuf::from) else {
        log::debug!("fluent2-egui: WINDIR is unset; keeping egui's default fonts");
        return None;
    };
    let fonts = dir.join("Fonts");

    if let Some(bytes) = read_font(&fonts.join("SegUIVar.ttf")) {
        // One variable face for both families. The item uses it as both regular and semibold;
        // egui has no per-style weight axis, so both families point at the same bytes.
        return Some(vec![
            Face {
                name: "Segoe UI Variable".to_owned(),
                family: FontFamily::Proportional,
                data: egui::FontData::from_owned(bytes.clone()),
            },
            Face {
                name: "Segoe UI Variable Semibold".to_owned(),
                family: FontFamily::Name(SEMIBOLD_FAMILY.into()),
                data: egui::FontData::from_owned(bytes),
            },
        ]);
    }

    let regular = read_font(&fonts.join("segoeui.ttf"))?;
    let mut faces = vec![Face {
        name: "Segoe UI".to_owned(),
        family: FontFamily::Proportional,
        data: egui::FontData::from_owned(regular.clone()),
    }];
    if let Some(semibold) = read_font(&fonts.join("seguisb.ttf")) {
        faces.push(Face {
            name: "Segoe UI Semibold".to_owned(),
            family: FontFamily::Name(SEMIBOLD_FAMILY.into()),
            data: egui::FontData::from_owned(semibold),
        });
    } else {
        // Regular face also fills the semibold family so every ramp step still resolves.
        faces.push(Face {
            name: "Segoe UI".to_owned(),
            family: FontFamily::Name(SEMIBOLD_FAMILY.into()),
            data: egui::FontData::from_owned(regular),
        });
    }
    Some(faces)
}

/// Read a font file, returning `None` on any error. Never panics.
///
/// A missing file or a failed read is logged at debug and skipped.
#[cfg(all(target_os = "windows", not(target_arch = "wasm32")))]
fn read_font(path: &std::path::Path) -> Option<Vec<u8>> {
    match std::fs::read(path) {
        Ok(bytes) => Some(bytes),
        Err(error) => {
            log::debug!("fluent2-egui: not using {}: {error}", path.display());
            None
        }
    }
}

/// Insert the faces first in their families, then replace the font definitions.
///
/// A second call with the same names is a no-op because [`egui::Context::set_fonts`] skips an
/// equal definition, and [`egui::Context::add_font`] skips a name it already has.
fn install_faces(ctx: &egui::Context, faces: Vec<Face>) {
    let mut fonts = FontDefinitions::default();
    for face in faces {
        insert_first(&mut fonts, &face.family, face.name.clone());
        fonts
            .font_data
            .entry(face.name)
            .or_insert_with(|| std::sync::Arc::new(face.data));
    }
    ctx.set_fonts(fonts);
}

/// Put `name` first in `family`, leaving the existing fallbacks after it.
fn insert_first(fonts: &mut FontDefinitions, family: &FontFamily, name: String) {
    let list = fonts.families.entry(family.clone()).or_default();
    if !list.contains(&name) {
        list.insert(0, name);
    }
}

/// Point `fluent2-semibold` at the proportional face when no semibold face was found.
///
/// The face is registered under [`SEMIBOLD_FAMILY`], once. A second call builds the same
/// definition, and [`egui::Context::set_fonts`] skips a definition equal to the current one.
fn install_semibold_fallback(ctx: &egui::Context) {
    let Some(fonts) = semibold_fallback_definitions() else {
        return;
    };
    ctx.set_fonts(fonts);
}

/// The definition [`install_semibold_fallback`] installs.
///
/// `None` when egui was built without `default_fonts` and so has no proportional face to alias.
/// The semibold family then stays unresolved.
fn semibold_fallback_definitions() -> Option<FontDefinitions> {
    let mut fonts = FontDefinitions::default();
    let name = fonts
        .families
        .get(&FontFamily::Proportional)
        .and_then(|names| names.first())
        .cloned()?;
    let data = fonts.font_data.get(&name)?.clone();
    fonts.font_data.insert(SEMIBOLD_FAMILY.to_owned(), data);
    fonts
        .families
        .entry(FontFamily::Name(SEMIBOLD_FAMILY.into()))
        .or_default()
        .push(SEMIBOLD_FAMILY.to_owned());
    Some(fonts)
}

#[cfg(test)]
// One-element and reversed range slices are inputs under test.
#[allow(clippy::single_range_in_vec_init, clippy::reversed_empty_ranges)]
mod tests {
    use super::*;

    fn sections(job: &LayoutJob) -> Vec<(&str, bool)> {
        let accent = Color32::RED;
        job.sections
            .iter()
            .map(|section| {
                (
                    &job.text[section.byte_range.start.0..section.byte_range.end.0],
                    section.format.color == accent,
                )
            })
            .collect()
    }

    fn job(text: &str, ranges: &[Range<usize>]) -> LayoutJob {
        highlighted_with(
            text,
            ranges,
            FontId::proportional(14.0),
            Color32::WHITE,
            Color32::RED,
        )
    }

    #[test]
    fn char_positions_become_byte_ranges() {
        let text = "é日🦀x";
        assert_eq!(char_ranges(text, &[]), Vec::<Range<usize>>::new());
        assert_eq!(char_ranges(text, &[1]), [2..5]);
        assert_eq!(char_ranges(text, &[3, 0, 1, 1]), [0..5, 9..10]);
        assert_eq!(char_ranges(text, &[2, 3, 99]), [5..10]);
        assert_eq!(char_ranges("", &[0]), Vec::<Range<usize>>::new());
        let job = job(text, &char_ranges(text, &[2]));
        assert_eq!(sections(&job), [("é日", false), ("🦀", true), ("x", false)]);
    }

    #[test]
    fn highlights_disjoint_ranges() {
        let job = job("Save as", &[5..7, 0..1]);
        assert_eq!(job.text, "Save as");
        assert_eq!(sections(&job), [("S", true), ("ave ", false), ("as", true)]);
        assert!(job.sections[0].format.underline.width > 0.0);
        assert_eq!(job.sections[1].format.underline, Stroke::NONE);
    }

    #[test]
    fn overlapping_and_touching_ranges_merge() {
        let job = job("abcdefgh", &[1..4, 2..3, 3..5, 5..6]);
        assert_eq!(sections(&job), [("a", false), ("bcdef", true), ("gh", false)]);
    }

    #[test]
    fn empty_and_out_of_range_ranges() {
        assert_eq!(sections(&job("abc", &[])), [("abc", false)]);
        assert_eq!(sections(&job("abc", &[1..1, 2..1])), [("abc", false)]);
        let reversed = [3..0];
        assert_eq!(sections(&job("abc", &reversed)), [("abc", false)]);
        assert_eq!(sections(&job("abc", &[2..99])), [("ab", false), ("c", true)]);
        assert_eq!(sections(&job("abc", &[50..99])), [("abc", false)]);
        assert_eq!(sections(&job("abc", &[0..usize::MAX])), [("abc", true)]);
        assert!(job("", &[0..4]).sections.is_empty());
    }

    #[test]
    fn multi_byte_ranges_snap_to_char_boundaries() {
        // "é" is two bytes, "日" three, "🦀" four.
        let text = "é日🦀x";
        assert_eq!(sections(&job(text, &[1..2])), [("é", true), ("日🦀x", false)]);
        assert_eq!(
            sections(&job(text, &[3..4, 6..7])),
            [("é", false), ("日🦀", true), ("x", false)]
        );
        for start in 0..=text.len() + 1 {
            for end in 0..=text.len() + 1 {
                let job = job(text, &[start..end]);
                assert_eq!(job.text, text);
                let joined: String = sections(&job).iter().map(|(s, _)| *s).collect();
                assert_eq!(joined, text);
            }
        }
    }

    #[test]
    fn theme_variant_uses_brand_foreground() {
        let theme = crate::Theme::default();
        let job = highlighted("ab", &[1..2], FontId::proportional(14.0), &theme);
        assert_eq!(
            job.sections[1].format.color,
            crate::theme::c(theme.color.brand_foreground_1)
        );
        assert_eq!(
            job.sections[0].format.color,
            crate::theme::c(theme.color.neutral_foreground_1)
        );
    }

    #[test]
    fn ramp_matches_the_generated_tokens() {
        use fluent2_tokens::generated::global::{font_size, line_height};
        assert_eq!(TypeRamp::Caption2.size(), font_size::BASE_100);
        assert_eq!(TypeRamp::Caption1.size(), font_size::BASE_200);
        assert_eq!(TypeRamp::Body1.size(), font_size::BASE_300);
        assert_eq!(TypeRamp::Body1Strong.size(), font_size::BASE_300);
        assert_eq!(TypeRamp::Subtitle2.size(), font_size::BASE_400);
        assert_eq!(TypeRamp::Subtitle1.size(), font_size::BASE_500);
        assert_eq!(TypeRamp::Title3.size(), font_size::BASE_600);
        assert_eq!(TypeRamp::Title2.size(), font_size::HERO_700);
        assert_eq!(TypeRamp::Title1.size(), font_size::HERO_800);
        assert_eq!(TypeRamp::LargeTitle.size(), font_size::HERO_900);
        assert_eq!(TypeRamp::Display.size(), font_size::HERO_1000);
        assert_eq!(TypeRamp::Body1.line_height(), line_height::BASE_300);
        assert_eq!(TypeRamp::Display.line_height(), line_height::HERO_1000);
        // The item's stated body size, so a token change cannot silently move it.
        assert!((TypeRamp::Body1.size() - 14.0).abs() < f32::EPSILON);
        assert!(TypeRamp::Body1Strong.semibold());
        assert!(!TypeRamp::Body1.semibold());
    }

    #[test]
    fn install_writes_every_step_and_the_builtins() {
        let ctx = egui::Context::default();
        crate::install(&ctx, &crate::Theme::default());

        let styles = ctx.style_of(egui::Theme::Light).text_styles.clone();
        for step in TypeRamp::ALL {
            let id = styles
                .get(&style(step))
                .unwrap_or_else(|| panic!("{} missing", step.name()));
            assert_eq!(id.size, step.size(), "{}", step.name());
            assert_eq!(id.family, step.family(), "{}", step.name());
        }
        assert_eq!(styles[&TextStyle::Body].size, 14.0);
        assert_eq!(styles[&TextStyle::Button].size, 14.0);
        assert_eq!(styles[&TextStyle::Small].size, 12.0);
        assert_eq!(styles[&TextStyle::Heading].size, 20.0);
        assert_eq!(
            styles[&TextStyle::Monospace],
            FontId::new(14.0, FontFamily::Monospace)
        );
    }

    fn family_names(ctx: &egui::Context) -> Vec<(FontFamily, Vec<String>)> {
        let output = ctx.run_ui(egui::RawInput::default(), |_| {});
        output.drop_without_applying_deltas();
        ctx.fonts(|fonts| {
            fonts
                .definitions()
                .families
                .iter()
                .map(|(family, names)| (family.clone(), names.clone()))
                .collect()
        })
    }

    fn assert_no_duplicate_names(families: &[(FontFamily, Vec<String>)]) {
        for (family, names) in families {
            let unique: std::collections::BTreeSet<_> = names.iter().collect();
            assert_eq!(
                unique.len(),
                names.len(),
                "duplicate font names in {family:?}: {names:?}"
            );
        }
    }

    #[test]
    fn installing_fonts_twice_does_not_duplicate_entries() {
        let ctx = egui::Context::default();
        // Build the atlas first. `set_fonts` only queues a definition when one is already
        // installed, and `fonts()` is what installs egui's defaults.
        let _ = family_names(&ctx);
        crate::install(&ctx, &crate::Theme::default());
        let after_once = family_names(&ctx);
        crate::install(&ctx, &crate::Theme::default());
        let after_twice = family_names(&ctx);
        assert_no_duplicate_names(&after_once);
        assert_no_duplicate_names(&after_twice);
        assert_eq!(after_once, after_twice);
    }

    #[cfg(all(feature = "bundled-font", not(target_os = "windows")))]
    #[test]
    fn bundled_inter_leads_both_families() {
        let ctx = egui::Context::default();
        let _ = family_names(&ctx);
        crate::install(&ctx, &crate::Theme::default());
        let families = family_names(&ctx);
        let first = |family: &FontFamily| {
            families
                .iter()
                .find(|(f, _)| f == family)
                .and_then(|(_, names)| names.first().cloned())
        };
        assert_eq!(
            first(&FontFamily::Proportional).as_deref(),
            Some("Inter"),
            "regular text should use Inter"
        );
        assert_eq!(
            first(&FontFamily::Name(SEMIBOLD_FAMILY.into())).as_deref(),
            Some("Inter SemiBold"),
            "semibold steps should use the static SemiBold face"
        );
    }

    #[cfg(all(feature = "bundled-font", not(target_os = "windows")))]
    #[test]
    fn bundled_faces_are_truetype_and_registered_once() {
        // TrueType outline magic. A wrong or truncated file fails here, not at first paint.
        assert_eq!(&INTER_REGULAR[..4], &[0x00, 0x01, 0x00, 0x00]);
        assert_eq!(&INTER_SEMIBOLD[..4], &[0x00, 0x01, 0x00, 0x00]);

        let ctx = egui::Context::default();
        let _ = family_names(&ctx);
        crate::install(&ctx, &crate::Theme::default());
        crate::install(&ctx, &crate::Theme::default());
        let families = family_names(&ctx);
        assert_no_duplicate_names(&families);
        let inter_entries = families
            .iter()
            .flat_map(|(_, names)| names.iter())
            .filter(|name| name.as_str() == "Inter")
            .count();
        assert_eq!(inter_entries, 1, "Inter should appear once in Proportional");
    }
}
