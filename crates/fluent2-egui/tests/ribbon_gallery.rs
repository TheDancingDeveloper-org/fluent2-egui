//! Ribbon gallery behaviour (spec §4.4), without the wgpu snapshot feature.

use std::cell::{Cell, RefCell};

use egui::Key;
use egui::accesskit::Role;
use egui_kittest::Harness;
use egui_kittest::kittest::{By, Queryable};
use fluent2_egui::office::ribbon::{Gallery, GalleryTileSize};
use fluent2_egui::surfaces::MenuItem;
use fluent2_egui::{Theme, install};

/// Gallery state a test reads between frames. Leaked so the harness closure is `'static`.
struct Shared {
    count: usize,
    selected: RefCell<Option<usize>>,
    first_visible: Cell<usize>,
    changes: Cell<u32>,
    popup_open: Cell<bool>,
    footer_clicks: Cell<u32>,
}

fn shared(count: usize) -> &'static Shared {
    Box::leak(Box::new(Shared {
        count,
        selected: RefCell::new(None),
        first_visible: Cell::new(0),
        changes: Cell::new(0),
        popup_open: Cell::new(false),
        footer_clicks: Cell::new(0),
    }))
}

fn harness(shared: &'static Shared, theme: Theme) -> Harness<'static> {
    Harness::new_ui(move |ui| {
        install(ui.ctx(), &theme);
        let mut selected = shared.selected.borrow_mut();
        let response = Gallery::new("styles", "Styles", shared.count, &mut selected)
            .columns(4)
            .tile_size(GalleryTileSize::Medium)
            .item_label(|index| format!("Style {index}"))
            .category("Basic", 0)
            .category("More", 6)
            .footer(|ui| {
                if ui.add(MenuItem::new("Clear formatting")).clicked() {
                    shared.footer_clicks.set(shared.footer_clicks.get() + 1);
                }
            })
            .show(ui, |ui, _index, rect, state| {
                let color = if state.selected {
                    ui.visuals().strong_text_color()
                } else {
                    ui.visuals().text_color()
                };
                ui.painter().circle_filled(rect.center(), 4.0, color);
            });
        shared.first_visible.set(response.first_visible);
        shared.popup_open.set(response.popup_open);
        if response.changed {
            shared.changes.set(shared.changes.get() + 1);
        }
    })
}

#[test]
fn renders_in_every_theme() {
    for theme in [
        Theme::default(),
        Theme::dark(fluent2_tokens::FLUENT),
        Theme::high_contrast(),
    ] {
        let shared = shared(12);
        let mut harness = harness(shared, theme);
        harness.run();
        harness.get_by_role_and_label(Role::Button, "Style 0");
        harness.get_by_role_and_label(Role::Button, "More Styles");
    }
}

#[test]
fn scrolling_the_inline_row_changes_the_first_visible_index() {
    let shared = shared(10);
    let mut harness = harness(shared, Theme::default());
    harness.run();
    assert_eq!(shared.first_visible.get(), 0);
    assert!(harness.query_by_label("Style 4").is_none());

    harness.get_by_label("Next Styles row").click();
    harness.run();
    assert_eq!(shared.first_visible.get(), 4);
    harness.get_by_role_and_label(Role::Button, "Style 4");
    assert!(harness.query_by_label("Style 0").is_none());

    harness.get_by_label("Next Styles row").click();
    harness.run();
    assert_eq!(shared.first_visible.get(), 8);

    // The last row: next is disabled and does nothing.
    harness.get_by_label("Next Styles row").click();
    harness.run();
    assert_eq!(shared.first_visible.get(), 8);

    harness.get_by_label("Previous Styles row").click();
    harness.run();
    assert_eq!(shared.first_visible.get(), 4);
}

#[test]
fn inline_click_selects() {
    let shared = shared(10);
    let mut harness = harness(shared, Theme::default());
    harness.run();
    harness.get_by_label("Style 2").click();
    harness.run();
    assert_eq!(*shared.selected.borrow(), Some(2));
    assert_eq!(shared.changes.get(), 1);
}

#[test]
fn popup_selects_by_click() {
    let shared = shared(20);
    let mut harness = harness(shared, Theme::default());
    harness.run();
    harness.get_by_label("More Styles").click();
    harness.run();
    assert!(shared.popup_open.get());
    harness.get_by_label("Basic");

    harness.get_by_label("Style 9").click();
    harness.run();
    assert_eq!(*shared.selected.borrow(), Some(9));
    assert_eq!(shared.changes.get(), 1);
    assert!(!shared.popup_open.get());
    // The inline row scrolls to show the selection.
    assert_eq!(shared.first_visible.get(), 8);
}

#[test]
fn popup_selects_by_keyboard_and_escape_closes() {
    let shared = shared(20);
    let mut harness = harness(shared, Theme::default());
    harness.run();
    harness.get_by_label("More Styles").focus();
    harness.run();
    harness.key_press(Key::Enter);
    harness.run();
    assert!(shared.popup_open.get());

    harness.key_press(Key::ArrowRight);
    harness.run();
    harness.key_press(Key::ArrowRight);
    harness.run();
    harness.key_press(Key::Enter);
    harness.run();
    assert_eq!(*shared.selected.borrow(), Some(2));
    assert!(!shared.popup_open.get());

    harness.key_press(Key::Enter);
    harness.run();
    assert!(shared.popup_open.get(), "focus returned to the expand button");
    harness.key_press(Key::Escape);
    harness.run();
    assert!(!shared.popup_open.get());
    assert_eq!(*shared.selected.borrow(), Some(2));
    assert_eq!(shared.changes.get(), 1);
}

#[test]
fn footer_click_closes_the_popup() {
    let shared = shared(20);
    let mut harness = harness(shared, Theme::default());
    harness.run();
    harness.get_by_label("More Styles").click();
    harness.run();
    harness.get_by_label("Clear formatting").click();
    harness.run();
    assert_eq!(shared.footer_clicks.get(), 1);
    assert!(!shared.popup_open.get());
}

#[test]
fn a_thousand_items_lay_out_fewer_than_200_widgets() {
    let shared = shared(1_000);
    let mut harness = harness(shared, Theme::default());
    harness.run();
    harness.get_by_label("More Styles").click();
    harness.run();
    assert!(shared.popup_open.get());
    let items = harness
        .query_all(By::new().role(Role::Button).label_contains("Style "))
        .count();
    assert!(items > 0, "the popup shows items");
    assert!(items < 200, "{items} item widgets laid out");
}

#[test]
fn disabled_gallery_does_not_select() {
    let selected = Box::leak(Box::new(RefCell::new(None::<usize>)));
    let mut harness = Harness::new_ui(|ui| {
        install(ui.ctx(), &Theme::default());
        let mut selected = selected.borrow_mut();
        ui.add_enabled_ui(false, |ui| {
            Gallery::new("off", "Off", 4, &mut selected)
                .columns(4)
                .show(ui, |_, _, _, _| {});
        });
    });
    harness.run();
    harness.get_by_label("Off 1").click();
    harness.run();
    assert_eq!(*selected.borrow(), None);
}
