//! Text fields paint their frame fill beneath their text (WI-1913).
//!
//! The fill used to be added after the TextEdit on the same layer, so it covered the typed
//! text and the placeholder. These tests read one frame's shapes in paint order.

use egui::Vec2;
use fluent2_egui::controls::{Combobox, Input, SpinButton, Textarea};
use fluent2_egui::theme::{Theme, install};

/// Paint `build` and return (index of the field's frame fill, index of the first text).
fn frame_and_text(build: impl Fn(&mut egui::Ui)) -> (Option<usize>, Option<usize>) {
    let ctx = egui::Context::default();
    install(&ctx, &Theme::default());
    let mut found = (None, None);
    for _ in 0..2 {
        let raw = egui::RawInput {
            screen_rect: Some(egui::Rect::from_min_size(
                egui::Pos2::ZERO,
                Vec2::new(400.0, 300.0),
            )),
            ..Default::default()
        };
        let mut output = ctx.run_ui(raw, |ui| build(ui));
        let shapes = &output.shapes;
        // The frame is the widest filled rectangle; nothing else in a lone field is as wide.
        let frame = shapes
            .iter()
            .enumerate()
            .filter_map(|(index, clipped)| match &clipped.shape {
                egui::Shape::Rect(rect) if rect.fill.a() > 0 => Some((index, rect.rect.width())),
                _ => None,
            })
            .max_by(|a, b| a.1.total_cmp(&b.1))
            .map(|(index, _)| index);
        let text = shapes
            .iter()
            .position(|clipped| matches!(&clipped.shape, egui::Shape::Text(text) if !text.galley.is_empty()));
        found = (frame, text);
        // egui panics if texture deltas are dropped unhandled; this test does not upload them.
        output.textures_delta.clear();
    }
    found
}

fn assert_text_above_frame(name: &str, build: impl Fn(&mut egui::Ui)) {
    let (frame, text) = frame_and_text(build);
    let (Some(frame), Some(text)) = (frame, text) else {
        panic!("{name}: frame {frame:?} or text {text:?} not painted");
    };
    assert!(
        frame < text,
        "{name}: frame fill (#{frame}) painted over the text (#{text})"
    );
}

#[test]
fn input_text_and_placeholder_paint_above_the_frame() {
    assert_text_above_frame("Input", |ui| {
        let mut text = String::from("Ada Lovelace");
        ui.add(Input::new(&mut text).label("Name"));
    });
    assert_text_above_frame("Input placeholder", |ui| {
        let mut text = String::new();
        ui.add(Input::new(&mut text).label("Name").placeholder("Full name"));
    });
}

#[test]
fn textarea_text_paints_above_the_frame() {
    assert_text_above_frame("Textarea", |ui| {
        let mut text = String::from("First line\nSecond line");
        ui.add(Textarea::new(&mut text).label("Notes").rows(3));
    });
}

#[test]
fn spin_button_value_paints_above_the_frame() {
    assert_text_above_frame("SpinButton", |ui| {
        let mut value = 42.0;
        ui.add(SpinButton::new(&mut value));
    });
}

#[test]
fn combobox_text_paints_above_the_frame() {
    assert_text_above_frame("Combobox", |ui| {
        let mut text = String::from("Calibri");
        ui.add(Combobox::new("font", &mut text, &["Calibri", "Arial"]));
    });
}
