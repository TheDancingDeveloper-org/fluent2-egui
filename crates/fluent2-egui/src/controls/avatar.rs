//! Avatar, AvatarGroup and Persona (WI-1932).
//!
//! An avatar shows a person as an image, their initials or a person icon, with an optional
//! presence badge. Sizes are Fluent's (16 to 128 pt). Colours: neutral (the default), brand, or
//! colourful, which picks one of Fluent's 30 named palette colours from the name. The pick is
//! our own stable hash, so the colour for a name may differ from Fluent web's.
//!
//! The crate ships no artwork: an image is a texture the app has loaded.
//!
//! ```
//! # use fluent2_egui::controls::{Avatar, AvatarColor, Presence};
//! # fn demo(ui: &mut egui::Ui) {
//! ui.add(Avatar::new("Ada Lovelace").size(40.0).color(AvatarColor::Colorful).presence(Presence::Available));
//! # }
//! ```

use egui::{
    Color32, CornerRadius, Pos2, Rect, Response, Sense, Stroke, StrokeKind, TextStyle, TextureId, Ui, Vec2,
    Widget, WidgetText,
};
use fluent2_tokens::{AliasColors, Rgba};

use crate::foundation::icon::{IconName, IconSize, IconStyle, paint_icon};
use crate::foundation::text::TypeRamp;
use crate::surfaces::TooltipExt;
use crate::theme::{Theme, ThemeKind, c};

/// Fluent's avatar sizes, in points. Other sizes snap to the nearest.
pub const AVATAR_SIZES: [f32; 14] = [
    16.0, 20.0, 24.0, 28.0, 32.0, 36.0, 40.0, 48.0, 56.0, 64.0, 72.0, 96.0, 120.0, 128.0,
];

/// How an avatar is coloured.
#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub enum AvatarColor {
    /// `neutral_background_6` with `neutral_foreground_3`. The default.
    #[default]
    Neutral,
    /// `brand_background` with `neutral_foreground_on_brand`.
    Brand,
    /// One of Fluent's named palette colours, picked from the name.
    Colorful,
}

/// Outline of an avatar.
#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub enum AvatarShape {
    /// A circle. The default.
    #[default]
    Circular,
    /// A rounded square, for groups, rooms and bots.
    Square,
}

/// A presence badge in the bottom trailing corner.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum Presence {
    /// Green.
    Available,
    /// Red.
    Busy,
    /// Marigold.
    Away,
    /// Red with a bar.
    DoNotDisturb,
    /// An empty ring.
    Offline,
}

impl Presence {
    /// The word screen readers read after the name.
    #[must_use]
    pub fn label(self) -> &'static str {
        match self {
            Self::Available => "available",
            Self::Busy => "busy",
            Self::Away => "away",
            Self::DoNotDisturb => "do not disturb",
            Self::Offline => "offline",
        }
    }
}

/// A Fluent avatar. See the module example.
#[derive(Clone, Debug)]
pub struct Avatar {
    name: String,
    initials: Option<String>,
    size: f32,
    shape: AvatarShape,
    color: AvatarColor,
    image: Option<TextureId>,
    presence: Option<Presence>,
    ring: bool,
}

impl Avatar {
    /// An avatar for `name`. The name is the accessible name and the source of the initials.
    /// An empty name shows the person icon.
    pub fn new(name: impl Into<String>) -> Self {
        Self {
            name: name.into(),
            initials: None,
            size: 32.0,
            shape: AvatarShape::Circular,
            color: AvatarColor::Neutral,
            image: None,
            presence: None,
            ring: false,
        }
    }

    /// Size in points, snapped to the nearest of [`AVATAR_SIZES`]. The default is 32.
    #[must_use]
    pub fn size(mut self, size: f32) -> Self {
        self.size = snap_size(size);
        self
    }

    /// Circular (the default) or square.
    #[must_use]
    pub fn shape(mut self, shape: AvatarShape) -> Self {
        self.shape = shape;
        self
    }

    /// Neutral (the default), brand or colourful.
    #[must_use]
    pub fn color(mut self, color: AvatarColor) -> Self {
        self.color = color;
        self
    }

