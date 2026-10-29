//! Spell checking on Textarea and RichTextEditor, with a checker the test supplies.
//!
//! The library has no dictionary. These tests count how often the checker runs, so a repaint
//! that does not change the text must not call it again.

use std::cell::RefCell;
use std::rc::Rc;

use egui_kittest::Harness;
use egui_kittest::kittest::Queryable;
use fluent2_egui::controls::{RichText, RichTextEditor, SpellChecker, Spelling, Textarea, WordListChecker};
use fluent2_egui::theme::{self, Theme};
use fluent2_tokens::FLUENT;

fn install(ui: &egui::Ui) {
    theme::install(ui.ctx(), &Theme::light(FLUENT));
}

/// Counts calls and flags a fixed word.
struct CountingChecker {
    calls: Rc<RefCell<usize>>,
    word: String,
    suggestions: Vec<String>,
}

impl SpellChecker for CountingChecker {
    fn check(&mut self, text: &str) -> Vec<Spelling> {
        *self.calls.borrow_mut() += 1;
        let mut list = WordListChecker {
            misspelled: vec![self.word.clone()],
            suggestions: self.suggestions.clone(),
        };
        list.check(text)
    }
}

#[test]
fn textarea_checker_runs_when_the_text_changes_and_not_on_repaint() {
    let calls = Rc::new(RefCell::new(0usize));
    let calls_ui = Rc::clone(&calls);
    let value = Rc::new(RefCell::new("teh cat".to_owned()));
    let shown = Rc::clone(&value);
    let mut harness = Harness::new_ui(move |ui| {
        install(ui);
        let mut text = shown.borrow().clone();
        let mut checker = CountingChecker {
            calls: Rc::clone(&calls_ui),
            word: "teh".to_owned(),
            suggestions: vec!["the".to_owned()],
        };
        ui.add(
            Textarea::new(&mut text)
                .label("Notes")
                .rows(2)
                .spell_checker(&mut checker),
        );
        *shown.borrow_mut() = text;
    });
    harness.run();
    let first = *calls.borrow();
    assert!(first >= 1, "the checker should run for the initial text");
    harness.run();
    assert_eq!(
        *calls.borrow(),
        first,
        "a repaint with the same text must not call the checker again"
    );

    harness.get_by_label("Notes").focus();
    harness.run();
    harness.get_by_label("Notes").type_text("s");
    harness.run();
    assert!(*calls.borrow() > first, "typing should check the new text");
    assert!(value.borrow().ends_with('s'));
}

#[test]
fn word_list_checker_flags_only_listed_words() {
    let mut checker = WordListChecker {
        misspelled: vec!["teh".to_owned()],
        suggestions: vec!["the".to_owned(), "tea".to_owned()],
    };
    let found = checker.check("teh cat teh.");
    assert_eq!(found.len(), 2);
    assert_eq!(found[0].range, 0..3);
    assert_eq!(found[1].range, 8..11);
    assert_eq!(found[0].suggestions, vec!["the".to_owned(), "tea".to_owned()]);
    assert!(checker.check("the cat").is_empty());
}

#[test]
fn rich_text_editor_accepts_a_checker() {
    let calls = Rc::new(RefCell::new(0usize));
    let calls_ui = Rc::clone(&calls);
    let doc = Rc::new(RefCell::new(RichText::from_plain("teh notes")));
    let shown = Rc::clone(&doc);
    let mut harness = Harness::new_ui(move |ui| {
        install(ui);
        let mut doc = shown.borrow().clone();
        let mut checker = CountingChecker {
            calls: Rc::clone(&calls_ui),
            word: "teh".to_owned(),
            suggestions: vec!["the".to_owned()],
        };
        let _ = RichTextEditor::new("body", &mut doc)
            .label("Message")
            .rows(3)
            .spell_checker(&mut checker)
            .show(ui);
        *shown.borrow_mut() = doc;
    });
    harness.run();
    assert!(*calls.borrow() >= 1);
    let again = *calls.borrow();
    harness.run();
    assert_eq!(*calls.borrow(), again);
    assert_eq!(doc.borrow().text(), "teh notes");
}
