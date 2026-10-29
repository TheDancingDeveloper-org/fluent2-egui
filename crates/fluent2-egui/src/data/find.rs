//! In-app find: a [`FindBar`] that searches [`FindProvider`]s and highlights the matches.
//!
//! The browser's own find cannot see a canvas, so an egui app needs its own. The pieces:
//!
//! - [`FindProvider`]: searchable content, as numbered items of text (grid rows, list items,
//!   lines of a text or log view, terminal lines). Implemented for string slices, [`TextLines`],
//!   [`FindItems`] (a count and a closure) and, with the `terminal` feature, `TerminalGrid`.
//! - [`FindState`]: the query, the matches and the current match. The app keeps one per
//!   searchable area.
//! - [`FindBar`]: the bar itself, built on [`SearchBox`]. Ctrl+F (Cmd+F on macOS) opens it,
//!   Escape closes it, Enter and Shift+Enter move to the next and previous match, and a counter
//!   shows "3 of 12".
//!
//! Views stay in charge of their own painting. A view asks [`FindState::layout_job`] for a
//! highlighted line of text, and [`FindState::take_reveal`] for the match to scroll to, which it
//! hands to `List::scroll_to_row`, `DataGrid::scroll_to_row` or `Response::scroll_to_me`.
//!
//! On the web, call [`capture_browser_find`] once at start-up (feature `web-find`) so Ctrl+F
//! reaches the app instead of opening the browser's find.
//!
//! ```
//! use fluent2_egui::data::find::{FindBar, FindProvider, FindState};
//!
//! # fn demo(ui: &mut egui::Ui, state: &mut FindState, lines: &Vec<String>) {
//! let providers: [&dyn FindProvider; 1] = [lines];
//! FindBar::new("find").show(ui, state, &providers);
//! let reveal = state.take_reveal();
//! for (item, line) in lines.iter().enumerate() {
//!     let job = state.layout_job(ui, 0, item, line);
//!     let response = ui.label(job);
//!     if reveal.as_ref().is_some_and(|m| m.provider == 0 && m.item == item) {
//!         response.scroll_to_me(Some(egui::Align::Center));
//!     }
//! }
//! # }
//! ```

use std::borrow::Cow;
use std::ops::Range;

use egui::text::{LayoutJob, TextFormat};
use egui::{Align, Color32, Id, Key, KeyboardShortcut, Layout, Modifiers, Response, Stroke, Ui};

use fluent2_icons::Icon;
use fluent2_tokens::generated::global::spacing_horizontal;

use crate::controls::{Button, SearchBox, ToggleButton};
use crate::theme::{Theme, ThemeKind, c};

/// Searchable content: `item_count` numbered items of text.
///
/// An item is whatever the view scrolls to and highlights: a grid row (its cells joined), a list
/// item, or one line of a text or log view.
///
/// ```
/// use std::borrow::Cow;
/// use fluent2_egui::data::find::FindProvider;
///
/// struct Rows(Vec<(String, String)>);
///
/// impl FindProvider for Rows {
///     fn item_count(&self) -> usize {
///         self.0.len()
///     }
///     fn item_text(&self, item: usize) -> Cow<'_, str> {
///         self.0.get(item).map_or(Cow::Borrowed(""), |(a, b)| Cow::Owned(format!("{a}\t{b}")))
///     }
/// }
/// ```
pub trait FindProvider {
    /// Number of items.
    fn item_count(&self) -> usize;

    /// Text of item `item`. Out-of-range items return an empty string.
    fn item_text(&self, item: usize) -> Cow<'_, str>;
}

impl FindProvider for [String] {
    fn item_count(&self) -> usize {
        self.len()
    }

    fn item_text(&self, item: usize) -> Cow<'_, str> {
        Cow::Borrowed(self.get(item).map_or("", String::as_str))
    }
}

impl FindProvider for [&str] {
    fn item_count(&self) -> usize {
        self.len()
    }

