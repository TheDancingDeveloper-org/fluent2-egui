//! HtmlView and its model (WI-1939), against synthetic mail fixtures in `tests/fixtures/html`:
//! a newsletter (layout tables, hidden preheader, tracking pixel, remote and data: images), an
//! Outlook message (MsoNormal, `<o:p>`, conditional comments, a data table, a quoted reply),
//! plain `div` mail, and hostile input.
#![cfg(feature = "html")]

use egui::accesskit::Role;
use egui_kittest::Harness;
use egui_kittest::kittest::Queryable;
use fluent2_egui::data::html::{
    Block, HtmlDocument, HtmlView, ImageDecision, LinkClick, LinkTarget, MAX_DEPTH, TextKind,
};
use fluent2_egui::{Theme, install};

fn fixture(name: &str) -> HtmlDocument {
    let path = format!("{}/tests/fixtures/html/{name}.html", env!("CARGO_MANIFEST_DIR"));
    HtmlDocument::parse(&std::fs::read_to_string(path).unwrap_or_default())
}

#[test]
fn newsletter_keeps_content_and_drops_what_mail_hides() {
    let doc = fixture("newsletter");
    let text = doc.to_plain_text();
    for wanted in [
        "Weekly digest",
        "Three things this week",
        "Release 0.3 shipped",
        "Card B text",
        "Unsubscribe",
    ] {
        assert!(text.contains(wanted), "missing {wanted:?} in:\n{text}");
    }
    for unwanted in ["Preheader", "document.write", "tracked", "margin:0"] {
        assert!(!text.contains(unwanted), "{unwanted:?} leaked into:\n{text}");
    }
    let images: Vec<_> = doc.images().iter().map(|i| i.src.clone()).collect();
    assert_eq!(images.len(), 2, "the 1×1 tracking pixel is dropped: {images:?}");
    let outline = doc.outline();
    assert!(
        outline.contains("cell bg#0f6cbd"),
        "header cell keeps its colour:\n{outline}"
    );
    assert!(outline.contains("[docs](https://example.com/docs)"));
    assert!(outline.contains("h2^ Three things this week"));
}

#[test]
fn outlook_paragraphs_tables_and_quotes() {
    let doc = fixture("outlook");
    let outline = doc.outline();
    assert!(
        outline.contains("p Please review the *Q3 numbers* below."),
        "{outline}"
    );
    assert!(
        !outline.contains("o:p") && !outline.contains("shapedefaults"),
        "{outline}"
    );
    assert!(outline.contains("p> 1,204"), "right-aligned cell:\n{outline}");
    assert!(
        outline.contains("quote\n  p Can you send the Q3 table?"),
        "{outline}"
    );
    let text = doc.to_plain_text();
    assert!(
        text.contains("Region\tRevenue\nNorth\t1,204\nSouth\t980"),
        "{text}"
    );
    assert!(text.contains("> Can you send the Q3 table?"), "{text}");
}

#[test]
fn plain_div_mail() {
    let doc = fixture("plain");
    assert_eq!(doc.to_plain_text(), "Hi,\nLunch at 12?\n-- \nKen");
}

#[test]
fn hostile_input_is_contained() {
    let doc = fixture("hostile");
    let text = doc.to_plain_text();
    assert!(
        !text.contains("javascript"),
        "a blocked href is not printed:\n{text}"
    );
    assert!(!text.contains("svg text") && !text.contains("password"), "{text}");
    assert!(text.contains("there <https://ok.example>"), "{text}");
    assert!(
        text.contains("deep"),
        "content past the depth limit still shows:\n{text}"
    );
    let max = depth(&doc.blocks);
    assert!(max <= MAX_DEPTH, "nesting is capped at {MAX_DEPTH}, got {max}");
}

fn depth(blocks: &[Block]) -> usize {
    blocks
        .iter()
        .map(|block| match block {
            Block::Quote(inner) => 1 + depth(inner),
            Block::List { items, .. } => 1 + items.iter().map(|i| depth(i)).max().unwrap_or(0),
            Block::Table(table) => {
                1 + table
                    .rows
                    .iter()
                    .flatten()
                    .map(|cell| depth(&cell.blocks))
                    .max()
                    .unwrap_or(0)
            }
            _ => 0,
        })
        .max()
        .unwrap_or(0)
}