    /// Initials to show instead of the ones derived from the name.
    #[must_use]
    pub fn initials(mut self, initials: impl Into<String>) -> Self {
        self.initials = Some(initials.into());
        self
    }

    /// A picture, as a texture the app has loaded. It replaces the initials.
    #[must_use]
    pub fn image(mut self, texture: TextureId) -> Self {
        self.image = Some(texture);
        self
    }

    /// A presence badge.
    #[must_use]
    pub fn presence(mut self, presence: Presence) -> Self {
        self.presence = Some(presence);
        self
    }

    /// The accessible name: the name, then the presence.
    #[must_use]
    pub fn accessible_label(&self) -> String {
        let name = if self.name.trim().is_empty() {
            "Person".to_owned()
        } else {
            self.name.trim().to_owned()
        };
        match self.presence {
            Some(presence) => format!("{name}, {}", presence.label()),
            None => name,
        }
    }

    /// Paint the avatar into `rect` (its side is the avatar size).
    fn paint(&self, ui: &mut Ui, rect: Rect) {
        let theme = Theme::get(ui.ctx());
        let (fill, ink) = colors(&theme, self.color, &self.name);
        let radius = match self.shape {
            AvatarShape::Circular => CornerRadius::same(255),
            AvatarShape::Square => CornerRadius::same(square_radius(self.size)),
        };
        let painter = ui.painter().clone();
        if self.ring {
            // A ring in the page colour separates stacked avatars.
            painter.rect_filled(rect.expand(2.0), radius, c(theme.color.neutral_background_1));
        }
        if let Some(texture) = self.image {
            egui::Image::from_texture(egui::load::SizedTexture::new(texture, rect.size()))
                .corner_radius(radius)
                .paint_at(ui, rect);
        } else {
            painter.rect_filled(rect, radius, fill);
            // High contrast: an edge, as the fills are not distinct from the page there.
            if theme.kind == ThemeKind::HighContrast {
                painter.rect_stroke(
                    rect,
                    radius,
                    Stroke::new(1.0, c(theme.color.neutral_stroke_accessible)),
                    StrokeKind::Inside,
                );
            }
            let text = self.initials.clone().unwrap_or_else(|| initials(&self.name));
            if text.is_empty() {
                let icon = if self.size <= 28.0 {
                    IconSize::S16
                } else if self.size <= 48.0 {
                    IconSize::S20
                } else {
                    IconSize::S24
                };
                let side: f32 = match icon {
                    IconSize::S16 => 16.0,
                    IconSize::S20 => 20.0,
                    IconSize::S24 => 24.0,
                };
                let icon_rect = Rect::from_center_size(rect.center(), Vec2::splat(side.min(rect.width())));
                paint_icon(
                    ui,
                    icon_rect,
                    IconName::Person,
                    icon,
                    IconStyle::Regular,
                    Some(ink),
                );
            } else {
                let galley = WidgetText::from(text).into_galley(
                    ui,
                    Some(egui::TextWrapMode::Extend),
                    f32::INFINITY,
                    TextStyle::Name(ramp_for(self.size).name().into()),
                );
                painter.galley(rect.center() - galley.size() / 2.0, galley, ink);
            }
        }
        if let Some(presence) = self.presence {
            paint_presence(&painter, &theme, rect, self.size, presence);
        }
    }
}

impl Widget for Avatar {
    fn ui(self, ui: &mut Ui) -> Response {
        crate::controls::ensure_type_ramp(ui);
        let (rect, response) = ui.allocate_exact_size(Vec2::splat(self.size), Sense::hover());
        if ui.is_rect_visible(rect) {
            self.paint(ui, rect);
        }
        let label = self.accessible_label();
        ui.ctx().accesskit_node_builder(response.id, |node| {
            node.set_role(egui::accesskit::Role::Image);
            node.set_label(label);
        });
        response
    }
}

/// How an [`AvatarGroup`] lays its avatars out.
#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub enum AvatarGroupLayout {
    /// Side by side with a gap. The default.
    #[default]
    Spread,
    /// Overlapping, each ringed in the page colour.
    Stack,
}

