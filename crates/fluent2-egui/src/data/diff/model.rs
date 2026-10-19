//! The diff model behind DiffView: line hunks with context, the inline and side-by-side row
//! layouts, and word-level changes inside changed lines. Built on `imara-diff` (Apache-2.0)
//! with the Histogram algorithm. Zed's `buffer_diff` (GPL) was a design reference only.

use std::fmt;
use std::ops::Range;

use imara_diff::{Algorithm, Diff, Hunk, InternedInput};
use unicode_width::UnicodeWidthChar;

/// Lines longer than this many columns get no word-level highlights, to bound the work.
pub const MAX_WORD_DIFF_COLUMNS: usize = 1_000;

/// Columns a tab advances to (the next multiple of this).
pub const TAB_WIDTH: usize = 4;

/// Default number of unchanged lines shown around each change.
pub const DEFAULT_CONTEXT: usize = 3;

/// One `@@ -a,b +c,d @@` hunk: 0-based first line and line count on each side.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct HunkHeader {
    /// First old line (0-based).
    pub old_start: usize,
    /// Number of old lines in the hunk, context included.
    pub old_len: usize,
    /// First new line (0-based).
    pub new_start: usize,
    /// Number of new lines in the hunk, context included.
    pub new_len: usize,
}

impl fmt::Display for HunkHeader {
    /// Unified diff form. A side with no lines shows the line before it, as `diff -u` does.
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let shown = |start: usize, len: usize| if len == 0 { start } else { start + 1 };
        write!(
            f,
            "@@ -{},{} +{},{} @@",
            shown(self.old_start, self.old_len),
            self.old_len,
            shown(self.new_start, self.new_len),
            self.new_len
        )
    }
}

/// What a line in the diff is.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum LineKind {
    /// Unchanged, on both sides.
    Context,
    /// Only in the old text.
    Removed,
    /// Only in the new text.
    Added,
}

/// A row of the inline (unified) layout.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum InlineRow {
    /// The header of hunk `n` (an index into [`DiffModel::hunks`]).
    Hunk(usize),
    /// A line. `old` and `new` are 0-based line indices; a context line has both.
    Line {
        /// Context, removed or added.
        kind: LineKind,
        /// Old line index, for context and removed lines.
        old: Option<usize>,
        /// New line index, for context and added lines.
        new: Option<usize>,
    },
}

/// A row of the side-by-side layout: the old line on the left, the new one on the right.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum SplitRow {
    /// The header of hunk `n` (an index into [`DiffModel::hunks`]).
    Hunk(usize),
    /// An aligned pair. Inside a change the k-th removed line sits beside the k-th added line;
    /// the longer side's extra lines face an empty cell (`None`).
    Pair {
        /// Old line index, if this row has one.
        old: Option<usize>,
        /// New line index, if this row has one.
        new: Option<usize>,
        /// Whether the row is part of a change (false for context).
        changed: bool,
    },
}

/// A computed diff of two texts, ready for [`super::DiffView`]. Build it once when the texts
/// change, not every frame.
///
/// Lines are compared without their terminators, so a `\r\n` versus `\n` change, or a missing
/// final newline, is not a difference. Tabs are expanded to [`TAB_WIDTH`] columns.
///
/// ```
/// use fluent2_egui::data::diff::DiffModel;
///
/// let model = DiffModel::new("a\nb\nc\n", "a\nB\nc\n");
/// assert_eq!(model.hunks().len(), 1);
/// assert_eq!(model.hunks()[0].to_string(), "@@ -1,3 +1,3 @@");
/// assert_eq!((model.removals(), model.additions()), (1, 1));
/// ```
#[derive(Clone, Debug, Default)]
pub struct DiffModel {
    old: Vec<String>,
    new: Vec<String>,
    old_words: Vec<Vec<Range<usize>>>,
    new_words: Vec<Vec<Range<usize>>>,
    hunks: Vec<HunkHeader>,
    inline: Vec<InlineRow>,
    split: Vec<SplitRow>,
    additions: usize,
    removals: usize,
    max_columns: usize,
}

impl DiffModel {
    /// Diff `old` against `new` with [`DEFAULT_CONTEXT`] lines of context.
    #[must_use]
    pub fn new(old: &str, new: &str) -> Self {
        Self::with_context(old, new, DEFAULT_CONTEXT)
    }

    /// Diff `old` against `new`, showing `context` unchanged lines around each change. Changes
    /// closer than `2 * context` lines share a hunk. `usize::MAX` shows the whole file.
    #[must_use]
    pub fn with_context(old: &str, new: &str, context: usize) -> Self {
        let old: Vec<String> = split_lines(old);
        let new: Vec<String> = split_lines(new);

        let mut input: InternedInput<&str> = InternedInput::default();
        input.update_before(old.iter().map(String::as_str));
        input.update_after(new.iter().map(String::as_str));
        let mut diff = Diff::compute(Algorithm::Histogram, &input);
        diff.postprocess_lines(&input);
        let raw: Vec<Hunk> = diff.hunks().collect();

        let mut model = Self {
            old_words: vec![Vec::new(); old.len()],
            new_words: vec![Vec::new(); new.len()],
            max_columns: old.iter().chain(&new).map(|l| columns(l)).max().unwrap_or(0),
            old,
            new,
            ..Self::default()
        };
        model.build(&raw, context);
        model
    }

