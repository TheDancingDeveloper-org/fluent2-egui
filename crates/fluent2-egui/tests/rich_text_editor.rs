//! RichTextEditor: the document model (edits, formats, HTML and plain output) and the editor
//! through kittest (no wgpu): rendering in each theme, toolbar buttons and their accessible
//! names, and formatting applied from the toolbar and the keyboard.

use egui::Vec2;
use egui::accesskit::Role;
use egui_kittest::Harness;
use egui_kittest::kittest::{NodeT, Queryable};
use fluent2_egui::controls::{BlockKind, CharStyle, InlineFormat, RichText, RichTextEditor};
use fluent2_egui::{Theme, install};

fn light() -> Theme {
    Theme::light(fluent2_tokens::FLUENT)
}

fn harness(doc: RichText, theme: fn() -> Theme) -> Harness<'static, RichText> {
    Harness::builder()
        .with_size(Vec2::new(640.0, 320.0))
        .build_ui_state(
            move |ui, doc: &mut RichText| {
                install(ui.ctx(), &theme());
                RichTextEditor::new("editor", doc).label("Message").show(ui);
            },
            doc,
        )
}

#[test]
fn html_wraps_lines_in_paragraphs_lists_and_quotes() {
    let mut doc = RichText::from_plain("Intro\nOne\nTwo\nFirst\nSaid\n");
    doc.set_block(6..13, BlockKind::Bulleted);
    doc.set_block(14..19, BlockKind::Numbered);
    doc.set_block(20..24, BlockKind::Quote);
    assert_eq!(
        doc.to_html(),
        "<p>Intro</p><ul><li>One</li><li>Two</li></ul><ol><li>First</li></ol>\
         <blockquote><p>Said</p></blockquote><p><br></p>"
    );
    assert_eq!(doc.to_plain(), "Intro\n- One\n- Two\n1. First\n> Said\n");
}

#[test]
fn inline_formats_nest_and_text_is_escaped() {
    let mut doc = RichText::from_plain("a<b> & c");
    doc.toggle_inline(0..4, InlineFormat::Bold);
    doc.toggle_inline(0..1, InlineFormat::Italic);
    doc.toggle_inline(7..8, InlineFormat::Underline);
    assert_eq!(
        doc.to_html(),
        "<p><strong><em>a</em></strong><strong>&lt;b&gt;</strong> &amp; <u>c</u></p>"
    );
    assert_eq!(doc.to_plain(), "a<b> & c");
}

#[test]
fn toggling_a_fully_formatted_range_turns_it_off() {
    let mut doc = RichText::from_plain("bold");
    doc.toggle_inline(0..4, InlineFormat::Bold);
    assert!(doc.has_inline(0..4, InlineFormat::Bold));
    doc.toggle_inline(1..3, InlineFormat::Bold);
    assert_eq!(doc.to_html(), "<p><strong>b</strong>ol<strong>d</strong></p>");
    // Out-of-range input is clamped, not a panic.
    doc.toggle_inline(3..99, InlineFormat::Italic);
    #[allow(clippy::reversed_empty_ranges)]
    doc.set_block(50..10, BlockKind::Quote);
}

#[test]
fn links_write_safe_schemes_only() {
    let mut doc = RichText::from_plain("site bad");
    doc.set_link(0..4, Some("https://example.com/?q=\"x\""));
    doc.set_link(5..8, Some("javascript:alert(1)"));
    assert_eq!(
        doc.to_html(),
        "<p><a href=\"https://example.com/?q=&quot;x&quot;\">site</a> bad</p>"
    );
    assert_eq!(
        doc.to_plain(),
        "site (https://example.com/?q=\"x\") bad (javascript:alert(1))"
    );
    assert_eq!(doc.link_in(0..4), Some("https://example.com/?q=\"x\""));
    doc.set_link(0..8, None);
    assert_eq!(doc.link_in(0..4), None);
}

