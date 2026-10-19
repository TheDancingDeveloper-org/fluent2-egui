//! CodeView: read-only, syntax-highlighted source with line numbers, selection and copy
//! (behind the `highlight` cargo feature).
//!
//! [`highlight`] is the shared highlighter (also for diff and markdown views); CodeView paints
//! its output. Only the rows in the viewport are laid out, so long files stay cheap after the
//! first highlight. Editing is out of scope. See `docs/specs/code-view.md`.

mod highlight;

pub use highlight::{Highlighter, SyntaxToken, TokenSpan, highlight, line_job, split_lines, syntax_color};

use egui::accesskit::Role;
use egui::{
    Align2, CornerRadius, Event, EventFilter, FontId, Key, Modifiers, Pos2, Rect, ScrollArea, Sense, Stroke,
    TextStyle, Ui, Vec2, WidgetInfo, WidgetType,
};

use crate::foundation::focus::paint_focus_ring;
use crate::theme::{Theme, c};
use fluent2_tokens::generated::global::{border_radius, spacing_horizontal, stroke_width};

/// Spaces a tab expands to.
pub const TAB_WIDTH: usize = 4;

/// A position in the text: line index and column in chars (after tab expansion).
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct CodePos {
    /// Zero-based line.
    pub line: usize,
    /// Zero-based column, in chars.
    pub col: usize,
}

#[derive(Clone, Debug, Default)]
struct Prepared {
    key: u64,
    lines: Vec<String>,
    /// `lines` joined, for the screen reader; built once per highlight.
    value: String,
    spans: Vec<Vec<TokenSpan>>,
    max_cols: usize,
}

/// View state of a CodeView: the selection and the highlight cache. Store one per view and
/// pass `&mut` to every `show`.
///
/// ```
/// let state = fluent2_egui::data::code::CodeState::default();
/// assert!(state.selection.is_none());
/// ```
#[derive(Clone, Debug, Default)]
pub struct CodeState {
    /// Selected range `(anchor, head)`; the end is exclusive.
    pub selection: Option<(CodePos, CodePos)>,
    prepared: Option<Prepared>,
}

impl CodeState {
    /// Highlight `text` as `language` unless the last call had the same input. With a
    /// `revision` the key is `(language, revision, len)` and the text is not hashed.
    fn prepare(&mut self, language: &str, text: &str, revision: Option<u64>) {
        let key = match revision {
            Some(rev) => hashed(&(language, rev, text.len())),
            None => hashed(&(language, text)),
        };
        if self.prepared.as_ref().is_none_or(|p| p.key != key) {
            let lines: Vec<String> = split_lines(text).map(expand_tabs).collect();
            let value = lines.join("\n");
            let spans = highlight(language, &value);
            let max_cols = lines.iter().map(|l| l.chars().count()).max().unwrap_or(0);
            self.selection = None;
            self.prepared = Some(Prepared {
                key,
                lines,
                value,
                spans,
                max_cols,
            });
        }
    }

    /// The selected text (tabs expanded), if any.
    #[must_use]
    pub fn selected_text(&self) -> Option<String> {
        selected_text(self.selection?, &self.prepared.as_ref()?.lines)
    }
}

fn selected_text((a, b): (CodePos, CodePos), lines: &[String]) -> Option<String> {
    let (a, b) = if a <= b { (a, b) } else { (b, a) };
    let mut out = String::new();
    for line in a.line..=b.line.min(lines.len().saturating_sub(1)) {
        let text = lines.get(line)?;
        let from = if line == a.line { a.col } else { 0 };
        let to = if line == b.line { b.col } else { usize::MAX };
        out.extend(text.chars().skip(from).take(to.saturating_sub(from)));
        if line != b.line {
            out.push('\n');
        }
    }
    Some(out)
}

fn expand_tabs(line: &str) -> String {
    if !line.contains('\t') {
        return line.to_owned();
    }
    let mut out = String::with_capacity(line.len() + TAB_WIDTH);
    let mut col = 0;
    for ch in line.chars() {
        if ch == '\t' {
            let n = TAB_WIDTH - col % TAB_WIDTH;
            out.extend(std::iter::repeat_n(' ', n));
            col += n;
        } else {
            out.push(ch);
            col += 1;
        }
    }
    out
}

/// What a CodeView reported this frame.
#[derive(Debug)]
pub struct CodeResponse {
    /// The response of the text area.
    pub response: egui::Response,
    /// Text copied to the clipboard this frame (Copy event with a selection).
    pub copied: Option<String>,
}

