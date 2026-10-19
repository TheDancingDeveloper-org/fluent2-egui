//! Pager: navigation buttons, page numbers, disabled ends and the page-size Dropdown, through
//! kittest (no wgpu).

use std::cell::Cell;

use egui::Vec2;
use egui_kittest::Harness;
use egui_kittest::kittest::{NodeT, Queryable};
use fluent2_egui::controls::{Pager, page_count};
use fluent2_egui::{Theme, install};

struct Probe {
    page: Cell<usize>,
    page_size: Cell<usize>,
    items: Cell<usize>,
    /// Last `PagerResponse::page` and `page_size` reported.
    reported: Cell<Option<usize>>,
    reported_size: Cell<Option<usize>>,
}

fn probe(page: usize, items: usize) -> &'static Probe {
    Box::leak(Box::new(Probe {
        page: Cell::new(page),
        page_size: Cell::new(10),
        items: Cell::new(items),
        reported: Cell::new(None),
        reported_size: Cell::new(None),
    }))
}

fn harness(probe: &'static Probe) -> Harness<'static> {
    Harness::builder()
        .with_size(Vec2::new(800.0, 300.0))
        .build_ui(move |ui| {
            install(ui.ctx(), &Theme::light(fluent2_tokens::FLUENT));
            let mut page = probe.page.get();
            let mut size = probe.page_size.get();
            let pages = page_count(probe.items.get(), size);
            let response = Pager::new("pager", &mut page, pages)
                .page_sizes(&mut size, &[10, 25, 50])
                .show(ui);
            probe.page.set(page);
            probe.page_size.set(size);
            if response.page.is_some() {
                probe.reported.set(response.page);
            }
            if response.page_size.is_some() {
                probe.reported_size.set(response.page_size);
            }
        })
}

fn disabled(harness: &Harness<'static>, label: &str) -> bool {
    harness.get_by_label(label).accesskit_node().is_disabled()
}

#[test]
fn next_previous_first_and_last_move_the_page() {
    let probe = probe(0, 200); // 20 pages
    let mut harness = harness(probe);
    harness.run();
    assert!(disabled(&harness, "Previous page") && disabled(&harness, "First"));
    assert!(!disabled(&harness, "Next page"));

    harness.get_by_label("Next page").click();
    harness.run();
    assert_eq!((probe.page.get(), probe.reported.get()), (1, Some(1)));

    harness.get_by_label("Last").click();
    harness.run();
    assert_eq!(probe.page.get(), 19);
    assert!(disabled(&harness, "Next page") && disabled(&harness, "Last"));

    harness.get_by_label("Previous page").click();
    harness.run();
    assert_eq!(probe.page.get(), 18);

    harness.get_by_label("First").click();
    harness.run();
    assert_eq!(probe.page.get(), 0);
}

#[test]
fn clicking_a_page_number_selects_it_and_ellipses_hide_the_rest() {
    let probe = probe(0, 200);
    let mut harness = harness(probe);
    harness.run();
    // 1 2 3 4 5 … 20
    assert!(harness.query_by_label("5").is_some());
    assert!(harness.query_by_label("6").is_none());
    assert!(harness.query_by_label("…").is_some());
    harness.get_by_label("20").click();
    harness.run();
    assert_eq!(probe.page.get(), 19);
    // 1 … 16 17 18 19 20
    assert!(harness.query_by_label("16").is_some());
    assert!(harness.query_by_label("2").is_none());
}

#[test]
fn zero_pages_disable_everything_and_one_page_shows_one_number() {
    let probe = probe(5, 0);
    let mut harness = harness(probe);
    harness.run();
    assert_eq!(probe.page.get(), 0, "page is clamped");
    for label in ["First", "Previous page", "Next page", "Last"] {
        assert!(disabled(&harness, label), "{label} enabled with no pages");
    }
    assert!(harness.query_by_label("1").is_none());

    probe.items.set(3);
    harness.run();
    assert!(harness.query_by_label("1").is_some());
    assert!(harness.query_by_label("2").is_none());
    assert!(disabled(&harness, "Next page"));
}

#[test]
fn page_past_the_end_is_clamped_when_rows_shrink() {
    let probe = probe(15, 200);
    let mut harness = harness(probe);
    harness.run();
    probe.items.set(35); // 4 pages
    harness.run();
    assert_eq!(probe.page.get(), 3);
}

#[test]
fn page_size_change_keeps_the_first_item_in_view() {
    let probe = probe(7, 200); // items 70..80
    let mut harness = harness(probe);
    harness.run();
    harness.get_by_label("Page size").click();
    harness.run();
    harness.get_by_label("25 per page").click();
    harness.run();
    assert_eq!(probe.page_size.get(), 25);
    assert_eq!(probe.reported_size.get(), Some(25));
    // Item 70 is on page 2 (items 50..75) at 25 per page.
    assert_eq!(probe.page.get(), 2);
}
