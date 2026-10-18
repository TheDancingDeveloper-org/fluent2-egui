//! TerminalView and its grid model: ANSI fixtures through the `vte` grid, then the view in
//! every theme through kittest (no wgpu). Needs the `terminal` feature.

#![cfg(feature = "terminal")]

use std::cell::RefCell;

use egui::accesskit::Role;
use egui::{Event, Modifiers, PointerButton, Pos2, Vec2};
use egui_kittest::Harness;
use egui_kittest::kittest::{NodeT, Queryable};
use fluent2_egui::data::terminal::{
    Cell, GridPos, TermColor, TerminalGrid, TerminalState, TerminalView, ansi_palette,
};
use fluent2_egui::{Theme, install};

fn grid(cols: usize, rows: usize, bytes: &[u8]) -> TerminalGrid {
    let mut g = TerminalGrid::new(cols, rows);
    g.process(bytes);
    g
}

fn cell(g: &TerminalGrid, row: usize, col: usize) -> Cell {
    g.line(g.screen_top() + row as u64)
        .and_then(|l| l.get(col).copied())
        .unwrap_or_default()
}

// ── ANSI fixtures ──────────────────────────────────────────────────────────────

#[test]
fn plain_text_crlf_and_trailing_blanks() {
    let g = grid(8, 3, b"hi\r\nthere");
    assert_eq!(g.visible_text(), "hi\nthere\n");
    assert_eq!(g.cursor(), (1, 5));
}

#[test]
fn sgr_basic_bright_256_and_truecolor() {
    let g = grid(
        20,
        2,
        b"\x1b[31ma\x1b[92mb\x1b[38;5;200mc\x1b[38;2;1;2;3md\x1b[48;5;4me\x1b[0mf",
    );
    assert_eq!(cell(&g, 0, 0).fg, TermColor::Indexed(1));
    assert_eq!(cell(&g, 0, 1).fg, TermColor::Indexed(10));
    assert_eq!(cell(&g, 0, 2).fg, TermColor::Indexed(200));
    assert_eq!(cell(&g, 0, 3).fg, TermColor::Rgb(1, 2, 3));
    assert_eq!(cell(&g, 0, 4).bg, TermColor::Indexed(4));
    assert_eq!(
        cell(&g, 0, 5),
        Cell {
            ch: 'f',
            ..Cell::default()
        }
    );
}

#[test]
fn sgr_colon_form_and_attributes() {
    let g = grid(10, 1, b"\x1b[38:2::9:8:7;1;3;4;7mx\x1b[22;23;24;27my");
    let x = cell(&g, 0, 0);
    assert_eq!(x.fg, TermColor::Rgb(9, 8, 7));
    assert!(x.bold && x.italic && x.underline && x.inverse);
    let y = cell(&g, 0, 1);
    assert!(!y.bold && !y.italic && !y.underline && !y.inverse);
}

#[test]
fn autowrap_happens_on_the_next_character() {
    let mut g = grid(4, 3, b"abcd");
    assert_eq!(g.cursor(), (0, 3));
    assert_eq!(g.visible_text(), "abcd\n\n");
    g.process(b"e");
    assert_eq!(g.visible_text(), "abcd\ne\n");
}

#[test]
fn scrollback_keeps_lines_and_evicts_past_the_limit() {
    let mut g = TerminalGrid::with_scrollback(6, 2, 3);
    for i in 0..10 {
        g.process(format!("line{i}\r\n").as_bytes());
    }
    assert_eq!(g.history_len(), 3);
    assert_eq!(g.first_line(), 6);
    assert_eq!(g.line_text(6), "line6");
    assert!(g.line(5).is_none());
    assert_eq!(g.visible_text(), "line9\n");
}

