//! FileDropZone: take files dragged in from the operating system (WI-1944, behind
//! `file-dialog`).
//!
//! egui already reports drags and drops in `RawInput::hovered_files` and `dropped_files`: the
//! eframe native backend gives paths, and the eframe web backend gives browser `File`s (from
//! the drop event's `DataTransfer`). This zone adds what an app needs on top:
//!
//! - a Fluent overlay over the zone while files are dragged over the window (a brand dashed
//!   border on a tinted fill, an icon and a label);
//! - reading the dropped files off the UI thread: a worker thread on native, the browser's
//!   async file read on wasm. Results come back as [`PickedFile`]s (the type
//!   [`super::file_dialog::FileDialog`] returns), in [`FileDropResponse::received`] on a later
//!   frame, so nothing blocks.
//!
//! egui does not report where a drop lands, so a zone takes a drop when the pointer was last
//! seen inside it (or was never seen), or always with [`FileDropZone::window_wide`].

use std::sync::{Arc, Mutex};

use egui::{Color32, CornerRadius, Id, Rect, Stroke, Ui, Vec2};
use fluent2_tokens::generated::global::{border_radius, stroke_width};

use super::file_dialog::PickedFile;
use crate::foundation::icon::{IconName, IconSize, IconStyle, paint_icon};
use crate::foundation::text::{TypeRamp, font_id};
use crate::theme::{Theme, c};

/// What a [`FileDropZone`] saw this frame.
#[derive(Debug, Default)]
pub struct FileDropResponse {
    /// Files are being dragged over the window and this zone would take them.
    pub hovering: bool,
    /// Names of files dropped on the zone this frame; their bytes arrive later.
    pub dropped: Vec<String>,
    /// Files read since the last frame.
    pub received: Vec<PickedFile>,
    /// Files that could not be read since the last frame: name and error.
    pub failed: Vec<(String, String)>,
}

type Results = Arc<Mutex<Vec<Result<PickedFile, (String, String)>>>>;

/// A drop target. See the module docs.
///
/// ```
/// # use fluent2_egui::platform::file_drop::FileDropZone;
/// # fn demo(ui: &mut egui::Ui, attachments: &mut Vec<String>) {
/// let out = FileDropZone::new("compose").label("Drop files to attach").show(ui, |ui| {
///     ui.label("Message body…");
/// });
/// for file in out.received {
///     attachments.push(file.name);
/// }
/// # }
/// ```
pub struct FileDropZone {
    id: Id,
    label: String,
    window_wide: bool,
}

impl FileDropZone {
    /// A zone named by `id_salt`.
    pub fn new(id_salt: impl std::hash::Hash + std::fmt::Debug) -> Self {
        Self {
            id: Id::new(id_salt),
            label: "Drop files here".to_owned(),
            window_wide: false,
        }
    }

    /// The overlay's text. The default is "Drop files here".
    #[must_use]
    pub fn label(mut self, label: impl Into<String>) -> Self {
        self.label = label.into();
        self
    }

    /// Take every drop on the window, wherever the pointer is.
    #[must_use]
    pub fn window_wide(mut self, window_wide: bool) -> Self {
        self.window_wide = window_wide;
        self
    }

    /// Show `content` inside the zone, the overlay over it while files are dragged, and collect
    /// drops.
    pub fn show(self, ui: &mut Ui, content: impl FnOnce(&mut Ui)) -> FileDropResponse {
        let id = ui.make_persistent_id(self.id);
        let results_id = id.with("results");
        let pointer_id = id.with("pointer");
        let inner = ui.scope(content);
        let rect = inner.response.rect;

        // The last pointer position seen over the window, kept through the drag (some
        // platforms stop reporting the pointer while an OS drag is in progress).
        let pointer = ui.input(|i| i.pointer.latest_pos());
        if let Some(pos) = pointer {
            ui.ctx().data_mut(|d| d.insert_temp(pointer_id, pos));
        }
        let last = ui.ctx().data(|d| d.get_temp::<egui::Pos2>(pointer_id));
        let targeted = self.window_wide || last.is_none_or(|pos| rect.contains(pos));

        let hovered_files = ui.input(|i| i.raw.hovered_files.len());
        let mut out = FileDropResponse {
            hovering: hovered_files > 0 && targeted,
            ..FileDropResponse::default()
        };
        if out.hovering {
            paint_overlay(ui, rect, &self.label);
            ui.ctx().request_repaint();
        }

        // Start reading dropped files. The input keeps them only for this frame.
        let dropped = if targeted {
            ui.input(|i| i.raw.dropped_files.clone())
        } else {
            Vec::new()
        };
        if !dropped.is_empty() {
            let results: Results = ui.ctx().data_mut(|d| {
                d.get_temp_mut_or_insert_with(results_id, Results::default)
                    .clone()
            });
            for file in dropped {
                out.dropped.push(file_name(file.path()));
                read(ui.ctx().clone(), file, results.clone());
            }
        }

        // Collect what finished.
        if let Some(results) = ui.ctx().data(|d| d.get_temp::<Results>(results_id)) {
            let finished: Vec<_> = results
                .lock()
                .map(|mut done| std::mem::take(&mut *done))
                .unwrap_or_default();
            for result in finished {
                match result {
                    Ok(file) => out.received.push(file),
                    Err(failure) => out.failed.push(failure),
                }
            }
        }
        out
    }
}

