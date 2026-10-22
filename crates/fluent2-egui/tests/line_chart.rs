//! LineChart: every theme with degenerate input, the AccessKit summary, the range chips and the
//! hover read-out.
#![cfg(feature = "chart")]

use egui::Vec2;
use egui::accesskit::Role;
use egui_kittest::Harness;
use egui_kittest::kittest::{NodeT, Queryable};
use fluent2_egui::controls::{ChartHover, ChartSeries, LineChart, TimeRange};
use fluent2_egui::{Theme, install};

fn themes() -> [Theme; 3] {
    [
        Theme::light(fluent2_tokens::FLUENT),
        Theme::dark(fluent2_tokens::FLUENT),
        Theme::high_contrast(),
    ]
}

/// Download and upload at 1 Hz for `secs` seconds, with a gap in the upload.
fn speed(secs: u32) -> Vec<ChartSeries> {
    let mut down = ChartSeries::new("Download", 21_600).fill(true);
    let mut up = ChartSeries::new("Upload", 21_600);
    for t in 0..secs {
        down.push(f64::from(t), (t % 50) as f32 * 10.0);
        up.push(
            f64::from(t),
            if t % 100 == 7 { f32::NAN } else { (t % 13) as f32 },
        );
    }
    vec![down, up]
}

#[test]
fn renders_in_every_theme_with_degenerate_input() {
    let mut nan = ChartSeries::new("NaN", 4);
    nan.push(0.0, f32::NAN);
    let mut flat = ChartSeries::new("Flat", 4);
    flat.push(0.0, 3.0);
    flat.push(1.0, 3.0);
    let cases: Vec<Vec<ChartSeries>> = vec![
        Vec::new(),
        vec![ChartSeries::new("Empty", 4)],
        vec![nan, flat],
        speed(21_600),
    ];
    for theme in themes() {
        for series in &cases {
            let theme = theme.clone();
            let series = series.clone();
            let mut h = Harness::builder()
                .with_size(Vec2::new(600.0, 320.0))
                .build_ui_state(
                    move |ui, range: &mut TimeRange| {
                        install(ui.ctx(), &theme);
                        LineChart::new("c", &series).label("Speed").show(ui, range);
                        LineChart::new("tiny", &series)
                            .height(f32::NAN)
                            .now(f64::INFINITY)
                            .legend(false)
                            .chips(false)
                            .show(ui, range);
                    },
                    TimeRange::SixHours,
                );
            h.run();
            assert_eq!(h.query_all_by_role(Role::Image).count(), 2);
        }
    }
}

#[test]
fn summary_reaches_accesskit() {
    let series = speed(300);
    let mut h = Harness::builder()
        .with_size(Vec2::new(600.0, 320.0))
        .build_ui_state(
            move |ui, range: &mut TimeRange| {
                LineChart::new("c", &series)
                    .label("Speed")
                    .unit(" B/s")
                    .show(ui, range);
            },
            TimeRange::FiveMinutes,
        );
    h.run();
    let label = h
        .query_by_role(Role::Image)
        .and_then(|n| n.accesskit_node().label())
        .unwrap_or_default();
    assert!(
        label.starts_with("Speed, last 5 minutes: Download: 300 values, min 0 B/s, max 490 B/s"),
        "{label}"
    );
    assert!(label.contains("; Upload: 297 values"), "{label}");
}

#[test]
fn chip_changes_the_range() {
    let series = speed(10);
    let mut h = Harness::builder()
        .with_size(Vec2::new(600.0, 320.0))
        .build_ui_state(
            move |ui, state: &mut (TimeRange, bool)| {
                let r = LineChart::new("c", &series).show(ui, &mut state.0);
                state.1 |= r.range_changed;
            },
            (TimeRange::FiveMinutes, false),
        );
    h.run();
    h.get_by_label("6 h").click();
    h.run();
    assert_eq!(*h.state(), (TimeRange::SixHours, true));
}

#[test]
fn hover_reads_out_every_series() {
    let series = speed(300);
    let mut h = Harness::builder()
        .with_size(Vec2::new(600.0, 320.0))
        .build_ui_state(
            move |ui, hover: &mut Option<ChartHover>| {
                let mut range = TimeRange::FiveMinutes;
                *hover = LineChart::new("c", &series).show(ui, &mut range).hover;
            },
            None,
        );
    h.run();
    h.get_by_role(Role::Image).hover();
    h.run();
    let Some(hover) = h.state().clone() else {
        panic!("no hover read-out");
    };
    assert_eq!(hover.values.len(), 2);
    assert!(hover.values.iter().all(Option::is_some), "{hover:?}");
    assert!((0.0..=300.0).contains(&hover.time));
}
