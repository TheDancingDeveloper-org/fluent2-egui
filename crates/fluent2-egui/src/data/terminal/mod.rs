//! TerminalView: paints a [`TerminalGrid`] with theme colours, scrollback, selection, search
//! highlights and links (behind the `terminal` cargo feature).
//!
//! The grid model (`grid.rs`) is adapted from rdpapp's `vte` terminal. The view only paints and
//! handles pointer selection, wheel scrolling and copy. Keyboard input to the program and the
//! PTY or SSH transport stay with the app: it feeds output to [`TerminalGrid::process`], sends
//! keys itself, and resizes the grid and PTY to [`TerminalResponse::fit`].

mod grid;

pub use grid::{Cell, DEFAULT_SCROLLBACK, GridPos, Span, TermColor, TerminalGrid, color256};

use egui::accesskit::Role;
use egui::{
    Color32, CornerRadius, Event, EventFilter, FontId, Key, Modifiers, Pos2, Rect, Sense, Stroke, TextStyle,
    Ui, Vec2, WidgetInfo, WidgetType,
};

use crate::foundation::focus::paint_focus_ring;
use crate::theme::{Theme, c};
use fluent2_tokens::generated::global::{border_radius, spacing_horizontal, stroke_width};

/// The 16 ANSI colours for a theme, taken from Fluent palette tokens: 0 to 7 normal, 8 to 15
/// bright.
///
/// ```
/// use fluent2_egui::Theme;
/// let palette = fluent2_egui::data::terminal::ansi_palette(&Theme::default());
/// assert_ne!(palette[1], palette[2]);
/// ```
#[must_use]
pub fn ansi_palette(theme: &Theme) -> [Color32; 16] {
    let t = &theme.color;
    [
        c(t.neutral_foreground_3),
        c(t.palette_red_foreground_1),
        c(t.palette_green_foreground_1),
        c(t.palette_marigold_foreground_1),
        c(t.palette_blue_foreground_2),
        c(t.palette_berry_foreground_1),
        c(t.palette_teal_foreground_2),
        c(t.neutral_foreground_2),
        c(t.neutral_foreground_4),
        c(t.palette_red_foreground_3),
        c(t.palette_green_foreground_3),
        c(t.palette_yellow_foreground_1),
        c(t.palette_cornflower_foreground_2),
        c(t.palette_berry_foreground_3),
        c(t.palette_light_teal_foreground_2),
        c(t.neutral_foreground_1),
    ]
}

fn resolve(color: TermColor, default: Color32, palette: &[Color32; 16]) -> Color32 {
    match color {
        TermColor::Default => default,
        TermColor::Indexed(n) if n < 16 => palette.get(usize::from(n)).copied().unwrap_or(default),
        TermColor::Indexed(n) => {
            let (r, g, b) = color256(n);
            Color32::from_rgb(r, g, b)
        }
        TermColor::Rgb(r, g, b) => Color32::from_rgb(r, g, b),
    }
}

/// View state of a TerminalView: selection, search and wheel remainder. Store it beside the
/// grid and pass `&mut` to every `show`.
///
/// ```
/// let state = fluent2_egui::data::terminal::TerminalState::default();
/// assert!(state.selection.is_none());
/// ```
#[derive(Clone, Debug, Default)]
pub struct TerminalState {
    /// Selected range `(anchor, head)`, inclusive, in stable line numbers.
    pub selection: Option<(GridPos, GridPos)>,
    /// Search query. Matches are highlighted; [`Self::find_next`] moves between them.
    pub search: String,
    /// Index into the matches of the current search match.
    pub current_match: Option<usize>,
    scroll_remainder: f32,
    cache: Option<(String, u64, Vec<Span>)>,
}

impl TerminalState {
    /// Matches of [`Self::search`] in `grid`, cached until the query or content changes.
    pub fn matches(&mut self, grid: &TerminalGrid) -> &[Span] {
        let fresh = matches!(&self.cache, Some((q, g, _)) if *q == self.search && *g == grid.generation());
        if !fresh {
            self.cache = Some((self.search.clone(), grid.generation(), grid.find(&self.search)));
            self.current_match = None;
        }
        self.cache.as_ref().map_or(&[], |(_, _, m)| m.as_slice())
    }

