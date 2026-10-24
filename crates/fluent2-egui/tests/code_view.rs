//! CodeView and the shared highlighter (`highlight` feature): token kinds for each supported
//! language, theme colours and contrast, and the view's selection, copy and AccessKit node.
#![cfg(feature = "highlight")]

use std::cell::RefCell;

use egui::accesskit::Role;
use egui::{Color32, Event, Modifiers, PointerButton, Pos2, Vec2};
use egui_kittest::Harness;
use egui_kittest::kittest::{NodeT, Queryable};
use fluent2_egui::data::code::{
    CodeState, CodeView, Highlighter, SyntaxToken, TokenSpan, highlight, line_job, split_lines, syntax_color,
};
use fluent2_egui::{Theme, install};

/// Every `(text, token)` run that is not plain, line by line.
fn runs(language: &str, text: &str) -> Vec<Vec<(String, SyntaxToken)>> {
    let lines = highlight(language, text);
    split_lines(text)
        .zip(&lines)
        .map(|(line, spans)| {
            spans
                .iter()
                .filter(|s| s.token != SyntaxToken::Plain)
                .map(|s| (line[s.range.clone()].to_owned(), s.token))
                .collect()
        })
        .collect()
}

fn has(runs: &[Vec<(String, SyntaxToken)>], line: usize, text: &str, token: SyntaxToken) -> bool {
    runs.get(line)
        .is_some_and(|l| l.iter().any(|(t, k)| t == text && *k == token))
}

#[test]
fn rust_keywords_strings_types_comments() {
    let r = runs(
        "rust",
        "#[derive(Debug)]\nfn main() { let s: String = \"hi\".into(); // note\n    Some(42) }",
    );
    assert!(has(&r, 0, "derive", SyntaxToken::Attribute), "{r:?}");
    assert!(has(&r, 1, "fn", SyntaxToken::Keyword), "{r:?}");
    assert!(has(&r, 1, "let", SyntaxToken::Keyword), "{r:?}");
    assert!(has(&r, 1, "main", SyntaxToken::Function), "{r:?}");
    assert!(has(&r, 1, "String", SyntaxToken::Type), "{r:?}");
    assert!(has(&r, 1, "\"hi\"", SyntaxToken::String), "{r:?}");
    assert!(has(&r, 1, "// note", SyntaxToken::Comment), "{r:?}");
    assert!(has(&r, 2, "42", SyntaxToken::Number), "{r:?}");
}

#[test]
fn python_keywords_strings_decorators() {
    let r = runs("python", "@cache\ndef f(x: int) -> None:\n    return 'a'  # c");
    assert!(has(&r, 0, "@cache", SyntaxToken::Attribute), "{r:?}");
    assert!(has(&r, 1, "def", SyntaxToken::Keyword), "{r:?}");
    assert!(has(&r, 1, "None", SyntaxToken::Constant), "{r:?}");
    assert!(has(&r, 2, "return", SyntaxToken::Keyword), "{r:?}");
    assert!(has(&r, 2, "'a'", SyntaxToken::String), "{r:?}");
    assert!(has(&r, 2, "# c", SyntaxToken::Comment), "{r:?}");
}

#[test]
fn typescript_highlights_types_and_interfaces() {
    assert!(Highlighter::shared().supports("ts"));
    assert!(Highlighter::shared().supports("TypeScript"));
    let r = runs(
        "ts",
        "interface Item { id: string; count: number }\nconst x: Item = load(\"s\", 1); // c",
    );
    assert!(has(&r, 0, "interface", SyntaxToken::Keyword), "{r:?}");
    assert!(has(&r, 0, "Item", SyntaxToken::Type), "{r:?}");
    assert!(has(&r, 0, "string", SyntaxToken::Type), "{r:?}");
    assert!(has(&r, 0, "number", SyntaxToken::Type), "{r:?}");
    assert!(has(&r, 1, "const", SyntaxToken::Keyword), "{r:?}");
    assert!(has(&r, 1, "Item", SyntaxToken::Type), "type annotation: {r:?}");
    assert!(has(&r, 1, "load", SyntaxToken::Function), "{r:?}");
    assert!(has(&r, 1, "1", SyntaxToken::Number), "{r:?}");
    assert!(has(&r, 1, "// c", SyntaxToken::Comment), "{r:?}");
}