#[test]
fn scroll_offset_views_history_and_clamps() {
    let mut g = grid(6, 2, b"a\r\nb\r\nc\r\nd");
    assert_eq!(g.view_text(), "c\nd");
    g.scroll_by(1);
    assert_eq!(g.view_text(), "b\nc");
    g.scroll_by(100);
    assert_eq!(g.scroll_offset(), 2);
    assert_eq!(g.view_text(), "a\nb");
    g.scroll_by(-100);
    assert_eq!(g.scroll_offset(), 0);
}

#[test]
fn erase_in_display_and_line() {
    let mut g = grid(5, 2, b"abcde\r\nfghij");
    g.process(b"\x1b[1;3H\x1b[K");
    assert_eq!(g.visible_text(), "ab\nfghij");
    g.process(b"\x1b[2;3H\x1b[1K");
    assert_eq!(g.visible_text(), "ab\n   ij");
    g.process(b"\x1b[2J");
    assert_eq!(g.visible_text(), "\n");
}

#[test]
fn insert_delete_and_erase_characters() {
    let mut g = grid(6, 1, b"abcdef\x1b[1;2H\x1b[2P");
    assert_eq!(g.visible_text(), "adef");
    g.process(b"\x1b[2@");
    assert_eq!(g.visible_text(), "a  def");
    g.process(b"\x1b[1;5H\x1b[5X");
    assert_eq!(g.visible_text(), "a  d");
}

#[test]
fn insert_and_delete_lines_inside_region() {
    let mut g = grid(3, 4, b"1\r\n2\r\n3\r\n4\x1b[2;1H\x1b[L");
    assert_eq!(g.visible_text(), "1\n\n2\n3");
    g.process(b"\x1b[M");
    assert_eq!(g.visible_text(), "1\n2\n3\n");
}

#[test]
fn scrolling_region_keeps_rows_outside_it() {
    let mut g = grid(5, 4, b"top\r\n---\r\na\r\nb");
    g.process(b"\x1b[3;4r");
    assert_eq!(g.scroll_region(), (2, 3));
    g.process(b"\x1b[4;1H\nc");
    assert_eq!(g.visible_text(), "top\n---\nb\nc");
    assert_eq!(g.history_len(), 0, "region scrolls do not feed history");
}

#[test]
fn reverse_index_at_top_scrolls_down() {
    let g = grid(3, 3, b"a\r\nb\x1b[1;1H\x1bM");
    assert_eq!(g.visible_text(), "\na\nb");
}

#[test]
fn alternate_screen_isolates_content() {
    let mut g = grid(10, 3, b"main");
    g.process(b"\x1b[?1049h");
    assert!(g.alt_screen());
    assert_eq!(g.visible_text().trim(), "");
    g.process(b"alt");
    assert_eq!(g.visible_text().trim(), "alt");
    g.process(b"\x1b[?1049l");
    assert!(!g.alt_screen());
    assert_eq!(g.visible_text().trim(), "main");
}

#[test]
fn cursor_visibility_save_restore_and_tab() {
    let mut g = grid(20, 5, b"\x1b[?25l");
    assert!(!g.cursor_visible());
    g.process(b"\x1b[?25h\x1b[3;5H\x1b7\x1b[H");
    assert_eq!(g.cursor(), (0, 0));
    g.process(b"\x1b8");
    assert_eq!(g.cursor(), (2, 4));
    g.process(b"\r\t");
    assert_eq!(g.cursor(), (2, 8));
}

#[test]
fn wide_characters_take_two_cells() {
    let g = grid(6, 1, "日本x".as_bytes());
    assert_eq!(cell(&g, 0, 0).ch, '日');
    assert!(cell(&g, 0, 1).spacer);
    assert_eq!(cell(&g, 0, 4).ch, 'x');
    assert_eq!(g.visible_text(), "日本x");
}

#[test]
fn bell_and_osc_title_are_taken_once() {
    let mut g = grid(10, 1, b"\x07\x1b]0;my title\x07");
    assert!(g.take_bell());
    assert!(!g.take_bell());
    assert_eq!(g.take_title().as_deref(), Some("my title"));
    assert_eq!(g.take_title(), None);
}