    fn item_text(&self, item: usize) -> Cow<'_, str> {
        Cow::Borrowed(self.get(item).copied().unwrap_or(""))
    }
}

impl FindProvider for Vec<String> {
    fn item_count(&self) -> usize {
        self.len()
    }

    fn item_text(&self, item: usize) -> Cow<'_, str> {
        self.as_slice().item_text(item)
    }
}

/// A block of text searched line by line: item `n` is line `n`.
///
/// ```
/// use fluent2_egui::data::find::{FindProvider, TextLines};
///
/// let text = TextLines::new("first\nsecond");
/// assert_eq!(text.item_count(), 2);
/// assert_eq!(text.item_text(1), "second");
/// ```
pub struct TextLines<'a> {
    lines: Vec<&'a str>,
}

impl<'a> TextLines<'a> {
    /// Split `text` into lines (`str::lines`).
    pub fn new(text: &'a str) -> Self {
        Self {
            lines: text.lines().collect(),
        }
    }

    /// The lines, for painting.
    pub fn lines(&self) -> &[&'a str] {
        &self.lines
    }
}

impl FindProvider for TextLines<'_> {
    fn item_count(&self) -> usize {
        self.lines.len()
    }

    fn item_text(&self, item: usize) -> Cow<'_, str> {
        Cow::Borrowed(self.lines.get(item).copied().unwrap_or(""))
    }
}

/// A provider from an item count and a closure, for content that is not stored as strings
/// (data grid rows, for example).
///
/// ```
/// use fluent2_egui::data::find::{FindItems, FindProvider};
///
/// let rows = [("Ada", 36), ("Grace", 45)];
/// let provider = FindItems::new(rows.len(), |i| {
///     rows.get(i).map(|(name, age)| format!("{name} {age}")).unwrap_or_default()
/// });
/// assert_eq!(provider.item_text(1), "Grace 45");
/// ```
pub struct FindItems<F> {
    count: usize,
    text: F,
}

impl<F: Fn(usize) -> String> FindItems<F> {
    /// `count` items whose text is `text(item)`.
    pub fn new(count: usize, text: F) -> Self {
        Self { count, text }
    }
}

impl<F: Fn(usize) -> String> FindProvider for FindItems<F> {
    fn item_count(&self) -> usize {
        self.count
    }

    fn item_text(&self, item: usize) -> Cow<'_, str> {
        if item < self.count {
            Cow::Owned((self.text)(item))
        } else {
            Cow::Borrowed("")
        }
    }
}

/// A terminal is searched line by line, scrollback included: item `n` is stable line
/// `first_line() + n`.
#[cfg(feature = "terminal")]
impl FindProvider for super::terminal::TerminalGrid {
    fn item_count(&self) -> usize {
        usize::try_from(self.end_line().saturating_sub(self.first_line())).unwrap_or(usize::MAX)
    }

    fn item_text(&self, item: usize) -> Cow<'_, str> {
        Cow::Owned(self.line_text(self.first_line().saturating_add(item as u64)))
    }
}

/// One match: a byte range in the text of one item of one provider.
#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub struct FindMatch {
    /// Index of the provider in the slice given to [`FindBar::show`] or [`FindState::search`].
    pub provider: usize,
    /// Item within the provider.
    pub item: usize,
    /// Byte range in the item's text. Always on `char` boundaries.
    pub range: Range<usize>,
}

