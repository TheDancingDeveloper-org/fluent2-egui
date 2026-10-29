//! Fluent Skeleton: placeholder shapes shown while content loads.
//!
//! Items stack top to bottom with `spacingVerticalS` between them. A rectangle or text line is
//! `borderRadiusMedium` high or full width, a circle is a disc. Heights and diameters snap to
//! Fluent's size ramp ([`SKELETON_SIZES`]).
//!
//! The fill is `neutral_stencil_1`. [`SkeletonAnimation::Wave`] sweeps `neutral_stencil_2` across
//! all items together, and [`SkeletonAnimation::Pulse`] fades between the two. Both run on
//! [`foundation::motion`] and hold still under reduced motion, as Spinner does. High contrast
//! draws an outline in `neutral_stroke_1` instead of a fill, and does not animate.
//!
//! The whole skeleton is one AccessKit node: a busy ProgressIndicator with a label, "Loading" by
//! default. The items are painted, not widgets, so they have no nodes of their own.

use egui::{
    Color32, CornerRadius, Id, Mesh, Pos2, Rect, Response, Role, Sense, Stroke, StrokeKind, Ui, Vec2, Widget,
    WidgetInfo,
};

use fluent2_tokens::generated::global::{border_radius, spacing_vertical, stroke_width};

use crate::foundation::motion::{self, Curve, Duration};
use crate::theme::{self, Theme, ThemeKind};

/// Fluent's skeleton size ramp, in points. Heights and diameters snap to the nearest entry.
pub const SKELETON_SIZES: [f32; 12] = [
    16.0, 24.0, 28.0, 32.0, 36.0, 40.0, 48.0, 56.0, 64.0, 72.0, 96.0, 128.0,
];

/// Width of one shimmer column, in points. Narrow enough that the gradient bends smoothly.
const COLUMN_WIDTH: f32 = 8.0;

/// One placeholder shape.
///
/// ```
/// use fluent2_egui::controls::SkeletonItem;
/// let _ = [
///     SkeletonItem::circle(40.0),
///     SkeletonItem::text_line(16.0, 1.0),
///     SkeletonItem::text_line(16.0, 0.6),
/// ];
/// ```
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum SkeletonItem {
    /// A full-width bar, `height` tall.
    Rectangle {
        /// Height in points, snapped to [`SKELETON_SIZES`].
        height: f32,
    },
    /// A disc, `diameter` across and left-aligned.
    Circle {
        /// Diameter in points, snapped to [`SKELETON_SIZES`].
        diameter: f32,
    },
    /// A bar `height` tall that fills `width` of the available width.
    TextLine {
        /// Height in points, snapped to [`SKELETON_SIZES`].
        height: f32,
        /// Fraction of the available width, from 0 to 1. Other values clamp.
        width: f32,
    },
}

impl SkeletonItem {
    /// A full-width bar, `height` tall.
    #[must_use]
    pub fn rectangle(height: f32) -> Self {
        Self::Rectangle { height }
    }

    /// A disc, `diameter` across.
    #[must_use]
    pub fn circle(diameter: f32) -> Self {
        Self::Circle { diameter }
    }

    /// A bar `height` tall, `width` (0 to 1) of the available width.
    #[must_use]
    pub fn text_line(height: f32, width: f32) -> Self {
        Self::TextLine { height, width }
    }
}

/// How the placeholder moves while content loads.
#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub enum SkeletonAnimation {
    /// A shimmer band sweeps across the items. The default.
    #[default]
    Wave,
    /// The fill fades between the two stencil tokens.
    Pulse,
    /// No motion, as under reduced motion.
    Static,
}

/// A loading placeholder built from [`SkeletonItem`]s.
///
/// ```
/// # use fluent2_egui::controls::{Skeleton, SkeletonItem};
/// # fn demo(ui: &mut egui::Ui) {
/// let _ = ui.add(
///     Skeleton::new()
///         .item(SkeletonItem::circle(40.0))
///         .item(SkeletonItem::text_line(16.0, 0.6))
///         .label("Loading contacts"),
/// );
/// # }
/// ```
pub struct Skeleton {
    items: Vec<SkeletonItem>,
    animation: SkeletonAnimation,
    label: String,
}

