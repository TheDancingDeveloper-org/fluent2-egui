//! DiffView: a read-only diff of two texts, inline or side by side, with hunk headers and
//! word-level changes (behind the `diff` cargo feature).
//!
//! [`DiffModel`] does the work once, on `imara-diff`; the view paints only the rows in the
//! viewport (`rows::visible_rows`), so a diff of any length scrolls at the same cost. Colours
//! come from Fluent palette tokens. In high contrast, where those fills are all the window
//! colour, changed rows get a bar and changed words an outline instead.

mod model;

#[rustfmt::skip]
pub use model::{
    DEFAULT_CONTEXT,
    DiffModel,
    HunkHeader,
    InlineRow,
    LineKind,
    MAX_WORD_DIFF_COLUMNS,
    SplitRow,
    TAB_WIDTH,
};

use std::ops::Range;

use egui::{
    Align2, Color32, CornerRadius, FontId, Painter, Pos2, Rect, ScrollArea, Sense, Stroke, StrokeKind,
    TextStyle, Ui, Vec2, WidgetInfo, WidgetType,
};

use super::rows::visible_rows;
use crate::theme::{Theme, ThemeKind, c};
use fluent2_tokens::generated::global::{spacing_horizontal, stroke_width};

/// Inline (one column, removed lines above added ones) or side by side (old left, new right).
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum DiffMode {
    /// Unified layout with old and new line numbers.
    #[default]
    Inline,
    /// Old text on the left, new text on the right, aligned row by row.
    SideBySide,
}

/// Colours a DiffView paints with, from the theme's tokens.
///
/// ```
/// use fluent2_egui::Theme;
/// use fluent2_egui::data::diff::DiffColors;
///
/// let colors = DiffColors::new(&Theme::default());
/// assert_ne!(colors.added_line, colors.removed_line);
/// assert!(DiffColors::new(&Theme::high_contrast()).outline.is_some());
/// ```
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct DiffColors {
    /// Background of the whole view.
    pub background: Color32,
    /// Line text.
    pub text: Color32,
    /// Line numbers.
    pub gutter: Color32,
    /// Fill of an added line.
    pub added_line: Color32,
    /// Fill of a removed line.
    pub removed_line: Color32,
    /// Fill of a changed word in an added line.
    pub added_word: Color32,
    /// Fill of a changed word in a removed line.
    pub removed_word: Color32,
    /// The `+` sign and change bar of added lines.
    pub added_sign: Color32,
    /// The `-` sign and change bar of removed lines.
    pub removed_sign: Color32,
    /// Fill of a hunk header row.
    pub hunk_background: Color32,
    /// Text of a hunk header row.
    pub hunk_text: Color32,
    /// Fill of the empty side of a side-by-side row.
    pub filler: Color32,
    /// High contrast only: outline of changed words, and a bar on changed lines.
    pub outline: Option<Color32>,
}

impl DiffColors {
    /// The colours for `theme`.
    #[must_use]
    pub fn new(theme: &Theme) -> Self {
        let t = &theme.color;
        let base = Self {
            background: c(t.neutral_background_1),
            text: c(t.neutral_foreground_1),
            gutter: c(t.neutral_foreground_4),
            added_line: c(t.palette_green_background_1),
            removed_line: c(t.palette_red_background_1),
            added_word: c(t.palette_green_background_2),
            removed_word: c(t.palette_red_background_2),
            added_sign: c(t.palette_green_foreground_1),
            removed_sign: c(t.palette_red_foreground_1),
            hunk_background: c(t.neutral_background_3),
            hunk_text: c(t.neutral_foreground_3),
            filler: c(t.neutral_background_2),
            outline: None,
        };
        if theme.kind == ThemeKind::HighContrast {
            // The palette fills are all the window colour here; mark changes with strokes.
            Self {
                added_line: base.background,
                removed_line: base.background,
                added_word: base.background,
                removed_word: base.background,
                added_sign: base.text,
                removed_sign: base.text,
                hunk_background: base.background,
                hunk_text: base.text,
                filler: base.background,
                outline: Some(c(t.neutral_stroke_1)),
                ..base
            }
        } else {
            base
        }
    }
}

