//! Text selection across read-only Fluent text: a drag or Shift+arrows select label text, Ctrl+C
//! copies it, across several labels in one card, and interactive controls beside the text keep
//! working. Selection comes from egui's label selection, which Fluent's theme keeps on.

use std::cell::Cell;
use std::rc::Rc;

use egui::output::OutputCommand;
use egui::{Event, Key, Modifiers, PointerButton, Pos2, Vec2};
use egui_kittest::Harness;
use egui_kittest::kittest::Queryable;
use fluent2_egui::controls::{Appearance, Button};
use fluent2_egui::{Card, Theme, ThemeKind, install};

fn theme(kind: ThemeKind) -> Theme {
    match kind {
        ThemeKind::Light => Theme::default(),
        ThemeKind::Dark => Theme::dark(fluent2_tokens::FLUENT),
        ThemeKind::HighContrast => Theme::high_contrast(),
    }
}

/// A card with two labels and a primary button beside them. `saves` counts the button's clicks.
fn harness(theme: Theme, saves: Rc<Cell<u32>>) -> Harness<'static> {
    Harness::builder()
        .with_size(Vec2::new(420.0, 240.0))
        .build_ui(move |ui| {
            install(ui.ctx(), &theme);
            Card::new().id_salt("card").show(ui, |ui| {
                ui.label("Alpha line of text");
                ui.label("Beta line of text");
                if ui
                    .add(Button::new("Save").appearance(Appearance::Primary))
                    .clicked()
                {
                    saves.set(saves.get() + 1);
                }
            });
        })
}

fn press(h: &Harness<'_>, at: Pos2, pressed: bool) {
    h.event(Event::PointerButton {
        pos: at,
        button: PointerButton::Primary,
        pressed,
        modifiers: Modifiers::NONE,
    });
}

/// Press at `from`, drag to `to`, release there.
fn drag(h: &mut Harness<'_>, from: Pos2, to: Pos2) {
    h.event(Event::PointerMoved(from));
    h.run();
    press(h, from, true);
    h.run();
    h.event(Event::PointerMoved(to));
    h.run();
    press(h, to, false);
    h.run();
}

fn click(h: &mut Harness<'_>, at: Pos2) {
    h.event(Event::PointerMoved(at));
    h.run();
    press(h, at, true);
    h.run();
    press(h, at, false);
    h.run();
}

/// The text the frame copied (Ctrl+C is an `Event::Copy`).
fn copy(h: &mut Harness<'_>) -> String {
    // One frame: the copy is emitted by the frame that sees the event.
    h.event(Event::Copy);
    h.step();
    h.output()
        .platform_output
        .commands
        .iter()
        .find_map(|command| match command {
            OutputCommand::CopyText(text) => Some(text.clone()),
            _ => None,
        })
        .unwrap_or_default()
}

#[test]
fn drag_across_two_labels_in_a_card_selects_and_copies_both() {
    for kind in [ThemeKind::Light, ThemeKind::Dark, ThemeKind::HighContrast] {
        let mut h = harness(theme(kind), Rc::new(Cell::new(0)));
        h.run();
        let alpha = h.get_by_label("Alpha line of text").rect();
        let beta = h.get_by_label("Beta line of text").rect();
        drag(
            &mut h,
            Pos2::new(alpha.left() + 0.5, alpha.center().y),
            Pos2::new(beta.right() - 0.5, beta.center().y),
        );
        let copied = copy(&mut h);
        assert!(copied.contains("Alpha line of text"), "{kind:?}: {copied:?}");
        assert!(copied.contains("Beta line of text"), "{kind:?}: {copied:?}");
    }
}

/// Install turns selection on in both egui themes, whatever the app had set before.
#[test]
fn install_turns_text_selection_on_in_both_egui_themes() {
    let ctx = egui::Context::default();
    for egui_theme in [egui::Theme::Light, egui::Theme::Dark] {
        ctx.style_mut_of(egui_theme, |style| {
            style.interaction.selectable_labels = false;
            style.interaction.multi_widget_text_select = false;
        });
    }
    install(&ctx, &Theme::default());
    for egui_theme in [egui::Theme::Light, egui::Theme::Dark] {
        let interaction = ctx.style_of(egui_theme).interaction.clone();
        assert!(interaction.selectable_labels, "{egui_theme:?}");
        assert!(interaction.multi_widget_text_select, "{egui_theme:?}");
    }
}

#[test]
fn shift_arrows_extend_a_selection_that_copies() {
    let mut h = harness(Theme::default(), Rc::new(Cell::new(0)));
    h.run();
    let alpha = h.get_by_label("Alpha line of text").rect();
    click(&mut h, Pos2::new(alpha.left() + 0.5, alpha.center().y));
    for _ in 0..5 {
        h.key_press_modifiers(Modifiers::SHIFT, Key::ArrowRight);
        h.run();
    }
    assert_eq!(copy(&mut h), "Alpha");
}

#[test]
fn a_button_beside_selectable_text_still_clicks() {
    let saves = Rc::new(Cell::new(0));
    let mut h = harness(Theme::default(), Rc::clone(&saves));
    h.run();
    h.get_by_label("Save").click();
    h.run();
    assert_eq!(saves.get(), 1);
}

#[test]
fn a_click_on_text_does_not_press_the_button() {
    let saves = Rc::new(Cell::new(0));
    let mut h = harness(Theme::default(), Rc::clone(&saves));
    h.run();
    let beta = h.get_by_label("Beta line of text").rect();
    click(&mut h, Pos2::new(beta.left() + 2.0, beta.center().y));
    assert_eq!(saves.get(), 0);
}
