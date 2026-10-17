//! SplitView: two panes and a draggable divider (`docs/specs/data-views.md` §8).
//!
//! The layout maths and the divider are adapted from craft-libs, as listed in `UPSTREAM.md`
//! and `NOTICE`. The adaptation changes are: two minimum sizes instead of one, theme colours
//! for the divider, caller-hashed ids, `String` labels, and Shift and Home/End keys.

// Adapted from storytold/craft-libs crates/ui/src/layout/geometry.rs @ 457a0788.
// Adapted from storytold/craft-libs crates/ui/src/layout/resize.rs @ 457a0788.
// Copyright (c) 2026 the craft-libs contributors. Licensed MIT OR Apache-2.0.

use egui::{
    Color32, CornerRadius, EventFilter, Id, Key, Modifiers, Pos2, Rect, Response, Sense, Ui, UiBuilder, Vec2,
    WidgetInfo, WidgetType, accesskit,
};

use crate::foundation::focus::{self, paint_focus_ring};
use crate::theme::{Theme, c};

/// Width of the divider that separates the panes, in points.
const DIVIDER_GAP: f32 = 1.0;
/// Extra hit area on each side of the divider, in points (§8.1).
const DIVIDER_HIT_MARGIN: f32 = 3.5;
/// Default minimum size of each pane, in points (§8.2).
const DEFAULT_MINIMUM: f32 = 48.0;
/// Default keyboard step, in points (§8.2).
const DEFAULT_STEP: f32 = 8.0;
/// Shift with an arrow key moves this many steps (§8.2).
const SHIFT_STEPS: f32 = 10.0;

/// The direction in which the children of a split are placed.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub enum SplitAxis {
    /// First pane on the left, second pane on the right.
    Horizontal,
    /// First pane above the second pane.
    Vertical,
}

/// The size of the first pane.
#[derive(Clone, Copy, Debug, PartialEq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub enum SplitSize {
    /// Fraction of the space left after the divider is removed, given to the first pane.
    Ratio(f32),
    /// First pane size in logical points.
    FixedFirst(f32),
    /// Second pane size in logical points.
    FixedSecond(f32),
}

impl SplitSize {
    fn value(self) -> f32 {
        match self {
            Self::Ratio(v) | Self::FixedFirst(v) | Self::FixedSecond(v) => v,
        }
    }
}

/// State of a SplitView. The caller owns it, so the size survives frames and restarts.
///
/// ```
/// let state = fluent2_egui::data::split_view::SplitState::default();
/// assert_eq!(state.size, fluent2_egui::data::split_view::SplitSize::Ratio(0.5));
/// ```
#[derive(Clone, Copy, Debug, PartialEq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct SplitState {
    /// The first pane's size. Non-finite values are reset to an even split on the next frame.
    pub size: SplitSize,
}

impl Default for SplitState {
    fn default() -> Self {
        Self {
            size: SplitSize::Ratio(0.5),
        }
    }
}

/// Rectangles of one split, including its divider.
#[derive(Clone, Copy, Debug, PartialEq)]
pub(crate) struct SplitLayout {
    pub first: Rect,
    pub divider: Rect,
    pub second: Rect,
    first_size: f32,
    available: f32,
}

impl SplitLayout {
    /// The first pane's size along the split axis, in logical points.
    pub(crate) fn first_size(self) -> f32 {
        self.first_size
    }

    /// The range the first pane may take, in logical points.
    pub(crate) fn bounds(self, minimums: (f32, f32)) -> (f32, f32) {
        let (low, high) = bounds(f64::from(self.available), minimums);
        (low as f32, high as f32)
    }

