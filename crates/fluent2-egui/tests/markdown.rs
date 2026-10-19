//! MarkdownView: outline snapshots of the parsed document (headings, lists, tables, code,
//! links), then the view in every theme through kittest (no wgpu). Needs the `markdown` feature.

#![cfg(feature = "markdown")]

use std::cell::RefCell;
use std::rc::Rc;

use egui::accesskit::Role;
use egui_kittest::Harness;
use egui_kittest::kittest::{NodeT, Queryable};
use fluent2_egui::data::markdown::{Document, LinkClick, LinkTarget, MarkdownView, default_link_policy};
use fluent2_egui::{Theme, install};

fn outline(src: &str) -> String {
    Document::parse(src).outline()
}

// ── Outline snapshots ──────────────────────────────────────────────────────────

#[test]
fn snapshot_headings() {
    assert_eq!(
        outline("# One\n## Two *em*\n### Three\n#### Four\n##### Five\n###### Six\nSetext\n===\n"),
        "h1 One\nh2 Two *em*\nh3 Three\nh4 Four\nh5 Five\nh6 Six\nh1 Setext\n"
    );
}

#[test]
fn snapshot_lists() {
    let src = "- a\n- b\n  - nested **bold**\n\n3. three\n4. four\n\n- [x] done\n- [ ] todo\n\n* loose\n\n  second para\n";
    assert_eq!(
        outline(src),
        "ul\n  li\n    p a\n  li\n    p b\n    ul\n      li\n        p nested **bold**\n\
         ol start=3\n  li\n    p three\n  li\n    p four\n\
         ul\n  li [x]\n    p done\n  li [ ]\n    p todo\n\
         ul\n  li\n    p loose\n    p second para\n"
    );
}

#[test]
fn snapshot_table() {
    let src = "| Name | Size | Kind |\n|:-----|-----:|:----:|\n| `a.rs` | 12 | **file** |\n| b | | dir |\n";
    assert_eq!(
        outline(src),
        "table lrc\n  th Name | Size | Kind\n  td `a.rs` | 12 | **file**\n  td b |  | dir\n"
    );
}

#[test]
fn snapshot_code() {
    let src = "```rust title\nfn main() {}\n\n// end\n```\n\n    indented\n\nInline `x + 1` and ~~gone~~.\n";
    assert_eq!(
        outline(src),
        "code rust\n  | fn main() {}\n  | \n  | // end\ncode -\n  | indented\np Inline `x + 1` and ~~gone~~.\n"
    );
}

#[test]
fn snapshot_links_quotes_and_breaks() {
    let src = "See [the *docs*](https://example.com/d) or <https://a.b>.\nnext  \nline\n\n> quoted\n> > deeper\n\n---\n\n![alt text](https://img/x.png) <b>raw</b>\n";
    assert_eq!(
        outline(src),
        "p See [the *docs*](https://example.com/d) or [https://a.b](https://a.b). next\\nline\n\
         quote\n  p quoted\n  quote\n    p deeper\n\
         hr\n\
         p alt text <b>raw</b>\n"
    );
}

#[test]
fn hostile_nesting_is_flattened() {
    let src = ">".repeat(5000) + " deep";
    let doc = Document::parse(&src);
    fn depth(blocks: &[fluent2_egui::data::markdown::Block]) -> usize {
        blocks
            .iter()
            .map(|b| match b {
                fluent2_egui::data::markdown::Block::Quote(inner) => 1 + depth(inner),
                _ => 0,
            })
            .max()
            .unwrap_or(0)
    }
    assert!(depth(&doc.blocks) <= fluent2_egui::data::markdown::MAX_DEPTH);
    assert!(doc.outline().contains("p deep"));
}

#[test]
fn default_policy_blocks_unsafe_schemes() {
    for href in [
        "javascript:alert(1)",
        "file:///etc/passwd",
        "docs/a.md",
        "#top",
        "data:text/html,x",
    ] {
        assert_eq!(default_link_policy(href), LinkTarget::Blocked, "{href}");
    }
    for href in ["https://x.y", "HTTP://x.y", "mailto:a@b.c"] {
        assert!(matches!(default_link_policy(href), LinkTarget::Web(_)), "{href}");
    }
}

