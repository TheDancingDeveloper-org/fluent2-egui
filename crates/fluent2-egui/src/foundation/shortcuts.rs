//! One registry for keyboard shortcuts: key handling, menu captions, tooltips and the help
//! overlay all read the same bindings, so they cannot drift apart.
//!
//! The app builds a [`Shortcuts`] once, [`install`]s it, and then asks
//! [`ShortcutsExt::shortcut_pressed`] each frame. [`crate::surfaces::MenuItem::bound`] draws the
//! caption and [`crate::surfaces::ShortcutOverlay`] lists every binding by group. Captions come
//! from [`egui::Context::format_shortcut`], so macOS shows ⌘ where other platforms show Ctrl.
//!
//! ```
//! use egui::{Key, KeyboardShortcut, Modifiers};
//! use fluent2_egui::foundation::shortcuts::{self, Shortcut, Shortcuts, ShortcutsExt};
//!
//! # fn demo(ctx: &egui::Context) -> Result<(), shortcuts::ShortcutError> {
//! let mut registry = Shortcuts::new();
//! registry.register(
//!     Shortcut::new("file.save", KeyboardShortcut::new(Modifiers::COMMAND, Key::S), "Save").group("File"),
//! )?;
//! shortcuts::install(ctx, registry);
//! if ctx.shortcut_pressed("file.save") {
//!     // the app decides what Save does
//! }
//! # Ok(())
//! # }
//! ```

use std::sync::Arc;

use egui::{Context, Event, Id, KeyboardShortcut, Modifiers};

/// Where a shortcut may fire.
#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub enum ShortcutScope {
    /// Anywhere, including while a text field has focus. The default.
    ///
    /// A binding without Ctrl, Cmd or Alt never fires while a text field has focus, whatever
    /// its scope: typing "j" must not trigger a "J" shortcut.
    #[default]
    Global,
    /// Only while no text field has keyboard focus, for bindings such as Ctrl+A that a text
    /// field also uses.
    NotTyping,
}

/// One binding in a [`Shortcuts`] registry.
#[derive(Clone, Debug, PartialEq)]
pub struct Shortcut {
    /// Stable id the app asks about, for example `"file.save"`. Never a translated label.
    pub id: &'static str,
    /// The keys.
    pub keys: KeyboardShortcut,
    /// What the shortcut does, shown in the overlay and in tooltips.
    pub label: String,
    /// Heading the overlay lists it under. Empty means "General".
    pub group: String,
    /// Where it may fire.
    pub scope: ShortcutScope,
}

impl Shortcut {
    /// A [`ShortcutScope::Global`] binding in the "General" group.
    ///
    /// ```
    /// # use egui::{Key, KeyboardShortcut, Modifiers};
    /// # use fluent2_egui::foundation::shortcuts::Shortcut;
    /// let find = Shortcut::new("edit.find", KeyboardShortcut::new(Modifiers::COMMAND, Key::F), "Find");
    /// ```
    pub fn new(id: &'static str, keys: KeyboardShortcut, label: impl Into<String>) -> Self {
        Self {
            id,
            keys,
            label: label.into(),
            group: String::new(),
            scope: ShortcutScope::Global,
        }
    }

    /// Heading the overlay lists this binding under.
    #[must_use]
    pub fn group(mut self, group: impl Into<String>) -> Self {
        self.group = group.into();
        self
    }

    /// Where the binding may fire.
    #[must_use]
    pub fn scope(mut self, scope: ShortcutScope) -> Self {
        self.scope = scope;
        self
    }

    /// Whether the keys carry Ctrl, Cmd or Alt. Bindings without them are "typing" keys.
    pub fn has_command_modifier(&self) -> bool {
        let m = self.keys.modifiers;
        m.ctrl || m.command || m.mac_cmd || m.alt
    }

    /// Whether this binding may fire while a text field has focus.
    pub fn fires_while_typing(&self) -> bool {
        self.scope == ShortcutScope::Global && self.has_command_modifier()
    }

    /// The group heading, with "General" for an empty group.
    pub fn group_name(&self) -> &str {
        if self.group.is_empty() {
            "General"
        } else {
            &self.group
        }
    }
}

/// Why [`Shortcuts::register`] refused a binding.
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum ShortcutError {
    /// The id is already registered.
    DuplicateId(&'static str),
    /// The keys are already bound to the id `existing`.
    Conflict {
        /// The id being registered.
        id: &'static str,
        /// The id that already holds the keys.
        existing: &'static str,
    },
}

impl std::fmt::Display for ShortcutError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::DuplicateId(id) => write!(f, "shortcut id `{id}` is registered twice"),
            Self::Conflict { id, existing } => {
                write!(f, "shortcut `{id}` uses the same keys as `{existing}`")
            }
        }
    }
}

impl std::error::Error for ShortcutError {}

/// The app's keyboard shortcuts, in registration order.
///
/// ```
/// # use egui::{Key, KeyboardShortcut, Modifiers};
/// # use fluent2_egui::foundation::shortcuts::{Shortcut, Shortcuts};
/// let mut registry = Shortcuts::new();
/// let save = KeyboardShortcut::new(Modifiers::COMMAND, Key::S);
/// assert!(registry.register(Shortcut::new("save", save, "Save")).is_ok());
/// assert!(registry.register(Shortcut::new("store", save, "Store")).is_err());
/// ```
#[derive(Clone, Debug, Default, PartialEq)]
pub struct Shortcuts {
    entries: Vec<Shortcut>,
}