    /// Propose a new size after moving the divider by `delta` points.
    ///
    /// The sizing mode is kept. Invalid input and zero available space return `None`, so the
    /// caller leaves its state unchanged.
    pub(crate) fn resized(self, size: SplitSize, delta: f32, minimums: (f32, f32)) -> Option<SplitSize> {
        if !size.value().is_finite()
            || !delta.is_finite()
            || !valid_minimums(minimums)
            || self.available <= 0.0
        {
            return None;
        }
        let available = f64::from(self.available);
        let (low, high) = bounds(available, minimums);
        let first = (f64::from(self.first_size) + f64::from(delta)).clamp(low, high);
        Some(match size {
            SplitSize::Ratio(_) => SplitSize::Ratio((first / available) as f32),
            SplitSize::FixedFirst(_) => SplitSize::FixedFirst(first as f32),
            SplitSize::FixedSecond(_) => SplitSize::FixedSecond((available - first) as f32),
        })
    }
}

/// Divide `rect` into two panes and a divider (upstream `split_rect`).
///
/// `gap` is the divider width. `minimums` are the first and second pane minima. When both
/// minima cannot fit, each pane gets half the space. Inverted rectangles, non-finite
/// coordinates or sizes, and negative or non-finite gaps and minima return `None`.
pub(crate) fn split_rect(
    rect: Rect,
    axis: SplitAxis,
    size: SplitSize,
    gap: f32,
    minimums: (f32, f32),
) -> Option<SplitLayout> {
    if !valid_rect(rect) || !valid_nonnegative(gap) || !valid_minimums(minimums) || !size.value().is_finite()
    {
        return None;
    }
    let total = match axis {
        SplitAxis::Horizontal => rect.width(),
        SplitAxis::Vertical => rect.height(),
    };
    let gap = gap.min(total);
    let available = total - gap;
    let space = f64::from(available);
    // Intermediate arithmetic is f64: two finite f32 inputs can overflow when multiplied.
    let wanted = match size {
        SplitSize::Ratio(ratio) => space * f64::from(ratio),
        SplitSize::FixedFirst(points) => f64::from(points),
        SplitSize::FixedSecond(points) => space - f64::from(points),
    };
    let (low, high) = bounds(space, minimums);
    let first_size = wanted.clamp(low, high) as f32;
    let mut first = rect;
    let mut divider = rect;
    let mut second = rect;
    match axis {
        SplitAxis::Horizontal => {
            first.max.x = (rect.min.x + first_size).min(rect.max.x);
            divider.min.x = first.max.x;
            divider.max.x = (divider.min.x + gap).min(rect.max.x);
            second.min.x = divider.max.x;
        }
        SplitAxis::Vertical => {
            first.max.y = (rect.min.y + first_size).min(rect.max.y);
            divider.min.y = first.max.y;
            divider.max.y = (divider.min.y + gap).min(rect.max.y);
            second.min.y = divider.max.y;
        }
    }
    Some(SplitLayout {
        first,
        divider,
        second,
        first_size,
        available,
    })
}

/// The range the first pane may take. Each minimum relaxes to half the space when both cannot
/// fit, so the bounds never cross (adapted from upstream `clamp_first`, now with two minima).
fn bounds(available: f64, minimums: (f32, f32)) -> (f64, f64) {
    let half = available * 0.5;
    let low = f64::from(minimums.0).min(half);
    let high = available - f64::from(minimums.1).min(half);
    (low, high)
}

fn valid_rect(rect: Rect) -> bool {
    rect.min.is_finite()
        && rect.max.is_finite()
        && rect.width().is_finite()
        && rect.height().is_finite()
        && rect.min.x <= rect.max.x
        && rect.min.y <= rect.max.y
}

fn valid_nonnegative(value: f32) -> bool {
    value.is_finite() && value >= 0.0
}

fn valid_minimums(minimums: (f32, f32)) -> bool {
    valid_nonnegative(minimums.0) && valid_nonnegative(minimums.1)
}

/// Colours of a resize handle. The idle handle paints no fill; the divider paints its idle
/// line itself.
#[derive(Clone, Copy, Debug)]
pub(crate) struct ResizeHandleStyle {
    pub hovered: Color32,
    pub dragged: Color32,
    pub focused: Color32,
}

/// A movement proposal. The caller applies `delta` with its own minimum-size policy.
pub(crate) struct ResizeResponse {
    /// Includes egui's drag and focus lifecycle.
    pub response: Response,
    /// Requested movement this frame in logical points along the split axis.
    pub delta: f32,
}

