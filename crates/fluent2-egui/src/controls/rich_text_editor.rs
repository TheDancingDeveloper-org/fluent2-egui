//! Fluent RichTextEditor: a formatting toolbar over egui's multiline [`TextEdit`].
//!
//! Text editing is egui's. The document ([`RichText`]) keeps one [`CharStyle`] per character
//! and one [`BlockKind`] per line, and a [`TextEdit`] layouter paints those styles. After each
//! edit the styles are spliced around the changed span, so typed characters take the current
//! typing style. Lists and quotes are painted in a left gutter: a bullet, a number or a bar.
//!
//! [`RichText::to_html`] writes a small HTML subset (`p`, `ul`/`ol`/`li`, `blockquote`,
//! `strong`, `em`, `u`, `a`) for mail; [`RichText::to_plain`] writes the plain-text alternative.

use egui::text::{CCursor, CCursorRange, LayoutJob, TextFormat};
use egui::text_edit::TextEditState;
use egui::{
    Align2, Color32, CornerRadius, FontFamily, FontId, Id, Key, KeyboardShortcut, Modifiers, Rect, Response,
    Stroke, TextEdit, Ui, Widget,
};

use super::{Appearance, Button, ButtonSize, ToggleButton};
use crate::foundation::focus::{focus_visible, paint_focus_ring};
use crate::foundation::icon::IconName;
use crate::foundation::text::{SEMIBOLD_FAMILY, TypeRamp, font_id};
use crate::theme::{Theme, c};
use fluent2_tokens::generated::global::{border_radius, spacing_horizontal, spacing_vertical, stroke_width};

/// Inline formatting of one character.
///
/// ```
/// # use fluent2_egui::controls::CharStyle;
/// let style = CharStyle { bold: true, ..CharStyle::default() };
/// assert!(!style.italic);
/// ```
#[derive(Clone, Debug, Default, Eq, Hash, PartialEq)]
pub struct CharStyle {
    /// Semibold weight (`<strong>`).
    pub bold: bool,
    /// Italic (`<em>`).
    pub italic: bool,
    /// Underlined (`<u>`).
    pub underline: bool,
    /// Link target (`<a href>`).
    pub link: Option<String>,
}

/// Kind of one line (paragraph) of a [`RichText`].
///
/// ```
/// # use fluent2_egui::controls::BlockKind;
/// assert_eq!(BlockKind::default(), BlockKind::Paragraph);
/// ```
#[derive(Clone, Copy, Debug, Default, Eq, Hash, PartialEq)]
pub enum BlockKind {
    /// Plain paragraph. The default.
    #[default]
    Paragraph,
    /// Item of a bulleted list.
    Bulleted,
    /// Item of a numbered list. Consecutive items count up from 1.
    Numbered,
    /// Line of a block quote.
    Quote,
}

/// An inline format the toolbar toggles.
///
/// ```
/// # use fluent2_egui::controls::{InlineFormat, RichText};
/// let mut doc = RichText::from_plain("Hello");
/// doc.toggle_inline(0..5, InlineFormat::Bold);
/// assert_eq!(doc.to_html(), "<p><strong>Hello</strong></p>");
/// ```
#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
pub enum InlineFormat {
    /// [`CharStyle::bold`].
    Bold,
    /// [`CharStyle::italic`].
    Italic,
    /// [`CharStyle::underline`].
    Underline,
}

impl InlineFormat {
    fn get(self, style: &CharStyle) -> bool {
        match self {
            Self::Bold => style.bold,
            Self::Italic => style.italic,
            Self::Underline => style.underline,
        }
    }

    fn set(self, style: &mut CharStyle, on: bool) {
        match self {
            Self::Bold => style.bold = on,
            Self::Italic => style.italic = on,
            Self::Underline => style.underline = on,
        }
    }
}

/// A rich-text document: text, one [`CharStyle`] per character and one [`BlockKind`] per line.
///
/// Ranges are in characters (not bytes) and are clamped to the text.
///
/// ```
/// # use fluent2_egui::controls::{BlockKind, RichText};
/// let mut doc = RichText::from_plain("Milk\nEggs");
/// doc.set_block(0..9, BlockKind::Bulleted);
/// assert_eq!(doc.to_html(), "<ul><li>Milk</li><li>Eggs</li></ul>");
/// assert_eq!(doc.to_plain(), "- Milk\n- Eggs");
/// ```
#[derive(Clone, Debug, Default, Eq, PartialEq)]
pub struct RichText {
    text: String,
    styles: Vec<CharStyle>,
    blocks: Vec<BlockKind>,
}