/// Every non-overlapping match of `query` in `text`, as byte ranges.
///
/// Without `match_case` the comparison folds case with Unicode lower-casing, one `char` at a
/// time, so the ranges stay on `char` boundaries of `text`.
///
/// ```
/// use fluent2_egui::data::find::find_in;
///
/// assert_eq!(find_in("Abc abc", "abc", false), vec![0..3, 4..7]);
/// assert_eq!(find_in("Abc abc", "abc", true), vec![4..7]);
/// ```
#[must_use]
pub fn find_in(text: &str, query: &str, match_case: bool) -> Vec<Range<usize>> {
    let mut out = Vec::new();
    if query.is_empty() {
        return out;
    }
    if match_case {
        let mut from = 0;
        while let Some(at) = text.get(from..).and_then(|rest| rest.find(query)) {
            let start = from + at;
            let end = start + query.len();
            out.push(start..end);
            from = end;
        }
        return out;
    }
    let folded: Vec<char> = query.chars().flat_map(char::to_lowercase).collect();
    let mut from = 0;
    while from < text.len() {
        let Some(rest) = text.get(from..) else { break };
        match folded_prefix_len(rest, &folded) {
            Some(len) => {
                out.push(from..from + len);
                from += len;
            }
            None => from += rest.chars().next().map_or(1, char::len_utf8),
        }
    }
    out
}

/// Byte length of the prefix of `text` whose lower-cased chars are exactly `folded`.
fn folded_prefix_len(text: &str, folded: &[char]) -> Option<usize> {
    let mut want = folded.iter();
    let mut pending = want.len();
    for (at, ch) in text.char_indices() {
        for lower in ch.to_lowercase() {
            if want.next() != Some(&lower) {
                return None;
            }
            pending -= 1;
        }
        if pending == 0 {
            return Some(at + ch.len_utf8());
        }
    }
    None
}

/// What [`FindState::search`] last ran with, to skip searches that would give the same result.
#[derive(Clone, Debug, PartialEq, Eq)]
struct SearchKey {
    query: String,
    match_case: bool,
    counts: Vec<usize>,
}

/// The find bar's memory: whether it is open, the query, the matches and the current match.
///
/// ```
/// use fluent2_egui::data::find::{FindProvider, FindState};
///
/// let lines = vec!["alpha".to_owned(), "beta".to_owned(), "alphabet".to_owned()];
/// let providers: [&dyn FindProvider; 1] = [&lines];
/// let mut state = FindState::default();
/// state.open();
/// state.set_query("alpha");
/// state.search(&providers);
/// assert_eq!(state.matches().len(), 2);
/// assert_eq!(state.current().map(|m| m.item), Some(0));
/// state.next();
/// assert_eq!(state.current().map(|m| m.item), Some(2));
/// ```
#[derive(Clone, Debug, Default)]
pub struct FindState {
    open: bool,
    query: String,
    match_case: bool,
    matches: Vec<FindMatch>,
    current: Option<usize>,
    searched: Option<SearchKey>,
    reveal: bool,
    focus_pending: bool,
    field_id: Option<Id>,
}

impl FindState {
    /// Open the bar and move keyboard focus to its field.
    pub fn open(&mut self) {
        if !self.open {
            self.searched = None;
        }
        self.open = true;
        self.focus_pending = true;
    }

    /// Close the bar. The query is kept for the next time it opens; highlights go away.
    pub fn close(&mut self) {
        self.open = false;
        self.matches.clear();
        self.current = None;
        self.searched = None;
        self.reveal = false;
    }

    /// Whether the bar is open.
    #[must_use]
    pub fn is_open(&self) -> bool {
        self.open
    }

    /// The query.
    #[must_use]
    pub fn query(&self) -> &str {
        &self.query
    }

    /// Replace the query. The next [`Self::search`] runs it.
    pub fn set_query(&mut self, query: impl Into<String>) {
        self.query = query.into();
    }

    /// Whether the search is case-sensitive. Default `false`.
    #[must_use]
    pub fn match_case(&self) -> bool {
        self.match_case
    }

    /// Make the search case-sensitive or not.
    pub fn set_match_case(&mut self, match_case: bool) {
        self.match_case = match_case;
    }

    /// Every match, ordered by provider, item and position.
    #[must_use]
    pub fn matches(&self) -> &[FindMatch] {
        &self.matches
    }

