//! Fluent SearchBox: an [`Input`] with a search icon and a clear button.
//!
//! Follows Fluent UI React SearchBox at the revision pinned on WI-1547. The dismiss
//! button is shown only while the string is not empty. Activating it, or pressing
//! Escape while the field is focused, clears the string and keeps focus.

use egui::{Id, Response, Ui, Widget};

use super::ButtonSize;
use super::input::{Input, InputAppearance};
use crate::foundation::icon::IconName;

/// A search field.
///
/// ```
/// # use fluent2_egui::controls::SearchBox;
/// # fn demo(ui: &mut egui::Ui, query: &mut String) {
/// let _ = ui.add(SearchBox::new(query).placeholder("Search"));
/// # }
/// ```
pub struct SearchBox<'a> {
    text: &'a mut String,
    placeholder: Option<String>,
    label: Option<String>,
    size: ButtonSize,
    appearance: InputAppearance,
    desired_width: Option<f32>,
    id_salt: Option<Id>,
}

impl<'a> SearchBox<'a> {
    /// A search box bound to `text`.
    pub fn new(text: &'a mut String) -> Self {
        Self {
            text,
            placeholder: None,
            label: None,
            size: ButtonSize::Medium,
            appearance: InputAppearance::Outline,
            desired_width: None,
            id_salt: None,
        }
    }

    /// Grey hint, drawn only while `text` is empty.
    #[must_use]
    pub fn placeholder(mut self, placeholder: impl Into<String>) -> Self {
        self.placeholder = Some(placeholder.into());
        self
    }

    /// Accessible name. Not drawn.
    #[must_use]
    pub fn label(mut self, label: impl Into<String>) -> Self {
        self.label = Some(label.into());
        self
    }

    /// Height. Defaults to [`ButtonSize::Medium`].
    #[must_use]
    pub fn size(mut self, size: ButtonSize) -> Self {
        self.size = size;
        self
    }

    /// Rest treatment. Defaults to [`InputAppearance::Outline`].
    #[must_use]
    pub fn appearance(mut self, appearance: InputAppearance) -> Self {
        self.appearance = appearance;
        self
    }

    /// Width of the field. Defaults to [`Ui::available_width`].
    #[must_use]
    pub fn desired_width(mut self, width: f32) -> Self {
        self.desired_width = Some(if width.is_finite() { width.max(0.0) } else { 0.0 });
        self
    }

    /// Extra salt for the widget id.
    #[must_use]
    pub fn id_salt(mut self, salt: impl std::hash::Hash + std::fmt::Debug) -> Self {
        self.id_salt = Some(Id::new(salt));
        self
    }
}

impl Widget for SearchBox<'_> {
    fn ui(mut self, ui: &mut Ui) -> Response {
        let mut input = Input::new(self.text)
            .size(self.size)
            .appearance(self.appearance)
            .content_before(IconName::Search)
            .clear_button();
        if let Some(placeholder) = self.placeholder.take() {
            input = input.placeholder(placeholder);
        }
        if let Some(label) = self.label.take() {
            input = input.label(label);
        }
        if let Some(width) = self.desired_width {
            input = input.desired_width(width);
        }
        if let Some(salt) = self.id_salt.take() {
            input = input.id_salt(salt.value());
        }
        ui.add(input)
    }
}
