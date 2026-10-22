//! URL fragment state: the pure encode and decode round trip, and the native no-op (issue #119).

use fluent2_egui::platform::hash_state::{HashState, decode, encode, value};

#[test]
fn round_trips_awkward_values() {
    let cases: &[&[(&str, &str)]] = &[
        &[("tab", "controls")],
        &[("tab", "controls"), ("q", "a & b = c # d")],
        &[("q", "100% sure"), ("sort", "name+size")],
        &[("q", "héllo wörld ✓"), ("empty", "")],
        &[("k&=#/?", "v&=#/?")],
    ];
    for &pairs in cases {
        let fragment = encode(pairs.iter().copied());
        let decoded = decode(&fragment);
        let expected: Vec<(String, String)> =
            pairs.iter().map(|&(k, v)| (k.to_owned(), v.to_owned())).collect();
        assert_eq!(decoded, expected, "fragment was {fragment:?}");
    }
}

#[test]
fn encodes_reserved_characters_and_keeps_unreserved_ones() {
    assert_eq!(encode([("q", "a b")]), "q=a%20b");
    assert_eq!(encode([("q", "a&b=c")]), "q=a%26b%3Dc");
    assert_eq!(encode([("a-b.c_d~", "x")]), "a-b.c_d~=x");
    assert_eq!(encode([("tab", "x"), ("q", "y")]), "tab=x&q=y");
    assert_eq!(encode(std::iter::empty::<(&str, &str)>()), "");
}

#[test]
fn decode_ignores_a_leading_hash_and_empty_pieces() {
    let pairs = decode("#&tab=controls&&q=a");
    assert_eq!(value(&pairs, "tab"), Some("controls"));
    assert_eq!(value(&pairs, "q"), Some("a"));
    assert_eq!(pairs.len(), 2);
    assert!(decode("").is_empty());
    assert!(decode("#").is_empty());
}

#[test]
fn decode_is_lenient_about_malformed_input() {
    // A key with no `=` has an empty value.
    assert_eq!(value(&decode("tab"), "tab"), Some(""));
    // A broken escape is kept as written, and never panics.
    assert_eq!(value(&decode("q=100%"), "q"), Some("100%"));
    assert_eq!(value(&decode("q=%zz%4"), "q"), Some("%zz%4"));
    // Bytes that are not UTF-8 are replaced rather than rejected.
    assert_eq!(value(&decode("q=%FF"), "q"), Some("\u{FFFD}"));
    // Only the first pair with a name counts.
    assert_eq!(value(&decode("q=1&q=2"), "q"), Some("1"));
    assert_eq!(value(&decode("q=1"), "missing"), None);
}

#[test]
fn hash_state_is_a_no_op_on_native() {
    let mut state = HashState::new();
    assert_eq!(state.read_external(), None);
    state.write("tab=controls");
    assert_eq!(state.read_external(), None);
}