impl RichText {
    /// An empty document.
    pub fn new() -> Self {
        Self::from_plain("")
    }

    /// Unformatted paragraphs, one per line.
    pub fn from_plain(text: &str) -> Self {
        let text = text.replace("\r\n", "\n");
        let chars = text.chars().count();
        let lines = text.matches('\n').count() + 1;
        Self {
            text,
            styles: vec![CharStyle::default(); chars],
            blocks: vec![BlockKind::Paragraph; lines],
        }
    }

    /// The text, lines separated by `\n`.
    pub fn text(&self) -> &str {
        &self.text
    }

    /// Whether the document has no text.
    pub fn is_empty(&self) -> bool {
        self.text.is_empty()
    }

    /// Number of characters.
    pub fn len_chars(&self) -> usize {
        self.styles.len()
    }

    /// Style of the character at `index`, or `None` past the end.
    pub fn style_at(&self, index: usize) -> Option<&CharStyle> {
        self.styles.get(index)
    }

    /// Kind of each line, in order.
    pub fn blocks(&self) -> &[BlockKind] {
        &self.blocks
    }

    fn clamp(&self, range: std::ops::Range<usize>) -> std::ops::Range<usize> {
        let len = self.styles.len();
        let start = range.start.min(len);
        start..range.end.clamp(start, len)
    }

    /// Whether every character in `range` has `format` on. False for an empty range.
    pub fn has_inline(&self, range: std::ops::Range<usize>, format: InlineFormat) -> bool {
        let range = self.clamp(range);
        let slice = self.styles.get(range).unwrap_or(&[]);
        !slice.is_empty() && slice.iter().all(|style| format.get(style))
    }

    /// Turn `format` on over `range`, or off if it was already on everywhere in it.
    pub fn toggle_inline(&mut self, range: std::ops::Range<usize>, format: InlineFormat) {
        let on = !self.has_inline(range.clone(), format);
        let range = self.clamp(range);
        for style in self.styles.get_mut(range).into_iter().flatten() {
            format.set(style, on);
        }
    }

    /// Link `range` to `url`, or remove links from it with `None`.
    pub fn set_link(&mut self, range: std::ops::Range<usize>, url: Option<&str>) {
        let range = self.clamp(range);
        let url = url.map(str::trim).filter(|url| !url.is_empty());
        for style in self.styles.get_mut(range).into_iter().flatten() {
            style.link = url.map(str::to_owned);
        }
    }

    /// The link shared by every character in `range`, if they all have the same one.
    pub fn link_in(&self, range: std::ops::Range<usize>) -> Option<&str> {
        let range = self.clamp(range);
        let slice = self.styles.get(range)?;
        let first = slice.first()?.link.as_deref()?;
        slice
            .iter()
            .all(|style| style.link.as_deref() == Some(first))
            .then_some(first)
    }

    /// Line index of the character at `index` (the end of the text maps to the last line).
    pub fn line_of(&self, index: usize) -> usize {
        self.text.chars().take(index).filter(|&ch| ch == '\n').count()
    }

    fn lines_of(&self, range: std::ops::Range<usize>) -> std::ops::Range<usize> {
        let range = self.clamp(range);
        self.line_of(range.start)..self.line_of(range.end) + 1
    }

    /// Whether every line touched by `range` has `kind`.
    pub fn has_block(&self, range: std::ops::Range<usize>, kind: BlockKind) -> bool {
        let lines = self.lines_of(range);
        self.blocks
            .get(lines)
            .is_some_and(|blocks| blocks.iter().all(|&block| block == kind))
    }

    /// Set every line touched by `range` to `kind`.
    pub fn set_block(&mut self, range: std::ops::Range<usize>, kind: BlockKind) {
        let lines = self.lines_of(range);
        for block in self.blocks.get_mut(lines).into_iter().flatten() {
            *block = kind;
        }
    }

    /// Set the lines touched by `range` to `kind`, or back to paragraphs if they all were.
    pub fn toggle_block(&mut self, range: std::ops::Range<usize>, kind: BlockKind) {
        let kind = if self.has_block(range.clone(), kind) {
            BlockKind::Paragraph
        } else {
            kind
        };
        self.set_block(range, kind);
    }