/// A focusable divider with pointer, arrow-key and accessibility resizing (upstream
/// `ResizeHandle`). It never stores a pane size. Adaptation: the label is a `String`, the id
/// comes from the caller, Shift scales arrow steps by [`SHIFT_STEPS`], and the style colours
/// come from the theme.
pub(crate) struct ResizeHandle {
    id: Id,
    axis: SplitAxis,
    rect: Rect,
    label: String,
    style: ResizeHandleStyle,
    hit_margin: f32,
    step: f32,
    value: Option<(f32, std::ops::RangeInclusive<f32>)>,
}

impl ResizeHandle {
    pub(crate) fn new(id: Id, axis: SplitAxis, rect: Rect, label: String, style: ResizeHandleStyle) -> Self {
        Self {
            id,
            axis,
            rect,
            label,
            style,
            hit_margin: 3.0,
            step: 8.0,
            value: None,
        }
    }

    /// Hit area on each side of the divider, along the movement axis.
    pub(crate) fn hit_margin(mut self, margin: f32) -> Self {
        self.hit_margin = margin;
        self
    }

    /// Arrow key step in logical points.
    pub(crate) fn keyboard_step(mut self, step: f32) -> Self {
        self.step = step;
        self
    }

    /// Expose the current size and bounds to accessibility, and support its SetValue action.
    pub(crate) fn value(mut self, value: f32, range: std::ops::RangeInclusive<f32>) -> Self {
        self.value = Some((value, range));
        self
    }

    pub(crate) fn show(self, ui: &mut Ui) -> ResizeResponse {
        let hit = self.rect.expand2(match self.axis {
            SplitAxis::Horizontal => Vec2::new(self.hit_margin, 0.0),
            SplitAxis::Vertical => Vec2::new(0.0, self.hit_margin),
        });
        let valid = valid_rect(self.rect)
            && self.rect.width() > 0.0
            && self.rect.height() > 0.0
            && valid_rect(hit)
            && self.hit_margin.is_finite()
            && (0.0..=4096.0).contains(&self.hit_margin)
            && self.step.is_finite()
            && self.step > 0.0
            && self.value.as_ref().is_none_or(|(value, range)| {
                value.is_finite()
                    && range.start().is_finite()
                    && range.end().is_finite()
                    && range.start() <= range.end()
            });
        let inert = Rect::from_min_size(ui.min_rect().min, Vec2::ZERO);
        let mut response = ui.interact(
            if valid { hit } else { inert },
            self.id,
            if valid { Sense::drag() } else { Sense::hover() },
        );
        let enabled = valid && response.enabled();
        let mut delta = 0.0;
        if enabled {
            if response.drag_started() {
                response.request_focus();
            }
            if response.dragged() {
                let drag = response.drag_delta();
                delta = match self.axis {
                    SplitAxis::Horizontal => drag.x,
                    SplitAxis::Vertical => drag.y,
                };
            }
            if response.has_focus() {
                ui.memory_mut(|memory| {
                    memory.set_focus_lock_filter(
                        response.id,
                        EventFilter {
                            horizontal_arrows: self.axis == SplitAxis::Horizontal,
                            vertical_arrows: self.axis == SplitAxis::Vertical,
                            ..Default::default()
                        },
                    );
                });
                let (back, forward) = match self.axis {
                    SplitAxis::Horizontal => (Key::ArrowLeft, Key::ArrowRight),
                    SplitAxis::Vertical => (Key::ArrowUp, Key::ArrowDown),
                };
                ui.input_mut(|input| {
                    let (modifiers, scale) = if input.modifiers.shift {
                        (Modifiers::SHIFT, SHIFT_STEPS)
                    } else {
                        (Modifiers::NONE, 1.0)
                    };
                    if input.consume_key(modifiers, back) {
                        delta -= self.step * scale;
                    }
                    if input.consume_key(modifiers, forward) {
                        delta += self.step * scale;
                    }
                });
            }
            ui.input_mut(|input| {
                input.consume_accesskit_action_requests(response.id, |request| match request.action {
                    accesskit::Action::Increment => {
                        delta += self.step;
                        true
                    }
                    accesskit::Action::Decrement => {
                        delta -= self.step;
                        true
                    }
                    accesskit::Action::SetValue => {
                        if let (Some((value, range)), Some(accesskit::ActionData::NumericValue(new_value))) =
                            (&self.value, &request.data)
                            && new_value.is_finite()
                        {
                            delta = (new_value.clamp(f64::from(*range.start()), f64::from(*range.end()))
                                - f64::from(*value)) as f32;
                        }
                        true
                    }
                    _ => false,
                });
            });
        }
        if !delta.is_finite() {
            delta = 0.0;
        }
        if delta != 0.0 {
            response.mark_changed();
        }
        response.widget_info(|| {
            if let Some((value, _)) = &self.value
                && valid
            {
                WidgetInfo::slider(enabled, f64::from(*value), self.label.as_str())
            } else {
                WidgetInfo::labeled(WidgetType::Other, enabled, self.label.as_str())
            }
        });
        ui.ctx().accesskit_node_builder(response.id, |node| {
            node.set_role(accesskit::Role::Splitter);
            // Orientation describes the divider itself, perpendicular to its movement axis.
            node.set_orientation(match self.axis {
                SplitAxis::Horizontal => accesskit::Orientation::Vertical,
                SplitAxis::Vertical => accesskit::Orientation::Horizontal,
            });
            if enabled {
                node.add_action(accesskit::Action::Increment);
                node.add_action(accesskit::Action::Decrement);
                if let Some((value, range)) = &self.value {
                    node.set_numeric_value(f64::from(*value));
                    node.set_min_numeric_value(f64::from(*range.start()));
                    node.set_max_numeric_value(f64::from(*range.end()));
                    node.set_numeric_value_step(f64::from(self.step));
                    node.add_action(accesskit::Action::SetValue);
                }
            }
        });
        if enabled {
            let fill = if response.dragged() {
                Some(self.style.dragged)
            } else if response.has_focus() {
                Some(self.style.focused)
            } else if response.hovered() {
                Some(self.style.hovered)
            } else {
                None
            };
            if let Some(fill) = fill {
                ui.painter().rect_filled(self.rect, 0.0, fill);
            }
            if response.hovered() || response.dragged() {
                ui.ctx().set_cursor_icon(match self.axis {
                    SplitAxis::Horizontal => egui::CursorIcon::ResizeHorizontal,
                    SplitAxis::Vertical => egui::CursorIcon::ResizeVertical,
                });
            }
        }
        ResizeResponse { response, delta }
    }
}