impl Skeleton {
    /// An empty skeleton that waves and is labelled "Loading".
    #[must_use]
    pub fn new() -> Self {
        Self {
            items: Vec::new(),
            animation: SkeletonAnimation::Wave,
            label: "Loading".to_owned(),
        }
    }

    /// Append one item, top to bottom.
    #[must_use]
    pub fn item(mut self, item: SkeletonItem) -> Self {
        self.items.push(item);
        self
    }

    /// Append several items, top to bottom.
    #[must_use]
    pub fn items(mut self, items: impl IntoIterator<Item = SkeletonItem>) -> Self {
        self.items.extend(items);
        self
    }

    /// Motion. Defaults to [`SkeletonAnimation::Wave`].
    #[must_use]
    pub fn animation(mut self, animation: SkeletonAnimation) -> Self {
        self.animation = animation;
        self
    }

    /// The accessible name. Defaults to "Loading".
    #[must_use]
    pub fn label(mut self, label: impl Into<String>) -> Self {
        self.label = label.into();
        self
    }
}

impl Default for Skeleton {
    fn default() -> Self {
        Self::new()
    }
}

impl Widget for Skeleton {
    fn ui(self, ui: &mut Ui) -> Response {
        let theme = Theme::get(ui.ctx());
        let width = finite_nonneg(ui.available_width());
        let (pieces, total) = layout(&self.items, width);
        let (rect, response) = ui.allocate_exact_size(Vec2::new(width, total), Sense::hover());
        if ui.is_rect_visible(rect) {
            let pieces: Vec<Piece> = pieces.iter().map(|piece| piece.moved_to(rect.min)).collect();
            let high_contrast = theme.kind == ThemeKind::HighContrast;
            let animated = !high_contrast
                && self.animation != SkeletonAnimation::Static
                && !motion::reduced_motion(ui.ctx());
            let fill = theme::c(theme.color.neutral_stencil_1);
            if high_contrast {
                let stroke = Stroke::new(stroke_width::THIN, theme::c(theme.color.neutral_stroke_1));
                for piece in &pieces {
                    piece.outline(ui, stroke);
                }
            } else if animated && self.animation == SkeletonAnimation::Wave {
                let shimmer = theme::c(theme.color.neutral_stencil_2);
                let phase = cycle(ui, response.id.with("wave"), Duration::UltraSlow, Curve::Linear);
                let band = rect.width() * 0.4;
                let centre = rect.left() - band / 2.0 + (rect.width() + band) * phase;
                let colour_at = |x: f32| fill.lerp_to_gamma(shimmer, band_weight(x, centre, band));
                let mut mesh = Mesh::default();
                for piece in &pieces {
                    piece.push_columns(&mut mesh, &colour_at);
                }
                ui.painter().add(egui::Shape::mesh(mesh));
            } else if animated {
                let shimmer = theme::c(theme.color.neutral_stencil_2);
                let phase = cycle(
                    ui,
                    response.id.with("pulse"),
                    Duration::UltraSlow,
                    Curve::EasyEase,
                );
                let colour = fill.lerp_to_gamma(shimmer, phase);
                for piece in &pieces {
                    piece.fill(ui, colour);
                }
            } else {
                for piece in &pieces {
                    piece.fill(ui, fill);
                }
            }
            if animated {
                ui.ctx().request_repaint();
            }
        }
        let label = self.label;
        response.widget_info(|| WidgetInfo::labeled(Role::ProgressIndicator, ui.is_enabled(), label.clone()));
        ui.ctx()
            .accesskit_node_builder(response.id, |node| node.set_busy());
        response
    }
}

/// A shape placed inside the skeleton, in points from its top left corner.
#[derive(Clone, Copy, Debug, PartialEq)]
enum Piece {
    /// A rectangle with rounded corners.
    Bar { rect: Rect, radius: f32 },
    /// A disc.
    Disc { centre: Pos2, radius: f32 },
}

impl Piece {
    fn moved_to(&self, origin: Pos2) -> Self {
        match *self {
            Self::Bar { rect, radius } => Self::Bar {
                rect: rect.translate(origin.to_vec2()),
                radius,
            },
            Self::Disc { centre, radius } => Self::Disc {
                centre: centre + origin.to_vec2(),
                radius,
            },
        }
    }