/// What a DiffView reported this frame.
#[derive(Debug)]
pub struct DiffResponse {
    /// The response of the content area.
    pub response: egui::Response,
    /// Rows painted this frame (indices into the mode's row list).
    pub visible_rows: Range<usize>,
}

/// A read-only diff view over a [`DiffModel`].
///
/// ```
/// use fluent2_egui::data::diff::{DiffMode, DiffModel, DiffView};
///
/// # fn demo(ui: &mut egui::Ui) {
/// let model = DiffModel::new("fn a() {}\n", "fn b() {}\n"); // keep it; rebuild on change
/// DiffView::new("changes").mode(DiffMode::SideBySide).show(ui, &model);
/// # }
/// ```
#[derive(Clone, Debug)]
pub struct DiffView {
    id_salt: egui::Id,
    mode: DiffMode,
    font: Option<FontId>,
    label: String,
}

impl DiffView {
    /// A view whose scroll state is keyed by `id_salt`.
    pub fn new(id_salt: impl std::hash::Hash) -> Self {
        Self {
            id_salt: egui::Id::new(hashed(&id_salt)),
            mode: DiffMode::Inline,
            font: None,
            label: "Diff".into(),
        }
    }

    /// Inline or side by side. Default inline.
    #[must_use]
    pub fn mode(mut self, mode: DiffMode) -> Self {
        self.mode = mode;
        self
    }

    /// Monospace font. Defaults to the `Monospace` text style.
    #[must_use]
    pub fn font(mut self, font: FontId) -> Self {
        self.font = Some(font);
        self
    }

    /// Accessible name. Defaults to "Diff".
    #[must_use]
    pub fn label(mut self, label: impl Into<String>) -> Self {
        self.label = label.into();
        self
    }

    /// Paint `model` into the available space, scrolling both ways.
    pub fn show(self, ui: &mut Ui, model: &DiffModel) -> DiffResponse {
        let theme = Theme::get(ui.ctx());
        let colors = DiffColors::new(&theme);
        let font = self
            .font
            .clone()
            .unwrap_or_else(|| TextStyle::Monospace.resolve(ui.style()));
        let (cw, ch) = ui.fonts_mut(|f| (f.glyph_width(&font, 'M'), f.row_height(&font)));
        let cell = Vec2::new(finite_pos(cw, 8.0), finite_pos(ch, 16.0));
        let row_h = cell.y + 2.0 * stroke_width::THIN;
        let pad = spacing_horizontal::S;

        let digits = model
            .old_lines()
            .len()
            .max(model.new_lines().len())
            .max(1)
            .to_string()
            .len()
            .max(3) as f32;
        let number_w = digits * cell.x + pad;
        let sign_w = 2.0 * cell.x;
        let text_w = model.max_columns() as f32 * cell.x + pad;
        let row_count = match self.mode {
            DiffMode::Inline => model.inline_rows().len(),
            DiffMode::SideBySide => model.split_rows().len(),
        };
        let content_w = match self.mode {
            DiffMode::Inline => 2.0 * number_w + sign_w + text_w,
            DiffMode::SideBySide => 2.0 * (number_w + sign_w + text_w),
        };

        let enabled = ui.is_enabled();
        let summary = format!(
            "{}: {} additions, {} removals",
            self.label,
            model.additions(),
            model.removals()
        );
        let area = ui.available_rect_before_wrap();
        ui.painter()
            .rect_filled(area, CornerRadius::ZERO, colors.background);
        let mut visible = 0..0;
        let output = ScrollArea::both()
            .id_salt(self.id_salt)
            .auto_shrink([false, false])
            .show_viewport(ui, |ui, viewport| {
                let width = content_w.max(ui.available_width());
                let height = row_count as f32 * row_h;
                let (rect, response) = ui.allocate_exact_size(Vec2::new(width, height), Sense::hover());
                visible = visible_rows(viewport, 0.0, row_h, row_count);
                let painter = ui.painter();
                let ctx = RowPainter {
                    painter,
                    model,
                    colors: &colors,
                    font: &font,
                    cell,
                    number_w,
                    sign_w,
                };
                for row in visible.clone() {
                    let y = rect.top() + row as f32 * row_h;
                    let band = Rect::from_min_size(Pos2::new(rect.left(), y), Vec2::new(width, row_h));
                    match self.mode {
                        DiffMode::Inline => {
                            if let Some(r) = model.inline_rows().get(row) {
                                ctx.inline_row(band, *r);
                            }
                        }
                        DiffMode::SideBySide => {
                            if let Some(r) = model.split_rows().get(row) {
                                ctx.split_row(band, *r, width / 2.0);
                            }
                        }
                    }
                }
                response
            });
        let response = output.inner;
        response.widget_info(|| WidgetInfo::labeled(WidgetType::Unknown, enabled, &summary));
        DiffResponse {
            response,
            visible_rows: visible,
        }
    }
}