    /// Move to the next match (`forward`) or the previous one, wrapping, and scroll `grid` so it
    /// is in view. Returns the match, or `None` when nothing matches.
    pub fn find_next(&mut self, grid: &mut TerminalGrid, forward: bool) -> Option<Span> {
        let count = self.matches(grid).len();
        if count == 0 {
            return None;
        }
        let next = match (self.current_match, forward) {
            (None, true) => 0,
            (None, false) => count - 1,
            (Some(i), true) => (i + 1) % count,
            (Some(i), false) => (i + count - 1) % count,
        };
        self.current_match = Some(next);
        let span = self.matches(grid).get(next).cloned()?;
        reveal(grid, span.line);
        Some(span)
    }

    /// The selected text, if any.
    #[must_use]
    pub fn selected_text(&self, grid: &TerminalGrid) -> Option<String> {
        self.selection.map(|(a, b)| grid.text_between(a, b))
    }
}

/// Scroll so `line` is in view.
fn reveal(grid: &mut TerminalGrid, line: u64) {
    let top = grid.view_top();
    let rows = grid.rows() as u64;
    let target_top = if line < top {
        line
    } else if line >= top + rows {
        line + 1 - rows
    } else {
        return;
    };
    let delta = i64::try_from(top).unwrap_or(0) - i64::try_from(target_top).unwrap_or(0);
    grid.scroll_by(isize::try_from(delta).unwrap_or(0));
}

/// What a TerminalView reported this frame.
#[derive(Debug)]
pub struct TerminalResponse {
    /// The response of the whole terminal area.
    pub response: egui::Response,
    /// `(cols, rows)` that fit the area. Resize the grid and the PTY when it differs.
    pub fit: (usize, usize),
    /// A link the user clicked (Ctrl+click, or a plain click when [`TerminalView::click_links`]
    /// is on).
    pub link_clicked: Option<String>,
    /// Text copied to the clipboard this frame (Copy event with a selection).
    pub copied: Option<String>,
}

/// A terminal grid view.
///
/// ```
/// use fluent2_egui::data::terminal::{TerminalGrid, TerminalState, TerminalView};
///
/// # fn demo(ui: &mut egui::Ui, grid: &mut TerminalGrid, state: &mut TerminalState) {
/// let response = TerminalView::new("shell").show(ui, grid, state);
/// if response.fit != (grid.cols(), grid.rows()) {
///     grid.resize(response.fit.0, response.fit.1);
///     // and resize the PTY
/// }
/// # }
/// ```
#[derive(Clone, Debug)]
pub struct TerminalView {
    id_salt: egui::Id,
    font: Option<FontId>,
    label: String,
    click_links: bool,
    min_size: (usize, usize),
}

impl TerminalView {
    /// A view whose id is derived from `id_salt` and the parent `Ui`.
    pub fn new(id_salt: impl std::hash::Hash) -> Self {
        Self {
            id_salt: egui::Id::new(hashed(&id_salt)),
            font: None,
            label: "Terminal".into(),
            click_links: false,
            min_size: (2, 1),
        }
    }

    /// Monospace font. Defaults to the `Monospace` text style.
    #[must_use]
    pub fn font(mut self, font: FontId) -> Self {
        self.font = Some(font);
        self
    }

    /// Accessible name. Defaults to "Terminal".
    #[must_use]
    pub fn label(mut self, label: impl Into<String>) -> Self {
        self.label = label.into();
        self
    }

    /// Open links on a plain click instead of Ctrl+click.
    #[must_use]
    pub fn click_links(mut self, on: bool) -> Self {
        self.click_links = on;
        self
    }

    /// Smallest `(cols, rows)` reported in [`TerminalResponse::fit`]. Default `(2, 1)`.
    #[must_use]
    pub fn min_size(mut self, cols: usize, rows: usize) -> Self {
        self.min_size = (cols.max(1), rows.max(1));
        self
    }