    /// Index of the current match in [`Self::matches`].
    #[must_use]
    pub fn current_index(&self) -> Option<usize> {
        self.current
    }

    /// The current match.
    #[must_use]
    pub fn current(&self) -> Option<&FindMatch> {
        self.current.and_then(|index| self.matches.get(index))
    }

    /// Forget the last search, so the next [`Self::search`] runs even if the query and the item
    /// counts did not change. Call it when the content changed in place.
    pub fn invalidate(&mut self) {
        self.searched = None;
    }

    /// Search `providers` for the query, unless the query, the case setting and every item
    /// count are what the last search ran with. Does nothing while the bar is closed.
    ///
    /// The current match stays on, or moves to the first match after, where it was.
    pub fn search(&mut self, providers: &[&dyn FindProvider]) {
        if !self.open {
            return;
        }
        let key = SearchKey {
            query: self.query.clone(),
            match_case: self.match_case,
            counts: providers.iter().map(|p| p.item_count()).collect(),
        };
        if self.searched.as_ref() == Some(&key) {
            return;
        }
        let previous = self.current().cloned();
        self.matches.clear();
        for (provider, source) in providers.iter().enumerate() {
            for item in 0..source.item_count() {
                let text = source.item_text(item);
                for range in find_in(&text, &self.query, self.match_case) {
                    self.matches.push(FindMatch {
                        provider,
                        item,
                        range,
                    });
                }
            }
        }
        self.current = if self.matches.is_empty() {
            None
        } else {
            let at = previous.map_or(0, |p| {
                self.matches.partition_point(|m| {
                    (m.provider, m.item, m.range.start) < (p.provider, p.item, p.range.start)
                })
            });
            Some(if at < self.matches.len() { at } else { 0 })
        };
        self.reveal = self.current.is_some();
        self.searched = Some(key);
    }

    /// Move to the next match, wrapping to the first.
    pub fn next(&mut self) {
        self.step(true);
    }

    /// Move to the previous match, wrapping to the last.
    pub fn previous(&mut self) {
        self.step(false);
    }

    fn step(&mut self, forward: bool) {
        let len = self.matches.len();
        if len == 0 {
            return;
        }
        self.current = Some(match (self.current, forward) {
            (None, true) => 0,
            (None, false) => len - 1,
            (Some(i), true) => (i + 1) % len,
            (Some(i), false) => (i + len - 1) % len,
        });
        self.reveal = true;
    }

    /// The match a view should scroll into view, once: `Some` after the current match changed,
    /// then `None` until it changes again.
    pub fn take_reveal(&mut self) -> Option<FindMatch> {
        if !std::mem::take(&mut self.reveal) {
            return None;
        }
        self.current().cloned()
    }

    /// The matches in one item, with `true` on the current one. Empty while the bar is closed.
    #[must_use]
    pub fn ranges(&self, provider: usize, item: usize) -> Vec<(Range<usize>, bool)> {
        if !self.open {
            return Vec::new();
        }
        let start = self
            .matches
            .partition_point(|m| (m.provider, m.item) < (provider, item));
        self.matches
            .get(start..)
            .unwrap_or_default()
            .iter()
            .enumerate()
            .take_while(|(_, m)| m.provider == provider && m.item == item)
            .map(|(offset, m)| (m.range.clone(), self.current == Some(start + offset)))
            .collect()
    }

    /// `text` (the item's text, or the part of it being painted) laid out with its matches
    /// highlighted, in the body font and the default text colour.
    #[must_use]
    pub fn layout_job(&self, ui: &Ui, provider: usize, item: usize, text: &str) -> LayoutJob {
        let font = egui::TextStyle::Body.resolve(ui.style());
        let color = ui.visuals().text_color();
        self.layout_job_with(ui.ctx(), provider, item, text, TextFormat::simple(font, color))
    }

