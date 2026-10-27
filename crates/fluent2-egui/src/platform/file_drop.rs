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
//!
//! Each drop is taken by one zone only: the first zone shown that frame that would take it,
//! so show specific zones before a window-wide one. Files larger than
//! [`FileDropZone::max_bytes`] (default [`DEFAULT_MAX_BYTES`], 50 MiB) are not read: their size
//! is checked first (file metadata on native, the browser `File`'s size on wasm) and they come
//! back in [`FileDropResponse::failed`]. A drop's files are read one after another by a single
//! worker, so a large drop cannot start many reads at once.

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

/// The default largest file a zone reads: 50 MiB.
pub const DEFAULT_MAX_BYTES: u64 = 50 * 1024 * 1024;

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
    max_bytes: u64,
}

impl FileDropZone {
    /// A zone named by `id_salt`.
    pub fn new(id_salt: impl std::hash::Hash + std::fmt::Debug) -> Self {
        Self {
            id: Id::new(id_salt),
            label: "Drop files here".to_owned(),
            window_wide: false,
            max_bytes: DEFAULT_MAX_BYTES,
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

    /// The largest file read, in bytes. Larger files are reported in
    /// [`FileDropResponse::failed`] without being read. The default is [`DEFAULT_MAX_BYTES`].
    #[must_use]
    pub fn max_bytes(mut self, max: u64) -> Self {
        self.max_bytes = max;
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

        // Start reading dropped files. The input keeps them only for this frame, and only one
        // zone takes each drop: the first targeted zone shown this frame claims it.
        let dropped = if targeted {
            ui.input(|i| i.raw.dropped_files.clone())
        } else {
            Vec::new()
        };
        if !dropped.is_empty() && claim(ui.ctx()) {
            let results: Results = ui.ctx().data_mut(|d| {
                d.get_temp_mut_or_insert_with(results_id, Results::default)
                    .clone()
            });
            out.dropped = dropped.iter().map(|file| file_name(file.path())).collect();
            read_all(ui.ctx().clone(), dropped, self.max_bytes, results);
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

/// Claim this frame's drop for the calling zone. False when another zone already took it.
fn claim(ctx: &egui::Context) -> bool {
    let key = Id::new("fluent2_file_drop_claim");
    let pass = ctx.cumulative_pass_nr();
    ctx.data_mut(|d| {
        if d.get_temp::<u64>(key) == Some(pass) {
            false
        } else {
            d.insert_temp(key, pass);
            true
        }
    })
}

fn too_large(name: &str, size: u64, max: u64) -> (String, String) {
    (
        name.to_owned(),
        format!(
            "{} is larger than the {} limit",
            readable_size(size),
            readable_size(max)
        ),
    )
}

/// "512 B", "4.0 KiB", "50.0 MiB", "1.5 GiB".
fn readable_size(bytes: u64) -> String {
    const UNITS: [&str; 3] = ["KiB", "MiB", "GiB"];
    if bytes < 1024 {
        return format!("{bytes} B");
    }
    let mut value = bytes as f64 / 1024.0;
    let mut unit = 0;
    while value >= 1024.0 && unit < UNITS.len() - 1 {
        value /= 1024.0;
        unit += 1;
    }
    format!("{value:.1} {}", UNITS[unit])
}

/// Read one file, refusing it when it is larger than `max`.
///
/// A regular file whose metadata reports a size over `max` is refused without being opened.
/// Metadata that is missing, not a regular file, or under the limit is not trusted: the bytes
/// are taken through [`take_bounded`], which stops after `max + 1` bytes. A file that grows
/// between the metadata check and the read, or a pathless drop, cannot pass the limit.
/// `max` itself is unchanged ([`FileDropZone::max_bytes`], default [`DEFAULT_MAX_BYTES`]).
#[cfg(not(target_arch = "wasm32"))]
fn read_one(file: &egui::DroppedFileHandle, max: u64) -> Result<PickedFile, (String, String)> {
    let name = file_name(file.path());
    let path = file.path().to_path_buf();
    if let Ok(meta) = std::fs::metadata(&path)
        && meta.is_file()
        && meta.len() > max
    {
        return Err(too_large(&name, meta.len(), max));
    }
    let (bytes, continued) = take_bounded(file, max).map_err(|err| (name.clone(), err))?;
    if continued || bytes.len() as u64 > max {
        let size = if continued {
            max.saturating_add(1)
        } else {
            bytes.len() as u64
        };
        return Err(too_large(&name, size, max));
    }
    Ok(PickedFile {
        name,
        path: (!path.as_os_str().is_empty()).then_some(path),
        bytes,
    })
}

/// Read at most `max + 1` bytes.
///
/// Returns the bytes kept and whether the stream had more. An openable path is read with
/// [`std::io::Read::take`], then one further byte is probed and discarded, so a file that grows
/// past the limit is cut off instead of read in full.
///
/// Pathless drops, and paths that cannot be opened, are a known limit: the platform's dropped-file
/// trait fully buffers them before this limit applies. [`egui::DroppedFile::bytes`] returns the
/// whole file. That buffer is then copied through [`read_limited`], so only `max + 1` bytes are
/// kept, but the platform has already held the rest.
#[cfg(not(target_arch = "wasm32"))]
fn take_bounded(file: &egui::DroppedFileHandle, max: u64) -> Result<(Vec<u8>, bool), String> {
    let limit = max.saturating_add(1);
    let path = file.path();
    if !path.as_os_str().is_empty()
        && let Ok(mut reader) = std::fs::File::open(path)
    {
        return read_limited(&mut reader, limit);
    }
    let bytes = file.bytes()?;
    read_limited(&mut std::io::Cursor::new(bytes), limit)
}

/// Read `limit` bytes through [`std::io::Read::take`], then probe one more byte.
#[cfg(not(target_arch = "wasm32"))]
fn read_limited(reader: &mut impl std::io::Read, limit: u64) -> Result<(Vec<u8>, bool), String> {
    let mut bounded = std::io::Read::take(&mut *reader, limit);
    let mut bytes = Vec::new();
    std::io::Read::read_to_end(&mut bounded, &mut bytes).map_err(|err| err.to_string())?;
    let mut extra = [0u8; 1];
    let continued = std::io::Read::read(reader, &mut extra).map_err(|err| err.to_string())? > 0;
    Ok((bytes, continued))
}

/// Read a drop's files one after another on one worker thread.
#[cfg(not(target_arch = "wasm32"))]
fn read_all(ctx: egui::Context, files: Vec<egui::DroppedFileHandle>, max: u64, results: Results) {
    let names: Vec<String> = files.iter().map(|f| file_name(f.path())).collect();
    let failure_results = results.clone();
    let spawned = std::thread::Builder::new()
        .name("fluent2-file-drop".into())
        .spawn(move || {
            for file in &files {
                let result = read_one(file, max);
                if let Ok(mut done) = results.lock() {
                    done.push(result);
                }
                ctx.request_repaint();
            }
        });
    if let Err(err) = spawned
        && let Ok(mut done) = failure_results.lock()
    {
        for name in names {
            done.push(Err((name, format!("could not start a reader: {err}"))));
        }
    }
}

/// Read a drop's files one after another in one browser task, checking each `File`'s size
/// before reading it.
#[cfg(target_arch = "wasm32")]
fn read_all(ctx: egui::Context, files: Vec<egui::DroppedFileHandle>, max: u64, results: Results) {
    wasm_bindgen_futures::spawn_local(async move {
        for file in &files {
            let name = file_name(file.path());
            let size = file.web_file().map(|f| f.size() as u64);
            let result = match size {
                Some(size) if size > max => Err(too_large(&name, size, max)),
                _ => match file.bytes_async().await {
                    Ok(bytes) if bytes.len() as u64 > max => Err(too_large(&name, bytes.len() as u64, max)),
                    Ok(bytes) => Ok(PickedFile {
                        name: name.clone(),
                        path: None,
                        bytes,
                    }),
                    Err(err) => Err((name, err)),
                },
            };
            if let Ok(mut done) = results.lock() {
                done.push(result);
            }
            ctx.request_repaint();
        }
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

    /// A stream longer than the limit is cut off at `max + 1` bytes.
    #[cfg(not(target_arch = "wasm32"))]
    #[test]
    fn an_oversize_stream_is_cut_off_at_the_limit() {
        /// Counts bytes pulled. `read_to_end` on an unbounded reader would drain `available`.
        struct Endless {
            pulled: usize,
            available: usize,
        }

        impl std::io::Read for Endless {
            fn read(&mut self, buf: &mut [u8]) -> std::io::Result<usize> {
                if self.available == 0 || buf.is_empty() {
                    return Ok(0);
                }
                let n = buf.len().min(self.available).min(16);
                buf[..n].fill(9);
                self.pulled += n;
                self.available -= n;
                Ok(n)
            }
        }

        let max = 32u64;
        assert_eq!(
            DEFAULT_MAX_BYTES,
            50 * 1024 * 1024,
            "the default limit is unchanged"
        );
        let mut stream = Endless {
            pulled: 0,
            available: 10_000,
        };
        let Ok((bytes, continued)) = read_limited(&mut stream, max + 1) else {
            panic!("bounded read failed");
        };
        assert!(continued, "the stream had more than the limit");
        assert_eq!(bytes.len() as u64, max + 1);
        assert_eq!(
            stream.pulled as u64,
            max + 2,
            "one byte past the cutoff was probed and not kept; the rest was never read"
        );
    }

    /// A stream that outgrows a metadata size still under the limit is cut off and refused.
    #[cfg(not(target_arch = "wasm32"))]
    #[test]
    fn metadata_under_the_limit_does_not_bypass_the_stream_limit() {
        use std::io::Write;
        let dir = std::env::temp_dir().join(format!("fluent2-drop-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        if std::fs::create_dir_all(&dir).is_err() {
            return;
        }
        let path = dir.join("note.bin");
        if std::fs::File::create(&path)
            .and_then(|mut file| file.write_all(b"ok"))
            .is_err()
        {
            return;
        }
        #[derive(Debug)]
        struct OnDisk {
            path: std::path::PathBuf,
        }
        impl egui::DroppedFile for OnDisk {
            fn path(&self) -> &std::path::Path {
                &self.path
            }
            fn bytes(&self) -> Result<Vec<u8>, String> {
                panic!("an openable path must not fall back to bytes()");
            }
        }
        let handle: egui::DroppedFileHandle = std::sync::Arc::new(OnDisk { path: path.clone() });
        let Ok(meta) = std::fs::metadata(&path) else {
            return;
        };
        let claimed = meta.len();
        assert!(claimed <= 8, "metadata says the file is under the limit");
        let Ok(picked) = read_one(&handle, 8) else {
            panic!("accepted while the stream is under the limit");
        };
        assert_eq!(picked.bytes, b"ok");

        if std::fs::OpenOptions::new()
            .append(true)
            .open(&path)
            .and_then(|mut file| file.write_all(&[9u8; 64]))
            .is_err()
        {
            return;
        }
        // The earlier metadata still says "under the limit"; the stream does not.
        assert!(claimed <= 8);
        let Ok((bytes, continued)) = take_bounded(&handle, 8) else {
            panic!("bounded");
        };
        assert!(continued, "the stream is over the limit the metadata claimed");
        assert_eq!(bytes.len(), 9);
        let Err(err) = read_one(&handle, 8) else {
            panic!("read_one refuses the grown stream");
        };
        assert!(err.1.contains("larger than"));
        let _ = std::fs::remove_dir_all(&dir);
    }

    /// A pathless drop is kept only up to `max + 1`, through the same bounded reader.
    #[cfg(not(target_arch = "wasm32"))]
    #[test]
    fn pathless_drop_is_cut_off_by_the_bounded_reader() {
        #[derive(Debug)]
        struct Pathless {
            len: usize,
        }
        impl egui::DroppedFile for Pathless {
            fn path(&self) -> &std::path::Path {
                std::path::Path::new("")
            }
            fn bytes(&self) -> Result<Vec<u8>, String> {
                Ok(vec![4u8; self.len])
            }
        }
        let handle: egui::DroppedFileHandle = std::sync::Arc::new(Pathless { len: 64 });
        let Ok((bytes, continued)) = take_bounded(&handle, 8) else {
            panic!("pathless drop is readable");
        };
        assert!(continued);
        assert_eq!(bytes.len(), 9);
        let Err(err) = read_one(&handle, 8) else {
            panic!("read_one refuses a pathless drop over the limit");
        };
        assert!(err.1.contains("larger than"));
        let small: egui::DroppedFileHandle = std::sync::Arc::new(Pathless { len: 3 });
        let picked = match read_one(&small, 8) {
            Ok(picked) => picked,
            Err(err) => panic!("small pathless drop is kept: {}", err.1),
        };
        assert_eq!(picked.bytes, vec![4, 4, 4]);
    }
}