/// A read-only highlighted code view.
///
/// ```
/// use fluent2_egui::data::code::{CodeState, CodeView};
///
/// # fn demo(ui: &mut egui::Ui, state: &mut CodeState) {
/// CodeView::new("main.rs").language("rust").show(ui, "fn main() {}\n", state);
/// # }
/// ```
#[derive(Clone, Debug)]
pub struct CodeView {
    id_salt: egui::Id,
    language: String,
    font: Option<FontId>,
    label: String,
    line_numbers: bool,
    revision: Option<u64>,
}

impl CodeView {
    /// A view whose id is derived from `id_salt` and the parent `Ui`.
    pub fn new(id_salt: impl std::hash::Hash) -> Self {
        Self {
            id_salt: egui::Id::new(hashed(&id_salt)),
            language: String::new(),
            font: None,
            label: "Code".into(),
            line_numbers: true,
            revision: None,
        }
    }

    /// A number the app changes whenever the text changes (an edit counter or a buffer
    /// generation). With it, the view re-highlights when the revision or the length changes and
    /// never hashes the text. Without it, the text is hashed each frame to detect changes,
    /// which costs O(length).
    #[must_use]
    pub fn revision(mut self, revision: u64) -> Self {
        self.revision = Some(revision);
        self
    }

    /// Language name or file extension (`rust`, `py`, `ts`, `json`, `yaml`, `diff`). Unknown or
    /// empty means plain text.
    #[must_use]
    pub fn language(mut self, language: impl Into<String>) -> Self {
        self.language = language.into();
        self
    }

    /// Monospace font. Defaults to the `Monospace` text style.
    #[must_use]
    pub fn font(mut self, font: FontId) -> Self {
        self.font = Some(font);
        self
    }

    /// Accessible name. Defaults to "Code".
    #[must_use]
    pub fn label(mut self, label: impl Into<String>) -> Self {
        self.label = label.into();
        self
    }

    /// Show the line-number gutter. Default on.
    #[must_use]
    pub fn line_numbers(mut self, on: bool) -> Self {
        self.line_numbers = on;
        self
    }