#[test]
fn split_sequences_carry_over_between_calls() {
    let mut g = TerminalGrid::new(10, 1);
    g.process(b"\x1b[3");
    g.process(b"1mr\xe2\x82");
    g.process(b"\xac");
    assert_eq!(cell(&g, 0, 0).fg, TermColor::Indexed(1));
    assert_eq!(cell(&g, 0, 1).ch, '€');
}

#[test]
fn search_is_case_insensitive_over_history_and_screen() {
    let g = grid(8, 2, b"Alpha\r\nBeta\r\nalpha");
    let hits = g.find("ALPHA");
    assert_eq!(hits.len(), 2);
    assert_eq!((hits[0].line, hits[0].start, hits[0].end), (0, 0, 5));
    assert!(g.find("  ").is_empty());
}

#[test]
fn links_are_found_without_trailing_punctuation() {
    let g = grid(60, 1, b"see https://example.com/a?b=1, or http://x.org.");
    let links: Vec<String> = g.links(0).into_iter().map(|(_, u)| u).collect();
    assert_eq!(links, ["https://example.com/a?b=1", "http://x.org"]);
}

#[test]
fn selection_text_spans_lines() {
    let g = grid(10, 3, b"hello\r\nworld\r\nagain");
    let text = g.text_between(GridPos { line: 2, col: 2 }, GridPos { line: 0, col: 3 });
    assert_eq!(text, "lo\nworld\naga");
}

#[test]
fn resize_keeps_the_cursor_row_and_clamps() {
    let mut g = grid(10, 4, b"a\r\nb\r\nc\r\nd");
    g.resize(3, 2);
    assert_eq!(g.visible_text(), "c\nd");
    assert_eq!(g.cursor(), (1, 1));
    g.resize(0, 0);
    assert_eq!((g.cols(), g.rows()), (1, 1));
}

#[test]
fn hostile_input_does_not_panic() {
    let mut g = TerminalGrid::new(3, 2);
    let mut junk = Vec::new();
    for seq in [
        &b"\x1b[65535;65535H"[..],
        b"\x1b[65535@",
        b"\x1b[65535P",
        b"\x1b[65535L",
        b"\x1b[65535M",
        b"\x1b[65535S",
        b"\x1b[65535T",
        b"\x1b[65535X",
        b"\x1b[9;1r",
        b"\x1b[0;0r",
        b"\x1b[38;2m",
        b"\x1b[38;5m",
        b"\x1b[48:2m",
        b"\x1b]2;\x07",
        b"\x1bc",
        b"\xff\xfe",
        "表表表".as_bytes(),
    ] {
        junk.extend_from_slice(seq);
        junk.extend_from_slice(b"x\n\x08\x08\x08\x08\x1bM\x1bM");
    }
    for _ in 0..50 {
        g.process(&junk);
    }
    g.resize(1, 1);
    g.process(&junk);
    let _ = g.buffer_text();
    let _ = g.text_between(
        GridPos {
            line: u64::MAX,
            col: usize::MAX,
        },
        GridPos::default(),
    );
}

// ── View ──────────────────────────────────────────────────────────────────────

struct Probe {
    grid: RefCell<TerminalGrid>,
    state: RefCell<TerminalState>,
    fit: RefCell<(usize, usize)>,
    link: RefCell<Option<String>>,
}

fn probe(bytes: &[u8]) -> &'static Probe {
    let mut grid = TerminalGrid::new(40, 10);
    grid.process(bytes);
    Box::leak(Box::new(Probe {
        grid: RefCell::new(grid),
        state: RefCell::new(TerminalState::default()),
        fit: RefCell::new((0, 0)),
        link: RefCell::new(None),
    }))
}