    /// Replace the text with `new`, keeping the styles of unchanged characters.
    ///
    /// The changed span is found by common prefix and suffix. Inserted characters take
    /// `typing`; inserted lines take the kind of the line the edit starts on.
    pub fn replace_text(&mut self, new: &str, typing: &CharStyle) {
        let new = new.replace("\r\n", "\n");
        if new == self.text {
            return;
        }
        let old: Vec<char> = self.text.chars().collect();
        let fresh: Vec<char> = new.chars().collect();
        let prefix = old.iter().zip(&fresh).take_while(|(a, b)| a == b).count();
        let max_suffix = old.len().min(fresh.len()) - prefix;
        let suffix = old
            .iter()
            .rev()
            .zip(fresh.iter().rev())
            .take(max_suffix)
            .take_while(|(a, b)| a == b)
            .count();
        let removed = old.get(prefix..old.len() - suffix).unwrap_or(&[]);
        let inserted = fresh.get(prefix..fresh.len() - suffix).unwrap_or(&[]);

        let line = old.iter().take(prefix).filter(|&&ch| ch == '\n').count();
        let removed_lines = removed.iter().filter(|&&ch| ch == '\n').count();
        let inserted_lines = inserted.iter().filter(|&&ch| ch == '\n').count();
        let kind = self.blocks.get(line).copied().unwrap_or_default();

        self.styles.splice(
            prefix.min(self.styles.len())..(old.len() - suffix).min(self.styles.len()),
            std::iter::repeat_n(typing.clone(), inserted.len()),
        );
        let start = (line + 1).min(self.blocks.len());
        let end = (line + 1 + removed_lines).min(self.blocks.len());
        self.blocks
            .splice(start..end, std::iter::repeat_n(kind, inserted_lines));
        self.text = new;
        self.repair();
    }

    /// Keep the per-character and per-line vectors in step with the text.
    fn repair(&mut self) {
        let chars = self.text.chars().count();
        self.styles.resize(chars, CharStyle::default());
        let lines = self.text.matches('\n').count() + 1;
        self.blocks.resize(lines, BlockKind::Paragraph);
    }

    /// Each line as runs of `(text, style)`, with its kind.
    fn lines(&self) -> Vec<(BlockKind, Vec<(String, CharStyle)>)> {
        let mut lines = Vec::with_capacity(self.blocks.len());
        let mut runs: Vec<(String, CharStyle)> = Vec::new();
        let mut line = 0;
        let default = CharStyle::default();
        for (index, ch) in self.text.chars().enumerate() {
            if ch == '\n' {
                lines.push((
                    self.blocks.get(line).copied().unwrap_or_default(),
                    std::mem::take(&mut runs),
                ));
                line += 1;
                continue;
            }
            let style = self.styles.get(index).unwrap_or(&default);
            match runs.last_mut() {
                Some((text, last)) if last == style => text.push(ch),
                _ => runs.push((ch.to_string(), style.clone())),
            }
        }
        lines.push((self.blocks.get(line).copied().unwrap_or_default(), runs));
        lines
    }

    /// The document as an HTML fragment.
    ///
    /// Lines become `<p>`, `<li>` inside `<ul>`/`<ol>`, or `<p>` inside `<blockquote>`. Runs
    /// become `<a href>`, `<strong>`, `<em>` and `<u>`, nested in that order. Text and
    /// attributes are escaped, and only `http:`, `https:` and `mailto:` links are written.
    ///
    /// ```
    /// # use fluent2_egui::controls::RichText;
    /// let mut doc = RichText::from_plain("see docs");
    /// doc.set_link(4..8, Some("https://example.com/?a=1&b=2"));
    /// assert_eq!(
    ///     doc.to_html(),
    ///     "<p>see <a href=\"https://example.com/?a=1&amp;b=2\">docs</a></p>"
    /// );
    /// ```
    pub fn to_html(&self) -> String {
        let mut html = String::new();
        let mut open: Option<BlockKind> = None;
        for (kind, runs) in self.lines() {
            if open != Some(kind) {
                close_container(&mut html, open);
                match kind {
                    BlockKind::Bulleted => html.push_str("<ul>"),
                    BlockKind::Numbered => html.push_str("<ol>"),
                    BlockKind::Quote => html.push_str("<blockquote>"),
                    BlockKind::Paragraph => {}
                }
                open = Some(kind);
            }
            let (start, end) = match kind {
                BlockKind::Bulleted | BlockKind::Numbered => ("<li>", "</li>"),
                BlockKind::Paragraph | BlockKind::Quote => ("<p>", "</p>"),
            };
            html.push_str(start);
            if runs.is_empty() {
                html.push_str("<br>");
            }
            for (text, style) in &runs {
                write_run(&mut html, text, style);
            }
            html.push_str(end);
        }
        close_container(&mut html, open);
        html
    }

