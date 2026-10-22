//! LayoutStore: the app's layout state (splits, side docks, dock trees, grid columns, density)
//! saved under keys as one JSON document (feature `serde`, issue #118).
//!
//! Each component keeps its own serde state struct ([`SplitState`](crate::data::SplitState),
//! `SideDockState`, `DockTree`, [`DataGridState`](crate::data::data_grid::DataGridState) with its
//! widths and column layout, [`Density`](crate::theme::Density)). The store does not wrap them:
//! the app [`put`](LayoutStore::put)s each one under a key it chooses and
//! [`restore`](LayoutStore::restore)s it on start. The store turns into one string, so any
//! key-value storage holds it: eframe's `Storage` (a file natively, `localStorage` on wasm) through
//! [`load`](LayoutStore::load) and [`save`](LayoutStore::save), or the app's own settings file.
//!
//! Loading never fails. A missing or unreadable document gives an empty store, and an entry that
//! no longer matches its type (a state struct changed shape between versions) is skipped, so the
//! component keeps its default. Entries the running app does not read are kept and saved back.
//!
//! ```
//! use fluent2_egui::data::SplitState;
//! use fluent2_egui::platform::LayoutStore;
//!
//! let mut split = SplitState::default();
//! split.size = fluent2_egui::data::SplitSize::Ratio(0.3);
//! let mut store = LayoutStore::new();
//! store.put("sidebar_split", &split);
//! let saved = store.to_json();
//!
//! let mut restored = SplitState::default();
//! assert!(LayoutStore::from_json(&saved).restore("sidebar_split", &mut restored));
//! assert_eq!(restored, split);
//! ```

use std::collections::BTreeMap;

use serde::Serialize;
use serde::de::DeserializeOwned;
use serde_json::Value;

/// Keyed layout state, saved and loaded as one JSON object. See the [module docs](self).
#[derive(Clone, Debug, Default, PartialEq)]
pub struct LayoutStore {
    entries: BTreeMap<String, Value>,
    dirty: bool,
}

impl LayoutStore {
    /// The key [`load`](Self::load) and [`save`](Self::save) use in the app's storage.
    pub const STORAGE_KEY: &'static str = "fluent2_layout";

    /// An empty store.
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    /// Parse a document written by [`to_json`](Self::to_json). Anything that is not a JSON
    /// object gives an empty store, with a `log` warning.
    #[must_use]
    pub fn from_json(json: &str) -> Self {
        match serde_json::from_str::<BTreeMap<String, Value>>(json) {
            Ok(entries) => Self {
                entries,
                dirty: false,
            },
            Err(error) => {
                log::warn!("LayoutStore: ignoring unreadable layout: {error}");
                Self::default()
            }
        }
    }

    /// The whole store as one JSON object, keys sorted.
    #[must_use]
    pub fn to_json(&self) -> String {
        // A map of `Value`s with string keys always serialises.
        serde_json::to_string(&self.entries).unwrap_or_default()
    }

    /// Load from a string storage, such as `|key| storage.get_string(key)` on eframe's
    /// `CreationContext::storage`. Nothing stored gives an empty store.
    #[must_use]
    pub fn load(get: impl FnOnce(&str) -> Option<String>) -> Self {
        get(Self::STORAGE_KEY)
            .map(|json| Self::from_json(&json))
            .unwrap_or_default()
    }

    /// Write to a string storage, such as `|key, value| storage.set_string(key, value)` in
    /// `eframe::App::save`. Clears [`is_dirty`](Self::is_dirty).
    pub fn save(&mut self, set: impl FnOnce(&str, String)) {
        set(Self::STORAGE_KEY, self.to_json());
        self.dirty = false;
    }

    /// Store `state` under `key`, replacing what was there. Marks the store dirty when the
    /// stored value changed. A state that does not serialise (a map with non-string keys, say)
    /// is left out with a `log` warning.
    pub fn put<T: Serialize + ?Sized>(&mut self, key: &str, state: &T) {
        match serde_json::to_value(state) {
            Ok(value) => {
                if self.entries.get(key) != Some(&value) {
                    self.entries.insert(key.to_owned(), value);
                    self.dirty = true;
                }
            }
            Err(error) => log::warn!("LayoutStore: cannot save {key:?}: {error}"),
        }
    }

    /// The state stored under `key`, or `None` when there is none or it no longer reads as `T`.
    #[must_use]
    pub fn get<T: DeserializeOwned>(&self, key: &str) -> Option<T> {
        let value = self.entries.get(key)?;
        match T::deserialize(value) {
            Ok(state) => Some(state),
            Err(error) => {
                log::warn!("LayoutStore: ignoring stale {key:?}: {error}");
                None
            }
        }
    }

    /// Overwrite `target` with the state under `key`. Returns whether it did; on `false`,
    /// `target` keeps its value.
    pub fn restore<T: DeserializeOwned>(&self, key: &str, target: &mut T) -> bool {
        match self.get(key) {
            Some(state) => {
                *target = state;
                true
            }
            None => false,
        }
    }

    /// Whether `key` holds an entry.
    #[must_use]
    pub fn contains(&self, key: &str) -> bool {
        self.entries.contains_key(key)
    }

    /// Drop the entry under `key`. Returns whether there was one.
    pub fn remove(&mut self, key: &str) -> bool {
        let removed = self.entries.remove(key).is_some();
        self.dirty |= removed;
        removed
    }

    /// Drop every entry, as a "Reset layout" command does.
    pub fn clear(&mut self) {
        self.dirty |= !self.entries.is_empty();
        self.entries.clear();
    }

    /// The stored keys, sorted.
    pub fn keys(&self) -> impl Iterator<Item = &str> {
        self.entries.keys().map(String::as_str)
    }

    /// Whether a [`put`](Self::put), [`remove`](Self::remove) or [`clear`](Self::clear) changed
    /// the store since it was loaded or last [`save`](Self::save)d.
    #[must_use]
    pub fn is_dirty(&self) -> bool {
        self.dirty
    }
}