/// A two-pane split with a draggable divider. Build it, then call [`SplitView::show`].
///
/// ```
/// # use fluent2_egui::data::split_view::{SplitAxis, SplitState, SplitView};
/// # fn demo(ui: &mut egui::Ui) {
/// let mut state = SplitState::default();
/// let _response = SplitView::new("editor", SplitAxis::Horizontal)
///     .min_sizes(120.0, 200.0)
///     .label("Resize editor")
///     .show(ui, &mut state, |ui| ui.label("Tree"), |ui| ui.label("Document"));
/// # }
/// ```
pub struct SplitView {
    id: Id,
    axis: SplitAxis,
    minimums: (f32, f32),
    step: f32,
    label: String,
}

impl SplitView {
    /// A split along `axis`. `id_salt` must be unique among the views in the same `Ui`.
    #[must_use]
    pub fn new(id_salt: impl std::hash::Hash + std::fmt::Debug, axis: SplitAxis) -> Self {
        Self {
            id: Id::new(id_salt),
            axis,
            minimums: (DEFAULT_MINIMUM, DEFAULT_MINIMUM),
            step: DEFAULT_STEP,
            label: "Resize panes".to_owned(),
        }
    }

    /// Minimum size of each pane, default 48 pt. When both cannot fit, each gets half.
    #[must_use]
    pub fn min_sizes(mut self, first: f32, second: f32) -> Self {
        self.minimums = (first, second);
        self
    }

