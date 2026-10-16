//! Fluent System Icons drawn in egui.

use std::collections::HashMap;
use std::sync::{Arc, Mutex, OnceLock};

use egui::{Color32, Image, Rect, Sense, Ui, Vec2, Widget};

use crate::theme::{self, Theme};

pub use fluent2_icons::{Icon as IconName, Size as IconSize, Style as IconStyle};

/// A Fluent System Icon, drawn at a fixed size and tinted with a theme colour.
///
/// ```
/// # use fluent2_egui::Icon;
/// # use fluent2_egui::IconName;
/// let icon = Icon::new(IconName::Add).filled().tint(egui::Color32::WHITE);
/// let _ = icon.size;
/// ```
pub struct Icon {
    /// Which icon.
    pub name: IconName,
    /// Pixel size. Defaults to 20.
    pub size: IconSize,
    /// Regular or filled. Defaults to regular.
    pub style: IconStyle,
    /// Tint. `None` uses the text colour, or the disabled foreground when the `Ui` is disabled.
    pub tint: Option<Color32>,
}

impl Icon {
    /// An icon at 20 px, regular, tinted with the text colour.
    #[must_use]
    pub fn new(name: IconName) -> Self {
        Self {
            name,
            size: IconSize::S20,
            style: IconStyle::Regular,
            tint: None,
        }
    }

    /// Set the pixel size.
    #[must_use]
    pub fn size(mut self, size: IconSize) -> Self {
        self.size = size;
        self
    }

    /// Use the filled style.
    #[must_use]
    pub fn filled(mut self) -> Self {
        self.style = IconStyle::Filled;
        self
    }

    /// Tint the icon. `None` keeps the theme default.
    #[must_use]
    pub fn tint(mut self, tint: impl Into<Option<Color32>>) -> Self {
        self.tint = tint.into();
        self
    }
}

impl Widget for Icon {
    fn ui(self, ui: &mut Ui) -> egui::Response {
        let points = size_points(self.size);
        let (rect, response) = ui.allocate_exact_size(Vec2::splat(points), Sense::hover());
        paint_icon(ui, rect, self.name, self.size, self.style, self.tint);
        response
    }
}

/// Paint `name` into `rect`. The icon is fitted to `size` points and centred in `rect`.
///
/// A missing SVG paints nothing and logs once. This never panics.
pub fn paint_icon(
    ui: &Ui,
    rect: Rect,
    name: IconName,
    size: IconSize,
    style: IconStyle,
    tint: Option<Color32>,
) {
    let Some(bytes) = tinted_svg(name, size, style) else {
        warn_missing(name, size, style);
        return;
    };
    let color = tint.unwrap_or_else(|| default_tint(ui));
    let points = size_points(size);
    let image = Image::from_bytes(uri(name, size, style), bytes)
        .tint(color)
        .fit_to_exact_size(Vec2::splat(points));
    image.paint_at(ui, rect);
}

fn default_tint(ui: &Ui) -> Color32 {
    if ui.is_enabled() {
        ui.visuals().text_color()
    } else {
        theme::c(Theme::get(ui.ctx()).color.neutral_foreground_disabled)
    }
}

fn size_points(size: IconSize) -> f32 {
    match size {
        IconSize::S16 => 16.0,
        IconSize::S20 => 20.0,
        IconSize::S24 => 24.0,
    }
}

fn uri(name: IconName, size: IconSize, style: IconStyle) -> String {
    format!(
        "bytes://fluent2-icons/{}_{}_{}.svg",
        slug(name.name()),
        size_token(size),
        style_token(style)
    )
}

fn slug(name: &str) -> String {
    name.to_ascii_lowercase().replace(' ', "_")
}

fn size_token(size: IconSize) -> &'static str {
    match size {
        IconSize::S16 => "16",
        IconSize::S20 => "20",
        IconSize::S24 => "24",
    }
}

fn style_token(style: IconStyle) -> &'static str {
    match style {
        IconStyle::Regular => "regular",
        IconStyle::Filled => "filled",
    }
}

type CacheKey = (IconName, IconSize, IconStyle);

fn cache() -> &'static Mutex<HashMap<CacheKey, Arc<[u8]>>> {
    static CACHE: OnceLock<Mutex<HashMap<CacheKey, Arc<[u8]>>>> = OnceLock::new();
    CACHE.get_or_init(|| Mutex::new(HashMap::new()))
}

/// White-source SVG bytes for one icon, cached. `None` when the catalogue has no such combo,
/// or when the cache lock is poisoned.
fn tinted_svg(name: IconName, size: IconSize, style: IconStyle) -> Option<Arc<[u8]>> {
    let key = (name, size, style);
    {
        let cache = cache().lock().ok()?;
        if let Some(bytes) = cache.get(&key) {
            return Some(Arc::clone(bytes));
        }
    }
    let source = fluent2_icons::svg(name, size, style)?;
    let bytes: Arc<[u8]> = Arc::from(whiten(source).into_boxed_slice());
    let mut cache = cache().lock().ok()?;
    Some(Arc::clone(cache.entry(key).or_insert(bytes)))
}

/// Fluent SVGs use `fill="#212121"`. Tint multiplies source colour, so the glyph has to be white.
fn whiten(source: &[u8]) -> Vec<u8> {
    const DARK: &[u8] = b"#212121";
    const WHITE: &[u8] = b"#FFFFFF";
    let mut out = Vec::with_capacity(source.len());
    let mut rest = source;
    while let Some(at) = find_sub(rest, DARK) {
        out.extend_from_slice(&rest[..at]);
        out.extend_from_slice(WHITE);
        rest = &rest[at + DARK.len()..];
    }
    out.extend_from_slice(rest);
    out
}

fn find_sub(haystack: &[u8], needle: &[u8]) -> Option<usize> {
    haystack.windows(needle.len()).position(|window| window == needle)
}

fn warn_missing(name: IconName, size: IconSize, style: IconStyle) {
    static WARNED: OnceLock<Mutex<HashMap<CacheKey, ()>>> = OnceLock::new();
    let warned = WARNED.get_or_init(|| Mutex::new(HashMap::new()));
    let Ok(mut warned) = warned.lock() else {
        return;
    };
    if warned.insert((name, size, style), ()).is_none() {
        log::warn!(
            "fluent2 icon missing: {} {} {}",
            name.name(),
            size_token(size),
            style_token(style)
        );
    }
}

#[cfg(test)]
mod tests {
    use super::whiten;

    #[test]
    fn whitening_replaces_the_fluent_fill_only() {
        let source = b"<path fill=\"#212121\" d=\"M1\"/>";
        let white = whiten(source);
        assert_eq!(white, b"<path fill=\"#FFFFFF\" d=\"M1\"/>");
    }
}