    fn bounds(&self) -> Rect {
        match *self {
            Self::Bar { rect, .. } => rect,
            Self::Disc { centre, radius } => Rect::from_center_size(centre, Vec2::splat(radius * 2.0)),
        }
    }

    fn fill(&self, ui: &Ui, colour: Color32) {
        match *self {
            Self::Bar { rect, radius } => {
                ui.painter()
                    .rect_filled(rect, CornerRadius::same(rounded_u8(radius)), colour);
            }
            Self::Disc { centre, radius } => {
                ui.painter().circle_filled(centre, radius, colour);
            }
        }
    }

    fn outline(&self, ui: &Ui, stroke: Stroke) {
        match *self {
            Self::Bar { rect, radius } => {
                ui.painter().rect_stroke(
                    rect,
                    CornerRadius::same(rounded_u8(radius)),
                    stroke,
                    StrokeKind::Inside,
                );
            }
            Self::Disc { centre, radius } => {
                ui.painter()
                    .circle_stroke(centre, (radius - stroke.width / 2.0).max(0.0), stroke);
            }
        }
    }

    /// Vertical extent of the shape at `x`, or `None` outside it. Rounded corners pull the top
    /// and bottom in, so the shimmer does not paint past them.
    fn span_at(&self, x: f32) -> Option<(f32, f32)> {
        match *self {
            Self::Bar { rect, radius } => {
                if x < rect.left() || x > rect.right() {
                    return None;
                }
                let inset = if radius <= 0.0 {
                    0.0
                } else {
                    let from_edge = (x - rect.left()).min(rect.right() - x);
                    if from_edge >= radius {
                        0.0
                    } else {
                        let along = radius - from_edge;
                        radius - (radius * radius - along * along).max(0.0).sqrt()
                    }
                };
                Some((rect.top() + inset, rect.bottom() - inset))
            }
            Self::Disc { centre, radius } => {
                let dx = x - centre.x;
                if dx.abs() > radius {
                    return None;
                }
                let dy = (radius * radius - dx * dx).max(0.0).sqrt();
                Some((centre.y - dy, centre.y + dy))
            }
        }
    }

    /// Add this shape as a strip of columns, each coloured by `colour_at` at its x.
    fn push_columns(&self, mesh: &mut Mesh, colour_at: &impl Fn(f32) -> Color32) {
        let bounds = self.bounds();
        let columns = ((bounds.width() / COLUMN_WIDTH).ceil() as usize).clamp(1, 128);
        let base = mesh.vertices.len() as u32;
        for column in 0..=columns {
            let x = bounds.left() + bounds.width() * column as f32 / columns as f32;
            let Some((top, bottom)) = self.span_at(x) else {
                continue;
            };
            let colour = colour_at(x);
            mesh.colored_vertex(Pos2::new(x, top), colour);
            mesh.colored_vertex(Pos2::new(x, bottom), colour);
        }
        let vertices = (mesh.vertices.len() as u32).saturating_sub(base) / 2;
        for column in 0..vertices.saturating_sub(1) {
            let a = base + column * 2;
            let b = a + 1;
            let c = a + 2;
            let d = a + 3;
            mesh.add_triangle(a, b, c);
            mesh.add_triangle(b, d, c);
        }
    }
}

/// Place the items top to bottom from the origin. Returns the shapes and the total height.
fn layout(items: &[SkeletonItem], width: f32) -> (Vec<Piece>, f32) {
    let mut pieces = Vec::with_capacity(items.len());
    let mut top = 0.0_f32;
    for (index, item) in items.iter().enumerate() {
        if index > 0 {
            top += spacing_vertical::S;
        }
        let piece = match *item {
            SkeletonItem::Rectangle { height } => {
                let height = snap(height);
                bar(0.0, top, width, height)
            }
            SkeletonItem::Circle { diameter } => {
                let diameter = snap(diameter);
                Piece::Disc {
                    centre: Pos2::new(diameter / 2.0, top + diameter / 2.0),
                    radius: diameter / 2.0,
                }
            }
            SkeletonItem::TextLine {
                height,
                width: fraction,
            } => {
                let height = snap(height);
                bar(0.0, top, width * clamp_unit(fraction), height)
            }
        };
        top += piece.bounds().height();
        pieces.push(piece);
    }
    (pieces, top)
}

