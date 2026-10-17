//! Targets that do not report appearance, and builds without `system-preferences`.

use super::SystemPreferences;

/// Every preference is unknown.
#[must_use]
pub fn read() -> SystemPreferences {
    SystemPreferences::default()
}
