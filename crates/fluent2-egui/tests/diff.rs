//! DiffView and its model: hunk, context and alignment fixtures, word-level changes, then the
//! view in every theme through kittest (no wgpu). Needs the `diff` feature.

#![cfg(feature = "diff")]
// Expected ranges are written as one-element vectors on purpose.
#![allow(clippy::single_range_in_vec_init)]

use std::cell::RefCell;
use std::ops::Range;
use std::rc::Rc;

use egui::Vec2;
use egui_kittest::Harness;
use fluent2_egui::data::diff::{
    DiffColors, DiffMode, DiffModel, DiffView, HunkHeader, InlineRow, LineKind, SplitRow,
};
use fluent2_egui::{Theme, install};

fn numbered(n: usize) -> String {
    (1..=n).map(|i| format!("line {i}\n")).collect()
}

fn header(model: &DiffModel, n: usize) -> String {
    model.hunks().get(n).map(ToString::to_string).unwrap_or_default()
}

fn kinds(model: &DiffModel) -> String {
    model
        .inline_rows()
        .iter()
        .map(|r| match r {
            InlineRow::Hunk(_) => '@',
            InlineRow::Line {
                kind: LineKind::Context,
                ..
            } => ' ',
            InlineRow::Line {
                kind: LineKind::Added,
                ..
            } => '+',
            InlineRow::Line {
                kind: LineKind::Removed,
                ..
            } => '-',
        })
        .collect()
}

// ── Hunks and context ─────────────────────────────────────────────────────────

#[test]
fn identical_texts_have_no_rows() {
    let model = DiffModel::new("a\nb\n", "a\nb\n");
    assert!(model.is_empty());
    assert!(model.inline_rows().is_empty());
    assert!(model.split_rows().is_empty());
    assert_eq!((model.additions(), model.removals()), (0, 0));
}

#[test]
fn line_terminators_are_not_differences() {
    assert!(DiffModel::new("a\r\nb\r\n", "a\nb").is_empty());
}

#[test]
fn empty_to_text_is_one_added_hunk() {
    let model = DiffModel::new("", "x\ny\n");
    assert_eq!(header(&model, 0), "@@ -0,0 +1,2 @@");
    assert_eq!(kinds(&model), "@++");
    assert_eq!(model.additions(), 2);
}

#[test]
fn text_to_empty_is_one_removed_hunk() {
    let model = DiffModel::new("x\ny\n", "");
    assert_eq!(header(&model, 0), "@@ -1,2 +0,0 @@");
    assert_eq!(kinds(&model), "@--");
}

#[test]
fn change_gets_three_lines_of_context() {
    let old = numbered(20);
    let new = old.replace("line 10\n", "line ten\n");
    let model = DiffModel::new(&old, &new);
    assert_eq!(model.hunks().len(), 1);
    assert_eq!(
        model.hunks()[0],
        HunkHeader {
            old_start: 6,
            old_len: 7,
            new_start: 6,
            new_len: 7
        }
    );
    assert_eq!(header(&model, 0), "@@ -7,7 +7,7 @@");
    assert_eq!(kinds(&model), "@   -+   ");
}

#[test]
fn context_is_clipped_at_the_file_edges() {
    let model = DiffModel::new("a\nb\nc\n", "A\nb\nc\n");
    assert_eq!(header(&model, 0), "@@ -1,3 +1,3 @@");
    assert_eq!(kinds(&model), "@-+  ");
}

#[test]
fn near_changes_share_a_hunk_far_ones_do_not() {
    let old = numbered(40);
    // Lines 5 and 11 are 5 apart (<= 6): one hunk. Line 30 is far: a second hunk.
    let new = old
        .replace("line 5\n", "five\n")
        .replace("line 11\n", "eleven\n")
        .replace("line 30\n", "thirty\n");
    let model = DiffModel::new(&old, &new);
    assert_eq!(model.hunks().len(), 2);
    assert_eq!(header(&model, 0), "@@ -2,13 +2,13 @@");
    assert_eq!(header(&model, 1), "@@ -27,7 +27,7 @@");
}