#[test]
fn edits_keep_styles_and_typed_text_takes_the_typing_style() {
    let mut doc = RichText::from_plain("ab");
    doc.toggle_inline(0..1, InlineFormat::Bold);
    let typing = CharStyle {
        italic: true,
        ..CharStyle::default()
    };
    doc.replace_text("aXb", &typing);
    assert_eq!(doc.to_html(), "<p><strong>a</strong><em>X</em>b</p>");
    doc.replace_text("b", &typing);
    assert_eq!(doc.to_html(), "<p>b</p>");
}

#[test]
fn a_new_line_continues_the_list_and_joining_lines_keeps_the_first_kind() {
    let mut doc = RichText::from_plain("one");
    doc.set_block(0..3, BlockKind::Numbered);
    doc.replace_text("one\ntwo", &CharStyle::default());
    assert_eq!(doc.blocks(), &[BlockKind::Numbered, BlockKind::Numbered]);
    assert_eq!(doc.to_plain(), "1. one\n2. two");

    let mut doc = RichText::from_plain("a\nb");
    doc.set_block(2..3, BlockKind::Quote);
    doc.replace_text("ab", &CharStyle::default());
    assert_eq!(doc.blocks(), &[BlockKind::Paragraph]);
}

#[test]
fn renders_in_light_dark_and_high_contrast() {
    let themes: [fn() -> Theme; 3] = [
        light,
        || Theme::dark(fluent2_tokens::FLUENT),
        Theme::high_contrast,
    ];
    for theme in themes {
        let mut doc = RichText::from_plain("Hello\nworld");
        doc.set_block(0..5, BlockKind::Bulleted);
        doc.set_block(6..11, BlockKind::Quote);
        doc.set_link(6..11, Some("https://example.com"));
        let mut harness = harness(doc, theme);
        harness.run();
        for name in [
            "Bold",
            "Italic",
            "Underline",
            "Link",
            "Bulleted list",
            "Numbered list",
            "Quote",
        ] {
            assert_eq!(
                harness.get_by_label(name).accesskit_node().role(),
                Role::Button,
                "{name} is a toolbar button"
            );
        }
        harness.get_by_label("Message");
    }
}

#[test]
fn toolbar_buttons_format_the_line_at_the_cursor() {
    let mut harness = harness(RichText::from_plain("item"), light);
    harness.run();
    harness.get_by_label("Bulleted list").click();
    harness.run();
    assert_eq!(harness.state().to_html(), "<ul><li>item</li></ul>");
    harness.get_by_label("Bulleted list").click();
    harness.run();
    assert_eq!(harness.state().to_html(), "<p>item</p>");
}

#[test]
fn bold_with_no_selection_applies_to_typed_text() {
    let mut harness = harness(RichText::new(), light);
    harness.run();
    harness.get_by_label("Bold").click();
    harness.run();
    harness.run();
    harness.get_by_label("Message").type_text("Hi");
    harness.run();
    assert_eq!(harness.state().to_html(), "<p><strong>Hi</strong></p>");
}

#[test]
fn ctrl_b_toggles_bold_while_typing() {
    let mut harness = harness(RichText::new(), light);
    harness.run();
    harness.get_by_label("Message").focus();
    harness.run();
    harness.key_press_modifiers(egui::Modifiers::COMMAND, egui::Key::B);
    harness.run();
    harness.get_by_label("Message").type_text("x");
    harness.run();
    assert_eq!(harness.state().to_html(), "<p><strong>x</strong></p>");
}

#[test]
fn disabled_editor_renders() {
    let mut harness = Harness::builder()
        .with_size(Vec2::new(480.0, 240.0))
        .build_ui(|ui| {
            install(ui.ctx(), &light());
            let mut doc = RichText::from_plain("read only");
            ui.add_enabled(false, RichTextEditor::new("off", &mut doc));
        });
    harness.run();
    harness.get_by_label("Rich text editor");
}