    /// [`Self::layout_job`] with the caller's base format (font and colour) for unmatched text.
    #[must_use]
    pub fn layout_job_with(
        &self,
        ctx: &egui::Context,
        provider: usize,
        item: usize,
        text: &str,
        base: TextFormat,
    ) -> LayoutJob {
        let colors = MatchColors::new(&Theme::get(ctx));
        let mut job = LayoutJob::default();
        let mut at = 0;
        for (range, current) in self.ranges(provider, item) {
            // A range that does not fit `text` (the content changed since the search) is skipped.
            let (Some(before), Some(hit)) = (text.get(at..range.start), text.get(range.clone())) else {
                continue;
            };
            if !before.is_empty() {
                job.append(before, 0.0, base.clone());
            }
            job.append(hit, 0.0, colors.format(&base, current));
            at = range.end;
        }
        let rest = text.get(at..).unwrap_or_default();
        if !rest.is_empty() || job.sections.is_empty() {
            job.append(rest, 0.0, base);
        }
        job
    }
}

/// Highlight colours from the theme: palette yellow for matches and marigold for the current
/// one, as `TerminalView` paints its own matches; in high contrast, the highlight colour for the
/// current match and an underline for the rest.
struct MatchColors {
    high_contrast: bool,
    match_bg: Color32,
    current_bg: Color32,
    current_fg: Option<Color32>,
    underline: Color32,
}

impl MatchColors {
    fn new(theme: &Theme) -> Self {
        let high_contrast = theme.kind == ThemeKind::HighContrast;
        if high_contrast {
            Self {
                high_contrast,
                match_bg: Color32::TRANSPARENT,
                current_bg: c(theme.color.brand_background),
                current_fg: Some(c(theme.color.neutral_foreground_on_brand)),
                underline: c(theme.color.neutral_foreground_1),
            }
        } else {
            Self {
                high_contrast,
                match_bg: c(theme.color.palette_yellow_background_2),
                current_bg: c(theme.color.palette_marigold_background_3),
                current_fg: None,
                underline: Color32::TRANSPARENT,
            }
        }
    }

    fn format(&self, base: &TextFormat, current: bool) -> TextFormat {
        let mut format = base.clone();
        if current {
            format.background = self.current_bg;
            if let Some(fg) = self.current_fg {
                format.color = fg;
            }
        } else {
            format.background = self.match_bg;
            if self.high_contrast {
                format.underline = Stroke::new(1.0, self.underline);
            }
        }
        format
    }
}

/// What a [`FindBar`] reports after `show`.
#[derive(Clone, Debug, Default)]
pub struct FindBarResponse {
    /// The bar opened this frame (Ctrl+F).
    pub opened: bool,
    /// The bar closed this frame (Escape or its close button).
    pub closed: bool,
    /// The current match changed this frame (a new search, Enter, or the arrow buttons).
    pub moved: bool,
    /// The search field, while the bar is open.
    pub field: Option<Response>,
}

/// The find bar: a search field, a "3 of 12" counter, previous and next buttons, an optional
/// match-case toggle and a close button.
///
/// Call [`FindBar::show`] every frame, open or closed: it also listens for Ctrl+F (Cmd+F on
/// macOS), which opens the bar and focuses the field. While the field has focus, Enter goes to
/// the next match, Shift+Enter to the previous one and Escape closes the bar.
///
/// ```
/// use fluent2_egui::data::find::{FindBar, FindProvider, FindState, TextLines};
///
/// # fn demo(ui: &mut egui::Ui, state: &mut FindState, log: &str) {
/// let lines = TextLines::new(log);
/// let providers: [&dyn FindProvider; 1] = [&lines];
/// let response = FindBar::new("log-find").match_case_toggle(true).show(ui, state, &providers);
/// if response.closed {
///     // give focus back to the view
/// }
/// # }
/// ```
pub struct FindBar {
    id_salt: Id,
    placeholder: String,
    match_case_toggle: bool,
    shortcut: bool,
}