#[test]
fn insertion_shifts_new_line_numbers() {
    let old = numbered(10);
    let new = old.replace("line 5\n", "line 5\nnew a\nnew b\n");
    let model = DiffModel::new(&old, &new);
    assert_eq!(header(&model, 0), "@@ -3,6 +3,8 @@");
    assert_eq!(kinds(&model), "@   ++   ");
    // The context line after the insertion is old line 6, new line 8 (0-based 5 and 7).
    assert!(model.inline_rows().contains(&InlineRow::Line {
        kind: LineKind::Context,
        old: Some(5),
        new: Some(7),
    }));
}

#[test]
fn zero_context_shows_only_changes() {
    let old = numbered(10);
    let new = old.replace("line 3\n", "x\n").replace("line 6\n", "y\n");
    let model = DiffModel::with_context(&old, &new, 0);
    assert_eq!(model.hunks().len(), 2);
    assert_eq!(kinds(&model), "@-+@-+");
}

#[test]
fn whole_file_context_is_one_hunk() {
    let old = numbered(50);
    let new = old.replace("line 2\n", "x\n").replace("line 48\n", "y\n");
    let model = DiffModel::with_context(&old, &new, usize::MAX);
    assert_eq!(header(&model, 0), "@@ -1,50 +1,50 @@");
    assert_eq!(model.inline_rows().len(), 1 + 50 + 2);
}

// ── Side-by-side alignment ────────────────────────────────────────────────────

#[test]
fn split_pairs_removed_with_added_and_pads_the_shorter_side() {
    let model = DiffModel::with_context("a\nb\nc\nz\n", "A\nz\n", 1);
    assert_eq!(
        model.split_rows(),
        &[
            SplitRow::Hunk(0),
            SplitRow::Pair {
                old: Some(0),
                new: Some(0),
                changed: true
            },
            SplitRow::Pair {
                old: Some(1),
                new: None,
                changed: true
            },
            SplitRow::Pair {
                old: Some(2),
                new: None,
                changed: true
            },
            SplitRow::Pair {
                old: Some(3),
                new: Some(1),
                changed: false
            },
        ]
    );
}

#[test]
fn split_and_inline_cover_the_same_lines() {
    let old = numbered(200);
    let new: String = old
        .lines()
        .enumerate()
        .filter(|(i, _)| i % 17 != 0)
        .map(|(i, l)| {
            if i % 23 == 0 {
                format!("{l} changed\n")
            } else {
                format!("{l}\n")
            }
        })
        .collect();
    let model = DiffModel::new(&old, &new);
    let mut a: (Vec<usize>, Vec<usize>) = Default::default();
    for r in model.inline_rows() {
        if let InlineRow::Line { old, new, .. } = *r {
            a.0.extend(old);
            a.1.extend(new);
        }
    }
    let mut b: (Vec<usize>, Vec<usize>) = Default::default();
    for r in model.split_rows() {
        if let SplitRow::Pair { old, new, .. } = *r {
            b.0.extend(old);
            b.1.extend(new);
        }
    }
    a.0.sort_unstable();
    a.1.sort_unstable();
    b.0.sort_unstable();
    b.1.sort_unstable();
    assert_eq!(a, b);
    // Every hunk's counts match the lines listed under it.
    let hunk_total: usize = model.hunks().iter().map(|h| h.old_len).sum();
    assert_eq!(hunk_total, a.0.len());
}

// ── Word-level changes ────────────────────────────────────────────────────────

fn words(model: &DiffModel) -> (Vec<Range<usize>>, Vec<Range<usize>>) {
    (
        model.old_word_changes(0).to_vec(),
        model.new_word_changes(0).to_vec(),
    )
}

#[test]
fn word_changes_mark_the_changed_token() {
    let model = DiffModel::new("let count = 1;\n", "let count = 2;\n");
    assert_eq!(words(&model), (vec![12..13], vec![12..13]));
}

