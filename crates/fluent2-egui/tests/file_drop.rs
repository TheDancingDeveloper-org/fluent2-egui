//! FileDropZone (WI-1944): hover overlay, off-thread reads, failures and targeting.
#![cfg(feature = "file-dialog")]

use std::path::{Path, PathBuf};
use std::sync::Arc;

use egui_kittest::Harness;
use fluent2_egui::platform::file_drop::{FileDropResponse, FileDropZone};
use fluent2_egui::{Theme, install};

/// A dropped file whose bytes are fixed, or whose read fails.
#[derive(Debug)]
struct FakeFile {
    path: PathBuf,
    bytes: Result<Vec<u8>, String>,
}

impl egui::DroppedFile for FakeFile {
    fn path(&self) -> &Path {
        &self.path
    }

    fn bytes(&self) -> Result<Vec<u8>, String> {
        self.bytes.clone()
    }
}

fn fake(name: &str, bytes: Result<&[u8], &str>) -> egui::DroppedFileHandle {
    Arc::new(FakeFile {
        path: PathBuf::from(format!("/home/user/{name}")),
        bytes: bytes.map(<[u8]>::to_vec).map_err(str::to_owned),
    })
}

#[derive(Default)]
struct Seen {
    hovering: bool,
    dropped: Vec<String>,
    received: Vec<(String, Vec<u8>, Option<PathBuf>)>,
    failed: Vec<(String, String)>,
    other_dropped: usize,
}

fn record(seen: &mut Seen, out: FileDropResponse) {
    seen.hovering |= out.hovering;
    seen.dropped.extend(out.dropped);
    seen.received
        .extend(out.received.into_iter().map(|f| (f.name, f.bytes, f.path)));
    seen.failed.extend(out.failed);
}

fn harness() -> Harness<'static, Seen> {
    Harness::builder()
        .with_size(egui::vec2(400.0, 400.0))
        .build_ui_state(
            |ui, seen: &mut Seen| {
                install(ui.ctx(), &Theme::default());
                let out = FileDropZone::new("compose")
                    .label("Drop files to attach")
                    .show(ui, |ui| {
                        ui.set_min_size(egui::vec2(380.0, 150.0));
                        ui.label("Message body");
                    });
                record(seen, out);
                ui.add_space(20.0);
                // A second zone the pointer is not over.
                let other = FileDropZone::new("elsewhere").show(ui, |ui| {
                    ui.set_min_size(egui::vec2(380.0, 100.0));
                    ui.label("Other");
                });
                seen.other_dropped += other.dropped.len();
            },
            Seen::default(),
        )
}

fn wait_for_reads(harness: &mut Harness<'static, Seen>, count: usize) {
    for _ in 0..500 {
        harness.run_steps(1);
        let seen = harness.state();
        if seen.received.len() + seen.failed.len() >= count {
            return;
        }
        std::thread::sleep(std::time::Duration::from_millis(2));
    }
}

#[test]
fn hover_then_drop_reads_files_off_the_ui_thread() {
    let mut harness = harness();
    harness.run_steps(2);
    // The pointer is over the first zone.
    harness.event(egui::Event::PointerMoved(egui::pos2(100.0, 60.0)));
    harness.input_mut().hovered_files.push(egui::HoveredFile {
        path: Some(PathBuf::from("/home/user/report.pdf")),
        mime: String::new(),
    });
    harness.run_steps(1);
    assert!(
        harness.state().hovering,
        "the zone under the pointer shows the overlay"
    );

    harness.input_mut().hovered_files.clear();
    harness
        .input_mut()
        .dropped_files
        .push(fake("report.pdf", Ok(b"%PDF")));
    harness
        .input_mut()
        .dropped_files
        .push(fake("broken.bin", Err("permission denied")));
    harness.run_steps(1);
    assert_eq!(harness.state().dropped, ["report.pdf", "broken.bin"]);
    assert_eq!(
        harness.state().other_dropped,
        0,
        "only the zone under the pointer takes the drop"
    );

    wait_for_reads(&mut harness, 2);
    let seen = harness.state();
    assert_eq!(seen.received.len(), 1);
    let (name, bytes, path) = &seen.received[0];
    assert_eq!(name, "report.pdf");
    assert_eq!(bytes, b"%PDF");
    assert_eq!(path.as_deref(), Some(Path::new("/home/user/report.pdf")));
    assert_eq!(
        seen.failed,
        [("broken.bin".to_owned(), "permission denied".to_owned())]
    );
}

