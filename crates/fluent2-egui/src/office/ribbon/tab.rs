//! Ribbon tabs: content tabs and contextual tab sets (spec §4.1, §4.3).
//!
//! A tab's body is built by a closure that runs once per frame. Groups are added by WI-1563;
//! until then a body is empty.

use std::marker::PhantomData;

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
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) struct Contextual {
    pub set: &'static str,
    pub accent: ContextualAccent,
    pub visible: bool,
}

/// A content or contextual tab of a [`super::Ribbon`].
///
/// ```
/// # use fluent2_egui::office::ribbon::{ContextualAccent, RibbonTab};
/// let home = RibbonTab::new("home", "Home", |_tab| {});
/// let table = RibbonTab::new("table", "Table design", |_tab| {})
///     .contextual("table", ContextualAccent::Brand, false);
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

    /// Makes this tab part of contextual set `set`. It shows only while `visible` is true.
    #[must_use]
    pub fn contextual(mut self, set: &'static str, accent: ContextualAccent, visible: bool) -> Self {
        self.contextual = Some(Contextual { set, accent, visible });
        self
    }

    /// The accent of this tab's contextual set, or `None` for a content tab.
    pub fn accent(&self) -> Option<ContextualAccent> {
        self.contextual.map(|placement| placement.accent)
    }
}