    /// The document as plain text.
    ///
    /// Bulleted lines start with `- `, numbered lines with `1. `, `2. ` and so on, and quoted
    /// lines with `> `. A link is written as `text (url)` unless the text is the url.
    ///
    /// ```
    /// # use fluent2_egui::controls::{BlockKind, RichText};
    /// let mut doc = RichText::from_plain("First\nSecond");
    /// doc.set_block(0..12, BlockKind::Numbered);
    /// assert_eq!(doc.to_plain(), "1. First\n2. Second");
    /// ```
    pub fn to_plain(&self) -> String {
        let mut out = String::new();
        let mut number = 0;
        for (index, (kind, runs)) in self.lines().into_iter().enumerate() {
            if index > 0 {
                out.push('\n');
            }
            number = if kind == BlockKind::Numbered {
                number + 1
            } else {
                0
            };
            match kind {
                BlockKind::Paragraph => {}
                BlockKind::Bulleted => out.push_str("- "),
                BlockKind::Numbered => out.push_str(&format!("{number}. ")),
                BlockKind::Quote => out.push_str("> "),
            }
            let mut link: Option<(String, String)> = None;
            for (text, style) in runs {
                match (&mut link, style.link) {
                    (Some((url, label)), Some(next)) if *url == next => label.push_str(&text),
                    (_, next) => {
                        flush_link(&mut out, link.take());
                        match next {
                            Some(url) => link = Some((url, text)),
                            None => out.push_str(&text),
                        }
                    }
                }
            }
            flush_link(&mut out, link);
        }
        out
    }
}

fn flush_link(out: &mut String, link: Option<(String, String)>) {
    if let Some((url, label)) = link {
        out.push_str(&label);
        if label != url {
            out.push_str(&format!(" ({url})"));
        }
    }
}

fn close_container(html: &mut String, open: Option<BlockKind>) {
    match open {
        Some(BlockKind::Bulleted) => html.push_str("</ul>"),
        Some(BlockKind::Numbered) => html.push_str("</ol>"),
        Some(BlockKind::Quote) => html.push_str("</blockquote>"),
        Some(BlockKind::Paragraph) | None => {}
    }
}

fn safe_url(url: &str) -> bool {
    let lower = url.trim().to_ascii_lowercase();
    ["http://", "https://", "mailto:"]
        .iter()
        .any(|scheme| lower.starts_with(scheme))
}

fn escape(out: &mut String, text: &str) {
    for ch in text.chars() {
        match ch {
            '&' => out.push_str("&amp;"),
            '<' => out.push_str("&lt;"),
            '>' => out.push_str("&gt;"),
            '"' => out.push_str("&quot;"),
            '\'' => out.push_str("&#39;"),
            _ => out.push(ch),
        }
    }
}

fn write_run(html: &mut String, text: &str, style: &CharStyle) {
    let link = style.link.as_deref().filter(|url| safe_url(url));
    if let Some(url) = link {
        html.push_str("<a href=\"");
        escape(html, url.trim());
        html.push_str("\">");
    }
    let tags = [
        (style.bold, "strong"),
        (style.italic, "em"),
        (style.underline, "u"),
    ];
    for (on, tag) in tags {
        if on {
            html.push_str(&format!("<{tag}>"));
        }
    }
    escape(html, text);
    for (on, tag) in tags.iter().rev() {
        if *on {
            html.push_str(&format!("</{tag}>"));
        }
    }
    if link.is_some() {
        html.push_str("</a>");
    }
}

/// Per-editor state kept in egui memory between frames.
#[derive(Clone, Debug, Default)]
struct EditorMemory {
    /// Style given to typed characters.
    typing: CharStyle,
    /// Selection seen last frame, in characters.
    cursor: Option<(usize, usize)>,
    /// The link row is open, editing this url.
    link_draft: Option<String>,
}

