//! Resize zones for undecorated windows (spec §2, WI-1560).

use egui::{
    Area, CursorIcon, Id, Order, Pos2, Rect, Sense, ViewportCommand, vec2, viewport::ResizeDirection,
};

/// Width of each resize zone, in points, measured inward from the window edge (spec §2).
///
/// Private until the chrome metrics decision (spec D2, §11) moves it onto `Theme`.
const RESIZE_ZONE_PT: f32 = 6.0;

/// Call once per frame in an undecorated window.
///
/// Creates eight resize zones (four edges, four corners) along the window's inner rect. Hovering
/// a zone sets the matching resize cursor, and a primary press sends
/// `ViewportCommand::BeginResize`. Nothing is created while the window is maximised or
/// fullscreen.
///
/// ```no_run
/// # fn ui(ctx: &egui::Context) {
/// fluent2_egui::office::window_edges::window_resize_edges(ctx);
/// # }
/// ```
pub fn window_resize_edges(ctx: &egui::Context) {
    let (maximized, fullscreen) = ctx.input(|input| {
        let viewport = input.viewport();
        (viewport.maximized, viewport.fullscreen)
    });
    if !resize_enabled(maximized, fullscreen) {
        return;
    }
    let inner = ctx.content_rect();
    let t = RESIZE_ZONE_PT;
    let (min, max) = (inner.min, inner.max);
    let mid_x = Rect::from_min_max(Pos2::new(min.x + t, min.y), Pos2::new(max.x - t, min.y + t));
    let mid_y = Rect::from_min_max(Pos2::new(min.x, min.y + t), Pos2::new(min.x + t, max.y - t));
    // Corners last, so they are the topmost areas where two zones overlap.
    let zones = [
        ("north", ResizeDirection::North, mid_x),
        (
            "south",
            ResizeDirection::South,
            shift_y(mid_x, inner.height() - t),
        ),
        ("west", ResizeDirection::West, mid_y),
        ("east", ResizeDirection::East, shift_x(mid_y, inner.width() - t)),
        ("north_west", ResizeDirection::NorthWest, square(min, t)),
        (
            "north_east",
            ResizeDirection::NorthEast,
            square(Pos2::new(max.x - t, min.y), t),
        ),
        (
            "south_west",
            ResizeDirection::SouthWest,
            square(Pos2::new(min.x, max.y - t), t),
        ),
        (
            "south_east",
            ResizeDirection::SouthEast,
            square(Pos2::new(max.x - t, max.y - t), t),
        ),
    ];
    for (name, direction, rect) in zones {
        zone(ctx, name, direction, rect);
    }
}

/// One zone: an invisible foreground area that owns its 6 pt strip.
fn zone(ctx: &egui::Context, name: &'static str, direction: ResizeDirection, rect: Rect) {
    Area::new(Id::unique(("fluent2_resize_zone", name)))
        .order(Order::Foreground)
        .fixed_pos(rect.min)
        .movable(false)
        .interactable(true)
        .show(ctx, |ui| {
            // Pointer only: resize zones are not Tab stops. Focusable, they were eight silent
            // stops ahead of the QAT for screen readers (WI-1586 Orca walk).
            let (_, response) = ui.allocate_exact_size(rect.size(), Sense::CLICK | Sense::DRAG);
            if response.hovered() {
                ctx.set_cursor_icon(cursor_for(direction));
                if ctx.input(|input| input.pointer.primary_pressed()) {
                    ctx.send_viewport_cmd(ViewportCommand::BeginResize(direction));
                }
            }
        });
}

fn cursor_for(direction: ResizeDirection) -> CursorIcon {
    match direction {
        ResizeDirection::North | ResizeDirection::South => CursorIcon::ResizeVertical,
        ResizeDirection::East | ResizeDirection::West => CursorIcon::ResizeHorizontal,
        ResizeDirection::NorthWest | ResizeDirection::SouthEast => CursorIcon::ResizeNwSe,
        ResizeDirection::NorthEast | ResizeDirection::SouthWest => CursorIcon::ResizeNeSw,
    }
}

fn square(min: Pos2, size: f32) -> Rect {
    Rect::from_min_size(min, vec2(size, size))
}

fn shift_x(rect: Rect, dx: f32) -> Rect {
    rect.translate(vec2(dx, 0.0))
}

fn shift_y(rect: Rect, dy: f32) -> Rect {
    rect.translate(vec2(0.0, dy))
}

/// Whether resize zones exist for a window in this state.
///
/// Maximised and fullscreen windows have no zones and no resize cursors.
#[must_use]
fn resize_enabled(maximized: Option<bool>, fullscreen: Option<bool>) -> bool {
    maximized != Some(true) && fullscreen != Some(true)
}