/// A row of avatars with a "+N" overflow for the people that don't fit.
///
/// ```
/// # use fluent2_egui::controls::{AvatarGroup, AvatarGroupLayout};
/// # fn demo(ui: &mut egui::Ui) {
/// AvatarGroup::new(&["Ada Lovelace", "Grace Hopper", "Alan Turing", "Edsger Dijkstra"])
///     .layout(AvatarGroupLayout::Stack)
///     .max_visible(3)
///     .show(ui);
/// # }
/// ```
pub struct AvatarGroup<'a> {
    names: &'a [&'a str],
    size: f32,
    layout: AvatarGroupLayout,
    max_visible: usize,
    color: AvatarColor,
    label: Option<String>,
}

impl<'a> AvatarGroup<'a> {
    /// A group of the people named in `names`.
    #[must_use]
    pub fn new(names: &'a [&'a str]) -> Self {
        Self {
            names,
            size: 32.0,
            layout: AvatarGroupLayout::Spread,
            max_visible: 5,
            color: AvatarColor::Colorful,
            label: None,
        }
    }

    /// Avatar size, snapped like [`Avatar::size`]. The default is 32.
    #[must_use]
    pub fn size(mut self, size: f32) -> Self {
        self.size = snap_size(size);
        self
    }

    /// Spread (the default) or stack.
    #[must_use]
    pub fn layout(mut self, layout: AvatarGroupLayout) -> Self {
        self.layout = layout;
        self
    }

    /// Most avatars shown, counting the "+N" overflow as one. The default is 5; at least 1.
    #[must_use]
    pub fn max_visible(mut self, max: usize) -> Self {
        self.max_visible = max.max(1);
        self
    }

    /// Avatar colours. The default is colourful.
    #[must_use]
    pub fn color(mut self, color: AvatarColor) -> Self {
        self.color = color;
        self
    }

    /// Accessible name of the group. The default is "N people".
    #[must_use]
    pub fn label(mut self, label: impl Into<String>) -> Self {
        self.label = Some(label.into());
        self
    }

    /// Show the group. Hovering or focusing the overflow lists the hidden names.
    pub fn show(self, ui: &mut Ui) -> Response {
        crate::controls::ensure_type_ramp(ui);
        let (shown, hidden) = split_overflow(self.names, self.max_visible);
        let step = match self.layout {
            AvatarGroupLayout::Spread => self.size + group_gap(self.size),
            AvatarGroupLayout::Stack => self.size * 0.75,
        };
        let count = shown.len() + usize::from(!hidden.is_empty());
        let width = if count == 0 {
            0.0
        } else {
            step * (count - 1) as f32 + self.size
        };
        let (rect, response) = ui.allocate_exact_size(Vec2::new(width, self.size), Sense::hover());
        let group_label = self
            .label
            .clone()
            .unwrap_or_else(|| format!("{} people", self.names.len()));
        let ring = self.layout == AvatarGroupLayout::Stack;
        let mut x = rect.left();
        for name in shown {
            let avatar_rect = Rect::from_min_size(Pos2::new(x, rect.top()), Vec2::splat(self.size));
            let mut child = ui.new_child(egui::UiBuilder::new().max_rect(avatar_rect));
            let mut avatar = Avatar::new(*name).size(self.size).color(self.color);
            avatar.ring = ring;
            child.add(avatar);
            x += step;
        }
        if !hidden.is_empty() {
            let more = Rect::from_min_size(Pos2::new(x, rect.top()), Vec2::splat(self.size));
            let id = response.id.with("overflow");
            let overflow = ui.interact(more, id, Sense::focusable_noninteractive());
            let mut avatar = Avatar::new("")
                .size(self.size)
                .initials(format!("+{}", hidden.len()))
                .color(AvatarColor::Neutral);
            avatar.ring = ring;
            if ui.is_rect_visible(more) {
                avatar.paint(ui, more);
            }
            let list = hidden.join(", ");
            let label = overflow_label(hidden.len(), &list);
            ui.ctx().accesskit_node_builder(id, |node| {
                node.set_role(egui::accesskit::Role::Image);
                node.set_label(label);
            });
            let _ = overflow.fluent_tooltip(list);
        }
        ui.ctx().accesskit_node_builder(response.id, |node| {
            node.set_role(egui::accesskit::Role::Group);
            node.set_label(group_label);
        });
        response
    }
}