struct RowPainter<'a> {
    painter: &'a Painter,
    model: &'a DiffModel,
    colors: &'a DiffColors,
    font: &'a FontId,
    cell: Vec2,
    number_w: f32,
    sign_w: f32,
}

impl RowPainter<'_> {
    fn inline_row(&self, band: Rect, row: InlineRow) {
        match row {
            InlineRow::Hunk(n) => self.hunk(band, n),
            InlineRow::Line { kind, old, new } => {
                let x = band.left();
                self.fill_line(band, kind);
                self.number(x, band, old);
                self.number(x + self.number_w, band, new);
                let sign_x = x + 2.0 * self.number_w;
                let (text, words) = match (kind, old, new) {
                    (LineKind::Added, _, Some(n)) => (self.new_line(n), self.model.new_word_changes(n)),
                    (_, Some(o), _) => (self.old_line(o), self.model.old_word_changes(o)),
                    _ => ("", &[][..]),
                };
                self.sign(sign_x, band, kind);
                self.text(sign_x + self.sign_w, band, text, words, kind);
            }
        }
    }

    fn split_row(&self, band: Rect, row: SplitRow, half: f32) {
        match row {
            SplitRow::Hunk(n) => self.hunk(band, n),
            SplitRow::Pair { old, new, changed } => {
                let left = Rect::from_min_max(band.min, Pos2::new(band.left() + half, band.bottom()));
                let right = Rect::from_min_max(Pos2::new(band.left() + half, band.top()), band.max);
                let (lk, rk) = if changed {
                    (LineKind::Removed, LineKind::Added)
                } else {
                    (LineKind::Context, LineKind::Context)
                };
                self.side(left, old, lk, true);
                self.side(right, new, rk, false);
                if let Some(color) = self.colors.outline.or(Some(self.colors.hunk_background)) {
                    self.painter.vline(
                        right.left(),
                        right.y_range(),
                        Stroke::new(stroke_width::THIN, color),
                    );
                }
            }
        }
    }

    fn side(&self, rect: Rect, line: Option<usize>, kind: LineKind, old: bool) {
        let painter = self
            .painter
            .with_clip_rect(self.painter.clip_rect().intersect(rect));
        let Some(line) = line else {
            painter.rect_filled(rect, CornerRadius::ZERO, self.colors.filler);
            return;
        };
        let this = RowPainter {
            painter: &painter,
            ..*self
        };
        this.fill_line(rect, kind);
        this.number(rect.left(), rect, Some(line));
        let (text, words) = if old {
            (self.old_line(line), self.model.old_word_changes(line))
        } else {
            (self.new_line(line), self.model.new_word_changes(line))
        };
        let sign_x = rect.left() + self.number_w;
        this.sign(sign_x, rect, kind);
        this.text(sign_x + self.sign_w, rect, text, words, kind);
    }

    fn old_line(&self, n: usize) -> &str {
        self.model.old_lines().get(n).map_or("", String::as_str)
    }

    fn new_line(&self, n: usize) -> &str {
        self.model.new_lines().get(n).map_or("", String::as_str)
    }

    fn hunk(&self, band: Rect, n: usize) {
        self.painter
            .rect_filled(band, CornerRadius::ZERO, self.colors.hunk_background);
        if let Some(color) = self.colors.outline {
            self.painter.hline(
                band.x_range(),
                band.top() + stroke_width::THIN / 2.0,
                Stroke::new(stroke_width::THIN, color),
            );
        }
        let text = self
            .model
            .hunks()
            .get(n)
            .map(ToString::to_string)
            .unwrap_or_default();
        self.painter.text(
            Pos2::new(band.left() + spacing_horizontal::S, band.center().y),
            Align2::LEFT_CENTER,
            text,
            self.font.clone(),
            self.colors.hunk_text,
        );
    }

    fn fill_line(&self, band: Rect, kind: LineKind) {
        let (fill, bar) = match kind {
            LineKind::Context => return,
            LineKind::Added => (self.colors.added_line, self.colors.added_sign),
            LineKind::Removed => (self.colors.removed_line, self.colors.removed_sign),
        };
        self.painter.rect_filled(band, CornerRadius::ZERO, fill);
        if self.colors.outline.is_some() {
            let bar_rect = Rect::from_min_size(band.min, Vec2::new(stroke_width::THICK, band.height()));
            self.painter.rect_filled(bar_rect, CornerRadius::ZERO, bar);
        }
    }

    fn number(&self, x: f32, band: Rect, line: Option<usize>) {
        if let Some(n) = line {
            self.painter.text(
                Pos2::new(x + self.number_w - spacing_horizontal::S / 2.0, band.center().y),
                Align2::RIGHT_CENTER,
                (n + 1).to_string(),
                self.font.clone(),
                self.colors.gutter,
            );
        }
    }

    fn sign(&self, x: f32, band: Rect, kind: LineKind) {
        let (sign, color) = match kind {
            LineKind::Context => return,
            LineKind::Added => ('+', self.colors.added_sign),
            LineKind::Removed => ('-', self.colors.removed_sign),
        };
        self.painter.text(
            Pos2::new(x + self.cell.x / 2.0, band.center().y),
            Align2::LEFT_CENTER,
            sign,
            self.font.clone(),
            color,
        );
    }

    fn text(&self, x: f32, band: Rect, text: &str, words: &[Range<usize>], kind: LineKind) {
        let word_fill = match kind {
            LineKind::Added => self.colors.added_word,
            LineKind::Removed => self.colors.removed_word,
            LineKind::Context => self.colors.background,
        };
        for r in words {
            let word = Rect::from_min_max(
                Pos2::new(x + r.start as f32 * self.cell.x, band.top()),
                Pos2::new(x + r.end as f32 * self.cell.x, band.bottom()),
            );
            self.painter.rect_filled(word, CornerRadius::ZERO, word_fill);
            if let Some(color) = self.colors.outline {
                self.painter.rect_stroke(
                    word,
                    CornerRadius::ZERO,
                    Stroke::new(stroke_width::THIN, color),
                    StrokeKind::Inside,
                );
            }
        }
        if !text.is_empty() {
            self.painter.text(
                Pos2::new(x, band.center().y),
                Align2::LEFT_CENTER,
                text,
                self.font.clone(),
                self.colors.text,
            );
        }
    }
}

fn finite_pos(v: f32, fallback: f32) -> f32 {
    if v.is_finite() && v > 0.0 { v } else { fallback }
}

fn hashed<T: std::hash::Hash + ?Sized>(value: &T) -> u64 {
    use std::hash::Hasher;
    let mut hasher = std::collections::hash_map::DefaultHasher::new();
    value.hash(&mut hasher);
    hasher.finish()
}
