//! EditableList: a column of text or number rows that are edited in place, with an add field.
//!
//! The list never changes its rows. It holds only what the user is typing (the row being edited
//! and the add field). Each committed change comes back as an [`EditableListEvent`] for the app to
//! apply, so the app's data stays the single source of truth (ARCHITECTURE rule 5).
//!
//! Keyboard: Tab reaches every row and the add field. Enter or a click on a row starts editing it.
//! In the editor, Enter commits, Escape cancels, and leaving the editor commits a valid edit.
//! Delete removes the focused row, and F2 edits it. Enter in the add field, or the Add button,
//! appends a row. A value that does not parse (for [`EditableKind::Number`]) stays in the editor
//! with a message.
//!
//! ```
//! use fluent2_egui::data::editable_list::{EditableList, EditableListEvent, EditableListState};
//!
//! # fn demo(ui: &mut egui::Ui) {
//! let mut rows = vec!["10.0.0.0/8".to_owned()];
//! let mut state = EditableListState::default();
//! let response = EditableList::new("ip_filters", &rows).label("IP filters").show(ui, &mut state);
//! for event in response.events {
//!     match event {
//!         EditableListEvent::Added(value) => rows.push(value),
//!         EditableListEvent::Edited { index, value } => {
//!             if let Some(row) = rows.get_mut(index) {
//!                 *row = value;
//!             }
//!         }
//!         EditableListEvent::Removed { index } => {
//!             if index < rows.len() {
//!                 rows.remove(index);
//!             }
//!         }
//!     }
//! }
//! # }
//! ```

use egui::accesskit::{Invalid, Role};
use egui::{Id, Key, Response, RichText, Ui, UiBuilder, WidgetInfo};

use fluent2_icons::Icon;

use crate::controls::{Button, Input};
use crate::theme::{Theme, c};

/// Share of the row the add field takes; the Add button takes the rest, so it stays in view.
const ADD_FIELD_SHARE: f32 = 0.7;

/// What a row holds. Both kinds refuse blank text.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum EditableKind {
    /// Free text, trimmed of surrounding spaces.
    #[default]
    Text,
    /// A finite decimal number, such as `1.5` or `-3`.
    Number,
}

/// A change the user committed. The list does not apply it; the app does.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum EditableListEvent {
    /// A new row from the add field, trimmed.
    Added(String),
    /// Row `index` now reads `value`, trimmed. Sent only when the text changed.
    Edited {
        /// Position of the row in the app's list.
        index: usize,
        /// The new text of the row.
        value: String,
    },
    /// Row `index` was removed. An edit in progress on any row is dropped, not committed.
    Removed {
        /// Position of the row that was removed.
        index: usize,
    },
}

/// UI state of an EditableList: the row being edited and the add field. Store it in the app and
/// pass `&mut` to every `show`.
///
/// ```
/// let state = fluent2_egui::data::editable_list::EditableListState::default();
/// assert_eq!(state, fluent2_egui::data::editable_list::EditableListState::default());
/// ```
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct EditableListState {
    editing: Option<Editing>,
    add_draft: String,
    add_invalid: bool,
    /// Row whose button takes focus on the next frame, after its editor closed with Enter or Escape.
    restore_focus: Option<usize>,
}

/// The row an editor is open on.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
struct Editing {
    index: usize,
    draft: String,
    /// Take keyboard focus on the first frame.
    fresh: bool,
    invalid: bool,
}

/// What an EditableList reports after `show`.
#[derive(Clone, Debug)]
pub struct EditableListResponse {
    /// The whole list.
    pub response: Response,
    /// Changes committed this frame, in the order they happened.
    pub events: Vec<EditableListEvent>,
}

/// An editable list of rows. Build it, then call [`EditableList::show`].
///
/// ```
/// # use fluent2_egui::data::editable_list::{EditableKind, EditableList, EditableListState};
/// # fn demo(ui: &mut egui::Ui) {
/// let rows = vec!["1.5".to_owned(), "-3".to_owned()];
/// let mut state = EditableListState::default();
/// let _response = EditableList::new("alt_speed", &rows)
///     .label("Speed limits")
///     .kind(EditableKind::Number)
///     .show(ui, &mut state);
/// # }
/// ```
pub struct EditableList<'a> {
    id: Id,
    rows: &'a [String],
    kind: EditableKind,
    label: String,
}

impl<'a> EditableList<'a> {
    /// A list over `rows`. The id comes from `id_salt`, never from a translated label.
    pub fn new(id_salt: impl std::hash::Hash + std::fmt::Debug, rows: &'a [String]) -> Self {
        Self {
            id: Id::unique(id_salt),
            rows,
            kind: EditableKind::Text,
            label: "List".to_owned(),
        }
    }

    /// Accessible name of the list.
    #[must_use]
    pub fn label(mut self, label: impl Into<String>) -> Self {
        self.label = label.into();
        self
    }

    /// Text or number rows. Defaults to [`EditableKind::Text`].
    #[must_use]
    pub fn kind(mut self, kind: EditableKind) -> Self {
        self.kind = kind;
        self
    }

    /// Paint the rows and the add field, and return the events committed this frame.
    pub fn show(self, ui: &mut Ui, state: &mut EditableListState) -> EditableListResponse {
        let mut events = Vec::new();
        let enabled = ui.is_enabled();
        let response = ui
            .scope_builder(UiBuilder::new().id_salt(self.id), |ui| {
                ui.vertical(|ui| {
                    for (index, value) in self.rows.iter().enumerate() {
                        self.row(ui, state, index, value, &mut events);
                    }
                    self.add_row(ui, state, &mut events);
                });
            })
            .response;
        response.widget_info(|| WidgetInfo::labeled(Role::Unknown, enabled, self.label.clone()));
        ui.ctx()
            .accesskit_node_builder(response.id, |node| node.set_role(Role::List));
        EditableListResponse { response, events }
    }

