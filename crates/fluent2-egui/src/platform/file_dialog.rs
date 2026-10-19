//! Open and save dialogs through [`rfd`], polled from the egui frame (feature `file-dialog`).
//!
//! Nothing here blocks the UI thread. [`FileDialog::open`] and [`FileDialog::save`] start the
//! platform dialog and return at once: on native targets the dialog future runs on a worker
//! thread, on wasm it runs on the browser's event loop. The app calls [`FileDialog::poll`]
//! every frame; it returns the outcome once, when the dialog closes, and asks egui to repaint
//! so that frame arrives.
//!
//! ```no_run
//! use fluent2_egui::platform::file_dialog::{FileDialog, FileDialogOutcome, FileFilter};
//!
//! # fn demo(ui: &mut egui::Ui, dialog: &mut FileDialog) {
//! if ui.button("Open…").clicked() {
//!     dialog.open(ui.ctx(), "Open a file", &[FileFilter::new("Text", &["txt", "md"])]);
//! }
//! if let Some(FileDialogOutcome::Opened(file)) = dialog.poll() {
//!     println!("{} ({} bytes)", file.name, file.bytes.len());
//! }
//! # }
//! ```
//!
//! On wasm a save writes through the browser's download prompt; filters are ignored there.

use std::future::Future;
use std::sync::mpsc::{Receiver, TryRecvError, channel};

/// A named group of file extensions shown in the dialog's type list.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct FileFilter {
    /// Label, for example `"Images"`.
    pub name: String,
    /// Extensions without the dot, for example `["png", "jpg"]`.
    pub extensions: Vec<String>,
}

impl FileFilter {
    /// Filter called `name` matching `extensions` (without the dot).
    #[must_use]
    pub fn new(name: impl Into<String>, extensions: &[&str]) -> Self {
        Self {
            name: name.into(),
            extensions: extensions.iter().map(|e| (*e).to_owned()).collect(),
        }
    }
}

/// A file the user picked, read into memory.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct PickedFile {
    /// File name without directories. Browsers give only this.
    pub name: String,
    /// Full path on native targets; [`None`] on wasm.
    pub path: Option<std::path::PathBuf>,
    /// File contents.
    pub bytes: Vec<u8>,
}

/// How a dialog ended. [`FileDialog::poll`] returns it once.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum FileDialogOutcome {
    /// An open dialog returned a file.
    Opened(PickedFile),
    /// A save dialog wrote the bytes to the file with this name.
    Saved {
        /// File name without directories.
        name: String,
        /// Full path on native targets; [`None`] on wasm.
        path: Option<std::path::PathBuf>,
    },
    /// The user closed the dialog without choosing.
    Cancelled,
    /// Writing the saved file failed. The text is the I/O error.
    Failed(String),
}

/// One pending dialog at a time, polled from the frame. Keep it in app state.
#[derive(Debug, Default)]
pub struct FileDialog {
    pending: Option<Receiver<FileDialogOutcome>>,
}

impl FileDialog {
    /// No dialog open.
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    /// `true` while a dialog started here has not reported back yet.
    #[must_use]
    pub fn is_pending(&self) -> bool {
        self.pending.is_some()
    }

    /// Show an open-file dialog. Ignored while another dialog is pending.
    pub fn open(&mut self, ctx: &egui::Context, title: &str, filters: &[FileFilter]) {
        if self.is_pending() {
            return;
        }
        let dialog = with_filters(rfd::AsyncFileDialog::new().set_title(title), filters);
        self.start(ctx, async move {
            match dialog.pick_file().await {
                Some(handle) => FileDialogOutcome::Opened(PickedFile {
                    name: handle.file_name(),
                    path: handle_path(&handle),
                    bytes: handle.read().await,
                }),
                None => FileDialogOutcome::Cancelled,
            }
        });
    }

