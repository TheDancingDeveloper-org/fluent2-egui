//! A pluggable spell checker for [`super::Textarea`] and [`super::RichTextEditor`].
//!
//! The library ships no dictionary and no language data. An app implements [`SpellChecker`]
//! and passes it in. Results are cached against the widget id and the text, so [`SpellChecker::check`]
//! runs when the text changes and not on every paint.

use std::ops::Range;

use egui::{Id, Response, Ui};

use crate::surfaces::{self, MenuItem};
use crate::theme::{Theme, c};

/// One misspelled span and the replacements the checker offers.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Spelling {
    /// Byte range in the text the checker was given.
    pub range: Range<usize>,
    /// Replacement strings, in the order the menu shows them.
    pub suggestions: Vec<String>,
}

/// A spell checker the application supplies.
///
/// The widget calls [`check`](SpellChecker::check) only after the text differs from the cached
/// result. A checker that talks to a dictionary should still return quickly: the library does
/// not run it on a background thread.
pub trait SpellChecker {
    /// Misspellings in `text`, in ascending byte order. Ranges must sit inside `text`.
    fn check(&mut self, text: &str) -> Vec<Spelling>;
}

/// The cached result of the last check, stored on the egui context.
#[derive(Clone, Debug, Default)]
struct SpellCache {
    text: String,
    findings: Vec<Spelling>,
    /// Byte ranges the user chose to ignore until the text changes.
    ignored: Vec<Range<usize>>,
}

impl SpellCache {
    fn sync(&mut self, text: &str, checker: &mut dyn SpellChecker) {
        if self.text == text {
            return;
        }
        // An edit moves every later range. Drop ignores rather than guess where they went.
        self.ignored.clear();
        self.findings = checker
            .check(text)
            .into_iter()
            .filter(|spelling| valid_range(text, &spelling.range))
            .collect();
        self.text = text.to_owned();
    }
}

fn valid_range(text: &str, range: &Range<usize>) -> bool {
    range.start <= range.end
        && range.end <= text.len()
        && text.is_char_boundary(range.start)
        && text.is_char_boundary(range.end)
}

/// What the suggestions menu did this frame.
#[derive(Clone, Debug, Default, Eq, PartialEq)]
pub(crate) struct SpellAction {
    /// Replace this byte range with the second value.
    pub replace: Option<(Range<usize>, String)>,
    /// Stop underlining this byte range until the text changes.
    pub ignore: Option<Range<usize>>,
}

/// Underlines and the suggestions menu for one text widget.
pub(crate) struct SpellSession<'a> {
    id: Id,
    text: &'a str,
    checker: &'a mut dyn SpellChecker,
}

impl<'a> SpellSession<'a> {
    pub(crate) fn new(id: Id, text: &'a str, checker: &'a mut dyn SpellChecker) -> Self {
        Self { id, text, checker }
    }

    fn cache_id(id: Id) -> Id {
        id.with("fluent2_spell")
    }

    fn menu_id(id: Id) -> Id {
        id.with("fluent2_spell_menu")
    }

    /// Findings that are not ignored. Refreshes the cache when the text changed.
    fn findings(&mut self, ui: &Ui) -> Vec<Spelling> {
        let mut cache: SpellCache = ui
            .data(|data| data.get_temp(Self::cache_id(self.id)))
            .unwrap_or_default();
        cache.sync(self.text, self.checker);
        let ignored = cache.ignored.clone();
        let live = cache
            .findings
            .iter()
            .filter(|spelling| !ignored.iter().any(|range| range == &spelling.range))
            .cloned()
            .collect();
        ui.data_mut(|data| data.insert_temp(Self::cache_id(self.id), cache));
        live
    }

    /// Paint a wavy underline under each misspelling.
    ///
    /// `origin` is the galley's screen position (`TextEditOutput::galley_pos`).
    pub(crate) fn paint(&mut self, ui: &Ui, galley: &egui::Galley, origin: egui::Pos2) {
        let theme = Theme::get(ui.ctx());
        let color = c(theme.color.status_danger_foreground_1);
        let painter = ui.painter();
        for spelling in self.findings(ui) {
            for (left, right, bottom) in spans(galley, origin, &spelling.range) {
                wave(painter, egui::pos2(left, bottom), (right - left).max(0.0), color);
            }
        }
    }