/// An avatar with up to three lines of text beside it.
///
/// ```
/// # use fluent2_egui::controls::{Avatar, Persona, Presence};
/// # fn demo(ui: &mut egui::Ui) {
/// Persona::new(Avatar::new("Ada Lovelace").presence(Presence::Away))
///     .primary("Ada Lovelace")
///     .secondary("Analyst")
///     .show(ui);
/// # }
/// ```
pub struct Persona {
    avatar: Avatar,
    primary: Option<String>,
    secondary: Option<String>,
    tertiary: Option<String>,
}

impl Persona {
    /// A persona around `avatar`. With no primary text, the avatar's name is used.
    #[must_use]
    pub fn new(avatar: Avatar) -> Self {
        Self {
            avatar,
            primary: None,
            secondary: None,
            tertiary: None,
        }
    }

    /// First line, `body1`.
    #[must_use]
    pub fn primary(mut self, text: impl Into<String>) -> Self {
        self.primary = Some(text.into());
        self
    }

    /// Second line, `caption1` in `neutral_foreground_2`.
    #[must_use]
    pub fn secondary(mut self, text: impl Into<String>) -> Self {
        self.secondary = Some(text.into());
        self
    }

    /// Third line, `caption1` in `neutral_foreground_2`.
    #[must_use]
    pub fn tertiary(mut self, text: impl Into<String>) -> Self {
        self.tertiary = Some(text.into());
        self
    }

    /// Show the persona.
    pub fn show(self, ui: &mut Ui) -> Response {
        crate::controls::ensure_type_ramp(ui);
        let theme = Theme::get(ui.ctx());
        let primary = self.primary.clone().unwrap_or_else(|| self.avatar.name.clone());
        ui.horizontal(|ui| {
            ui.spacing_mut().item_spacing.x = 8.0;
            ui.add(self.avatar);
            ui.vertical(|ui| {
                ui.spacing_mut().item_spacing.y = 0.0;
                ui.label(
                    crate::foundation::text::rich(primary, TypeRamp::Body1)
                        .color(c(theme.color.neutral_foreground_1)),
                );
                for line in [self.secondary, self.tertiary].into_iter().flatten() {
                    ui.label(
                        crate::foundation::text::rich(line, TypeRamp::Caption1)
                            .color(c(theme.color.neutral_foreground_2)),
                    );
                }
            });
        })
        .response
    }
}

/// Initials for `name`: the first letter of the first and the last word, uppercased. Words in
/// brackets ("(Contractor)") and words that start with no letter or digit are skipped. One word
/// gives one letter; no usable word gives an empty string.
#[must_use]
pub fn initials(name: &str) -> String {
    let mut depth = 0_i32;
    let mut words = Vec::new();
    for word in name.split_whitespace() {
        let opens = word.starts_with(['(', '[', '{']);
        if opens {
            depth += 1;
        }
        if depth == 0
            && let Some(first) = word.chars().next().filter(|ch| ch.is_alphanumeric())
        {
            words.push(first);
        }
        if word.ends_with([')', ']', '}']) {
            depth = (depth - 1).max(0);
        }
    }
    let picked: Vec<char> = match words.as_slice() {
        [] => Vec::new(),
        [only] => vec![*only],
        [first, .., last] => vec![*first, *last],
    };
    picked.into_iter().flat_map(char::to_uppercase).collect()
}

/// The palette index (0..30) [`AvatarColor::Colorful`] uses for `name`: FNV-1a over its
/// trimmed, lowercased bytes, so a name always gets the same colour.
#[must_use]
pub fn palette_index(name: &str) -> usize {
    let mut hash: u32 = 0x811c_9dc5;
    for byte in name.trim().to_lowercase().bytes() {
        hash ^= u32::from(byte);
        hash = hash.wrapping_mul(0x0100_0193);
    }
    (hash as usize) % PALETTE.len()
}

type Pair = (fn(&AliasColors) -> Rgba, fn(&AliasColors) -> Rgba);