#[test]
fn tsx_highlights_as_typescript() {
    assert!(Highlighter::shared().supports("tsx"));
    let r = runs("tsx", "const x: string = name;\nconst view = <Item id={x}/>;");
    assert!(has(&r, 0, "const", SyntaxToken::Keyword), "{r:?}");
    assert!(has(&r, 0, "string", SyntaxToken::Type), "type annotation: {r:?}");
    assert!(has(&r, 1, "Item", SyntaxToken::Type), "jsx tag: {r:?}");
}

#[test]
fn json_keys_differ_from_string_values() {
    let r = runs("json", "{\"key\": \"value\", \"n\": 1, \"b\": true}");
    assert!(has(&r, 0, "\"key\"", SyntaxToken::Key), "{r:?}");
    assert!(has(&r, 0, "\"value\"", SyntaxToken::String), "{r:?}");
    assert!(has(&r, 0, "1", SyntaxToken::Number), "{r:?}");
    assert!(has(&r, 0, "true", SyntaxToken::Constant), "{r:?}");
}

#[test]
fn yaml_keys_values_comments() {
    let r = runs("yml", "name: fluent\ncount: 3\n# comment");
    assert!(has(&r, 0, "name", SyntaxToken::Key), "{r:?}");
    assert!(has(&r, 0, "fluent", SyntaxToken::String), "{r:?}");
    assert!(has(&r, 1, "3", SyntaxToken::Number), "{r:?}");
    assert!(has(&r, 2, "# comment", SyntaxToken::Comment), "{r:?}");
}

#[test]
fn diff_inserted_deleted_and_headers() {
    let r = runs("diff", "--- a/x\n+++ b/x\n@@ -1 +1 @@\n-old\n+new\n same");
    assert!(has(&r, 2, "@@ -1 +1 @@", SyntaxToken::Meta), "{r:?}");
    assert!(has(&r, 3, "-old", SyntaxToken::Deleted), "{r:?}");
    assert!(has(&r, 4, "+new", SyntaxToken::Inserted), "{r:?}");
    assert!(r[5].is_empty(), "context lines are plain: {r:?}");
}

#[test]
fn spans_cover_each_line_in_order() {
    let text = "fn a() {}\r\n\n\tlet b = \"x\"; // é ünïcode\nlast";
    let lines = highlight("rust", text);
    assert_eq!(lines.len(), split_lines(text).count());
    for (line, spans) in split_lines(text).zip(&lines) {
        let mut at = 0;
        for s in spans {
            assert_eq!(s.range.start, at, "{line:?}: {spans:?}");
            assert!(line.get(s.range.clone()).is_some(), "char boundary: {spans:?}");
            at = s.range.end;
        }
        assert_eq!(at, line.len(), "{line:?}");
    }
}

#[test]
fn unknown_language_is_plain_text() {
    assert!(!Highlighter::shared().supports("no-such-language"));
    assert!(!Highlighter::shared().supports(""));
    let lines = highlight("no-such-language", "fn main() {}");
    assert_eq!(
        lines,
        vec![vec![TokenSpan {
            range: 0..12,
            token: SyntaxToken::Plain
        }]]
    );
}

#[test]
fn line_job_paints_theme_colours() {
    let theme = Theme::light(fluent2_tokens::FLUENT);
    let line = "let x = 1;";
    let spans = &highlight("rust", line)[0];
    let job = line_job(line, spans, egui::FontId::monospace(12.0), &theme);
    assert_eq!(job.text, line);
    let first = job.sections.first().map(|s| s.format.color);
    assert_eq!(first, Some(syntax_color(SyntaxToken::Keyword, &theme)));
    // Out-of-range spans are clipped, not a panic.
    let wild = [TokenSpan {
        range: 3..99,
        token: SyntaxToken::String,
    }];
    assert_eq!(
        line_job(line, &wild, egui::FontId::monospace(12.0), &theme).text,
        line
    );
}