    fn build(&mut self, raw: &[Hunk], context: usize) {
        let (old_len, new_len) = (self.old.len(), self.new.len());
        let mut i = 0;
        let mut prev_end = 0usize; // old line after the previous group's last change
        while let Some(first) = raw.get(i) {
            // Extend the group while the gap to the next change is at most 2 * context.
            let mut j = i;
            let mut last = first;
            while let Some(next) = raw.get(j + 1) {
                let gap = range(&next.before).start - range(&last.before).end;
                if gap > context.saturating_mul(2) {
                    break;
                }
                j += 1;
                last = next;
            }
            let (first_old, first_new) = (range(&first.before), range(&first.after));
            let (last_old, last_new) = (range(&last.before), range(&last.after));

            // Unchanged lines before a change are the same on both sides, so the lead and
            // trail counts are too.
            let lead = context.min(first_old.start - prev_end.min(first_old.start));
            let trail = context.min(old_len.saturating_sub(last_old.end));
            let header = HunkHeader {
                old_start: first_old.start - lead,
                old_len: (last_old.end + trail) - (first_old.start - lead),
                new_start: first_new.start.saturating_sub(lead),
                new_len: (last_new.end + trail).min(new_len) - first_new.start.saturating_sub(lead),
            };
            let index = self.hunks.len();
            self.hunks.push(header);
            self.inline.push(InlineRow::Hunk(index));
            self.split.push(SplitRow::Hunk(index));

            let (mut o, mut n) = (header.old_start, header.new_start);
            for hunk in raw.get(i..=j).unwrap_or_default() {
                let (before, after) = (range(&hunk.before), range(&hunk.after));
                while o < before.start {
                    self.context_row(o, n);
                    o += 1;
                    n += 1;
                }
                self.change_rows(before.clone(), after.clone());
                o = before.end;
                n = after.end;
            }
            for _ in 0..trail {
                self.context_row(o, n);
                o += 1;
                n += 1;
            }
            prev_end = last_old.end;
            i = j + 1;
        }
    }

    fn context_row(&mut self, old: usize, new: usize) {
        self.inline.push(InlineRow::Line {
            kind: LineKind::Context,
            old: Some(old),
            new: Some(new),
        });
        self.split.push(SplitRow::Pair {
            old: Some(old),
            new: Some(new),
            changed: false,
        });
    }

    fn change_rows(&mut self, before: Range<usize>, after: Range<usize>) {
        self.removals += before.len();
        self.additions += after.len();
        for old in before.clone() {
            self.inline.push(InlineRow::Line {
                kind: LineKind::Removed,
                old: Some(old),
                new: None,
            });
        }
        for new in after.clone() {
            self.inline.push(InlineRow::Line {
                kind: LineKind::Added,
                old: None,
                new: Some(new),
            });
        }
        for k in 0..before.len().max(after.len()) {
            let old = (before.start + k < before.end).then_some(before.start + k);
            let new = (after.start + k < after.end).then_some(after.start + k);
            if let (Some(o), Some(n)) = (old, new) {
                let (a, b) = word_changes(&self.old[o], &self.new[n]);
                if let Some(slot) = self.old_words.get_mut(o) {
                    *slot = a;
                }
                if let Some(slot) = self.new_words.get_mut(n) {
                    *slot = b;
                }
            }
            self.split.push(SplitRow::Pair {
                old,
                new,
                changed: true,
            });
        }
    }

    /// The old text's lines (terminators removed, tabs expanded).
    #[must_use]
    pub fn old_lines(&self) -> &[String] {
        &self.old
    }

    /// The new text's lines (terminators removed, tabs expanded).
    #[must_use]
    pub fn new_lines(&self) -> &[String] {
        &self.new
    }

    /// Changed column ranges inside old line `line`; empty for unchanged lines and for removed
    /// lines with no added partner.
    #[must_use]
    pub fn old_word_changes(&self, line: usize) -> &[Range<usize>] {
        self.old_words.get(line).map_or(&[], Vec::as_slice)
    }

    /// Changed column ranges inside new line `line`.
    #[must_use]
    pub fn new_word_changes(&self, line: usize) -> &[Range<usize>] {
        self.new_words.get(line).map_or(&[], Vec::as_slice)
    }

    /// The hunks, in order.
    #[must_use]
    pub fn hunks(&self) -> &[HunkHeader] {
        &self.hunks
    }

    /// Rows of the inline layout.
    #[must_use]
    pub fn inline_rows(&self) -> &[InlineRow] {
        &self.inline
    }

    /// Rows of the side-by-side layout.
    #[must_use]
    pub fn split_rows(&self) -> &[SplitRow] {
        &self.split
    }

    /// Number of added lines.
    #[must_use]
    pub fn additions(&self) -> usize {
        self.additions
    }

