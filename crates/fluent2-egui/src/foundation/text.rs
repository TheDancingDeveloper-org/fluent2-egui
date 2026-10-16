//! Fluent 2 type ramp as egui [`TextStyle`]s, and the OS UI font when available.
//!
//! Sizes and line heights come from [`fluent2_tokens::generated::global`]. egui's [`FontId`] has
//! no line-height field, so each step's line height is recorded beside its size for callers that
//! lay text out themselves.
//!
//! No font file ships with this crate. On Windows the system UI face is read from the OS at
//! runtime; everywhere else egui's own defaults stay in place. A font file that cannot be read
//! is skipped and logged at debug.

use egui::{FontDefinitions, FontFamily, FontId, RichText, TextStyle};

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

/// Load the OS UI font and register the `fluent2-semibold` family.
///
/// On Windows this tries `%WINDIR%\Fonts\SegUIVar.ttf` first (Segoe UI Variable, used for both
/// the regular and the semibold family), then `segoeui.ttf` with `seguisb.ttf`. Any file that
/// cannot be read is skipped and egui's defaults remain. Linux, macOS and wasm keep egui's
/// defaults; the open fallback face is a later decision.
///
/// Calling this twice does not add the font a second time: a face already present under the
/// same name is left as it is.
pub fn install_system_fonts(ctx: &egui::Context) {
    #[cfg(all(target_os = "windows", not(target_arch = "wasm32")))]
    if let Some(faces) = windows_faces() {
        install_windows_faces(ctx, faces);
        return;
    }

    // No OS face. The semibold family still has to resolve, so point it at whatever egui
    // already uses for proportional text. Fonts exist only after the first pass, so a call
    // before that queues the fallback and a later call sees it already registered.
    install_semibold_fallback(ctx);
}

/// A face read from the OS, ready to go first in a family.
#[cfg(all(target_os = "windows", not(target_arch = "wasm32")))]
struct SystemFace {
    name: String,
    family: FontFamily,
    bytes: Vec<u8>,
}

/// Segoe UI Variable if present, otherwise Segoe UI regular plus its semibold face.
#[cfg(all(target_os = "windows", not(target_arch = "wasm32")))]
fn windows_faces() -> Option<Vec<SystemFace>> {
    let Some(dir) = std::env::var_os("WINDIR").map(std::path::PathBuf::from) else {
        log::debug!("fluent2-egui: WINDIR is unset; keeping egui's default fonts");
        return None;
    };
    let fonts = dir.join("Fonts");

    if let Some(bytes) = read_font(&fonts.join("SegUIVar.ttf")) {
        // One variable face for both families. The item uses it as both regular and semibold;
        // egui has no per-style weight axis, so both families point at the same bytes.
        return Some(vec![
            SystemFace {
                name: "Segoe UI Variable".to_owned(),
                family: FontFamily::Proportional,
                bytes: bytes.clone(),
            },
            SystemFace {
                name: "Segoe UI Variable Semibold".to_owned(),
                family: FontFamily::Name(SEMIBOLD_FAMILY.into()),
                bytes,
            },
        ]);
    }

    let regular = read_font(&fonts.join("segoeui.ttf"))?;
    let mut faces = vec![SystemFace {
        name: "Segoe UI".to_owned(),
        family: FontFamily::Proportional,
        bytes: regular,
    }];
    if let Some(semibold) = read_font(&fonts.join("seguisb.ttf")) {
        faces.push(SystemFace {
            name: "Segoe UI Semibold".to_owned(),
            family: FontFamily::Name(SEMIBOLD_FAMILY.into()),
            bytes: semibold,
        });
    } else {
        // Regular face also fills the semibold family so every ramp step still resolves.
        faces.push(SystemFace {
            name: "Segoe UI".to_owned(),
            family: FontFamily::Name(SEMIBOLD_FAMILY.into()),
            bytes: faces[0].bytes.clone(),
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

/// Insert the Windows faces first in their families, then replace the font definitions.
///
/// A second call with the same names is a no-op because [`egui::Context::set_fonts`] skips an
/// equal definition, and [`egui::Context::add_font`] skips a name it already has.
#[cfg(all(target_os = "windows", not(target_arch = "wasm32")))]
fn install_windows_faces(ctx: &egui::Context, faces: Vec<SystemFace>) {
    let mut fonts = FontDefinitions::default();
    for face in faces {
        insert_first(&mut fonts, &face.family, face.name.clone());
        fonts
            .font_data
            .entry(face.name)
            .or_insert_with(|| std::sync::Arc::new(egui::FontData::from_owned(face.bytes)));
    }
    ctx.set_fonts(fonts);
}

/// Put `name` first in `family`, leaving the existing fallbacks after it.
#[cfg(all(target_os = "windows", not(target_arch = "wasm32")))]
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
/// The semibold family then stays unresolved until a later item supplies an open fallback face.
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
mod tests {
    use super::*;

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
}