    /// Paint `grid` into the available space.
    pub fn show(self, ui: &mut Ui, grid: &mut TerminalGrid, state: &mut TerminalState) -> TerminalResponse {
        let theme = Theme::get(ui.ctx());
        let font = self
            .font
            .clone()
            .unwrap_or_else(|| TextStyle::Monospace.resolve(ui.style()));
        let (cell_w, cell_h) = ui.fonts_mut(|f| (f.glyph_width(&font, 'M'), f.row_height(&font)));
        let cell = Vec2::new(finite_pos(cell_w, 8.0), finite_pos(cell_h, 16.0));
        let pad = spacing_horizontal::XS;

        let id = ui.make_persistent_id(self.id_salt);
        let size = ui.available_size();
        let size = Vec2::new(
            finite_pos(size.x, cell.x * 80.0),
            finite_pos(size.y, cell.y * 24.0),
        );
        let (_, rect) = ui.allocate_space(size);
        let response = ui.interact(rect, id, Sense::click_and_drag());
        let inner = rect.shrink(pad);
        let fit = (
            ((inner.width() / cell.x).floor().max(0.0) as usize).max(self.min_size.0),
            ((inner.height() / cell.y).floor().max(0.0) as usize).max(self.min_size.1),
        );

        if response.clicked() || response.drag_started() {
            response.request_focus();
        }

        let mut out = TerminalResponse {
            response: response.clone(),
            fit,
            link_clicked: None,
            copied: None,
        };

        // Wheel and keyboard scrollback.
        if response.hovered() {
            let dy = ui.input(|i| i.smooth_scroll_delta.y);
            if dy.is_finite() && dy != 0.0 {
                state.scroll_remainder += dy / cell.y;
                let whole = state.scroll_remainder.trunc();
                state.scroll_remainder -= whole;
                grid.scroll_by(whole as isize);
            }
        }
        if response.has_focus() {
            ui.memory_mut(|m| {
                m.set_focus_lock_filter(
                    id,
                    EventFilter {
                        tab: false,
                        horizontal_arrows: true,
                        vertical_arrows: true,
                        escape: false,
                    },
                )
            });
            let page = grid.rows().saturating_sub(1).max(1) as isize;
            let (up, down, copy) = ui.input_mut(|i| {
                let up = i.consume_key(Modifiers::SHIFT, Key::PageUp);
                let down = i.consume_key(Modifiers::SHIFT, Key::PageDown);
                let copy = i.events.iter().any(|e| matches!(e, Event::Copy));
                (up, down, copy)
            });
            if up {
                grid.scroll_by(page);
            }
            if down {
                grid.scroll_by(-page);
            }
            if copy && let Some(text) = state.selected_text(grid).filter(|t| !t.is_empty()) {
                ui.ctx().copy_text(text.clone());
                out.copied = Some(text);
            }
        }

        // Pointer: selection and links.
        let view_top = grid.view_top();
        let pos_at = |p: Pos2| -> GridPos {
            let col = ((p.x - inner.left()) / cell.x).floor();
            let row = ((p.y - inner.top()) / cell.y).floor();
            let col = if col.is_finite() {
                col.clamp(0.0, (grid.cols().saturating_sub(1)) as f32) as usize
            } else {
                0
            };
            let row = if row.is_finite() {
                row.clamp(0.0, (grid.rows().saturating_sub(1)) as f32) as u64
            } else {
                0
            };
            GridPos {
                line: view_top + row,
                col,
            }
        };
        if let Some(p) = response.interact_pointer_pos().or_else(|| response.hover_pos()) {
            let at = pos_at(p);
            if response.drag_started() {
                let origin = ui.input(|i| i.pointer.press_origin()).map_or(at, pos_at);
                state.selection = Some((origin, at));
            } else if response.dragged()
                && let Some((anchor, _)) = state.selection
            {
                state.selection = Some((anchor, at));
            }
            if response.clicked() {
                // The release event carries the modifiers held at click time.
                let ctrl = ui.input(|i| {
                    i.modifiers.command
                        || i.events.iter().any(
                            |e| matches!(e, Event::PointerButton { pressed: false, modifiers, .. } if modifiers.command),
                        )
                });
                let link = grid
                    .links(at.line)
                    .into_iter()
                    .find(|(s, _)| (s.start..s.end).contains(&at.col));
                match link {
                    Some((_, url)) if ctrl || self.click_links => out.link_clicked = Some(url),
                    _ => state.selection = None,
                }
            }
        }
        if let Some(hover) = response.hover_pos() {
            let at = pos_at(hover);
            if grid
                .links(at.line)
                .iter()
                .any(|(s, _)| (s.start..s.end).contains(&at.col))
            {
                ui.ctx().set_cursor_icon(egui::CursorIcon::PointingHand);
            } else {
                ui.ctx().set_cursor_icon(egui::CursorIcon::Text);
            }
        }

        // Paint.
        let palette = ansi_palette(&theme);
        let fg_default = c(theme.color.neutral_foreground_1);
        let bg_default = c(theme.color.neutral_background_1);
        let select_bg = c(theme.color.brand_background_2);
        let match_bg = c(theme.color.palette_yellow_background_2);
        let current_bg = c(theme.color.palette_marigold_background_3);
        let link_fg = c(theme.color.brand_foreground_link);
        let cursor_color = c(theme.color.neutral_foreground_1);
        let radius = CornerRadius::same(border_radius::MEDIUM as u8);

        let selection = state.selection.map(|(a, b)| if a <= b { (a, b) } else { (b, a) });
        let current = state.current_match;
        let matches: Vec<(usize, Span)> = state
            .matches(grid)
            .iter()
            .enumerate()
            .filter(|(_, s)| s.line >= view_top && s.line < view_top + grid.rows() as u64)
            .map(|(i, s)| (i, s.clone()))
            .collect();

        let painter = ui.painter_at(rect);
        painter.rect_filled(rect, radius, bg_default);
        for row in 0..grid.rows() {
            let line = view_top + row as u64;
            let Some(cells) = grid.line(line) else { continue };
            let links = grid.links(line);
            let y = inner.top() + row as f32 * cell.y;
            for (col, cell_data) in cells.iter().enumerate() {
                let x = inner.left() + col as f32 * cell.x;
                let cell_rect = Rect::from_min_size(Pos2::new(x, y), cell);
                if cell_rect.left() >= inner.right() {
                    break;
                }
                let (mut fg, mut bg) = (
                    resolve(cell_data.fg, fg_default, &palette),
                    resolve(cell_data.bg, bg_default, &palette),
                );
                if cell_data.inverse {
                    std::mem::swap(&mut fg, &mut bg);
                }
                let pos = GridPos { line, col };
                let selected = selection.is_some_and(|(a, b)| a <= pos && pos <= b);
                let matched = matches
                    .iter()
                    .find(|(_, s)| s.line == line && (s.start..s.end).contains(&col));
                let in_link = links.iter().any(|(s, _)| (s.start..s.end).contains(&col));
                let fill = if selected {
                    Some(select_bg)
                } else if let Some((i, _)) = matched {
                    Some(if Some(*i) == current { current_bg } else { match_bg })
                } else if bg != bg_default {
                    Some(bg)
                } else {
                    None
                };
                if let Some(fill) = fill {
                    painter.rect_filled(cell_rect, CornerRadius::ZERO, fill);
                    if selected || matched.is_some() {
                        fg = fg_default;
                    }
                }
                if in_link && cell_data.fg == TermColor::Default && !selected {
                    fg = link_fg;
                }
                if !cell_data.spacer && cell_data.ch != ' ' {
                    painter.text(
                        cell_rect.left_top(),
                        egui::Align2::LEFT_TOP,
                        cell_data.ch,
                        font.clone(),
                        fg,
                    );
                }
                if cell_data.underline || in_link {
                    let uy = cell_rect.bottom() - stroke_width::THIN;
                    painter.hline(cell_rect.x_range(), uy, Stroke::new(stroke_width::THIN, fg));
                }
            }
        }

        // Cursor, only on the live screen.
        if grid.cursor_visible() && grid.scroll_offset() == 0 {
            let (row, col) = grid.cursor();
            let r = Rect::from_min_size(
                Pos2::new(
                    inner.left() + col as f32 * cell.x,
                    inner.top() + row as f32 * cell.y,
                ),
                cell,
            );
            if response.has_focus() {
                painter.rect_filled(r, CornerRadius::ZERO, cursor_color.gamma_multiply(0.6));
            } else {
                painter.rect_stroke(
                    r,
                    CornerRadius::ZERO,
                    Stroke::new(stroke_width::THIN, cursor_color),
                    egui::StrokeKind::Inside,
                );
            }
        }

        if response.has_focus() {
            paint_focus_ring(ui, rect, radius);
        }

        let enabled = ui.is_enabled();
        let label = self.label.clone();
        response.widget_info(|| WidgetInfo::labeled(WidgetType::Other, enabled, &label));
        let text = grid.view_text();
        ui.ctx().accesskit_node_builder(id, |node| {
            node.set_role(Role::Terminal);
            node.set_label(self.label.clone());
            node.set_value(text);
        });

        out
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