    /// Suggestions for a secondary click, or for Shift+F10 / the Menu key while focused.
    ///
    /// The menu stays open across frames through egui's popup memory, keyed on this widget.
    pub(crate) fn menu(
        &mut self,
        ui: &Ui,
        response: &Response,
        galley: &egui::Galley,
        origin: egui::Pos2,
    ) -> SpellAction {
        let findings = self.findings(ui);
        let menu_id = Self::menu_id(self.id);
        let secondary = response.secondary_clicked();
        let keyboard = response.has_focus()
            && ui.input_mut(|input| {
                input.consume_key(egui::Modifiers::SHIFT, egui::Key::F10)
                    || input.consume_key(egui::Modifiers::NONE, egui::Key::Menu)
            });

        // `surfaces::context_menu` opens `Popup::context_menu(response)`, whose id is
        // `response.id.with("popup")`. Keyboard open has to use that same id.
        let popup_id = response.id.with("popup");
        if secondary || keyboard {
            let hit = if secondary {
                response
                    .interact_pointer_pos()
                    .or_else(|| ui.input(|input| input.pointer.interact_pos()))
                    .and_then(|pos| word_at_pos(&findings, galley, origin, pos))
            } else {
                word_at_caret(&findings, self.text, ui, response)
            };
            if let Some(spelling) = hit {
                ui.data_mut(|data| data.insert_temp(menu_id, spelling));
                if keyboard {
                    egui::Popup::open_id(ui.ctx(), popup_id);
                }
            }
        }

        let open = egui::Popup::is_id_open(ui.ctx(), popup_id);
        let spelling: Option<Spelling> = ui.data(|data| data.get_temp(menu_id));
        let Some(spelling) = spelling.filter(|_| open) else {
            return SpellAction::default();
        };
        // The stored range can outlive an edit that landed before the menu closed.
        if !valid_range(self.text, &spelling.range) {
            egui::Popup::close_id(ui.ctx(), popup_id);
            return SpellAction::default();
        }

        let mut action = SpellAction::default();
        surfaces::context_menu(response, |ui| {
            if spelling.suggestions.is_empty() {
                ui.add_enabled(false, MenuItem::new("No suggestions"));
            }
            for (index, suggestion) in spelling.suggestions.iter().enumerate() {
                let item = MenuItem::new(suggestion).id_salt(("spell_suggestion", index));
                if ui.add(item).clicked() {
                    action.replace = Some((spelling.range.clone(), suggestion.clone()));
                }
            }
            surfaces::divider(ui);
            if ui.add(MenuItem::new("Ignore").id_salt("spell_ignore")).clicked() {
                action.ignore = Some(spelling.range.clone());
            }
        });

        if action.replace.is_some() || action.ignore.is_some() {
            egui::Popup::close_id(ui.ctx(), popup_id);
        }
        if let Some(range) = action.ignore.clone() {
            let mut cache: SpellCache = ui
                .data(|data| data.get_temp(Self::cache_id(self.id)))
                .unwrap_or_default();
            if !cache.ignored.iter().any(|known| known == &range) {
                cache.ignored.push(range);
            }
            ui.data_mut(|data| data.insert_temp(Self::cache_id(self.id), cache));
        }
        action
    }
}

/// Screen-space underline segments for `range`: one per visual line the range crosses.
fn spans(galley: &egui::Galley, origin: egui::Pos2, range: &Range<usize>) -> Vec<(f32, f32, f32)> {
    // `Spelling.range` is bytes. `CCursor` is a char index.
    let text = galley.text();
    let start = galley.pos_from_cursor(egui::text::CCursor::new(byte_to_char(text, range.start)));
    let end = galley.pos_from_cursor(egui::text::CCursor::new(byte_to_char(text, range.end)));
    // Cursor rects are in galley space. `origin` is where `rect.min` is on screen.
    let shift = origin.to_vec2() - galley.rect.min.to_vec2();
    let start = start.translate(shift);
    let end = end.translate(shift);
    let height = start.height().max(1.0);
    if (start.top() - end.top()).abs() < 0.5 {
        return vec![(start.left(), end.left(), start.bottom())];
    }
    let mut out = Vec::new();
    // Wrapped rows span the galley's own width, which starts at `rect.left()` in galley space.
    let content_left = origin.x;
    let content_right = origin.x + galley.rect.width();
    out.push((start.left(), content_right, start.bottom()));
    let mut top = start.bottom();
    while top + 0.5 < end.top() {
        out.push((content_left, content_right, top + height));
        top += height;
    }
    out.push((content_left, end.left(), end.bottom()));
    out
}