/// The resize direction for `pos` in the `thickness`-wide border of `screen_rect`.
///
/// Corners win over edges: a point in both a horizontal and a vertical band is a corner. Returns
/// `None` for the interior and for points outside `screen_rect`, and for non-finite input.
///
/// ```
/// # use egui::{Pos2, Rect, viewport::ResizeDirection};
/// # use fluent2_egui::office::window_edges::zone_at;
/// let screen = Rect::from_min_size(Pos2::ZERO, egui::vec2(800.0, 600.0));
/// assert_eq!(zone_at(Pos2::new(1.0, 1.0), screen, 6.0), Some(ResizeDirection::NorthWest));
/// assert_eq!(zone_at(Pos2::new(400.0, 300.0), screen, 6.0), None);
/// ```
#[must_use]
pub fn zone_at(pos: Pos2, screen_rect: Rect, thickness: f32) -> Option<ResizeDirection> {
    let inside = pos.x >= screen_rect.min.x
        && pos.x <= screen_rect.max.x
        && pos.y >= screen_rect.min.y
        && pos.y <= screen_rect.max.y;
    if !inside || !thickness.is_finite() {
        return None;
    }
    let thickness = thickness.max(0.0);
    let north = pos.y - screen_rect.min.y < thickness;
    let south = screen_rect.max.y - pos.y < thickness;
    let west = pos.x - screen_rect.min.x < thickness;
    let east = screen_rect.max.x - pos.x < thickness;
    match (north, south, west, east) {
        (true, _, true, _) => Some(ResizeDirection::NorthWest),
        (true, _, _, true) => Some(ResizeDirection::NorthEast),
        (_, true, true, _) => Some(ResizeDirection::SouthWest),
        (_, true, _, true) => Some(ResizeDirection::SouthEast),
        (true, _, _, _) => Some(ResizeDirection::North),
        (_, true, _, _) => Some(ResizeDirection::South),
        (_, _, true, _) => Some(ResizeDirection::West),
        (_, _, _, true) => Some(ResizeDirection::East),
        _ => None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const T: f32 = 6.0;

    fn screen() -> Rect {
        Rect::from_min_size(Pos2::new(10.0, 20.0), egui::vec2(800.0, 600.0))
    }

    fn at(x: f32, y: f32) -> Option<ResizeDirection> {
        zone_at(Pos2::new(x, y), screen(), T)
    }

    #[test]
    fn edges_hit_their_direction() {
        assert_eq!(at(400.0, 22.0), Some(ResizeDirection::North));
        assert_eq!(at(400.0, 615.0), Some(ResizeDirection::South));
        assert_eq!(at(12.0, 300.0), Some(ResizeDirection::West));
        assert_eq!(at(805.0, 300.0), Some(ResizeDirection::East));
    }

    #[test]
    fn corners_hit_their_direction() {
        assert_eq!(at(11.0, 21.0), Some(ResizeDirection::NorthWest));
        assert_eq!(at(805.0, 21.0), Some(ResizeDirection::NorthEast));
        assert_eq!(at(11.0, 615.0), Some(ResizeDirection::SouthWest));
        assert_eq!(at(805.0, 615.0), Some(ResizeDirection::SouthEast));
    }

    #[test]
    fn corner_beats_edge_inside_the_square() {
        // Inside the 6 pt square at the corner, not just the band along one side.
        assert_eq!(at(15.0, 24.0), Some(ResizeDirection::NorthWest));
    }

    #[test]
    fn interior_is_none() {
        assert_eq!(at(400.0, 300.0), None);
        assert_eq!(at(17.0, 27.0), None);
    }

    #[test]
    fn outside_the_window_is_none() {
        assert_eq!(at(9.0, 300.0), None);
        assert_eq!(at(400.0, 19.0), None);
        assert_eq!(at(811.0, 300.0), None);
        assert_eq!(at(400.0, 627.0), None);
    }

    #[test]
    fn non_finite_input_is_none() {
        assert_eq!(zone_at(Pos2::new(f32::NAN, 300.0), screen(), T), None);
        assert_eq!(zone_at(Pos2::new(400.0, 22.0), screen(), f32::INFINITY), None);
    }

    #[test]
    fn maximised_and_fullscreen_disable_zones() {
        assert!(resize_enabled(None, None));
        assert!(resize_enabled(Some(false), Some(false)));
        assert!(!resize_enabled(Some(true), None));
        assert!(!resize_enabled(None, Some(true)));
        assert!(!resize_enabled(Some(true), Some(true)));
    }
}
