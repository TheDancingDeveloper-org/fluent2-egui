//! View state in the URL fragment, so a browser view can be bookmarked or shared (issue #119).
//!
//! The fragment holds `key=value` pairs joined by `&`, percent-encoded, for example
//! `#tab=controls&q=button`. [`encode`] and [`decode`] are pure and work on every target.
//! [`HashState`] reads and writes the browser's `location.hash` on wasm. On native targets
//! there is no URL, so every read returns [`None`] and every write does nothing.
//!
//! Writes use `history.replaceState`, so changing a filter does not add a history entry for
//! each keystroke. A navigation to a different fragment (back or forward, a pasted link, a
//! hand-edited address) shows up in [`HashState::read_external`], which the app calls once per
//! frame. That call is a cheap read of `location.hash`; it is how the app hears `popstate` and
//! `hashchange` without holding a closure.
//!
//! ```
//! use fluent2_egui::platform::hash_state::{decode, encode, value};
//!
//! let fragment = encode([("tab", "controls"), ("q", "a & b")]);
//! assert_eq!(fragment, "tab=controls&q=a%20%26%20b");
//!
//! let pairs = decode(&fragment);
//! assert_eq!(value(&pairs, "q"), Some("a & b"));
//! ```

/// Encode `(key, value)` pairs as a fragment without the leading `#`.
///
/// Keys and values are percent-encoded, except the unreserved characters `A-Z a-z 0-9 - . _ ~`,
/// so a value may hold `&`, `=`, `#` or spaces.
pub fn encode<'a>(pairs: impl IntoIterator<Item = (&'a str, &'a str)>) -> String {
    let mut out = String::new();
    for (key, value) in pairs {
        if !out.is_empty() {
            out.push('&');
        }
        push_escaped(&mut out, key);
        out.push('=');
        push_escaped(&mut out, value);
    }
    out
}

/// Decode a fragment into its pairs, in order. A leading `#` is ignored.
///
/// Never fails: an empty piece is skipped, a piece without `=` gives an empty value, and an
/// invalid `%` escape is kept as written. Invalid UTF-8 is replaced.
pub fn decode(fragment: &str) -> Vec<(String, String)> {
    fragment
        .strip_prefix('#')
        .unwrap_or(fragment)
        .split('&')
        .filter(|piece| !piece.is_empty())
        .map(|piece| {
            let (key, value) = piece.split_once('=').unwrap_or((piece, ""));
            (unescape(key), unescape(value))
        })
        .collect()
}

/// The value of the first pair named `key`, if there is one.
pub fn value<'a>(pairs: &'a [(String, String)], key: &str) -> Option<&'a str> {
    pairs
        .iter()
        .find(|(name, _)| name == key)
        .map(|(_, value)| value.as_str())
}

fn push_escaped(out: &mut String, text: &str) {
    for byte in text.bytes() {
        if byte.is_ascii_alphanumeric() || matches!(byte, b'-' | b'.' | b'_' | b'~') {
            out.push(char::from(byte));
        } else {
            out.push_str(&format!("%{byte:02X}"));
        }
    }
}

fn unescape(text: &str) -> String {
    let bytes = text.as_bytes();
    let mut out = Vec::with_capacity(bytes.len());
    let mut i = 0;
    while let Some(&byte) = bytes.get(i) {
        match escape_at(bytes, i) {
            Some(decoded) => {
                out.push(decoded);
                i += 3;
            }
            None => {
                out.push(byte);
                i += 1;
            }
        }
    }
    String::from_utf8_lossy(&out).into_owned()
}

/// The byte written as `%XX` at `index`, if there is one.
fn escape_at(bytes: &[u8], index: usize) -> Option<u8> {
    if bytes.get(index) != Some(&b'%') {
        return None;
    }
    let hex = bytes.get(index + 1..index + 3)?;
    if !hex.iter().all(u8::is_ascii_hexdigit) {
        return None;
    }
    std::str::from_utf8(hex)
        .ok()
        .and_then(|hex| u8::from_str_radix(hex, 16).ok())
}

/// Two-way sync between app state and the browser's URL fragment.
///
/// Call [`read_external`](Self::read_external) at the top of each frame and apply any fragment
/// it returns. Call [`write`](Self::write) with the fragment for the current state. Both are
/// no-ops on native targets.
///
/// ```no_run
/// use fluent2_egui::platform::hash_state::{self, HashState};
///
/// # fn demo(state: &mut HashState, tab: &mut usize) {
/// if let Some(fragment) = state.read_external() {
///     let pairs = hash_state::decode(&fragment);
///     if let Some(index) = hash_state::value(&pairs, "tab").and_then(|t| t.parse().ok()) {
///         *tab = index;
///     }
/// }
/// state.write(&hash_state::encode([("tab", tab.to_string().as_str())]));
/// # }
/// ```
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct HashState {
    /// The fragment this state last read or wrote, without `#`.
    known: Option<String>,
}

impl HashState {
    /// A sync that has not read or written yet.
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    /// The browser's fragment, if it differs from the one this sync last saw.
    ///
    /// The first call on wasm returns the fragment the page was opened with, even when it is
    /// empty. Returns [`None`] on native targets, and whenever the fragment is unchanged.
    pub fn read_external(&mut self) -> Option<String> {
        let now = location_hash()?;
        if self.known.as_deref() == Some(now.as_str()) {
            return None;
        }
        self.known = Some(now.clone());
        Some(now)
    }

    /// Set the browser's fragment to `fragment` (without `#`) with `replaceState`.
    ///
    /// Does nothing when `fragment` is what this sync last saw. Pass an empty string to drop
    /// the fragment. The path and query are kept.
    pub fn write(&mut self, fragment: &str) {
        if self.known.as_deref() == Some(fragment) {
            return;
        }
        self.known = Some(fragment.to_owned());
        replace_hash(fragment);
    }
}

#[cfg(target_arch = "wasm32")]
fn location_hash() -> Option<String> {
    let window = web_sys::window()?;
    let hash = window.location().hash().ok()?;
    Some(hash.strip_prefix('#').unwrap_or(&hash).to_owned())
}

#[cfg(target_arch = "wasm32")]
fn replace_hash(fragment: &str) {
    let Some(window) = web_sys::window() else {
        return;
    };
    let Ok(history) = window.history() else {
        return;
    };
    let location = window.location();
    let mut url = match (location.pathname(), location.search()) {
        (Ok(path), Ok(query)) => format!("{path}{query}"),
        _ => String::new(),
    };
    if !fragment.is_empty() {
        url.push('#');
        url.push_str(fragment);
    }
    let _ = history.replace_state_with_url(&wasm_bindgen::JsValue::NULL, "", Some(&url));
}

#[cfg(not(target_arch = "wasm32"))]
fn location_hash() -> Option<String> {
    None
}

#[cfg(not(target_arch = "wasm32"))]
fn replace_hash(_fragment: &str) {}