/// Fluent's 30 named avatar colours: background 2 and foreground 2 of each palette.
const PALETTE: [Pair; 30] = [
    (
        |c| c.palette_dark_red_background_2,
        |c| c.palette_dark_red_foreground_2,
    ),
    (
        |c| c.palette_cranberry_background_2,
        |c| c.palette_cranberry_foreground_2,
    ),
    (|c| c.palette_red_background_2, |c| c.palette_red_foreground_2),
    (
        |c| c.palette_pumpkin_background_2,
        |c| c.palette_pumpkin_foreground_2,
    ),
    (|c| c.palette_peach_background_2, |c| c.palette_peach_foreground_2),
    (
        |c| c.palette_marigold_background_2,
        |c| c.palette_marigold_foreground_2,
    ),
    (|c| c.palette_gold_background_2, |c| c.palette_gold_foreground_2),
    (|c| c.palette_brass_background_2, |c| c.palette_brass_foreground_2),
    (|c| c.palette_brown_background_2, |c| c.palette_brown_foreground_2),
    (
        |c| c.palette_forest_background_2,
        |c| c.palette_forest_foreground_2,
    ),
    (
        |c| c.palette_seafoam_background_2,
        |c| c.palette_seafoam_foreground_2,
    ),
    (
        |c| c.palette_dark_green_background_2,
        |c| c.palette_dark_green_foreground_2,
    ),
    (
        |c| c.palette_light_teal_background_2,
        |c| c.palette_light_teal_foreground_2,
    ),
    (|c| c.palette_teal_background_2, |c| c.palette_teal_foreground_2),
    (|c| c.palette_steel_background_2, |c| c.palette_steel_foreground_2),
    (|c| c.palette_blue_background_2, |c| c.palette_blue_foreground_2),
    (
        |c| c.palette_royal_blue_background_2,
        |c| c.palette_royal_blue_foreground_2,
    ),
    (
        |c| c.palette_cornflower_background_2,
        |c| c.palette_cornflower_foreground_2,
    ),
    (|c| c.palette_navy_background_2, |c| c.palette_navy_foreground_2),
    (
        |c| c.palette_lavender_background_2,
        |c| c.palette_lavender_foreground_2,
    ),
    (
        |c| c.palette_purple_background_2,
        |c| c.palette_purple_foreground_2,
    ),
    (|c| c.palette_grape_background_2, |c| c.palette_grape_foreground_2),
    (|c| c.palette_lilac_background_2, |c| c.palette_lilac_foreground_2),
    (|c| c.palette_pink_background_2, |c| c.palette_pink_foreground_2),
    (
        |c| c.palette_magenta_background_2,
        |c| c.palette_magenta_foreground_2,
    ),
    (|c| c.palette_plum_background_2, |c| c.palette_plum_foreground_2),
    (|c| c.palette_beige_background_2, |c| c.palette_beige_foreground_2),
    (|c| c.palette_mink_background_2, |c| c.palette_mink_foreground_2),
    (
        |c| c.palette_platinum_background_2,
        |c| c.palette_platinum_foreground_2,
    ),
    (
        |c| c.palette_anchor_background_2,
        |c| c.palette_anchor_foreground_2,
    ),
];

fn colors(theme: &Theme, color: AvatarColor, name: &str) -> (Color32, Color32) {
    let t = &theme.color;
    match color {
        AvatarColor::Neutral => (c(t.neutral_background_6), c(t.neutral_foreground_3)),
        AvatarColor::Brand => (c(t.brand_background), c(t.neutral_foreground_on_brand)),
        AvatarColor::Colorful => {
            let (bg, fg) = PALETTE[palette_index(name)];
            (c(bg(t)), c(fg(t)))
        }
    }
}

/// The nearest Fluent size. Non-finite sizes give the default, 32.
fn snap_size(size: f32) -> f32 {
    if !size.is_finite() {
        return 32.0;
    }
    AVATAR_SIZES
        .iter()
        .copied()
        .min_by(|a, b| (a - size).abs().total_cmp(&(b - size).abs()))
        .unwrap_or(32.0)
}

/// Corner radius of a square avatar: Fluent's small to extra-large radii by size.
fn square_radius(size: f32) -> u8 {
    if size <= 24.0 {
        2
    } else if size <= 40.0 {
        4
    } else if size <= 64.0 {
        6
    } else {
        8
    }
}