#[test]
fn a_window_wide_zone_takes_drops_anywhere() {
    let mut harness = Harness::builder()
        .with_size(egui::vec2(400.0, 400.0))
        .build_ui_state(
            |ui, seen: &mut Seen| {
                install(ui.ctx(), &Theme::default());
                let out = FileDropZone::new("window").window_wide(true).show(ui, |ui| {
                    ui.label("small");
                });
                record(seen, out);
            },
            Seen::default(),
        );
    harness.run_steps(2);
    harness.event(egui::Event::PointerMoved(egui::pos2(390.0, 390.0)));
    harness.run_steps(1);
    harness
        .input_mut()
        .dropped_files
        .push(fake("notes.txt", Ok(b"hi")));
    harness.run_steps(1);
    wait_for_reads(&mut harness, 1);
    assert_eq!(harness.state().received.len(), 1);
}

#[test]
fn overlay_renders_in_every_theme() {
    for theme in [
        Theme::default(),
        Theme::dark(fluent2_egui::tokens::FLUENT),
        Theme::high_contrast(),
    ] {
        let mut harness = Harness::new_ui(move |ui| {
            install(ui.ctx(), &theme);
            let _ = FileDropZone::new("z").window_wide(true).show(ui, |ui| {
                ui.set_min_size(egui::vec2(200.0, 120.0));
            });
        });
        harness
            .input_mut()
            .hovered_files
            .push(egui::HoveredFile::default());
        harness.run_steps(2);
    }
}

/// A dropped file that counts how often its bytes are read.
#[derive(Debug)]
struct CountingFile {
    path: PathBuf,
    size: usize,
    reads: Arc<std::sync::atomic::AtomicUsize>,
}

impl egui::DroppedFile for CountingFile {
    fn path(&self) -> &Path {
        &self.path
    }

    fn bytes(&self) -> Result<Vec<u8>, String> {
        self.reads.fetch_add(1, std::sync::atomic::Ordering::SeqCst);
        Ok(vec![0; self.size])
    }
}

fn capped_harness(max: u64) -> Harness<'static, Seen> {
    Harness::builder()
        .with_size(egui::vec2(400.0, 400.0))
        .build_ui_state(
            move |ui, seen: &mut Seen| {
                install(ui.ctx(), &Theme::default());
                let out = FileDropZone::new("capped")
                    .window_wide(true)
                    .max_bytes(max)
                    .show(ui, |ui| {
                        ui.label("zone");
                    });
                record(seen, out);
            },
            Seen::default(),
        )
}