/// What [`RichTextEditor::show`] reports.
///
/// ```
/// # use fluent2_egui::controls::{RichText, RichTextEditor};
/// # fn demo(ui: &mut egui::Ui, doc: &mut RichText) {
/// let out = RichTextEditor::new("body", doc).show(ui);
/// if out.changed {
///     // save a draft
/// }
/// # }
/// ```
pub struct RichTextEditorResponse {
    /// The text area.
    pub response: Response,
    /// The text or formatting changed this frame.
    pub changed: bool,
}

/// A rich-text editor: a toolbar for bold, italic, underline, link, lists and quote over a
/// multiline text area.
///
/// Ctrl+B, Ctrl+I and Ctrl+U toggle the inline formats while the text area has focus. With no
/// selection, a toggled format applies to the next characters typed.
///
/// ```
/// # use fluent2_egui::controls::{RichText, RichTextEditor};
/// # fn demo(ui: &mut egui::Ui, doc: &mut RichText) {
/// ui.add(RichTextEditor::new("compose", doc).label("Message").rows(8));
/// let html = doc.to_html();
/// let plain = doc.to_plain();
/// # }
/// ```
pub struct RichTextEditor<'a> {
    id_salt: Id,
    doc: &'a mut RichText,
    label: Option<String>,
    placeholder: Option<String>,
    rows: usize,
    desired_width: Option<f32>,
    spell: Option<&'a mut dyn super::spell::SpellChecker>,
}

impl<'a> RichTextEditor<'a> {
    /// An editor for `doc`, identified by `id_salt`. Defaults to six rows.
    pub fn new(id_salt: impl std::hash::Hash + std::fmt::Debug, doc: &'a mut RichText) -> Self {
        Self {
            id_salt: Id::unique(id_salt),
            doc,
            label: None,
            placeholder: None,
            rows: 6,
            desired_width: None,
            spell: None,
        }
    }

    /// Accessible name of the text area. Not drawn. Defaults to "Rich text editor".
    #[must_use]
    pub fn label(mut self, label: impl Into<String>) -> Self {
        self.label = Some(label.into());
        self
    }

    /// Grey hint drawn while the document is empty.
    #[must_use]
    pub fn placeholder(mut self, placeholder: impl Into<String>) -> Self {
        self.placeholder = Some(placeholder.into());
        self
    }

    /// Visible line count of the text area. Zero becomes one row.
    #[must_use]
    pub fn rows(mut self, rows: usize) -> Self {
        self.rows = rows.max(1);
        self
    }

    /// Width of the editor. Defaults to [`Ui::available_width`].
    #[must_use]
    pub fn desired_width(mut self, width: f32) -> Self {
        self.desired_width = Some(if width.is_finite() { width.max(0.0) } else { 0.0 });
        self
    }

    /// Underline misspellings in the plain text and offer replacements.
    ///
    /// The library ships no dictionary. A secondary click on a misspelling, or Shift+F10 /
    /// the Menu key with the caret in one, opens the suggestions. Choosing one replaces that
    /// span and keeps the surrounding character styles. Ignore hides the underline until the
    /// text changes. The checker runs only when the plain text changes.
    #[must_use]
    pub fn spell_checker(mut self, checker: &'a mut dyn super::spell::SpellChecker) -> Self {
        self.spell = Some(checker);
        self
    }

