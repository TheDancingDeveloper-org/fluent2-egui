//! Inline `data:image/…;base64,…` images: decoded once and handed to egui's image loaders as
//! `bytes://` URIs, so the app's installed loaders (for example `egui_extras`) decode them.

use egui::{Context, Id};

/// Decoded images larger than this are not shown.
pub const MAX_BYTES: usize = 8 * 1024 * 1024;

/// Register the image in `src` with egui and return its `bytes://` URI. `None` for anything that
/// is not a base64 `data:image/` URI, or that fails to decode.
pub(crate) fn register(ctx: &Context, src: &str) -> Option<String> {
    let src = src.trim();
    let key = Id::new(("fluent2_html_data_uri", src));
    if let Some(uri) = ctx.data(|data| data.get_temp::<Option<String>>(key)) {
        return uri;
    }
    let uri = parse(src).map(|(mime, bytes)| {
        let ext = extension(&mime);
        let uri = format!("bytes://fluent2-html/{:016x}.{ext}", key.value());
        ctx.include_bytes(uri.clone(), bytes);
        uri
    });
    ctx.data_mut(|data| data.insert_temp(key, uri.clone()));
    uri
}

/// The MIME type and bytes of a base64 `data:image/…` URI.
pub(crate) fn parse(src: &str) -> Option<(String, Vec<u8>)> {
    let rest = src
        .get(..5)
        .filter(|p| p.eq_ignore_ascii_case("data:"))
        .and_then(|_| src.get(5..))?;
    let (meta, payload) = rest.split_once(',')?;
    let mut parts = meta.split(';');
    let mime = parts.next()?.trim().to_ascii_lowercase();
    if !mime.starts_with("image/") || !parts.any(|p| p.trim().eq_ignore_ascii_case("base64")) {
        return None;
    }
    let bytes = decode_base64(payload)?;
    (!bytes.is_empty() && bytes.len() <= MAX_BYTES).then_some((mime, bytes))
}

fn extension(mime: &str) -> &'static str {
    match mime {
        "image/png" => "png",
        "image/jpeg" | "image/jpg" | "image/pjpeg" => "jpg",
        "image/gif" => "gif",
        "image/webp" => "webp",
        "image/bmp" => "bmp",
        "image/svg+xml" => "svg",
        _ => "bin",
    }
}