fn word_at_pos(
    findings: &[Spelling],
    galley: &egui::Galley,
    origin: egui::Pos2,
    screen: egui::Pos2,
) -> Option<Spelling> {
    let local = screen - origin.to_vec2() + galley.rect.min.to_vec2();
    let cursor = galley.cursor_from_pos(local.to_vec2());
    let index = char_to_byte(galley.text(), cursor.index.0);
    findings
        .iter()
        .find(|spelling| spelling.range.start <= index && index < spelling.range.end)
        .cloned()
}

fn word_at_caret(findings: &[Spelling], text: &str, ui: &Ui, response: &Response) -> Option<Spelling> {
    let state = egui::text_edit::TextEditState::load(ui.ctx(), response.id)?;
    let char_index = state.cursor.char_range().map(|range| range.primary.index.0)?;
    let index = char_to_byte(text, char_index);
    findings
        .iter()
        .find(|spelling| spelling.range.start <= index && index <= spelling.range.end)
        .cloned()
}

/// A short squiggle, one segment every three points, alternating one and a half points.
fn wave(painter: &egui::Painter, origin: egui::Pos2, width: f32, color: egui::Color32) {
    if width <= 0.0 {
        return;
    }
    let mut points = Vec::new();
    let mut x = 0.0_f32;
    let mut up = true;
    while x <= width {
        let y = if up { -1.5 } else { 1.5 };
        points.push(egui::pos2(origin.x + x, origin.y + y));
        up = !up;
        x += 3.0;
    }
    if points.len() >= 2 {
        painter.add(egui::Shape::line(points, egui::Stroke::new(1.0, color)));
    }
}

/// A checker that flags every word in [`misspelled`](WordListChecker::misspelled).
///
/// Words are split on whitespace and on bytes that are not ASCII letters, an apostrophe, or
/// a non-ASCII byte. Tests and the gallery use this. An application wraps its own dictionary.
pub struct WordListChecker {
    /// Words that should be underlined, compared case-insensitively.
    pub misspelled: Vec<String>,
    /// The suggestions offered for every misspelling.
    pub suggestions: Vec<String>,
}

impl SpellChecker for WordListChecker {
    fn check(&mut self, text: &str) -> Vec<Spelling> {
        let mut out = Vec::new();
        let bytes = text.as_bytes();
        let mut index = 0;
        while index < bytes.len() {
            while index < bytes.len() && !is_word_byte(bytes[index]) {
                index += 1;
            }
            let start = index;
            while index < bytes.len() && is_word_byte(bytes[index]) {
                index += 1;
            }
            if start == index {
                break;
            }
            let word = &text[start..index];
            let bad = self
                .misspelled
                .iter()
                .any(|known| known.eq_ignore_ascii_case(word));
            if bad {
                out.push(Spelling {
                    range: start..index,
                    suggestions: self.suggestions.clone(),
                });
            }
        }
        out
    }
}

fn is_word_byte(byte: u8) -> bool {
    byte.is_ascii_alphabetic() || byte == b'\'' || byte >= 0x80
}

/// Apply a replacement to `text`, returning the new caret (a char index) after the insert.
pub(crate) fn apply_replace(text: &mut String, range: Range<usize>, with: &str) -> Option<usize> {
    if !valid_range(text, &range) {
        return None;
    }
    let caret_bytes = range.start + with.len();
    text.replace_range(range, with);
    Some(text[..caret_bytes].chars().count())
}

/// Char index of the char that begins at `byte` (or the char count, when `byte` is the end).
fn byte_to_char(text: &str, byte: usize) -> usize {
    let byte = byte.min(text.len());
    text[..byte].chars().count()
}

/// Byte offset of char index `index`.
fn char_to_byte(text: &str, index: usize) -> usize {
    text.char_indices()
        .nth(index)
        .map_or(text.len(), |(byte, _)| byte)
}

#[cfg(test)]
mod tests {
    use super::{byte_to_char, char_to_byte};

    #[test]
    fn cursors_convert_across_multibyte_chars() {
        let text = "café lait";
        let byte = text.len() - "lait".len();
        assert!(text.ends_with("lait"));
        let chars = byte_to_char(text, byte);
        assert_eq!(chars, 5);
        assert_eq!(char_to_byte(text, chars), byte);
        assert_eq!(byte_to_char(text, text.len()), text.chars().count());
        // A caret on the last char of a misspelling still lands inside the byte range.
        let end_byte = byte + "lait".len();
        let end_chars = byte_to_char(text, end_byte);
        let caret_byte = char_to_byte(text, end_chars);
        assert!(byte <= caret_byte && caret_byte <= end_byte);
    }
}