    /// Paint the editor and apply this frame's edits to the document.
    pub fn show(self, ui: &mut Ui) -> RichTextEditorResponse {
        super::ensure_type_ramp(ui);
        let theme = Theme::get(ui.ctx());
        let id = ui.make_persistent_id(self.id_salt);
        let edit_id = id.with("text");
        let enabled = ui.is_enabled();
        let spell = self.spell;
        let doc = self.doc;
        let mut memory: EditorMemory = ui.data(|data| data.get_temp(id)).unwrap_or_default();
        let mut changed = false;

        let selection = TextEditState::load(ui.ctx(), edit_id)
            .and_then(|state| state.cursor.char_range())
            .map(|range| {
                let [a, b] = range.sorted_cursors();
                (a.index.0.min(doc.len_chars()), b.index.0.min(doc.len_chars()))
            })
            .unwrap_or((doc.len_chars(), doc.len_chars()));
        let range = selection.0..selection.1;
        let collapsed = range.is_empty();

        let width = self.desired_width.unwrap_or_else(|| ui.available_width());
        let radius = CornerRadius::same(rounded(border_radius::MEDIUM));
        let mut refocus = false;

        let frame = egui::Frame::NONE
            .fill(c(theme.color.neutral_background_1))
            .stroke(Stroke::new(stroke_width::THIN, c(theme.color.neutral_stroke_1)))
            .corner_radius(radius)
            .inner_margin(egui::Margin::symmetric(
                rounded_i8(spacing_horizontal::S),
                rounded_i8(spacing_vertical::S),
            ));
        let framed = frame.show(ui, |ui| {
            ui.set_width((width - 2.0 * spacing_horizontal::S).max(0.0));

            // Shortcuts are read before the TextEdit, which ignores them.
            let had_focus = ui.memory(|m| m.has_focus(edit_id));
            let mut shortcut = None;
            if had_focus && enabled {
                for (key, format) in [
                    (Key::B, InlineFormat::Bold),
                    (Key::I, InlineFormat::Italic),
                    (Key::U, InlineFormat::Underline),
                ] {
                    let pressed = ui.input_mut(|input| {
                        input.consume_shortcut(&KeyboardShortcut::new(Modifiers::COMMAND, key))
                    });
                    if pressed {
                        shortcut = Some(format);
                    }
                }
            }

            let mut toggle_inline = shortcut;
            let mut toggle_block = None;
            let mut open_link = false;
            ui.horizontal_wrapped(|ui| {
                ui.spacing_mut().item_spacing.x = spacing_horizontal::XXS;
                for (format, icon, name) in [
                    (InlineFormat::Bold, IconName::TextBold, "Bold"),
                    (InlineFormat::Italic, IconName::TextItalic, "Italic"),
                    (InlineFormat::Underline, IconName::TextUnderline, "Underline"),
                ] {
                    let mut on = if collapsed {
                        format.get(&memory.typing)
                    } else {
                        doc.has_inline(range.clone(), format)
                    };
                    let response = ui.add(
                        ToggleButton::icon_only(&mut on, icon, name)
                            .appearance(Appearance::Subtle)
                            .size(ButtonSize::Small)
                            .id_salt(id.with(name)),
                    );
                    if response.clicked() {
                        toggle_inline = Some(format);
                    }
                }
                let mut linked = memory.link_draft.is_some() || doc.link_in(range.clone()).is_some();
                let response = ui.add(
                    ToggleButton::icon_only(&mut linked, IconName::Link, "Link")
                        .appearance(Appearance::Subtle)
                        .size(ButtonSize::Small)
                        .id_salt(id.with("Link")),
                );
                if response.clicked() {
                    open_link = true;
                }
                for (kind, name) in [
                    (BlockKind::Bulleted, "Bulleted list"),
                    (BlockKind::Numbered, "Numbered list"),
                    (BlockKind::Quote, "Quote"),
                ] {
                    let mut on = doc.has_block(range.clone(), kind);
                    let response = ui.add(
                        ToggleButton::new(&mut on, name)
                            .appearance(Appearance::Subtle)
                            .size(ButtonSize::Small)
                            .id_salt(id.with(name)),
                    );
                    if response.clicked() {
                        toggle_block = Some(kind);
                    }
                }
            });

            if let Some(format) = toggle_inline {
                if collapsed {
                    let on = !format.get(&memory.typing);
                    format.set(&mut memory.typing, on);
                } else {
                    doc.toggle_inline(range.clone(), format);
                    changed = true;
                }
                refocus = true;
            }
            if let Some(kind) = toggle_block {
                doc.toggle_block(range.clone(), kind);
                changed = true;
                refocus = true;
            }
            if open_link {
                memory.link_draft = match memory.link_draft {
                    Some(_) => None,
                    None => Some(doc.link_in(range.clone()).unwrap_or("https://").to_owned()),
                };
            }

            if let Some(mut url) = memory.link_draft.take() {
                let mut keep = true;
                ui.horizontal(|ui| {
                    ui.add(
                        super::Input::new(&mut url)
                            .label("Link address")
                            .desired_width((ui.available_width() * 0.5).max(0.0)),
                    );
                    if ui
                        .add(
                            Button::new("Apply")
                                .size(ButtonSize::Small)
                                .appearance(Appearance::Primary),
                        )
                        .clicked()
                    {
                        if !collapsed {
                            doc.set_link(range.clone(), Some(&url));
                            changed = true;
                        }
                        keep = false;
                        refocus = true;
                    }
                    if ui
                        .add(Button::new("Remove link").size(ButtonSize::Small))
                        .clicked()
                    {
                        doc.set_link(range.clone(), None);
                        changed = true;
                        keep = false;
                        refocus = true;
                    }
                });
                if keep {
                    memory.link_draft = Some(url);
                }
            }

            ui.add_space(spacing_vertical::XS);
            ui.add(egui::Separator::default().spacing(0.0));
            ui.add_space(spacing_vertical::XS);

            let gutter = spacing_horizontal::XXL;
            let fonts = Fonts::of(ui, &theme);
            let snapshot = doc.clone();
            let typing = memory.typing.clone();
            let mut layouter = |ui: &Ui, buffer: &dyn egui::TextBuffer, wrap_width: f32| {
                let job = if buffer.as_str() == snapshot.text() {
                    layout_job(&snapshot, &fonts, wrap_width)
                } else {
                    let mut edited = snapshot.clone();
                    edited.replace_text(buffer.as_str(), &typing);
                    layout_job(&edited, &fonts, wrap_width)
                };
                ui.fonts_mut(|f| f.layout_job(job))
            };
            let mut text = doc.text().to_owned();
            let hint = self.placeholder.clone().unwrap_or_default();
            let edit = TextEdit::multiline(&mut text)
                .id(edit_id)
                .frame(egui::Frame::NONE)
                .interactive(enabled)
                .hint_text(egui::WidgetText::from(hint).color(c(theme.color.neutral_foreground_4)))
                .desired_rows(self.rows)
                .desired_width(f32::INFINITY)
                .layouter(&mut layouter);
            // The gutter holds list markers and quote bars, left of the text.
            let output = ui
                .horizontal_top(|ui| {
                    ui.add_space(gutter);
                    edit.show(ui)
                })
                .inner;
            if text != doc.text() {
                doc.replace_text(&text, &memory.typing);
                changed = true;
            }
            if let Some(checker) = spell {
                let mut session = super::spell::SpellSession::new(edit_id, doc.text(), checker);
                session.paint(ui, &output.galley, output.galley_pos);
                let action = session.menu(ui, &output.response.response, &output.galley, output.galley_pos);
                if let Some((range, with)) = action.replace.clone()
                    && super::spell::apply_replace(&mut text, range, &with).is_some()
                {
                    doc.replace_text(&text, &memory.typing);
                    changed = true;
                }
            }
            paint_gutter(ui, doc, &output.galley, output.galley_pos, gutter, &fonts);

            let cursor = output.cursor_range.map(|range| {
                let [a, b] = range.sorted_cursors();
                (a.index.0, b.index.0)
            });
            // A moved caret picks up the style around it. The first caret after focus keeps
            // whatever the toolbar set while the text area had none.
            if cursor != memory.cursor
                && memory.cursor.is_some()
                && text == snapshot.text()
                && let Some((start, _)) = cursor
            {
                memory.typing = typing_style(doc, start);
            }
            memory.cursor = cursor;
            output.response.response
        });
        let response = framed.inner;

        if refocus {
            ui.memory_mut(|m| m.request_focus(edit_id));
            if let Some(mut state) = TextEditState::load(ui.ctx(), edit_id) {
                state.cursor.set_char_range(Some(CCursorRange::two(
                    CCursor::new(selection.0),
                    CCursor::new(selection.1),
                )));
                state.store(ui.ctx(), edit_id);
            }
        }
        if response.has_focus() && focus_visible(ui, &response) {
            paint_focus_ring(ui, framed.response.rect, radius);
        }
        let label = self.label.unwrap_or_else(|| "Rich text editor".to_owned());
        ui.ctx().accesskit_node_builder(response.id, |node| {
            node.set_label(label);
        });
        ui.data_mut(|data| data.insert_temp(id, memory));
        if changed {
            ui.ctx().request_repaint();
        }
        RichTextEditorResponse { response, changed }
    }
}