fn file_name(path: &std::path::Path) -> String {
    path.file_name()
        .map(|name| name.to_string_lossy().into_owned())
        .unwrap_or_else(|| path.to_string_lossy().into_owned())
}

#[cfg(not(target_arch = "wasm32"))]
fn read(ctx: egui::Context, file: egui::DroppedFileHandle, results: Results) {
    let name = file_name(file.path());
    let worker_name = name.clone();
    let failure_results = results.clone();
    let spawned = std::thread::Builder::new()
        .name("fluent2-file-drop".into())
        .spawn(move || {
            let path = file.path().to_path_buf();
            let result = file
                .bytes()
                .map(|bytes| PickedFile {
                    name: worker_name.clone(),
                    path: (!path.as_os_str().is_empty()).then_some(path),
                    bytes,
                })
                .map_err(|err| (worker_name, err));
            if let Ok(mut done) = results.lock() {
                done.push(result);
            }
            ctx.request_repaint();
        });
    if let Err(err) = spawned
        && let Ok(mut done) = failure_results.lock()
    {
        done.push(Err((name, format!("could not start a reader: {err}"))));
    }
}

#[cfg(target_arch = "wasm32")]
fn read(ctx: egui::Context, file: egui::DroppedFileHandle, results: Results) {
    wasm_bindgen_futures::spawn_local(async move {
        let name = file_name(file.path());
        let result = file
            .bytes_async()
            .await
            .map(|bytes| PickedFile {
                name: name.clone(),
                path: None,
                bytes,
            })
            .map_err(|err| (name, err));
        if let Ok(mut done) = results.lock() {
            done.push(result);
        }
        ctx.request_repaint();
    });
}

/// The drop overlay: a tinted fill, a dashed brand border, an icon and the label.
fn paint_overlay(ui: &Ui, rect: Rect, label: &str) {
    let theme = Theme::get(ui.ctx());
    let color = &theme.color;
    // The zone's content is already painted, so painting on its layer puts the overlay on top.
    let painter = ui.painter().clone();
    let rect = rect.shrink(1.0);
    let fill = c(color.brand_background_2);
    painter.rect_filled(
        rect,
        CornerRadius::same(border_radius::LARGE as u8),
        Color32::from_rgba_unmultiplied(fill.r(), fill.g(), fill.b(), 230),
    );
    let stroke = Stroke::new(stroke_width::THICK, c(color.compound_brand_stroke));
    dashed_rect(&painter, rect, stroke);
    let center = rect.center();
    paint_icon(
        ui,
        Rect::from_center_size(center - Vec2::new(0.0, 16.0), Vec2::splat(24.0)),
        IconName::Attach,
        IconSize::S24,
        IconStyle::Regular,
        Some(c(color.brand_foreground_1)),
    );
    painter.text(
        center + Vec2::new(0.0, 12.0),
        egui::Align2::CENTER_CENTER,
        label,
        font_id(TypeRamp::Body1),
        c(color.neutral_foreground_1),
    );
}

fn dashed_rect(painter: &egui::Painter, rect: Rect, stroke: Stroke) {
    let corners = [
        rect.left_top(),
        rect.right_top(),
        rect.right_bottom(),
        rect.left_bottom(),
        rect.left_top(),
    ];
    for pair in corners.windows(2) {
        if let [a, b] = pair {
            painter.add(egui::Shape::dashed_line(&[*a, *b], stroke, 6.0, 4.0));
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn names_from_paths() {
        assert_eq!(file_name(std::path::Path::new("/tmp/report.pdf")), "report.pdf");
        assert_eq!(file_name(std::path::Path::new("photo.jpg")), "photo.jpg");
    }
}