#[test]
fn whitespace_entities_and_implied_ends() {
    let doc = HtmlDocument::parse("<p>  a\n\n  b &amp; c&nbsp;d <p>second<li>x<li>y");
    assert_eq!(doc.to_plain_text(), "a b & c d\n\nsecond\n\nx\ny");
    assert!(
        doc.outline().contains("c\u{a0}d"),
        "the model keeps the no-break space"
    );
    let heading = HtmlDocument::parse("<h3 style='text-align:right'>T</h3>");
    assert!(matches!(
        heading.blocks.first(),
        Some(Block::Text {
            kind: TextKind::Heading(3),
            ..
        })
    ));
}

fn view_harness(
    doc: HtmlDocument,
    theme: Theme,
    out: std::rc::Rc<std::cell::RefCell<(Option<LinkClick>, usize)>>,
) -> Harness<'static> {
    Harness::builder()
        .with_size(egui::vec2(700.0, 1400.0))
        .build_ui(move |ui| {
            install(ui.ctx(), &theme);
            let response = HtmlView::new("mail", &doc).show(ui);
            let mut state = out.borrow_mut();
            if response.clicked.is_some() {
                state.0 = response.clicked;
            }
            state.1 = response.blocked_images;
        })
}

#[test]
fn links_are_focusable_link_nodes_and_report_clicks() {
    let out = std::rc::Rc::new(std::cell::RefCell::new((None, 0)));
    let mut harness = view_harness(fixture("newsletter"), Theme::default(), out.clone());
    harness.run_steps(2);
    let docs = harness.get_by_role_and_label(Role::Link, "docs");
    docs.click();
    harness.run_steps(2);
    assert_eq!(
        out.borrow().0,
        Some(LinkClick {
            href: "https://example.com/docs".to_owned(),
            target: LinkTarget::Web("https://example.com/docs".to_owned()),
        })
    );
    // The heading is exposed as one.
    let _ = harness.get_by_role_and_label(Role::Heading, "Three things this week");
}

#[test]
fn javascript_links_are_not_links() {
    let out = std::rc::Rc::new(std::cell::RefCell::new((None, 0)));
    let mut harness = view_harness(fixture("hostile"), Theme::default(), out);
    harness.run_steps(2);
    assert!(harness.query_by_role_and_label(Role::Link, "here").is_none());
    let _ = harness.get_by_role_and_label(Role::Link, "there");
}

#[test]
fn remote_images_are_blocked_by_default_and_data_images_are_not() {
    let out = std::rc::Rc::new(std::cell::RefCell::new((None, 0)));
    let mut harness = view_harness(fixture("newsletter"), Theme::default(), out.clone());
    harness.run_steps(2);
    assert_eq!(out.borrow().1, 1, "only the remote Card A image is blocked");
    let _ = harness.get_by_role_and_label(Role::Image, "Blocked image: Card A");

    // An app policy can hide images instead.
    let doc = fixture("newsletter");
    let hidden = std::rc::Rc::new(std::cell::RefCell::new(0));
    let seen = hidden.clone();
    let mut harness = Harness::builder()
        .with_size(egui::vec2(700.0, 1400.0))
        .build_ui(move |ui| {
            install(ui.ctx(), &Theme::default());
            let policy = |_: &fluent2_egui::data::html::ImageRef| ImageDecision::Hidden;
            *seen.borrow_mut() = HtmlView::new("mail", &doc)
                .image_policy(&policy)
                .show(ui)
                .blocked_images;
        });
    harness.run_steps(2);
    assert_eq!(*hidden.borrow(), 0);
}

#[test]
fn every_fixture_renders_in_every_theme() {
    for name in ["newsletter", "outlook", "plain", "hostile"] {
        for theme in [
            Theme::default(),
            Theme::dark(fluent2_egui::tokens::FLUENT),
            Theme::high_contrast(),
        ] {
            let out = std::rc::Rc::new(std::cell::RefCell::new((None, 0)));
            let mut harness = view_harness(fixture(name), theme, out);
            harness.run_steps(2);
        }
    }
}

#[test]
fn find_text_is_case_insensitive_and_stays_inside_a_block() {
    use fluent2_egui::data::html::{TextMatch, find_text};
    let doc = HtmlDocument::parse("<p>Alpha beta</p><p>BETA gamma</p>");
    let hits = find_text(&doc, "beta");
    assert_eq!(
        hits,
        vec![
            TextMatch {
                block: 0,
                start: 6,
                end: 10
            },
            TextMatch {
                block: 1,
                start: 0,
                end: 4
            },
        ]
    );
    assert!(find_text(&doc, "").is_empty());
    assert!(
        find_text(&doc, "alphabeta").is_empty(),
        "a match does not cross blocks"
    );
}