fn bar(left: f32, top: f32, width: f32, height: f32) -> Piece {
    Piece::Bar {
        rect: Rect::from_min_size(Pos2::new(left, top), Vec2::new(width, height)),
        radius: border_radius::MEDIUM.min(height / 2.0),
    }
}

/// Weight of the shimmer band at `x`: 1 at its centre, falling to 0 at its edges.
fn band_weight(x: f32, centre: f32, band: f32) -> f32 {
    let half = band / 2.0;
    if !half.is_finite() || half <= 0.0 {
        return 0.0;
    }
    (1.0 - (x - centre).abs() / half).clamp(0.0, 1.0)
}

/// Sweep phase in `0..=1`, driven by [`motion::animate`]. Runs out and back, as the progress bar
/// does, and is only called when motion is on.
fn cycle(ui: &Ui, id: Id, duration: Duration, curve: Curve) -> f32 {
    let going = ui
        .ctx()
        .data(|data| data.get_temp::<bool>(id.with("dir")).unwrap_or(true));
    let target = if going { 1.0 } else { 0.0 };
    let phase = motion::animate(ui.ctx(), id.with("cycle"), target, duration, curve);
    let moved = (phase - if going { 0.0 } else { 1.0 }).abs() > 0.01;
    if moved && (phase - target).abs() < 0.01 {
        ui.ctx().data_mut(|data| data.insert_temp(id.with("dir"), !going));
    }
    phase
}

/// Snap to the nearest entry of [`SKELETON_SIZES`]. Non-finite values take the smallest.
fn snap(value: f32) -> f32 {
    if !value.is_finite() {
        return SKELETON_SIZES[0];
    }
    SKELETON_SIZES
        .iter()
        .copied()
        .min_by(|a, b| (a - value).abs().total_cmp(&(b - value).abs()))
        .unwrap_or(SKELETON_SIZES[0])
}

fn clamp_unit(value: f32) -> f32 {
    if value.is_finite() {
        value.clamp(0.0, 1.0)
    } else {
        1.0
    }
}

fn finite_nonneg(value: f32) -> f32 {
    if value.is_finite() && value >= 0.0 {
        value
    } else {
        0.0
    }
}

fn rounded_u8(radius: f32) -> u8 {
    radius.round().clamp(0.0, f32::from(u8::MAX)) as u8
}

#[cfg(test)]
mod tests {
    use super::{SKELETON_SIZES, SkeletonItem, band_weight, layout, snap};

    #[test]
    fn heights_snap_to_the_fluent_ramp() {
        assert_eq!(snap(13.0), 16.0);
        assert_eq!(snap(100.0), 96.0);
        assert_eq!(snap(1000.0), 128.0);
        assert_eq!(snap(f32::NAN), SKELETON_SIZES[0]);
    }

    #[test]
    fn items_stack_with_a_gap_and_clamp_their_width() {
        let items = [
            SkeletonItem::rectangle(16.0),
            SkeletonItem::circle(40.0),
            SkeletonItem::text_line(16.0, 2.0),
        ];
        let (pieces, total) = layout(&items, 200.0);
        assert_eq!(pieces.len(), 3);
        // 16 + 8 gap + 40 + 8 gap + 16.
        assert_eq!(total, 88.0);
        assert_eq!(pieces[2].bounds().width(), 200.0);
    }

    #[test]
    fn non_finite_inputs_lay_out_without_panic() {
        let items = [
            SkeletonItem::rectangle(f32::NAN),
            SkeletonItem::text_line(f32::INFINITY, f32::NAN),
        ];
        let (_, total) = layout(&items, f32::NAN);
        assert!(total.is_finite());
    }

    #[test]
    fn band_peaks_at_its_centre_and_is_zero_outside() {
        assert_eq!(band_weight(50.0, 50.0, 20.0), 1.0);
        assert_eq!(band_weight(70.0, 50.0, 20.0), 0.0);
        assert_eq!(band_weight(50.0, 50.0, 0.0), 0.0);
    }
}
