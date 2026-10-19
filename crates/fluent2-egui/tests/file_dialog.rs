//! FileDialog: the frame-polled wrapper around `rfd` (feature `file-dialog`). The dialogs
//! themselves need a desktop; these tests cover the state the app sees without one.
#![cfg(feature = "file-dialog")]

use fluent2_egui::platform::file_dialog::{FileDialog, FileDialogOutcome, FileFilter, PickedFile};

#[test]
fn idle_dialog_is_not_pending_and_polls_nothing() {
    let mut dialog = FileDialog::new();
    assert!(!dialog.is_pending());
    assert_eq!(dialog.poll(), None);
    assert_eq!(dialog.poll(), None);
}

#[test]
fn filter_keeps_name_and_extensions() {
    let filter = FileFilter::new("Images", &["png", "jpg"]);
    assert_eq!(filter.name, "Images");
    assert_eq!(filter.extensions, vec!["png".to_owned(), "jpg".to_owned()]);
}

#[test]
fn opened_outcome_carries_the_file() {
    let file = PickedFile {
        name: "notes.txt".into(),
        path: None,
        bytes: b"hi".to_vec(),
    };
    let outcome = FileDialogOutcome::Opened(file.clone());
    assert_eq!(outcome, FileDialogOutcome::Opened(file));
    assert_ne!(outcome, FileDialogOutcome::Cancelled);
}
