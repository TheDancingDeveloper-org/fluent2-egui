//! Ribbon tabs: content tabs and contextual tab sets (spec §4.1, §4.3).
//!
//! A tab's body is built by a closure that runs once per frame. Groups are added by WI-1563;
//! until then a body is empty.

use std::marker::PhantomData;

use egui::Color32;

use crate::theme::{Theme, c};

/// Accent of a contextual tab set. Each accent maps to one theme token (spec §4.3, D6).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ContextualAccent {
    /// `color.brand_foreground_1`.
    Brand,
    /// `color.status_success_foreground_1`.
    Success,
    /// `color.status_warning_foreground_1`.
    Warning,
    /// `color.status_danger_foreground_1`.
    Danger,
    /// `color.neutral_foreground_2`.
    Neutral,
}

impl ContextualAccent {
    /// The colour of this accent's header label in `theme`.
    pub fn color(self, theme: &Theme) -> Color32 {
        let color = &theme.color;
        match self {
            Self::Brand => c(color.brand_foreground_1),
            Self::Success => c(color.status_success_foreground_1),
            Self::Warning => c(color.status_warning_foreground_1),
            Self::Danger => c(color.status_danger_foreground_1),
            Self::Neutral => c(color.neutral_foreground_2),
        }
    }
}

/// Adds the groups of one tab body. The body is empty until WI-1563 adds groups.
pub struct TabBuilder<'a> {
    _bound: PhantomData<&'a ()>,
}

impl TabBuilder<'_> {
    pub(crate) fn new() -> Self {
        Self { _bound: PhantomData }
    }
}

/// Placement of a tab in a contextual set (spec §4.3).
#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) struct Contextual {
    pub set: &'static str,
    pub label: String,
    pub accent: ContextualAccent,
    pub visible: bool,
}

/// A content or contextual tab of a [`super::Ribbon`].
///
/// ```
/// # use fluent2_egui::office::ribbon::{ContextualAccent, RibbonTab};
/// let home = RibbonTab::new("home", "Home", |_tab| {});
/// let table = RibbonTab::new("table", "Table design", |_tab| {})
///     .contextual("table", "Table tools", ContextualAccent::Brand, false);
/// ```
pub struct RibbonTab<'a> {
    pub(crate) key: &'static str,
    pub(crate) label: String,
    pub(crate) contextual: Option<Contextual>,
    pub(crate) build: Box<dyn FnOnce(&mut TabBuilder<'a>) + 'a>,
}

impl<'a> RibbonTab<'a> {
    /// A content tab. `key` is stable and unique within the ribbon; `build` adds its groups.
    pub fn new(key: &'static str, label: &str, build: impl FnOnce(&mut TabBuilder<'a>) + 'a) -> Self {
        Self {
            key,
            label: label.to_owned(),
            contextual: None,
            build: Box::new(build),
        }
    }

    /// Makes this tab part of contextual set `set`. Tabs of one set share a header that reads
    /// `header`, drawn in the accent colour above them. Every tab of a set should pass the same
    /// header; the first visible tab's text is used. The tab shows only while `visible` is true.
    #[must_use]
    pub fn contextual(
        mut self,
        set: &'static str,
        header: &str,
        accent: ContextualAccent,
        visible: bool,
    ) -> Self {
        self.contextual = Some(Contextual {
            set,
            label: header.to_owned(),
            accent,
            visible,
        });
        self
    }
}
