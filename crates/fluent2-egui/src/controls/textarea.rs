//! Fluent Textarea: a multiline [`Input`] frame around egui's [`TextEdit`].

use egui::{Id, Response, Ui, Widget};

use super::input::{Field, InputAppearance, show_field};
use crate::foundation::icon::IconSize;

/// How a [`Textarea`] may be resized.
#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub enum TextareaResize {
    /// The inner [`TextEdit`] keeps a vertical resize handle. The default.
    #[default]
    Vertical,
    /// Fixed to `rows` lines.
    None,
}

/// A multiline Fluent input with the same frame as [`super::Input`].
///
/// ```
/// # use fluent2_egui::controls::Textarea;
/// # fn demo(ui: &mut egui::Ui, value: &mut String) {
/// let _ = ui.add(Textarea::new(value).rows(4).placeholder("Notes"));
/// # }
/// ```
pub struct Textarea<'a> {
    text: &'a mut String,
    placeholder: Option<String>,
    label: Option<String>,
    rows: usize,
    resize: TextareaResize,
    desired_width: Option<f32>,
    id_salt: Option<Id>,
}

impl<'a> Textarea<'a> {
    /// A textarea bound to `text`. Defaults to three rows.
    pub fn new(text: &'a mut String) -> Self {
        Self {
            text,
            placeholder: None,
            label: None,
            rows: 3,
            resize: TextareaResize::Vertical,
            desired_width: None,
            id_salt: None,
        }
    }

    /// Grey hint (`neutral_foreground_4`), drawn only while `text` is empty.
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

    /// Visible line count. Zero becomes one row.
    #[must_use]
    pub fn rows(mut self, rows: usize) -> Self {
        self.rows = rows.max(1);
        self
    }

    /// Resize handle. Defaults to [`TextareaResize::Vertical`].
    #[must_use]
    pub fn resize(mut self, resize: TextareaResize) -> Self {
        self.resize = resize;
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

impl Widget for Textarea<'_> {
    fn ui(self, ui: &mut Ui) -> Response {
        show_field(
            ui,
            Field {
                text: self.text,
                placeholder: self.placeholder,
                label: self.label,
                appearance: InputAppearance::Outline,
                content_before: None,
                content_after: None,
                password: false,
                multiline: true,
                rows: self.rows.max(1),
                resize_vertical: matches!(self.resize, TextareaResize::Vertical),
                height: 0.0,
                icon: IconSize::S16,
                desired_width: self.desired_width,
                id_salt: self.id_salt,
                clear_button: false,
            },
        )
    }
}