fn luminance(c: Color32) -> f64 {
    let lin = |v: u8| {
        let v = f64::from(v) / 255.0;
        if v <= 0.040_45 {
            v / 12.92
        } else {
            ((v + 0.055) / 1.055).powf(2.4)
        }
    };
    0.2126 * lin(c.r()) + 0.7152 * lin(c.g()) + 0.0722 * lin(c.b())
}

fn contrast(a: Color32, b: Color32) -> f64 {
    let (la, lb) = (luminance(a), luminance(b));
    (la.max(lb) + 0.05) / (la.min(lb) + 0.05)
}

#[test]
fn syntax_colours_meet_text_contrast_and_follow_the_theme() {
    const ALL: [SyntaxToken; 15] = [
        SyntaxToken::Plain,
        SyntaxToken::Comment,
        SyntaxToken::Keyword,
        SyntaxToken::String,
        SyntaxToken::Number,
        SyntaxToken::Constant,
        SyntaxToken::Type,
        SyntaxToken::Function,
        SyntaxToken::Key,
        SyntaxToken::Attribute,
        SyntaxToken::Punctuation,
        SyntaxToken::Inserted,
        SyntaxToken::Deleted,
        SyntaxToken::Meta,
        SyntaxToken::Heading,
    ];
    let light = Theme::light(fluent2_tokens::FLUENT);
    let dark = Theme::dark(fluent2_tokens::FLUENT);
    for theme in [&light, &dark, &Theme::high_contrast()] {
        let bg = fluent2_egui::theme::c(theme.color.neutral_background_1);
        for token in ALL {
            let ratio = contrast(syntax_color(token, theme), bg);
            assert!(ratio >= 4.5, "{token:?} in {:?}: {ratio:.2}:1", theme.kind);
        }
    }
    assert_ne!(
        syntax_color(SyntaxToken::Keyword, &light),
        syntax_color(SyntaxToken::Keyword, &dark)
    );
    assert_ne!(
        syntax_color(SyntaxToken::Keyword, &light),
        syntax_color(SyntaxToken::String, &light)
    );
}

struct Probe {
    text: String,
    state: RefCell<CodeState>,
    copied: RefCell<Option<String>>,
}

fn probe(text: &str) -> &'static Probe {
    Box::leak(Box::new(Probe {
        text: text.to_owned(),
        state: RefCell::new(CodeState::default()),
        copied: RefCell::new(None),
    }))
}

