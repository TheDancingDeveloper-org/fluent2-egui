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
pub(crate) fn decode_base64(input: &str) -> Option<Vec<u8>> {
    let mut out = Vec::with_capacity(input.len() * 3 / 4);
    let mut buffer: u32 = 0;
    let mut bits = 0;
    for byte in input.bytes() {
        let value = match byte {
            b'A'..=b'Z' => byte - b'A',
            b'a'..=b'z' => byte - b'a' + 26,
            b'0'..=b'9' => byte - b'0' + 52,
            b'+' | b'-' => 62,
            b'/' | b'_' => 63,
            b'=' => break,
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
    fn data_uris() {
        let (mime, bytes) = parse("data:image/png;base64,iVBORw0KGgo=").unwrap_or_default();
        assert_eq!(mime, "image/png");
        assert_eq!(bytes[..4], [0x89, b'P', b'N', b'G']);
        assert!(parse("data:text/html;base64,PGI+").is_none());
        assert!(parse("data:image/png,raw").is_none());
        assert!(parse("https://example.com/a.png").is_none());
    }
}