    /// Keyboard step, default 8 pt. Shift with an arrow moves ten steps.
    #[must_use]
    pub fn step(mut self, step: f32) -> Self {
        self.step = step;
        self
    }

    /// Accessible name of the divider, e.g. "Resize navigation".
    #[must_use]
    pub fn label(mut self, label: impl Into<String>) -> Self {
        self.label = label.into();
        self
    }

    /// Lay out both panes and the divider. Drag the divider, or focus it and use the arrow
    /// keys (Shift for larger steps), Home or End to reach the minimum or maximum, or
    /// double-click to restore the size the state had on its first frame.
    pub fn show<A, B>(
        self,
        ui: &mut Ui,
        state: &mut SplitState,
        first: impl FnOnce(&mut Ui) -> A,
        second: impl FnOnce(&mut Ui) -> B,
    ) -> SplitResponse<A, B> {
        let theme = Theme::get(ui.ctx());
        let id = ui.make_persistent_id(self.id);
        if !state.size.value().is_finite() {
            state.size = SplitSize::Ratio(0.5);
        }
        let initial_key = id.with("initial_size");
        let initial = ui
            .ctx()
            .data_mut(|data| *data.get_temp_mut_or(initial_key, state.size));

        let mut resized = false;
        let rect = ui.available_rect_before_wrap();
        let layout = split_rect(rect, self.axis, state.size, DIVIDER_GAP, self.minimums);
        let anchor = if rect.min.is_finite() {
            rect.min
        } else {
            Pos2::ZERO
        };
        let (first_rect, second_rect) = match layout {
            Some(layout) => (layout.first, layout.second),
            None => {
                let empty = Rect::from_min_size(anchor, Vec2::ZERO);
                (empty, empty)
            }
        };
        let first_out = ui
            .scope_builder(UiBuilder::new().max_rect(first_rect), first)
            .inner;
        let second_out = ui
            .scope_builder(UiBuilder::new().max_rect(second_rect), second)
            .inner;

        let divider = match layout {
            Some(layout) => {
                ui.allocate_rect(rect, Sense::hover());
                let (low, high) = layout.bounds(self.minimums);
                let first_size = layout.first_size();
                ui.painter()
                    .rect_filled(layout.divider, 0.0, c(theme.color.neutral_stroke_1));
                let style = ResizeHandleStyle {
                    hovered: c(theme.color.neutral_stroke_1_hover),
                    dragged: c(theme.color.brand_stroke_1),
                    focused: c(theme.color.neutral_stroke_1),
                };
                let handle =
                    ResizeHandle::new(id.with("divider"), self.axis, layout.divider, self.label, style)
                        .hit_margin(DIVIDER_HIT_MARGIN)
                        .keyboard_step(self.step)
                        .value(first_size, low..=high);
                let out = handle.show(ui);
                let response = out.response;
                let mut next = state.size;
                if response.double_clicked() {
                    next = initial;
                } else {
                    let mut delta = out.delta;
                    if response.has_focus() {
                        let (home, end) = ui.input_mut(|input| {
                            (
                                input.consume_key(Modifiers::NONE, Key::Home),
                                input.consume_key(Modifiers::NONE, Key::End),
                            )
                        });
                        if home {
                            delta = low - first_size;
                        } else if end {
                            delta = high - first_size;
                        }
                    }
                    if delta != 0.0
                        && let Some(moved) = layout.resized(state.size, delta, self.minimums)
                    {
                        next = moved;
                    }
                }
                if next != state.size {
                    state.size = next;
                    resized = true;
                }
                if focus::focus_visible(ui, &response) {
                    let hit = match self.axis {
                        SplitAxis::Horizontal => Vec2::new(DIVIDER_HIT_MARGIN, 0.0),
                        SplitAxis::Vertical => Vec2::new(0.0, DIVIDER_HIT_MARGIN),
                    };
                    paint_focus_ring(ui, layout.divider.expand2(hit), CornerRadius::ZERO);
                }
                response
            }
            None => ui.allocate_rect(Rect::from_min_size(anchor, Vec2::ZERO), Sense::hover()),
        };
        SplitResponse {
            first: first_out,
            second: second_out,
            divider,
            resized,
        }
    }
}