impl FindBar {
    /// A find bar. `id_salt` names its field and must be unique among siblings.
    pub fn new(id_salt: impl std::hash::Hash + std::fmt::Debug) -> Self {
        Self {
            id_salt: Id::unique(id_salt),
            placeholder: "Find".to_owned(),
            match_case_toggle: false,
            shortcut: true,
        }
    }

    /// Placeholder of the field, which is also its accessible name. Default "Find".
    #[must_use]
    pub fn placeholder(mut self, placeholder: impl Into<String>) -> Self {
        self.placeholder = placeholder.into();
        self
    }

    /// Show a match-case toggle. Default off.
    #[must_use]
    pub fn match_case_toggle(mut self, show: bool) -> Self {
        self.match_case_toggle = show;
        self
    }

    /// Listen for Ctrl+F (Cmd+F on macOS). Default on. Turn it off when several find bars
    /// share a window and the app decides which one opens.
    #[must_use]
    pub fn shortcut(mut self, listen: bool) -> Self {
        self.shortcut = listen;
        self
    }

    /// Handle the keys, search `providers`, and paint the bar when it is open.
    pub fn show(
        self,
        ui: &mut Ui,
        state: &mut FindState,
        providers: &[&dyn FindProvider],
    ) -> FindBarResponse {
        let mut out = FindBarResponse::default();
        let before = state.current().cloned();

        let find = KeyboardShortcut::new(Modifiers::COMMAND, Key::F);
        if self.shortcut && ui.is_enabled() && ui.input_mut(|input| input.consume_shortcut(&find)) {
            out.opened = !state.open;
            state.open();
        }
        if !state.open {
            return out;
        }

        let field_focused = state
            .field_id
            .is_some_and(|id| ui.memory(|memory| memory.had_focus_last_frame(id)));
        if field_focused {
            if ui.input_mut(|input| input.consume_key(Modifiers::NONE, Key::Escape)) {
                state.close();
                out.closed = true;
                return out;
            }
            if ui.input_mut(|input| input.consume_key(Modifiers::SHIFT, Key::Enter)) {
                state.search(providers);
                state.previous();
            } else if ui.input_mut(|input| input.consume_key(Modifiers::NONE, Key::Enter)) {
                state.search(providers);
                state.next();
            }
        }

        let mut close = false;
        let mut step = None;
        let mut match_case = state.match_case;
        ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
            ui.spacing_mut().item_spacing.x = spacing_horizontal::XS;
            if ui
                .add(Button::icon_only(Icon::Dismiss, "Close find").id_salt(self.id_salt.with("close")))
                .clicked()
            {
                close = true;
            }
            if ui
                .add(Button::icon_only(Icon::ChevronDown, "Next match").id_salt(self.id_salt.with("next")))
                .clicked()
            {
                step = Some(true);
            }
            if ui
                .add(
                    Button::icon_only(Icon::ChevronUp, "Previous match")
                        .id_salt(self.id_salt.with("previous")),
                )
                .clicked()
            {
                step = Some(false);
            }
            if self.match_case_toggle {
                ui.add(ToggleButton::new(&mut match_case, "Aa").id_salt(self.id_salt.with("case")));
            }
            ui.label(counter_text(state));
            let field = ui.add(
                SearchBox::new(&mut state.query)
                    .placeholder(self.placeholder.clone())
                    .label(self.placeholder.clone())
                    .id_salt(self.id_salt.with("field")),
            );
            state.field_id = Some(field.id);
            if std::mem::take(&mut state.focus_pending) {
                field.request_focus();
            }
            out.field = Some(field);
        });

        if close {
            state.close();
            out.closed = true;
            return out;
        }
        state.match_case = match_case;
        state.search(providers);
        match step {
            Some(true) => state.next(),
            Some(false) => state.previous(),
            None => {}
        }
        out.moved = state.current() != before.as_ref() || step.is_some();
        out
    }
}