    /// Show a save dialog proposing `file_name`, then write `bytes` to the chosen file.
    /// Ignored while another dialog is pending.
    pub fn save(
        &mut self,
        ctx: &egui::Context,
        title: &str,
        file_name: &str,
        filters: &[FileFilter],
        bytes: Vec<u8>,
    ) {
        if self.is_pending() {
            return;
        }
        let dialog = with_filters(
            rfd::AsyncFileDialog::new()
                .set_title(title)
                .set_file_name(file_name),
            filters,
        );
        self.start(ctx, async move {
            let Some(handle) = dialog.save_file().await else {
                return FileDialogOutcome::Cancelled;
            };
            match handle.write(&bytes).await {
                Ok(()) => FileDialogOutcome::Saved {
                    name: handle.file_name(),
                    path: handle_path(&handle),
                },
                Err(err) => FileDialogOutcome::Failed(err.to_string()),
            }
        });
    }

    /// The outcome of the pending dialog, once, on the first frame after it closes.
    /// [`None`] while it is still open or when nothing is pending.
    pub fn poll(&mut self) -> Option<FileDialogOutcome> {
        let rx = self.pending.as_ref()?;
        match rx.try_recv() {
            Ok(outcome) => {
                self.pending = None;
                Some(outcome)
            }
            Err(TryRecvError::Empty) => None,
            Err(TryRecvError::Disconnected) => {
                self.pending = None;
                Some(FileDialogOutcome::Cancelled)
            }
        }
    }

    fn start<F>(&mut self, ctx: &egui::Context, task: F)
    where
        F: Future<Output = FileDialogOutcome> + MaybeSend + 'static,
    {
        let (tx, rx) = channel();
        self.pending = Some(rx);
        let ctx = ctx.clone();
        spawn(async move {
            let outcome = task.await;
            // The receiver is gone if the app dropped its FileDialog; nothing to report then.
            let _ = tx.send(outcome);
            ctx.request_repaint();
        });
    }
}

fn with_filters(mut dialog: rfd::AsyncFileDialog, filters: &[FileFilter]) -> rfd::AsyncFileDialog {
    for filter in filters {
        dialog = dialog.add_filter(&filter.name, &filter.extensions);
    }
    dialog
}

#[cfg(not(target_arch = "wasm32"))]
fn handle_path(handle: &rfd::FileHandle) -> Option<std::path::PathBuf> {
    Some(handle.path().to_path_buf())
}

#[cfg(target_arch = "wasm32")]
fn handle_path(_handle: &rfd::FileHandle) -> Option<std::path::PathBuf> {
    None
}

/// `Send` on native, where the dialog runs on a worker thread; anything on wasm.
#[cfg(not(target_arch = "wasm32"))]
trait MaybeSend: Send {}
#[cfg(not(target_arch = "wasm32"))]
impl<T: Send> MaybeSend for T {}
#[cfg(target_arch = "wasm32")]
trait MaybeSend {}
#[cfg(target_arch = "wasm32")]
impl<T> MaybeSend for T {}

#[cfg(not(target_arch = "wasm32"))]
fn spawn(task: impl Future<Output = ()> + Send + 'static) {
    let spawned = std::thread::Builder::new()
        .name("fluent2-file-dialog".into())
        .spawn(move || pollster::block_on(task));
    if let Err(err) = spawned {
        // The receiver sees the dropped sender and reports Cancelled.
        log::warn!("file dialog thread did not start: {err}");
    }
}

#[cfg(target_arch = "wasm32")]
fn spawn(task: impl Future<Output = ()> + 'static) {
    wasm_bindgen_futures::spawn_local(task);
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn outcome_arrives_once_and_clears_pending() {
        let ctx = egui::Context::default();
        let mut dialog = FileDialog::new();
        dialog.start(&ctx, async { FileDialogOutcome::Cancelled });
        let mut outcome = None;
        for _ in 0..500 {
            outcome = dialog.poll();
            if outcome.is_some() {
                break;
            }
            std::thread::sleep(std::time::Duration::from_millis(2));
        }
        assert_eq!(outcome, Some(FileDialogOutcome::Cancelled));
        assert!(!dialog.is_pending());
        assert_eq!(dialog.poll(), None);
    }

    #[test]
    fn dropped_task_reports_cancelled() {
        let (tx, rx) = channel::<FileDialogOutcome>();
        drop(tx);
        let mut dialog = FileDialog { pending: Some(rx) };
        assert_eq!(dialog.poll(), Some(FileDialogOutcome::Cancelled));
        assert!(!dialog.is_pending());
    }
}