#[test]
fn cross_block_copy_joins_selected_blocks() {
    let doc = HtmlDocument::parse("<p>Alpha beta</p><p>BETA gamma</p>");
    let copied = std::rc::Rc::new(std::cell::RefCell::new(String::new()));
    let seen = copied.clone();
    let mut harness = Harness::builder()
        .with_size(egui::vec2(700.0, 400.0))
        .build_ui(move |ui| {
            install(ui.ctx(), &Theme::default());
            let out = HtmlView::new("mail", &doc).show(ui);
            if !out.selection.is_empty() {
                *seen.borrow_mut() = out.selection;
            }
        });
    harness.run_steps(2);
    // Drag from the first paragraph into the second, then copy.
    let first = harness.get_by_role_and_label(Role::Paragraph, "Alpha beta");
    let second = harness.get_by_role_and_label(Role::Paragraph, "BETA gamma");
    let start = first.rect().left_top();
    let end = second.rect().right_bottom();
    harness.event(egui::Event::PointerMoved(start));
    harness.event(egui::Event::PointerButton {
        pos: start,
        button: egui::PointerButton::Primary,
        pressed: true,
        modifiers: egui::Modifiers::default(),
    });
    harness.event(egui::Event::PointerMoved(end));
    harness.event(egui::Event::PointerButton {
        pos: end,
        button: egui::PointerButton::Primary,
        pressed: false,
        modifiers: egui::Modifiers::default(),
    });
    harness.run_steps(2);
    let text = copied.borrow().clone();
    assert!(text.contains("Alpha"), "copied {text:?}");
    assert!(text.contains("gamma"), "copied {text:?}");
}

#[test]
fn find_reports_matches_and_document_exposes_structure() {
    let doc = HtmlDocument::parse("<h2>Weekly</h2><p>See the <a href=\"https://example.com\">docs</a>.</p>");
    let status = std::rc::Rc::new(std::cell::RefCell::new((0usize, 0usize, false)));
    let seen = status.clone();
    let mut harness = Harness::builder()
        .with_size(egui::vec2(700.0, 400.0))
        .build_ui(move |ui| {
            install(ui.ctx(), &Theme::default());
            let out = HtmlView::new("mail", &doc).find("the", 0).show(ui);
            *seen.borrow_mut() = (out.find.matches, out.find.current, out.hovered.is_some());
        });
    harness.run_steps(2);
    assert_eq!(status.borrow().0, 1);
    assert_eq!(status.borrow().1, 1);
    let _ = harness.get_by_role_and_label(Role::Heading, "Weekly");
    let _ = harness.get_by_role_and_label(Role::Paragraph, "See the docs.");
    let _ = harness.get_by_role_and_label(Role::Link, "docs");
}

#[test]
fn find_text_matches_across_multibyte_chars() {
    use fluent2_egui::data::html::{TextMatch, find_text};
    let doc = HtmlDocument::parse("<p>café au lait</p>");
    let hits = find_text(&doc, "lait");
    let text = &doc.text_blocks()[0];
    let at = match text.find("lait") {
        Some(at) => at,
        None => panic!("fixture"),
    };
    assert_eq!(
        hits,
        vec![TextMatch {
            block: 0,
            start: at,
            end: at + "lait".len(),
        }]
    );
    assert_eq!(&text[hits[0].start..hits[0].end], "lait");

    let accented = HtmlDocument::parse("<p>CAFÉ au lait</p>");
    let hits = find_text(&accented, "café");
    let text = &accented.text_blocks()[0];
    assert_eq!(&text[hits[0].start..hits[0].end], "CAFÉ");

    let cjk = HtmlDocument::parse("<p>東京の夜</p>");
    let hits = find_text(&cjk, "の");
    let text = &cjk.text_blocks()[0];
    assert_eq!(hits.len(), 1);
    assert_eq!(&text[hits[0].start..hits[0].end], "の");
    // "東" is three bytes; the match must not start on a continuation byte.
    assert!(text.is_char_boundary(hits[0].start));
    assert!(text.is_char_boundary(hits[0].end));
}