    /// Paint `text` into the available space, scrolling both ways.
    pub fn show(self, ui: &mut Ui, text: &str, state: &mut CodeState) -> CodeResponse {
        let theme = Theme::get(ui.ctx());
        let font = self
            .font
            .clone()
            .unwrap_or_else(|| TextStyle::Monospace.resolve(ui.style()));
        let (cw, ch) = ui.fonts_mut(|f| (f.glyph_width(&font, 'M'), f.row_height(&font)));
        let cell = Vec2::new(finite_pos(cw, 8.0), finite_pos(ch, 16.0));
        let pad = spacing_horizontal::S;
        let id = ui.make_persistent_id(self.id_salt);

        state.prepare(&self.language, text, self.revision);
        let prepared = state.prepared.take().unwrap_or_default();
        let line_count = prepared.lines.len();
        let gutter = if self.line_numbers {
            let digits = line_count.max(1).to_string().len() as f32;
            digits * cell.x + 2.0 * pad
        } else {
            0.0
        };

        let outer = ui.available_rect_before_wrap();
        let outer = Rect::from_min_size(
            outer.min,
            Vec2::new(
                finite_pos(outer.width(), 400.0),
                finite_pos(outer.height(), 300.0),
            ),
        );
        let radius = CornerRadius::same(border_radius::MEDIUM as u8);
        ui.painter()
            .rect_filled(outer, radius, c(theme.color.neutral_background_1));
        ui.painter().rect_stroke(
            outer,
            radius,
            Stroke::new(stroke_width::THIN, c(theme.color.neutral_stroke_2)),
            egui::StrokeKind::Inside,
        );

        let mut copied = None;
        let mut child = ui.new_child(egui::UiBuilder::new().max_rect(outer.shrink(stroke_width::THIN)));
        let scroll = ScrollArea::both()
            .id_salt(id.with("scroll"))
            .auto_shrink([false, false])
            .show_viewport(&mut child, |ui, viewport| {
                let content = Vec2::new(
                    gutter + pad + prepared.max_cols as f32 * cell.x + pad,
                    line_count as f32 * cell.y + pad,
                );
                let (rect, _) = ui.allocate_exact_size(content.max(ui.available_size()), Sense::hover());
                let response = ui.interact(rect, id, Sense::click_and_drag());
                let text_left = rect.left() + gutter + pad;
                let top = rect.top() + pad / 2.0;

                let pos_at = |p: Pos2| -> CodePos {
                    let row = ((p.y - top) / cell.y).floor();
                    let line = if row.is_finite() && row > 0.0 {
                        row as usize
                    } else {
                        0
                    };
                    let line = line.min(line_count.saturating_sub(1));
                    let col = ((p.x - text_left) / cell.x).round();
                    let col = if col.is_finite() && col > 0.0 {
                        col as usize
                    } else {
                        0
                    };
                    let len = prepared.lines.get(line).map_or(0, |l| l.chars().count());
                    CodePos {
                        line,
                        col: col.min(len),
                    }
                };

                if response.clicked() || response.drag_started() {
                    response.request_focus();
                }
                if let Some(p) = response.interact_pointer_pos() {
                    let at = pos_at(p);
                    if response.drag_started() {
                        let origin = ui.input(|i| i.pointer.press_origin()).map_or(at, pos_at);
                        state.selection = Some((origin, at));
                    } else if response.dragged()
                        && let Some((anchor, _)) = state.selection
                    {
                        state.selection = Some((anchor, at));
                    } else if response.clicked() {
                        state.selection = None;
                    }
                }
                if response.hovered() {
                    ui.ctx().set_cursor_icon(egui::CursorIcon::Text);
                }

                if response.has_focus() {
                    ui.memory_mut(|m| {
                        m.set_focus_lock_filter(
                            id,
                            EventFilter {
                                tab: false,
                                horizontal_arrows: false,
                                vertical_arrows: false,
                                escape: true,
                            },
                        )
                    });
                    let (all, escape, copy) = ui.input_mut(|i| {
                        (
                            i.consume_key(Modifiers::COMMAND, Key::A),
                            i.consume_key(Modifiers::NONE, Key::Escape),
                            i.events.iter().any(|e| matches!(e, Event::Copy)),
                        )
                    });
                    if all && line_count > 0 {
                        let last = line_count - 1;
                        let len = prepared.lines.get(last).map_or(0, |l| l.chars().count());
                        state.selection = Some((CodePos::default(), CodePos { line: last, col: len }));
                    }
                    if escape {
                        state.selection = None;
                    }
                    if copy
                        && let Some(t) = state
                            .selection
                            .and_then(|s| selected_text(s, &prepared.lines))
                            .filter(|t| !t.is_empty())
                    {
                        ui.ctx().copy_text(t.clone());
                        copied = Some(t);
                    }
                }

                // Paint the visible rows only.
                let first = ((viewport.min.y - pad / 2.0) / cell.y).floor().max(0.0) as usize;
                let last = (((viewport.max.y - pad / 2.0) / cell.y).ceil().max(0.0) as usize).min(line_count);
                let painter = ui.painter_at(ui.clip_rect());
                let select_bg = c(theme.color.brand_background_2);
                let number_fg = c(theme.color.neutral_foreground_3);
                let selection = state.selection.map(|(a, b)| if a <= b { (a, b) } else { (b, a) });
                for line in first..last {
                    let y = top + line as f32 * cell.y;
                    let Some(text) = prepared.lines.get(line) else {
                        continue;
                    };
                    if let Some((a, b)) = selection
                        && a.line <= line
                        && line <= b.line
                        && a != b
                    {
                        let len = text.chars().count();
                        let from = if line == a.line { a.col } else { 0 };
                        let to = if line == b.line { b.col } else { len + 1 };
                        if to > from {
                            let r = Rect::from_min_size(
                                Pos2::new(text_left + from as f32 * cell.x, y),
                                Vec2::new((to - from) as f32 * cell.x, cell.y),
                            );
                            painter.rect_filled(r, CornerRadius::ZERO, select_bg);
                        }
                    }
                    if self.line_numbers {
                        painter.text(
                            Pos2::new(rect.left() + gutter - pad, y),
                            Align2::RIGHT_TOP,
                            (line + 1).to_string(),
                            font.clone(),
                            number_fg,
                        );
                    }
                    let spans = prepared.spans.get(line).map_or(&[][..], Vec::as_slice);
                    let job = line_job(text, spans, font.clone(), &theme);
                    let galley = ui.fonts_mut(|f| f.layout_job(job));
                    painter.galley(Pos2::new(text_left, y), galley, number_fg);
                }
                if self.line_numbers {
                    let x = rect.left() + gutter;
                    painter.vline(
                        x,
                        viewport.y_range(),
                        Stroke::new(stroke_width::THIN, c(theme.color.neutral_stroke_3)),
                    );
                }
                response
            });
        let response = scroll.inner;
        ui.advance_cursor_after_rect(outer);

        if response.has_focus() {
            paint_focus_ring(ui, outer, radius);
        }

        let enabled = ui.is_enabled();
        let label = self.label.clone();
        response.widget_info(|| WidgetInfo::labeled(WidgetType::TextEdit, enabled, &label));
        // The builder runs only while AccessKit is on; the value was joined at highlight time.
        ui.ctx().accesskit_node_builder(id, |node| {
            node.set_role(Role::MultilineTextInput);
            node.set_read_only();
            node.set_label(self.label.clone());
            node.set_value(prepared.value.clone());
        });
        state.prepared = Some(prepared);

        CodeResponse { response, copied }
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