impl Widget for RichTextEditor<'_> {
    fn ui(self, ui: &mut Ui) -> Response {
        self.show(ui).response
    }
}

/// The style typed characters take at `cursor`: the preceding character's, without its link.
fn typing_style(doc: &RichText, cursor: usize) -> CharStyle {
    let before = cursor.checked_sub(1).and_then(|index| doc.style_at(index));
    let mut style = before
        .or_else(|| doc.style_at(cursor))
        .cloned()
        .unwrap_or_default();
    style.link = None;
    style
}

struct Fonts {
    regular: FontId,
    bold: FontId,
    text: Color32,
    link: Color32,
    marker: Color32,
    quote: Color32,
}

impl Fonts {
    fn of(ui: &Ui, theme: &Theme) -> Self {
        let regular = font_id(TypeRamp::Body1);
        let semibold = ui.fonts(|fonts| {
            fonts
                .families()
                .iter()
                .any(|family| matches!(family, FontFamily::Name(name) if name.as_ref() == SEMIBOLD_FAMILY))
        });
        let bold = if semibold {
            FontId::new(regular.size, FontFamily::Name(SEMIBOLD_FAMILY.into()))
        } else {
            regular.clone()
        };
        let enabled = ui.is_enabled();
        let text = if enabled {
            theme.color.neutral_foreground_1
        } else {
            theme.color.neutral_foreground_disabled
        };
        Self {
            regular,
            bold,
            text: c(text),
            link: c(theme.color.brand_foreground_link),
            marker: c(theme.color.neutral_foreground_3),
            quote: c(theme.color.neutral_stroke_accessible),
        }
    }
}

