//! Linux appearance via the XDG desktop portal `Settings` interface.
//!
//! Calls `org.freedesktop.portal.Settings.ReadOne` on `org.freedesktop.portal.Desktop` for the
//! `org.freedesktop.appearance` namespace. The session connection uses a 300 ms method timeout.
//! Any error, including a missing portal or a timed-out call, becomes [`None`].
//!
//! Blocking is acceptable because apps call [`super::system_preferences`] once at startup or
//! when the user asks to follow the system. Do not call it every frame.

use std::time::Duration;

use zbus::blocking::Proxy;
use zbus::blocking::connection::Builder;
use zbus::zvariant::{OwnedValue, Value};

use super::{Rgba, SystemPreferences};

const PORTAL_DEST: &str = "org.freedesktop.portal.Desktop";
const PORTAL_PATH: &str = "/org/freedesktop/portal/desktop";
const SETTINGS_IFACE: &str = "org.freedesktop.portal.Settings";
const APPEARANCE: &str = "org.freedesktop.appearance";
const TIMEOUT: Duration = Duration::from_millis(300);

/// Read colour scheme, accent, contrast and reduced motion from the portal.
#[must_use]
pub fn read() -> SystemPreferences {
    let proxy = match portal() {
        Some(proxy) => proxy,
        None => return SystemPreferences::default(),
    };
    SystemPreferences {
        accent: read_one(&proxy, "accent-color").and_then(parse_accent),
        high_contrast: read_one(&proxy, "contrast").and_then(parse_flag),
        reduced_motion: read_one(&proxy, "reduced-motion").and_then(parse_flag),
        dark: read_one(&proxy, "color-scheme").and_then(parse_flag),
    }
}

fn portal() -> Option<Proxy<'static>> {
    let conn = Builder::session().ok()?.method_timeout(TIMEOUT).build().ok()?;
    Proxy::new(&conn, PORTAL_DEST, PORTAL_PATH, SETTINGS_IFACE).ok()
}

fn read_one(proxy: &Proxy<'_>, key: &str) -> Option<OwnedValue> {
    proxy.call("ReadOne", &(APPEARANCE, key)).ok()
}

/// Portal `color-scheme` and `contrast` / `reduced-motion` use `1` for the affirmative.
fn parse_flag(value: OwnedValue) -> Option<bool> {
    match unwrap_variant(value) {
        Value::U32(n) => Some(n == 1),
        Value::I32(n) => Some(n == 1),
        Value::U64(n) => Some(n == 1),
        Value::I64(n) => Some(n == 1),
        _ => None,
    }
}

/// `accent-color` is a struct of three doubles in `0..=1` (`(ddd)`).
fn parse_accent(value: OwnedValue) -> Option<Rgba> {
    let (r, g, b): (f64, f64, f64) = match unwrap_variant(value) {
        Value::Structure(fields) => fields.try_into().ok()?,
        _ => return None,
    };
    if ![r, g, b].into_iter().all(f64::is_finite) {
        return None;
    }
    Some(Rgba::new(r as f32, g as f32, b as f32))
}

/// `ReadOne` returns a variant. One extra layer is stripped when the portal double-wraps it.
fn unwrap_variant(value: OwnedValue) -> Value<'static> {
    match Value::from(value) {
        Value::Value(inner) => *inner,
        other => other,
    }
}
