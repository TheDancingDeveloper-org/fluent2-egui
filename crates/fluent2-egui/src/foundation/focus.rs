//! Fluent keyboard focus indicator.

use egui::{CornerRadius, Event, Id, Response, Stroke, StrokeKind, Ui, epaint::Rect};

use crate::theme::{Theme, c};

const KEYBOARD_FOCUS_KEY: &str = "fluent2_keyboard_focus";

/// Paint the Fluent focus indicator around `rect`.
///
/// A 1px `stroke_focus_1` ring sits just outside the control, and a 2px `stroke_focus_2` ring
/// sits outside that.
pub fn paint_focus_ring(ui: &Ui, rect: Rect, radius: CornerRadius) {
    let theme = Theme::get(ui.ctx());
    let painter = ui.painter();
    let inner = rect.expand(1.0);
    let outer = rect.expand(2.0);
    painter.rect_stroke(
        inner,
        grow_radius(radius, 1),
        Stroke::new(1.0, c(theme.color.stroke_focus_1)),
        StrokeKind::Middle,
    );
    painter.rect_stroke(
        outer,
        grow_radius(radius, 2),
        Stroke::new(2.0, c(theme.color.stroke_focus_2)),
        StrokeKind::Middle,
    );
}

/// Whether a focus ring should be shown: the widget has focus and the last input was a key.
///
/// A pointer press clears the sticky keyboard-focus flag stored on the context.
#[must_use]
pub fn focus_visible(ui: &Ui, response: &Response) -> bool {
    let id = keyboard_focus_id();
    let pointer_pressed = ui.input(|input| {
        input
            .events
            .iter()
            .any(|event| matches!(event, Event::PointerButton { pressed: true, .. }))
    });
    let key_pressed = ui.input(|input| {
        input
            .events
            .iter()
            .any(|event| matches!(event, Event::Key { .. }))
    });
    if pointer_pressed {
        ui.ctx().data_mut(|data| data.insert_temp(id, false));
    } else if key_pressed {
        ui.ctx().data_mut(|data| data.insert_temp(id, true));
    }
    let keyboard = ui.ctx().data(|data| data.get_temp::<bool>(id).unwrap_or(false));
    let focused = ui.memory(|memory| memory.focused()) == Some(response.id);
    response.has_focus() && focused && keyboard
}

fn keyboard_focus_id() -> Id {
    Id::NULL.with(KEYBOARD_FOCUS_KEY)
}

fn grow_radius(radius: CornerRadius, by: u8) -> CornerRadius {
    CornerRadius {
        nw: radius.nw.saturating_add(by),
        ne: radius.ne.saturating_add(by),
        sw: radius.sw.saturating_add(by),
        se: radius.se.saturating_add(by),
    }
}