/// The type ramp step of the initials at `size`.
fn ramp_for(size: f32) -> TypeRamp {
    if size <= 24.0 {
        TypeRamp::Caption2
    } else if size <= 32.0 {
        TypeRamp::Caption1
    } else if size <= 48.0 {
        TypeRamp::Body1Strong
    } else if size <= 64.0 {
        TypeRamp::Subtitle2
    } else if size <= 96.0 {
        TypeRamp::Subtitle1
    } else {
        TypeRamp::Title3
    }
}

fn group_gap(size: f32) -> f32 {
    if size <= 24.0 {
        2.0
    } else if size <= 48.0 {
        4.0
    } else {
        8.0
    }
}

/// The names shown and the names behind the overflow, for at most `max` slots. When there is
/// overflow, it takes one slot.
fn split_overflow<'n>(names: &'n [&'n str], max: usize) -> (&'n [&'n str], &'n [&'n str]) {
    if names.len() <= max {
        (names, &[])
    } else {
        names.split_at(max.saturating_sub(1))
    }
}

fn overflow_label(count: usize, names: &str) -> String {
    format!("{count} more: {names}")
}

/// The presence badge in the bottom trailing corner, ringed in the page colour.
fn paint_presence(painter: &egui::Painter, theme: &Theme, rect: Rect, size: f32, presence: Presence) {
    let t = &theme.color;
    let side = (size * 0.3).clamp(6.0, 20.0);
    let center = Pos2::new(rect.right() - side / 2.0, rect.bottom() - side / 2.0);
    let ring = c(t.neutral_background_1);
    painter.circle_filled(center, side / 2.0 + 1.0, ring);
    let radius = side / 2.0;
    match presence {
        Presence::Available => {
            painter.circle_filled(center, radius, c(t.palette_light_green_foreground_3));
        }
        Presence::Busy => {
            painter.circle_filled(center, radius, c(t.palette_red_background_3));
        }
        Presence::Away => {
            painter.circle_filled(center, radius, c(t.palette_marigold_background_3));
        }
        Presence::DoNotDisturb => {
            painter.circle_filled(center, radius, c(t.palette_red_background_3));
            let half = radius * 0.5;
            painter.line_segment(
                [center - Vec2::new(half, 0.0), center + Vec2::new(half, 0.0)],
                Stroke::new((radius * 0.35).max(1.0), ring),
            );
        }
        Presence::Offline => {
            painter.circle_stroke(center, radius - 0.5, Stroke::new(1.0, c(t.neutral_foreground_3)));
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn initials_take_the_first_and_last_words() {
        assert_eq!(initials("Ada Lovelace"), "AL");
        assert_eq!(initials("ada augusta king lovelace"), "AL");
        assert_eq!(initials("Grace"), "G");
        assert_eq!(initials("Grace Hopper (Contractor)"), "GH");
        assert_eq!(initials("  "), "");
        assert_eq!(initials("-- !!"), "");
        assert_eq!(initials("Émile Zola"), "ÉZ");
    }

    #[test]
    fn palette_index_is_stable_and_in_range() {
        assert_eq!(palette_index("Ada Lovelace"), palette_index(" ada lovelace "));
        for name in ["", "a", "Grace Hopper", "Alan Turing", "Ünïcødé"] {
            assert!(palette_index(name) < PALETTE.len());
        }
    }

    #[test]
    fn sizes_snap_to_fluent_steps() {
        assert_eq!(snap_size(33.0), 32.0);
        assert_eq!(snap_size(1000.0), 128.0);
        assert_eq!(snap_size(0.0), 16.0);
        assert_eq!(snap_size(f32::NAN), 32.0);
    }

    #[test]
    fn overflow_keeps_one_slot_for_the_counter() {
        let names = ["a", "b", "c", "d", "e"];
        let (shown, hidden) = split_overflow(&names, 3);
        assert_eq!(shown, &["a", "b"]);
        assert_eq!(hidden, &["c", "d", "e"]);
        let (shown, hidden) = split_overflow(&names, 5);
        assert_eq!(shown.len(), 5);
        assert!(hidden.is_empty());
    }
}