#[test]
fn files_over_the_limit_are_refused_before_reading() {
    // A real file on disk over the limit: its size comes from metadata and it is never read.
    let dir = std::env::temp_dir().join(format!("fluent2-file-drop-{}", std::process::id()));
    let _ = std::fs::create_dir_all(&dir);
    let big = dir.join("big.bin");
    let _ = std::fs::write(&big, vec![7u8; 4096]);
    let reads = Arc::new(std::sync::atomic::AtomicUsize::new(0));

    let mut harness = capped_harness(1024);
    harness.run_steps(2);
    harness.input_mut().dropped_files.push(Arc::new(CountingFile {
        path: big.clone(),
        size: 4096,
        reads: reads.clone(),
    }));
    // No metadata (no file at the path): checked on the bytes read instead.
    harness.input_mut().dropped_files.push(Arc::new(CountingFile {
        path: PathBuf::from("/nonexistent/also-big.bin"),
        size: 2048,
        reads: Arc::new(std::sync::atomic::AtomicUsize::new(0)),
    }));
    harness
        .input_mut()
        .dropped_files
        .push(fake("small.txt", Ok(b"ok")));
    harness.run_steps(1);
    wait_for_reads(&mut harness, 3);
    let _ = std::fs::remove_dir_all(&dir);

    let seen = harness.state();
    assert_eq!(
        reads.load(std::sync::atomic::Ordering::SeqCst),
        0,
        "the big file was never read"
    );
    let failed: Vec<&str> = seen.failed.iter().map(|(name, _)| name.as_str()).collect();
    assert_eq!(failed, ["big.bin", "also-big.bin"]);
    assert_eq!(seen.failed[0].1, "4.0 KiB is larger than the 1.0 KiB limit");
    assert_eq!(seen.received.len(), 1);
    assert_eq!(seen.received[0].0, "small.txt");
}

#[test]
fn a_drop_is_read_by_one_zone_only() {
    // Two zones and no pointer seen yet: both would take the drop, but only the first does.
    let first = Arc::new(std::sync::atomic::AtomicUsize::new(0));
    let second = Arc::new(std::sync::atomic::AtomicUsize::new(0));
    let (a, b) = (first.clone(), second.clone());
    let mut harness = Harness::builder()
        .with_size(egui::vec2(400.0, 400.0))
        .build_ui(move |ui| {
            install(ui.ctx(), &Theme::default());
            let out = FileDropZone::new("one").show(ui, |ui| {
                ui.set_min_size(egui::vec2(380.0, 100.0));
            });
            a.fetch_add(out.dropped.len(), std::sync::atomic::Ordering::SeqCst);
            let out = FileDropZone::new("two").show(ui, |ui| {
                ui.set_min_size(egui::vec2(380.0, 100.0));
            });
            b.fetch_add(out.dropped.len(), std::sync::atomic::Ordering::SeqCst);
        });
    harness.run_steps(2);
    harness.input_mut().dropped_files.push(fake("a.txt", Ok(b"a")));
    harness.run_steps(1);
    assert_eq!(first.load(std::sync::atomic::Ordering::SeqCst), 1);
    assert_eq!(second.load(std::sync::atomic::Ordering::SeqCst), 0);
}

#[test]
fn the_zone_under_the_pointer_beats_a_later_window_wide_zone() {
    let specific = Arc::new(std::sync::atomic::AtomicUsize::new(0));
    let wide = Arc::new(std::sync::atomic::AtomicUsize::new(0));
    let (a, b) = (specific.clone(), wide.clone());
    let mut harness = Harness::builder()
        .with_size(egui::vec2(400.0, 400.0))
        .build_ui(move |ui| {
            install(ui.ctx(), &Theme::default());
            let out = FileDropZone::new("attachments").show(ui, |ui| {
                ui.set_min_size(egui::vec2(380.0, 100.0));
            });
            a.fetch_add(out.dropped.len(), std::sync::atomic::Ordering::SeqCst);
            let out = FileDropZone::new("window").window_wide(true).show(ui, |ui| {
                ui.set_min_size(egui::vec2(380.0, 100.0));
            });
            b.fetch_add(out.dropped.len(), std::sync::atomic::Ordering::SeqCst);
        });
    harness.run_steps(2);
    harness.event(egui::Event::PointerMoved(egui::pos2(100.0, 40.0)));
    harness.run_steps(1);
    harness.input_mut().dropped_files.push(fake("a.txt", Ok(b"a")));
    harness.run_steps(1);
    assert_eq!(specific.load(std::sync::atomic::Ordering::SeqCst), 1);
    assert_eq!(wide.load(std::sync::atomic::Ordering::SeqCst), 0);
}