fn harness(probe: &'static Probe, theme: Theme) -> Harness<'static> {
    Harness::builder()
        .with_size(Vec2::new(400.0, 200.0))
        .build_ui(move |ui| {
            install(ui.ctx(), &theme);
            let mut grid = probe.grid.borrow_mut();
            let mut state = probe.state.borrow_mut();
            let response = TerminalView::new("term")
                .label("Shell")
                .show(ui, &mut grid, &mut state);
            *probe.fit.borrow_mut() = response.fit;
            if let Some(link) = response.link_clicked {
                *probe.link.borrow_mut() = Some(link);
            }
        })
}

#[test]
fn renders_in_every_theme_with_terminal_role() {
    for theme in [
        Theme::light(fluent2_tokens::FLUENT),
        Theme::dark(fluent2_tokens::FLUENT),
        Theme::high_contrast(),
    ] {
        let p = probe(b"\x1b[31mred\x1b[0m \x1b[7minverse\x1b[0m https://example.com");
        p.state.borrow_mut().search = "inverse".into();
        let mut h = harness(p, theme);
        h.run();
        let node = h.get_by_role(Role::Terminal);
        assert_eq!(node.accesskit_node().label().as_deref(), Some("Shell"));
        assert!(
            node.accesskit_node()
                .value()
                .unwrap_or_default()
                .contains("red inverse")
        );
        let (cols, rows) = *p.fit.borrow();
        assert!(cols > 10 && rows > 3, "fit {cols}x{rows}");
    }
}

#[test]
fn ansi_palette_differs_between_light_and_dark() {
    let light = ansi_palette(&Theme::light(fluent2_tokens::FLUENT));
    let dark = ansi_palette(&Theme::dark(fluent2_tokens::FLUENT));
    assert_ne!(light, dark);
}

#[test]
fn click_focuses_and_paints_focus_ring_drag_selects() {
    let p = probe(b"hello world");
    let mut h = harness(p, Theme::default());
    h.run();
    h.get_by_role(Role::Terminal).click();
    h.run();
    assert!(h.get_by_role(Role::Terminal).accesskit_node().is_focused());

    let start = Pos2::new(6.0, 8.0);
    let end = Pos2::new(60.0, 8.0);
    h.event(Event::PointerMoved(start));
    h.event(Event::PointerButton {
        pos: start,
        button: PointerButton::Primary,
        pressed: true,
        modifiers: Modifiers::NONE,
    });
    h.run();
    h.event(Event::PointerMoved(end));
    h.run();
    h.event(Event::PointerButton {
        pos: end,
        button: PointerButton::Primary,
        pressed: false,
        modifiers: Modifiers::NONE,
    });
    h.run();
    let text = p
        .state
        .borrow()
        .selected_text(&p.grid.borrow())
        .unwrap_or_default();
    assert!(text.starts_with("hel"), "selected {text:?}");
}

#[test]
fn ctrl_click_on_link_reports_it() {
    let url = "https://example.com/abcdefghijklmnopqrst";
    let p = probe(format!("{url}\r\n").repeat(10).as_bytes());
    let mut h = harness(p, Theme::default());
    h.run();
    h.get_by_role(Role::Terminal).click();
    h.run();
    assert_eq!(*p.link.borrow(), None, "a plain click does not open links");
    h.get_by_role(Role::Terminal).click_modifiers(Modifiers::COMMAND);
    h.run();
    assert_eq!(p.link.borrow().as_deref(), Some(url));
}

#[test]
fn find_next_walks_matches_and_scrolls_to_them() {
    let mut g = TerminalGrid::new(10, 2);
    g.process(b"needle\r\nx\r\ny\r\nz\r\nneedle");
    let mut state = TerminalState::default();
    state.search = "needle".into();
    let first = state.find_next(&mut g, true).map(|s| s.line);
    assert_eq!(first, Some(0));
    assert_eq!(g.view_top(), 0);
    let second = state.find_next(&mut g, true).map(|s| s.line);
    assert_eq!(second, Some(4));
    assert_eq!(g.scroll_offset(), 0);
    assert_eq!(state.find_next(&mut g, false).map(|s| s.line), Some(0));
}
