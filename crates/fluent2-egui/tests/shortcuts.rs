//! Shortcut registry: conflicts, key handling, text-field focus, menu captions and the overlay.

use std::cell::{Cell, RefCell};

use egui::{Key, KeyboardShortcut, Modifiers};
use egui_kittest::Harness;
use egui_kittest::kittest::Queryable;
use fluent2_egui::foundation::shortcuts::{self, Shortcut, ShortcutError, Shortcuts, ShortcutsExt};
use fluent2_egui::surfaces::{MenuItem, ShortcutOverlay};
use fluent2_egui::{Theme, install};

const SAVE: KeyboardShortcut = KeyboardShortcut::new(Modifiers::COMMAND, Key::S);
const SAVE_AS: KeyboardShortcut = KeyboardShortcut::new(Modifiers::COMMAND.plus(Modifiers::SHIFT), Key::S);
const NEXT: KeyboardShortcut = KeyboardShortcut::new(Modifiers::NONE, Key::J);

fn registry() -> Shortcuts {
    let mut registry = Shortcuts::new();
    for shortcut in [
        Shortcut::new("file.save", SAVE, "Save").group("File"),
        Shortcut::new("file.save_as", SAVE_AS, "Save as").group("File"),
        Shortcut::new("nav.next", NEXT, "Next message").group("Navigation"),
    ] {
        assert!(registry.register(shortcut).is_ok());
    }
    registry
}

#[test]
fn duplicate_bindings_are_refused() {
    let mut registry = registry();
    let ctrl_s = KeyboardShortcut::new(Modifiers::CTRL, Key::S);
    assert_eq!(
        registry.register(Shortcut::new("file.store", ctrl_s, "Store")),
        Err(ShortcutError::Conflict {
            id: "file.store",
            existing: "file.save",
        })
    );
    let other = KeyboardShortcut::new(Modifiers::COMMAND, Key::O);
    assert_eq!(
        registry.register(Shortcut::new("file.save", other, "Open")),
        Err(ShortcutError::DuplicateId("file.save"))
    );
    assert_eq!(registry.len(), 3);
    assert_eq!(registry.groups(), vec!["File", "Navigation"]);
}

struct Counts {
    save: Cell<u32>,
    save_twice: Cell<u32>,
    save_as: Cell<u32>,
    next: Cell<u32>,
}

fn counts() -> &'static Counts {
    Box::leak(Box::new(Counts {
        save: Cell::new(0),
        save_twice: Cell::new(0),
        save_as: Cell::new(0),
        next: Cell::new(0),
    }))
}

fn key_harness(counts: &'static Counts, text: &'static RefCell<String>) -> Harness<'static> {
    let mut harness = Harness::new_ui(move |ui| {
        install(ui.ctx(), &Theme::default());
        shortcuts::install(ui.ctx(), registry());
        let ctx = ui.ctx().clone();
        // Wider bindings first or last must not matter.
        if ctx.shortcut_pressed("file.save") {
            counts.save.set(counts.save.get() + 1);
        }
        if ctx.shortcut_pressed("file.save") {
            counts.save_twice.set(counts.save_twice.get() + 1);
        }
        if ctx.shortcut_pressed("file.save_as") {
            counts.save_as.set(counts.save_as.get() + 1);
        }
        if ctx.shortcut_pressed("nav.next") {
            counts.next.set(counts.next.get() + 1);
        }
        let mut text = text.borrow_mut();
        let _ = ui.add(egui::TextEdit::singleline(&mut *text).hint_text("Search"));
    });
    harness.run();
    harness
}

fn press(harness: &mut Harness<'_>, modifiers: Modifiers, key: Key) {
    harness.key_press_modifiers(modifiers, key);
    harness.run();
}

#[test]
fn a_press_fires_once_and_is_consumed() {
    let counts = counts();
    let text = Box::leak(Box::new(RefCell::new(String::new())));
    let mut harness = key_harness(counts, text);
    press(&mut harness, Modifiers::COMMAND, Key::S);
    assert_eq!(counts.save.get(), 1);
    assert_eq!(counts.save_twice.get(), 0);
    assert_eq!(counts.save_as.get(), 0);
}

#[test]
fn a_wider_binding_wins() {
    let counts = counts();
    let text = Box::leak(Box::new(RefCell::new(String::new())));
    let mut harness = key_harness(counts, text);
    press(&mut harness, Modifiers::COMMAND | Modifiers::SHIFT, Key::S);
    assert_eq!(counts.save.get(), 0);
    assert_eq!(counts.save_as.get(), 1);
}

#[test]
fn text_fields_do_not_trigger_single_letter_shortcuts() {
    let counts = counts();
    let text = Box::leak(Box::new(RefCell::new(String::new())));
    let mut harness = key_harness(counts, text);
    press(&mut harness, Modifiers::NONE, Key::J);
    assert_eq!(counts.next.get(), 1);

    harness.get_by_role(egui::accesskit::Role::TextInput).focus();
    harness.run();
    press(&mut harness, Modifiers::NONE, Key::J);
    assert_eq!(counts.next.get(), 1, "a bare key fired while typing");
    // Command bindings still fire from a text field.
    press(&mut harness, Modifiers::COMMAND, Key::S);
    assert_eq!(counts.save.get(), 1);
}

#[test]
fn menu_items_and_tooltips_read_the_registry() {
    let mut harness = Harness::new_ui(|ui| {
        install(ui.ctx(), &Theme::default());
        shortcuts::install(ui.ctx(), registry());
        let _ = ui.add(MenuItem::new("Save").bound("file.save"));
        let _ = ui.add(MenuItem::new("Unknown").bound("no.such"));
    });
    harness.run();
    let caption = harness.ctx.format_shortcut(&SAVE);
    assert_eq!(harness.ctx.shortcut_caption("file.save"), Some(caption.clone()));
    assert_eq!(
        harness.ctx.shortcut_tooltip("file.save"),
        Some(format!("Save ({caption})"))
    );
    assert_eq!(harness.ctx.shortcut_caption("no.such"), None);
    assert!(harness.query_by_label("Save").is_some());
}

#[test]
fn overlay_lists_bindings_by_group_in_every_theme() {
    for theme in [
        Theme::default(),
        Theme::dark(fluent2_egui::tokens::FLUENT),
        Theme::high_contrast(),
    ] {
        let mut harness = Harness::new_ui(move |ui| {
            let ctx = &ui.ctx().clone();
            install(ctx, &theme);
            shortcuts::install(ctx, registry());
            let mut open = true;
            let _ = ShortcutOverlay::new("help").show(ctx, &mut open);
        });
        harness.run();
        for label in [
            "Keyboard shortcuts",
            "File",
            "Navigation",
            "Save",
            "Save as",
            "Next message",
        ] {
            assert!(harness.query_by_label(label).is_some(), "missing {label}");
        }
        let caption = harness.ctx.format_shortcut(&NEXT);
        assert!(harness.query_by_label(&caption).is_some());
    }
}