fn harness(probe: &'static Probe, theme: Theme) -> Harness<'static> {
    Harness::builder()
        .with_size(Vec2::new(400.0, 200.0))
        .build_ui(move |ui| {
            install(ui.ctx(), &theme);
            let mut state = probe.state.borrow_mut();
            let response =
                CodeView::new("code")
                    .language("rust")
                    .label("main.rs")
                    .show(ui, &probe.text, &mut state);
            if let Some(text) = response.copied {
                *probe.copied.borrow_mut() = Some(text);
            }
        })
}

#[test]
fn renders_in_every_theme_as_read_only_text() {
    for theme in [
        Theme::light(fluent2_tokens::FLUENT),
        Theme::dark(fluent2_tokens::FLUENT),
        Theme::high_contrast(),
    ] {
        let p = probe("fn main() {\n\tprintln!(\"hi\");\n}");
        let mut h = harness(p, theme);
        h.run();
        let node = h.get_by_role(Role::MultilineTextInput);
        let node = node.accesskit_node();
        assert_eq!(node.label().as_deref(), Some("main.rs"));
        assert!(node.is_read_only());
        assert_eq!(
            node.value().as_deref(),
            Some("fn main() {\n    println!(\"hi\");\n}"),
            "tabs expand to four spaces"
        );
    }
}

#[test]
fn ctrl_a_then_copy_copies_everything_and_escape_clears() {
    let p = probe("let a = 1;\nlet b = 2;");
    let mut h = harness(p, Theme::default());
    h.run();
    h.get_by_role(Role::MultilineTextInput).click();
    h.run();
    assert!(
        h.get_by_role(Role::MultilineTextInput)
            .accesskit_node()
            .is_focused()
    );

    h.key_press_modifiers(Modifiers::COMMAND, egui::Key::A);
    h.run();
    h.event(Event::Copy);
    h.run();
    assert_eq!(p.copied.borrow().as_deref(), Some("let a = 1;\nlet b = 2;"));

    h.key_press(egui::Key::Escape);
    h.run();
    assert!(p.state.borrow().selection.is_none());
    assert!(
        h.get_by_role(Role::MultilineTextInput)
            .accesskit_node()
            .is_focused(),
        "Escape clears the selection and keeps focus"
    );
}

#[test]
fn drag_selects_across_lines() {
    let p = probe("first line\nsecond line\nthird");
    let mut h = harness(p, Theme::default());
    h.run();
    let rect = h.get_by_role(Role::MultilineTextInput).rect();
    // Line 1 starts after the gutter; drag from inside "first" down into "second".
    let start = Pos2::new(rect.left() + 60.0, rect.top() + 8.0);
    let end = Pos2::new(rect.left() + 60.0, rect.top() + 26.0);
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
    let text = p.state.borrow().selected_text().unwrap_or_default();
    assert!(text.contains('\n'), "selected {text:?}");
    assert!(
        text.starts_with(|c: char| c.is_alphabetic() || c == ' '),
        "{text:?}"
    );
}

#[test]
fn long_files_lay_out_only_visible_rows() {
    let text: String = (0..20_000).map(|i| format!("let v{i} = {i};\n")).collect();
    let p = probe(&text);
    let mut h = harness(p, Theme::default());
    h.run();
    let texts = h
        .output()
        .shapes
        .iter()
        .filter(|s| matches!(s.shape, egui::Shape::Text(_)))
        .count();
    assert!(texts < 100, "{texts} text shapes for 20 000 lines");
}

#[test]
fn new_text_rehighlights_and_drops_the_selection() {
    let mut state = CodeState::default();
    let ctx = egui::Context::default();
    let frame = |text: &str, state: &mut CodeState| {
        let mut out = ctx.run_ui(egui::RawInput::default(), |ui| {
            CodeView::new("c").language("json").show(ui, text, state);
        });
        out.textures_delta.clear();
    };
    frame("{\"a\": 1}", &mut state);
    state.selection = Some((
        Default::default(),
        fluent2_egui::data::code::CodePos { line: 0, col: 3 },
    ));
    assert_eq!(state.selected_text().as_deref(), Some("{\"a"));
    frame("{\"a\": 1}", &mut state);
    assert!(state.selection.is_some(), "same text keeps the selection");
    frame("[]", &mut state);
    assert!(state.selection.is_none());
}

#[test]
fn revision_keys_the_cache_without_hashing_the_text() {
    let mut state = CodeState::default();
    let ctx = egui::Context::default();
    let frame = |text: &str, rev: u64, state: &mut CodeState| {
        let mut out = ctx.run_ui(egui::RawInput::default(), |ui| {
            CodeView::new("c").revision(rev).show(ui, text, state);
        });
        out.textures_delta.clear();
    };
    let sel = Some((
        Default::default(),
        fluent2_egui::data::code::CodePos { line: 0, col: 1 },
    ));
    frame("abc", 1, &mut state);
    state.selection = sel;
    // Same revision and length: the cache stands, even though the text differs.
    frame("xyz", 1, &mut state);
    assert_eq!(state.selected_text().as_deref(), Some("a"));
    // A new revision re-highlights.
    frame("xyz", 2, &mut state);
    assert!(state.selection.is_none());
    state.selection = sel;
    assert_eq!(state.selected_text().as_deref(), Some("x"));
    // A length change re-highlights with the same revision.
    frame("xyzw", 2, &mut state);
    assert!(state.selection.is_none());
}