// ── View ───────────────────────────────────────────────────────────────────────

const SAMPLE: &str = "# Title\n\nText with [web](https://example.com), [file](src/lib.rs) and [evil](javascript:x).\n\n- one\n- [x] task\n\n| A | B |\n|---|---|\n| 1 | 2 |\n\n```rust\nlet x = 1;\n```\n";

fn policy(href: &str) -> LinkTarget {
    if href.starts_with("src/") {
        LinkTarget::File(href.to_owned())
    } else {
        default_link_policy(href)
    }
}

fn harness(doc: Rc<Document>, clicks: Rc<RefCell<Vec<LinkClick>>>, theme: Theme) -> Harness<'static> {
    Harness::new_ui(move |ui| {
        install(ui.ctx(), &theme);
        let out = MarkdownView::new("md", &doc).link_policy(&policy).show(ui);
        if let Some(click) = out.clicked {
            clicks.borrow_mut().push(click);
        }
    })
}

#[test]
fn renders_in_every_theme_with_roles() {
    for theme in [
        Theme::light(fluent2_tokens::FLUENT),
        Theme::dark(fluent2_tokens::FLUENT),
        Theme::high_contrast(),
    ] {
        let clicks = Rc::default();
        let mut h = harness(Rc::new(Document::parse(SAMPLE)), clicks, theme);
        h.run();
        let heading = h.get_by_role(Role::Heading);
        assert_eq!(heading.accesskit_node().label().as_deref(), Some("Title"));
        let _ = h.get_by_role_and_label(Role::Link, "web");
        let _ = h.get_by_role_and_label(Role::Link, "file");
        assert!(
            h.query_by_role_and_label(Role::Link, "evil").is_none(),
            "blocked link is plain text"
        );
        let _ = h.get_by_role(Role::CheckBox);
        let _ = h.get_by_label("let x = 1;");
    }
}

#[test]
fn links_are_reported_through_the_policy() {
    let clicks: Rc<RefCell<Vec<LinkClick>>> = Rc::default();
    let mut h = harness(Rc::new(Document::parse(SAMPLE)), clicks.clone(), Theme::default());
    h.run();
    h.get_by_role_and_label(Role::Link, "web").click();
    h.run();
    h.get_by_role_and_label(Role::Link, "file").click();
    h.run();
    h.get_by_label("evil").click();
    h.run();
    assert_eq!(
        *clicks.borrow(),
        vec![
            LinkClick {
                href: "https://example.com".into(),
                target: LinkTarget::Web("https://example.com".into())
            },
            LinkClick {
                href: "src/lib.rs".into(),
                target: LinkTarget::File("src/lib.rs".into())
            },
        ]
    );
}

#[test]
fn link_activates_from_keyboard() {
    let clicks: Rc<RefCell<Vec<LinkClick>>> = Rc::default();
    let mut h = harness(
        Rc::new(Document::parse("[go](https://go.dev)")),
        clicks.clone(),
        Theme::default(),
    );
    h.run();
    h.key_press(egui::Key::Tab);
    h.run();
    assert!(h.get_by_role(Role::Link).accesskit_node().is_focused());
    h.key_press(egui::Key::Enter);
    h.run();
    assert_eq!(clicks.borrow().len(), 1);
}

#[test]
fn highlighter_hook_is_used_for_code() {
    let calls = Rc::new(RefCell::new(Vec::<String>::new()));
    let seen = calls.clone();
    let doc = Document::parse("```toml\na = 1\n```\n");
    let mut h = Harness::new_ui(move |ui| {
        install(ui.ctx(), &Theme::default());
        let hl = |lang: Option<&str>, code: &str, _: &Theme, font: &egui::FontId| {
            seen.borrow_mut().push(format!("{}:{code}", lang.unwrap_or("")));
            Some(egui::text::LayoutJob::simple(
                code.to_owned(),
                font.clone(),
                egui::Color32::RED,
                f32::INFINITY,
            ))
        };
        MarkdownView::new("hl", &doc).highlighter(&hl).show(ui);
    });
    h.run();
    assert_eq!(calls.borrow().first().map(String::as_str), Some("toml:a = 1"));
}