/// "3 of 12", "No results", or nothing for an empty query.
fn counter_text(state: &FindState) -> String {
    if state.query.is_empty() {
        String::new()
    } else if state.matches.is_empty() {
        "No results".to_owned()
    } else {
        let current = state.current.map_or(0, |i| i + 1);
        format!("{current} of {}", state.matches.len())
    }
}

/// Stop the browser's own find from taking Ctrl+F (Cmd+F), so a [`FindBar`] gets it.
///
/// Call it once at start-up. It does anything only on `wasm32` with the `web-find` feature,
/// where it adds a capturing `keydown` listener to the window that cancels the browser's default
/// for Ctrl+F and Cmd+F. The key event still reaches egui. Calling it again does nothing.
pub fn capture_browser_find() {
    #[cfg(all(target_arch = "wasm32", feature = "web-find"))]
    web::capture();
}

#[cfg(all(target_arch = "wasm32", feature = "web-find"))]
mod web {
    use std::sync::atomic::{AtomicBool, Ordering};

    use wasm_bindgen::JsCast;
    use wasm_bindgen::closure::Closure;

    static INSTALLED: AtomicBool = AtomicBool::new(false);

    pub(super) fn capture() {
        if INSTALLED.swap(true, Ordering::Relaxed) {
            return;
        }
        let Some(window) = web_sys::window() else {
            INSTALLED.store(false, Ordering::Relaxed);
            return;
        };
        let listener = Closure::<dyn FnMut(web_sys::KeyboardEvent)>::new(|event: web_sys::KeyboardEvent| {
            if (event.ctrl_key() || event.meta_key())
                && !event.alt_key()
                && event.key().eq_ignore_ascii_case("f")
            {
                event.prevent_default();
            }
        });
        let added = window.add_event_listener_with_callback_and_bool(
            "keydown",
            listener.as_ref().unchecked_ref(),
            true,
        );
        if added.is_err() {
            INSTALLED.store(false, Ordering::Relaxed);
            return;
        }
        // The listener lives as long as the page.
        listener.forget();
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn folds_case_on_char_boundaries() {
        assert_eq!(find_in("ÄBC äbc", "äb", false), vec![0..3, 5..8]);
        assert_eq!(find_in("aaaa", "aa", false), vec![0..2, 2..4]);
        assert!(find_in("abc", "", false).is_empty());
        assert!(find_in("ab", "abc", false).is_empty());
    }

    #[test]
    fn search_keeps_the_current_match_and_wraps() {
        let lines = vec!["one".to_owned(), "two one".to_owned(), "three".to_owned()];
        let providers: [&dyn FindProvider; 1] = [&lines];
        let mut state = FindState::default();
        state.open();
        state.set_query("o");
        state.search(&providers);
        assert_eq!(state.matches().len(), 3);
        state.next();
        state.next();
        assert_eq!(state.current().map(|m| (m.item, m.range.start)), Some((1, 4)));
        state.set_query("on");
        state.search(&providers);
        assert_eq!(state.current().map(|m| (m.item, m.range.start)), Some((1, 4)));
        state.next();
        assert_eq!(state.current_index(), Some(0), "wraps to the first match");
        state.previous();
        assert_eq!(state.current_index(), Some(1), "wraps back to the last");
    }

    #[test]
    fn ranges_mark_the_current_match_and_vanish_when_closed() {
        let lines = vec!["aXa".to_owned()];
        let providers: [&dyn FindProvider; 1] = [&lines];
        let mut state = FindState::default();
        state.open();
        state.set_query("a");
        state.search(&providers);
        assert_eq!(state.ranges(0, 0), vec![(0..1, true), (2..3, false)]);
        assert!(state.ranges(1, 0).is_empty());
        state.close();
        assert!(state.ranges(0, 0).is_empty());
    }
}