/// Standard or URL-safe base64, ignoring whitespace (mail wraps long lines) and padding.
///
/// The encoded length is checked before any allocation. Four encoded characters produce at most
/// three bytes, so an input whose decoded size must exceed [`MAX_BYTES`] is rejected with no
/// buffer. Whitespace is skipped, so a whitespace-heavy payload under the decoded limit still
/// decodes. The initial buffer is capped at [`MAX_BYTES`]; that cap is the decoded-byte limit,
/// not an image-pixel limit.
pub(crate) fn decode_base64(input: &str) -> Option<Vec<u8>> {
    // Four significant characters decode to three bytes. Reject before allocating when even a
    // payload with no whitespace would exceed the decoded-byte limit.
    let max_significant = MAX_BYTES.checked_mul(4)?.checked_div(3)?.saturating_add(2);
    if input.len() > max_significant {
        return None;
    }
    let capacity = input.len().checked_mul(3)?.checked_div(4)?.min(MAX_BYTES);
    let mut out = Vec::with_capacity(capacity);
    let mut buffer: u32 = 0;
    let mut bits = 0;
    let mut saw_padding = false;
    for byte in input.bytes() {
        let value = match byte {
            b'A'..=b'Z' => byte - b'A',
            b'a'..=b'z' => byte - b'a' + 26,
            b'0'..=b'9' => byte - b'0' + 52,
            b'+' | b'-' => 62,
            b'/' | b'_' => 63,
            b'=' => {
                saw_padding = true;
                break;
            }
            b' ' | b'\n' | b'\r' | b'\t' => continue,
            _ => return None,
        };
        buffer = (buffer << 6) | u32::from(value);
        bits += 6;
        if bits >= 8 {
            bits -= 8;
            out.push(((buffer >> bits) & 0xff) as u8);
            if out.len() > MAX_BYTES {
                return None;
            }
        }
    }
    if saw_padding {
        let mut after = input.split('=').nth(1).unwrap_or("").bytes();
        if after.any(|byte| !matches!(byte, b'=' | b' ' | b'\n' | b'\r' | b'\t')) {
            return None;
        }
    }
    // One significant character cannot complete a byte: truncated (`A`) or bad padding (`A===`).
    // '=' alone never enters the sextet loop, so it stays an empty payload.
    if bits == 6 {
        return None;
    }
    Some(out)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn base64() {
        assert_eq!(decode_base64("aGVsbG8="), Some(b"hello".to_vec()));
        assert_eq!(decode_base64("aGVs\r\nbG8"), Some(b"hello".to_vec()));
        assert_eq!(decode_base64("a$b"), None);
    }

    #[test]
    fn oversized_encoded_input_is_rejected_without_a_decoded_buffer() {
        // Longer than any payload that can decode to MAX_BYTES, including padding slack.
        let input = "A".repeat(MAX_BYTES.saturating_mul(4) / 3 + 3);
        assert!(decode_base64(&input).is_none());
    }

    #[test]
    fn whitespace_heavy_input_under_the_limit_decodes() {
        let mut input = String::new();
        for (index, byte) in b"aGVsbG8=".iter().enumerate() {
            if index > 0 {
                input.push_str(" \n\r\t");
            }
            input.push(*byte as char);
        }
        assert!(input.len() > 8);
        assert_eq!(decode_base64(&input).as_deref(), Some(b"hello".as_slice()));
    }

    #[test]
    fn malformed_padding_outcomes() {
        // Extra '=' stops decoding and does not change a complete payload.
        assert_eq!(decode_base64("aGVsbG8===").as_deref(), Some(b"hello".as_slice()));
        // Non-alphabet after padding, or a single significant character, is an error.
        assert!(decode_base64("aGVsbG8=!!!!").is_none());
        assert_eq!(decode_base64("=").as_deref(), Some(b"".as_slice()));
        assert!(decode_base64("A").is_none());
        assert!(decode_base64("A===").is_none());
    }

    #[test]
    fn decoded_length_at_and_one_byte_over_the_limit() {
        // 'A' decodes to 0x00. The largest multiple of 3 that fits in MAX_BYTES is the exact
        // boundary the quartet decoder can land on.
        let exact = MAX_BYTES / 3 * 3;
        let at_limit = "A".repeat(exact / 3 * 4);
        let decoded = decode_base64(&at_limit);
        let len = decoded.as_ref().map(Vec::len);
        assert_eq!(len, Some(exact));
        assert!(len.is_some_and(|len| len <= MAX_BYTES && MAX_BYTES - len < 3));
        // One more quartet is three more bytes, which crosses MAX_BYTES.
        let one_over = format!("{at_limit}AAAA");
        assert!(decode_base64(&one_over).is_none());
    }

    #[test]
    fn data_uris() {
        let (mime, bytes) = parse("data:image/png;base64,iVBORw0KGgo=").unwrap_or_default();
        assert_eq!(mime, "image/png");
        assert_eq!(bytes[..4], [0x89, b'P', b'N', b'G']);
        assert!(parse("data:text/html;base64,PGI+").is_none());
        assert!(parse("data:image/png,raw").is_none());
        assert!(parse("https://example.com/a.png").is_none());
    }

    fn prop_config() -> proptest::test_runner::Config {
        proptest::test_runner::Config {
            cases: 64,
            failure_persistence: None,
            rng_seed: proptest::test_runner::RngSeed::Fixed(0xF2E6_0158),
            ..proptest::test_runner::Config::default()
        }
    }

    proptest::proptest! {
        #![proptest_config(prop_config())]

        /// Arbitrary input never panics, and a successful decode never exceeds [`MAX_BYTES`].
        #[test]
        fn decode_never_panics_and_stays_within_max(
            bytes in proptest::collection::vec(proptest::num::u8::ANY, 0..512),
        ) {
            let input = String::from_utf8_lossy(&bytes);
            if let Some(out) = decode_base64(&input) {
                proptest::prop_assert!(out.len() <= MAX_BYTES);
            }
        }
    }
}