/// What a SplitView reports after `show`.
pub struct SplitResponse<A, B> {
    /// Whatever the first pane's closure returned.
    pub first: A,
    /// Whatever the second pane's closure returned.
    pub second: B,
    /// The divider's response.
    pub divider: Response,
    /// The size in `state` changed this frame.
    pub resized: bool,
}

#[cfg(test)]
mod tests {
    use super::*;

    fn rect(width: f32, height: f32) -> Rect {
        Rect::from_min_size(Pos2::ZERO, Vec2::new(width, height))
    }

    /// A layout for input the test knows is valid.
    fn layout(rect: Rect, size: SplitSize, minimums: (f32, f32)) -> SplitLayout {
        match split_rect(rect, SplitAxis::Horizontal, size, 1.0, minimums) {
            Some(layout) => layout,
            None => panic!("split_rect returned None for valid input"),
        }
    }

    #[test]
    fn ratio_splits_the_space_after_the_divider() {
        let split = layout(rect(201.0, 10.0), SplitSize::Ratio(0.5), (0.0, 0.0));
        assert_eq!(split.first_size(), 100.0);
        assert_eq!(split.divider.width(), 1.0);
    }

    #[test]
    fn minimums_are_respected_when_dragging() {
        let split = layout(rect(400.0, 10.0), SplitSize::FixedFirst(200.0), (48.0, 48.0));
        let far_left = split.resized(SplitSize::FixedFirst(200.0), -1000.0, (48.0, 48.0));
        assert_eq!(far_left, Some(SplitSize::FixedFirst(48.0)));
        let far_right = split.resized(SplitSize::FixedFirst(200.0), 1000.0, (48.0, 48.0));
        // 400 points less the 1-point divider, less the 48-point second minimum.
        assert_eq!(far_right, Some(SplitSize::FixedFirst(351.0)));
    }

    #[test]
    fn both_minima_relax_to_half_in_tiny_rects() {
        // 49 points of space after the divider: each pane gets half.
        let split = layout(rect(50.0, 10.0), SplitSize::Ratio(0.9), (48.0, 48.0));
        assert_eq!(split.first_size(), 24.5);
    }

    #[test]
    fn non_finite_and_inverted_input_never_panics() {
        let good = rect(100.0, 100.0);
        let vertical = |size, gap, minimums| split_rect(good, SplitAxis::Vertical, size, gap, minimums);
        assert!(vertical(SplitSize::Ratio(f32::NAN), 1.0, (0.0, 0.0)).is_none());
        assert!(vertical(SplitSize::FixedSecond(f32::INFINITY), 1.0, (0.0, 0.0)).is_none());
        assert!(vertical(SplitSize::Ratio(0.5), f32::NAN, (0.0, 0.0)).is_none());
        assert!(vertical(SplitSize::Ratio(0.5), 1.0, (-1.0, 0.0)).is_none());
        let inverted = Rect::from_min_max(Pos2::new(10.0, 0.0), Pos2::ZERO);
        assert!(
            split_rect(
                inverted,
                SplitAxis::Horizontal,
                SplitSize::Ratio(0.5),
                1.0,
                (0.0, 0.0)
            )
            .is_none()
        );
        let split = layout(good, SplitSize::Ratio(0.5), (0.0, 0.0));
        assert!(
            split
                .resized(SplitSize::Ratio(0.5), f32::NAN, (0.0, 0.0))
                .is_none()
        );
        assert!(
            split
                .resized(SplitSize::Ratio(f32::NAN), 1.0, (0.0, 0.0))
                .is_none()
        );
    }

    #[test]
    fn fixed_second_keeps_its_mode() {
        let split = layout(rect(301.0, 10.0), SplitSize::FixedSecond(100.0), (0.0, 0.0));
        assert_eq!(split.first_size(), 200.0);
        assert_eq!(
            split.resized(SplitSize::FixedSecond(100.0), 10.0, (0.0, 0.0)),
            Some(SplitSize::FixedSecond(90.0))
        );
    }
}