impl Shortcuts {
    /// An empty registry.
    pub fn new() -> Self {
        Self::default()
    }

    /// Add a binding. Refuses a repeated id or keys another binding already holds.
    ///
    /// Keys compare by their logical key and modifiers, with Cmd and Ctrl treated as one, so
    /// `COMMAND+S` and `CTRL+S` conflict.
    pub fn register(&mut self, shortcut: Shortcut) -> Result<(), ShortcutError> {
        if self.entries.iter().any(|e| e.id == shortcut.id) {
            return Err(ShortcutError::DuplicateId(shortcut.id));
        }
        if let Some(existing) = self.entries.iter().find(|e| same_keys(&e.keys, &shortcut.keys)) {
            return Err(ShortcutError::Conflict {
                id: shortcut.id,
                existing: existing.id,
            });
        }
        self.entries.push(shortcut);
        Ok(())
    }

    /// The binding with this id.
    pub fn get(&self, id: &str) -> Option<&Shortcut> {
        self.entries.iter().find(|e| e.id == id)
    }

    /// Every binding, in registration order.
    pub fn iter(&self) -> impl Iterator<Item = &Shortcut> {
        self.entries.iter()
    }

    /// Number of bindings.
    pub fn len(&self) -> usize {
        self.entries.len()
    }

    /// Whether the registry is empty.
    pub fn is_empty(&self) -> bool {
        self.entries.is_empty()
    }

    /// Group names in the order their first binding was registered.
    pub fn groups(&self) -> Vec<&str> {
        let mut groups: Vec<&str> = Vec::new();
        for entry in &self.entries {
            let name = entry.group_name();
            if !groups.contains(&name) {
                groups.push(name);
            }
        }
        groups
    }
}

/// Whether two key combinations are the same, with Cmd and Ctrl folded together.
fn same_keys(a: &KeyboardShortcut, b: &KeyboardShortcut) -> bool {
    a.logical_key == b.logical_key && fold(a.modifiers) == fold(b.modifiers)
}

/// Modifiers with Ctrl, Cmd and the Mac Cmd key folded into `command`.
fn fold(m: Modifiers) -> (bool, bool, bool) {
    (m.alt, m.shift, m.ctrl || m.command || m.mac_cmd)
}

fn store_id() -> Id {
    Id::NULL.with("fluent2_shortcuts")
}

/// Make `shortcuts` the registry for `ctx`. Call again to replace it.
pub fn install(ctx: &Context, shortcuts: Shortcuts) {
    ctx.data_mut(|data| data.insert_temp(store_id(), Arc::new(shortcuts)));
}

/// The installed registry, or an empty one.
pub fn get(ctx: &Context) -> Arc<Shortcuts> {
    ctx.data(|data| data.get_temp::<Arc<Shortcuts>>(store_id()))
        .unwrap_or_default()
}

/// Shortcut queries on [`egui::Context`].
pub trait ShortcutsExt {
    /// Whether the binding `id` was pressed this frame. The key press is consumed, so a second
    /// call (or a widget further down) does not see it again.
    ///
    /// Returns `false` for an unknown id, for a binding that may not fire while a text field
    /// has focus, and when the keys also match a registered binding with more modifiers that
    /// was pressed instead (Ctrl+Shift+S does not trigger Ctrl+S).
    fn shortcut_pressed(&self, id: &str) -> bool;

    /// The platform caption for `id`, for example `"Ctrl+S"` or `"⌘S"`.
    fn shortcut_caption(&self, id: &str) -> Option<String>;

    /// Tooltip text for `id`: `"Save (Ctrl+S)"`. `None` for an unknown id.
    fn shortcut_tooltip(&self, id: &str) -> Option<String>;
}

impl ShortcutsExt for Context {
    fn shortcut_pressed(&self, id: &str) -> bool {
        let registry = get(self);
        let Some(entry) = registry.get(id) else {
            return false;
        };
        if self.egui_wants_keyboard_input() && !entry.fires_while_typing() {
            return false;
        }
        let keys = entry.keys;
        // A binding with the same key and a superset of modifiers wins when those were held.
        let wider: Vec<KeyboardShortcut> = registry
            .iter()
            .filter(|e| e.id != entry.id && e.keys.logical_key == keys.logical_key)
            .map(|e| e.keys)
            .filter(|k| fold(k.modifiers) != fold(keys.modifiers))
            .collect();
        self.input_mut(|input| {
            let held = input.events.iter().find_map(|event| match event {
                Event::Key {
                    key,
                    pressed: true,
                    modifiers,
                    ..
                } if *key == keys.logical_key && modifiers.matches_logically(keys.modifiers) => {
                    Some(*modifiers)
                }
                _ => None,
            });
            let Some(held) = held else {
                return false;
            };
            if wider
                .iter()
                .any(|k| held.matches_logically(k.modifiers) && k.modifiers.shift && !keys.modifiers.shift)
            {
                return false;
            }
            input.consume_shortcut(&keys)
        })
    }

    fn shortcut_caption(&self, id: &str) -> Option<String> {
        let keys = get(self).get(id)?.keys;
        Some(self.format_shortcut(&keys))
    }

    fn shortcut_tooltip(&self, id: &str) -> Option<String> {
        let registry = get(self);
        let entry = registry.get(id)?;
        Some(format!("{} ({})", entry.label, self.format_shortcut(&entry.keys)))
    }
}