    fn row(
        &self,
        ui: &mut Ui,
        state: &mut EditableListState,
        index: usize,
        value: &str,
        events: &mut Vec<EditableListEvent>,
    ) {
        if state.editing.as_ref().is_some_and(|edit| edit.index == index) {
            self.editor(ui, state, index, value, events);
            return;
        }
        let key = (self.id, index);
        ui.horizontal(|ui| {
            let row = ui.add(Button::new(value).id_salt(key));
            if state.restore_focus == Some(index) {
                row.request_focus();
                state.restore_focus = None;
            }
            let focused = row.has_focus();
            if row.clicked() || (focused && ui.input(|input| input.key_pressed(Key::F2))) {
                state.editing = Some(Editing {
                    index,
                    draft: value.to_owned(),
                    fresh: true,
                    invalid: false,
                });
            }
            let delete = focused && ui.input(|input| input.key_pressed(Key::Delete));
            let remove =
                ui.add(Button::icon_only(Icon::Delete, format!("Remove {value}")).id_salt((key, "remove")));
            if delete || remove.clicked() {
                // Removing a row drops any edit in progress, so no index shifts under an open editor.
                state.editing = None;
                events.push(EditableListEvent::Removed { index });
            }
        });
    }

    fn editor(
        &self,
        ui: &mut Ui,
        state: &mut EditableListState,
        index: usize,
        value: &str,
        events: &mut Vec<EditableListEvent>,
    ) {
        let Some(mut edit) = state.editing.take() else {
            return;
        };
        let key = (self.id, index);
        let field = ui
            .horizontal(|ui| {
                ui.add(
                    Input::new(&mut edit.draft)
                        .label(format!("Edit {value}"))
                        .id_salt((key, "edit")),
                )
            })
            .inner;
        if edit.fresh {
            field.request_focus();
            edit.fresh = false;
        }
        if field.changed() {
            edit.invalid = false;
        }
        // egui's singleline TextEdit gives up focus on Escape within its own frame, so a lost focus counts.
        let escape =
            (field.has_focus() || field.lost_focus()) && ui.input(|input| input.key_pressed(Key::Escape));
        let enter =
            (field.has_focus() || field.lost_focus()) && ui.input(|input| input.key_pressed(Key::Enter));
        let blurred = field.lost_focus() && !enter && !escape;

        let mut close = escape;
        if enter || blurred {
            match accept(self.kind, &edit.draft) {
                Some(new) => {
                    if new != value {
                        events.push(EditableListEvent::Edited { index, value: new });
                    }
                    close = true;
                }
                // Leaving an invalid edit reverts it; Enter keeps it open to correct.
                None if blurred => close = true,
                None => edit.invalid = true,
            }
        }
        if edit.invalid {
            ui.ctx()
                .accesskit_node_builder(field.id, |node| node.set_invalid(Invalid::True));
            invalid_message(ui, self.kind);
        }
        if close {
            if enter || escape {
                // The row button is not drawn this frame, so it takes focus on the next one.
                state.restore_focus = Some(index);
            }
        } else {
            state.editing = Some(edit);
        }
    }

    fn add_row(&self, ui: &mut Ui, state: &mut EditableListState, events: &mut Vec<EditableListEvent>) {
        let key = (self.id, "add");
        let placeholder = match self.kind {
            EditableKind::Text => "Add an item",
            EditableKind::Number => "Add a number",
        };
        let (field, add) = ui
            .horizontal(|ui| {
                let field = ui.add(
                    Input::new(&mut state.add_draft)
                        .label("New item")
                        .placeholder(placeholder)
                        .desired_width(ui.available_width() * ADD_FIELD_SHARE)
                        .id_salt(key),
                );
                let add = ui.add(Button::new("Add").icon(Icon::Add).id_salt((key, "button")));
                (field, add)
            })
            .inner;
        if field.changed() {
            state.add_invalid = false;
        }
        let enter =
            (field.has_focus() || field.lost_focus()) && ui.input(|input| input.key_pressed(Key::Enter));
        if enter || add.clicked() {
            match accept(self.kind, &state.add_draft) {
                Some(value) => {
                    events.push(EditableListEvent::Added(value));
                    state.add_draft.clear();
                    state.add_invalid = false;
                }
                None => state.add_invalid = true,
            }
            if enter {
                field.request_focus();
            }
        }
        if state.add_invalid {
            ui.ctx()
                .accesskit_node_builder(field.id, |node| node.set_invalid(Invalid::True));
            invalid_message(ui, self.kind);
        }
    }
}

/// The trimmed text a row would hold, or `None` when it is blank or, for numbers, not finite.
fn accept(kind: EditableKind, text: &str) -> Option<String> {
    let trimmed = text.trim();
    if trimmed.is_empty() {
        return None;
    }
    match kind {
        EditableKind::Text => Some(trimmed.to_owned()),
        EditableKind::Number => trimmed
            .parse::<f64>()
            .ok()
            .filter(|number| number.is_finite())
            .map(|_| trimmed.to_owned()),
    }
}

/// The message under a refused value, in the Fluent danger colour.
fn invalid_message(ui: &mut Ui, kind: EditableKind) {
    let theme = Theme::get(ui.ctx());
    let text = match kind {
        EditableKind::Text => "Enter some text",
        EditableKind::Number => "Enter a number",
    };
    ui.label(RichText::new(text).color(c(theme.color.status_danger_foreground_1)));
}