fn layout_job(doc: &RichText, fonts: &Fonts, wrap_width: f32) -> LayoutJob {
    let mut job = LayoutJob::default();
    job.wrap.max_width = if wrap_width.is_finite() {
        wrap_width.max(0.0)
    } else {
        f32::INFINITY
    };
    let default = CharStyle::default();
    let mut run = String::new();
    let mut run_style: Option<&CharStyle> = None;
    for (index, ch) in doc.text().chars().enumerate() {
        let style = doc.style_at(index).unwrap_or(&default);
        if run_style.is_some_and(|last| last != style) {
            push_run(&mut job, &run, run_style.unwrap_or(&default), fonts);
            run.clear();
        }
        run_style = Some(style);
        run.push(ch);
    }
    push_run(&mut job, &run, run_style.unwrap_or(&default), fonts);
    job
}

fn push_run(job: &mut LayoutJob, text: &str, style: &CharStyle, fonts: &Fonts) {
    let color = if style.link.is_some() {
        fonts.link
    } else {
        fonts.text
    };
    let underline = if style.underline || style.link.is_some() {
        Stroke::new(stroke_width::THIN, color)
    } else {
        Stroke::NONE
    };
    job.append(
        text,
        0.0,
        TextFormat {
            font_id: if style.bold {
                fonts.bold.clone()
            } else {
                fonts.regular.clone()
            },
            color,
            italics: style.italic,
            underline,
            ..TextFormat::default()
        },
    );
}

/// Paint bullets, numbers and quote bars to the left of each line.
fn paint_gutter(
    ui: &Ui,
    doc: &RichText,
    galley: &egui::Galley,
    galley_pos: egui::Pos2,
    gutter: f32,
    fonts: &Fonts,
) {
    let painter = ui.painter();
    let mut line = 0;
    let mut first_row = true;
    let mut number = 0;
    let mut last_kind = BlockKind::Paragraph;
    for row in &galley.rows {
        let kind = doc.blocks().get(line).copied().unwrap_or_default();
        let rect = row.rect().translate(galley_pos.to_vec2());
        let left = galley_pos.x - gutter;
        if first_row {
            number = if kind == BlockKind::Numbered && last_kind == BlockKind::Numbered {
                number + 1
            } else {
                1
            };
            let marker = match kind {
                BlockKind::Bulleted => Some("•".to_owned()),
                BlockKind::Numbered => Some(format!("{number}.")),
                BlockKind::Paragraph | BlockKind::Quote => None,
            };
            if let Some(marker) = marker {
                painter.text(
                    egui::pos2(galley_pos.x - spacing_horizontal::XS, rect.center().y),
                    Align2::RIGHT_CENTER,
                    marker,
                    fonts.regular.clone(),
                    fonts.marker,
                );
            }
            last_kind = kind;
        }
        if kind == BlockKind::Quote {
            let bar = Rect::from_min_max(
                egui::pos2(left + spacing_horizontal::S, rect.top()),
                egui::pos2(
                    left + spacing_horizontal::S + stroke_width::THICKER,
                    rect.bottom(),
                ),
            );
            painter.rect_filled(bar, CornerRadius::ZERO, fonts.quote);
        }
        first_row = row.ends_with_newline;
        if row.ends_with_newline {
            line += 1;
        }
    }
}

fn rounded(value: f32) -> u8 {
    value.round().clamp(0.0, f32::from(u8::MAX)) as u8
}

fn rounded_i8(value: f32) -> i8 {
    value.round().clamp(0.0, f32::from(i8::MAX)) as i8
}