    /// Number of removed lines.
    #[must_use]
    pub fn removals(&self) -> usize {
        self.removals
    }

    /// Whether the texts have no line differences.
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.hunks.is_empty()
    }

    /// Width in columns of the widest line on either side.
    #[must_use]
    pub fn max_columns(&self) -> usize {
        self.max_columns
    }
}

fn range(r: &Range<u32>) -> Range<usize> {
    r.start as usize..r.end as usize
}

/// Lines without terminators, tabs expanded. A trailing newline does not add an empty line.
fn split_lines(text: &str) -> Vec<String> {
    text.lines().map(expand_tabs).collect()
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
            col += char_columns(ch);
        }
    }
    out
}

/// Display columns of `ch`: 2 for wide characters, 0 for combining marks.
#[must_use]
pub(crate) fn char_columns(ch: char) -> usize {
    ch.width().unwrap_or(0)
}

/// Display columns of `text`.
#[must_use]
pub(crate) fn columns(text: &str) -> usize {
    text.chars().map(char_columns).sum()
}

/// Word tokens: runs of letters, digits and `_`, runs of whitespace, and single other chars.
fn words(line: &str) -> Vec<&str> {
    #[derive(PartialEq)]
    enum Class {
        Word,
        Space,
        Other,
    }
    let class = |c: char| {
        if c.is_alphanumeric() || c == '_' {
            Class::Word
        } else if c.is_whitespace() {
            Class::Space
        } else {
            Class::Other
        }
    };
    let mut out = Vec::new();
    let mut start = 0;
    let mut current: Option<Class> = None;
    for (i, ch) in line.char_indices() {
        let k = class(ch);
        let split = match &current {
            None => false,
            Some(Class::Other) => true,
            Some(prev) => *prev != k,
        };
        if split {
            out.push(&line[start..i]);
            start = i;
        }
        current = Some(k);
    }
    if start < line.len() {
        out.push(&line[start..]);
    }
    out
}

/// Changed column ranges in `old` and `new`, from a word diff. Adjacent ranges are merged.
/// Lines wider than [`MAX_WORD_DIFF_COLUMNS`] get none.
#[must_use]
pub(crate) fn word_changes(old: &str, new: &str) -> (Vec<Range<usize>>, Vec<Range<usize>>) {
    if columns(old) > MAX_WORD_DIFF_COLUMNS || columns(new) > MAX_WORD_DIFF_COLUMNS {
        return (Vec::new(), Vec::new());
    }
    let (a, b) = (words(old), words(new));
    let mut input: InternedInput<&str> = InternedInput::default();
    input.update_before(a.iter().copied());
    input.update_after(b.iter().copied());
    let diff = Diff::compute(Algorithm::Myers, &input);
    let starts = |tokens: &[&str]| -> Vec<usize> {
        let mut acc = 0;
        let mut out = Vec::with_capacity(tokens.len() + 1);
        out.push(0);
        for t in tokens {
            acc += columns(t);
            out.push(acc);
        }
        out
    };
    let (sa, sb) = (starts(&a), starts(&b));
    let mut old_ranges: Vec<Range<usize>> = Vec::new();
    let mut new_ranges: Vec<Range<usize>> = Vec::new();
    let col = |s: &[usize], i: u32| s.get(i as usize).copied().unwrap_or(0);
    for hunk in diff.hunks() {
        push_merged(
            &mut old_ranges,
            col(&sa, hunk.before.start)..col(&sa, hunk.before.end),
        );
        push_merged(
            &mut new_ranges,
            col(&sb, hunk.after.start)..col(&sb, hunk.after.end),
        );
    }
    (old_ranges, new_ranges)
}

fn push_merged(ranges: &mut Vec<Range<usize>>, r: Range<usize>) {
    if r.is_empty() {
        return;
    }
    match ranges.last_mut() {
        Some(last) if last.end >= r.start => last.end = last.end.max(r.end),
        _ => ranges.push(r),
    }
}

#[cfg(test)]
#[allow(clippy::single_range_in_vec_init)]
mod tests {
    use super::{expand_tabs, word_changes, words};

    #[test]
    fn words_split_by_class() {
        assert_eq!(
            words("let x_1 = f(a);"),
            vec!["let", " ", "x_1", " ", "=", " ", "f", "(", "a", ")", ";"]
        );
        assert!(words("").is_empty());
    }

    #[test]
    fn tabs_expand_to_stops() {
        assert_eq!(expand_tabs("\tx"), "    x");
        assert_eq!(expand_tabs("ab\tc"), "ab  c");
    }

    #[test]
    fn word_changes_mark_only_the_changed_word() {
        let (a, b) = word_changes("let count = 1;", "let total = 1;");
        assert_eq!(a, vec![4..9]);
        assert_eq!(b, vec![4..9]);
    }

    #[test]
    fn word_changes_handle_wide_chars_in_columns() {
        // "日本" is 4 columns wide, so the changed word after it starts at column 5.
        let (a, b) = word_changes("日本 old", "日本 new");
        assert_eq!(a, vec![5..8]);
        assert_eq!(b, vec![5..8]);
    }
}
