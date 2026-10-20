//! File dialog: open a file and show its name and size, or save a short text file. The
//! dialog runs off the UI thread and the page polls it every frame.

use fluent2_egui::controls::{Appearance, Button};
use fluent2_egui::platform::file_dialog::{FileDialog, FileDialogOutcome, FileFilter};
use fluent2_egui::platform::file_drop::FileDropZone;

use super::Page;

#[derive(Default)]
pub struct FileDialogPage {
    dialog: FileDialog,
    status: String,
    dropped: Vec<String>,
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

        ui.add_space(16.0);
        ui.strong("Drop files");
        ui.label("Drag files from the desktop onto the box. They are read off the UI thread.");
        let out = FileDropZone::new("gallery_drop")
            .label("Drop files here")
            .show(ui, |ui| {
                egui::Frame::new()
                    .stroke(egui::Stroke::new(1.0, ui.visuals().weak_text_color()))
                    .corner_radius(4.0)
                    .inner_margin(12.0)
                    .show(ui, |ui| {
                        ui.set_min_size(egui::vec2(ui.available_width().min(480.0), 96.0));
                        if self.dropped.is_empty() {
                            ui.weak("Nothing dropped yet");
                        }
                        for line in &self.dropped {
                            ui.label(line);
                        }
                    });
            });
        for file in out.received {
            self.dropped
                .push(format!("{} ({} bytes)", file.name, file.bytes.len()));
        }
        for (name, error) in out.failed {
            self.dropped.push(format!("{name}: {error}"));
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