#[test]
fn word_changes_handle_insertions_and_tabs() {
    let model = DiffModel::new("\tf(a)\n", "\tf(a, b)\n");
    // The tab expands to 4 columns; ", b" is inserted after "a".
    assert_eq!(model.new_lines()[0], "    f(a, b)");
    assert_eq!(words(&model), (vec![], vec![7..10]));
}

#[test]
fn unpaired_lines_have_no_word_changes() {
    let model = DiffModel::new("a\n", "a\nb\n");
    assert!(model.new_word_changes(1).is_empty());
    assert!(model.old_word_changes(99).is_empty());
}

#[test]
fn very_long_lines_skip_the_word_diff() {
    let old = "x".repeat(5_000) + "\n";
    let new = "y".repeat(5_000) + "\n";
    let model = DiffModel::new(&old, &new);
    assert_eq!(words(&model), (vec![], vec![]));
}

#[test]
fn ten_thousand_line_diff_builds() {
    let old = numbered(10_000);
    let new: String = old
        .lines()
        .map(|l| {
            if l.ends_with('7') {
                format!("{l}!\n")
            } else {
                format!("{l}\n")
            }
        })
        .collect();
    let model = DiffModel::new(&old, &new);
    assert_eq!(model.removals(), 1_000);
    assert_eq!(model.additions(), 1_000);
}

// ── View ──────────────────────────────────────────────────────────────────────

fn themes() -> [Theme; 3] {
    [
        Theme::light(fluent2_tokens::FLUENT),
        Theme::dark(fluent2_tokens::FLUENT),
        Theme::high_contrast(),
    ]
}

fn run_view(model: Rc<DiffModel>, mode: DiffMode, theme: Theme) -> Range<usize> {
    let seen = Rc::new(RefCell::new(0..0));
    let out = Rc::clone(&seen);
    let mut harness = Harness::builder()
        .with_size(Vec2::new(500.0, 300.0))
        .build_ui(move |ui| {
            install(ui.ctx(), &theme);
            let response = DiffView::new("diff").mode(mode).show(ui, &model);
            *out.borrow_mut() = response.visible_rows;
        });
    harness.run();
    seen.borrow().clone()
}

#[test]
fn view_paints_only_the_visible_rows() {
    let old = numbered(10_000);
    let new = old.replace('3', "three");
    let model = Rc::new(DiffModel::with_context(&old, &new, usize::MAX));
    for mode in [DiffMode::Inline, DiffMode::SideBySide] {
        let rows = run_view(Rc::clone(&model), mode, Theme::default());
        assert_eq!(rows.start, 0);
        assert!(rows.end > 5 && rows.end < 40, "{mode:?}: painted {rows:?}");
    }
}

#[test]
fn view_runs_in_every_theme_and_mode_without_panics() {
    let model = Rc::new(DiffModel::new("a\nb\tc\n日本\n", "a\nB\tc\n日本語\nnew\n"));
    for theme in themes() {
        for mode in [DiffMode::Inline, DiffMode::SideBySide] {
            let rows = run_view(Rc::clone(&model), mode, theme.clone());
            assert!(rows.end <= model.inline_rows().len().max(model.split_rows().len()));
        }
    }
    let empty = Rc::new(DiffModel::new("", ""));
    assert_eq!(run_view(empty, DiffMode::Inline, Theme::default()), 0..0);
}

#[test]
fn colors_differ_in_light_and_dark_and_use_strokes_in_high_contrast() {
    for theme in &themes()[..2] {
        let colors = DiffColors::new(theme);
        assert_ne!(colors.added_line, colors.removed_line);
        assert_ne!(colors.added_line, colors.background);
        assert_ne!(colors.added_word, colors.added_line);
        assert!(colors.outline.is_none());
    }
    let hc = DiffColors::new(&Theme::high_contrast());
    let outline = hc.outline.unwrap_or(hc.background);
    assert_ne!(outline, hc.background);
}
