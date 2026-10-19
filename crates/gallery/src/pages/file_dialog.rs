//! File dialog: open a file and show its name and size, or save a short text file. The
//! dialog runs off the UI thread and the page polls it every frame.

use fluent2_egui::controls::{Appearance, Button};
use fluent2_egui::platform::file_dialog::{FileDialog, FileDialogOutcome, FileFilter};

use super::Page;

#[derive(Default)]
pub struct FileDialogPage {
    dialog: FileDialog,
    status: String,
}

impl Page for FileDialogPage {
    fn name(&self) -> &'static str {
        "File dialog"
    }

    fn ui(&mut self, ui: &mut egui::Ui) {
        if let Some(outcome) = self.dialog.poll() {
            self.status = describe(&outcome);
        }

        ui.heading("File dialog");
        ui.label("The dialog runs off the UI thread; the page polls it every frame.");
        ui.add_space(8.0);
        let idle = !self.dialog.is_pending();
        ui.horizontal(|ui| {
            let open = Button::new("Open file…")
                .appearance(Appearance::Primary)
                .id_salt("file_dialog_open");
            if ui.add_enabled(idle, open).clicked() {
                self.dialog.open(ui.ctx(), "Open a file", &[]);
            }
            let many = Button::new("Open several…").id_salt("file_dialog_open_many");
            if ui.add_enabled(idle, many).clicked() {
                self.dialog.open_many(ui.ctx(), "Open files", &[]);
            }
            let save = Button::new("Save sample…").id_salt("file_dialog_save");
            if ui.add_enabled(idle, save).clicked() {
                self.dialog.save(
                    ui.ctx(),
                    "Save sample",
                    "fluent2-sample.txt",
                    &[FileFilter::new("Text", &["txt"])],
                    b"Saved from the fluent2-egui gallery.\n".to_vec(),
                );
            }
        });
        ui.add_space(8.0);
        if !idle {
            ui.label("Waiting for the dialog…");
        } else if !self.status.is_empty() {
            ui.label(&self.status);
        }
    }
}

fn describe(outcome: &FileDialogOutcome) -> String {
    match outcome {
        FileDialogOutcome::Opened(file) => {
            format!("Opened {} ({} bytes)", file.name, file.bytes.len())
        }
        FileDialogOutcome::OpenedMany(files) => {
            let names: Vec<&str> = files.iter().map(|f| f.name.as_str()).collect();
            format!("Opened {} files: {}", files.len(), names.join(", "))
        }
        FileDialogOutcome::Paths(paths) => format!("Picked {} paths", paths.len()),
        FileDialogOutcome::SavePath(path) => format!("Save to {}", path.display()),
        FileDialogOutcome::Saved { name, .. } => format!("Saved {name}"),
        FileDialogOutcome::Cancelled => "Cancelled".to_owned(),
        FileDialogOutcome::Failed(err) => format!("Save failed: {err}"),
    }
}
