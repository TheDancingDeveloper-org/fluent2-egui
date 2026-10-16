//! Catalogue smoke test: one known icon, and a size bound on the whole subset.

use fluent2_icons::{Icon, Size, Style, svg};

#[test]
fn add_regular_20_is_svg() {
    let bytes = match svg(Icon::Add, Size::S20, Style::Regular) {
        Some(bytes) => bytes,
        None => panic!("Add 20 regular is missing"),
    };
    let text = match std::str::from_utf8(bytes) {
        Ok(text) => text,
        Err(err) => panic!("Add 20 regular is not utf-8: {err}"),
    };
    assert!(text.starts_with("<svg"), "{text:.80}");
    assert!(text.contains("fill="));
}

#[test]
fn catalogue_stays_under_two_megabytes() {
    let mut total = 0usize;
    for icon in Icon::ALL {
        for size in [Size::S16, Size::S20, Size::S24] {
            for style in [Style::Regular, Style::Filled] {
                if let Some(bytes) = svg(*icon, size, style) {
                    total += bytes.len();
                }
            }
        }
    }
    assert!(total < 2 * 1024 * 1024, "catalogue is {total} bytes");
    assert!(total > 0);
}
