# Drawer and BottomSheet

Fluent reference: the React v9 Drawer,
[`@fluentui/react-drawer` 9.9.4](https://github.com/microsoft/fluentui/tree/990e2c65adab04fccc1f9ace3ecae5487ba94348/packages/react-components/react-drawer)
(sizes, the start/end/bottom positions, the transparent-stroke edge, `shadow64` on the overlay).
fluent-gpui `fluent-layout/src/flyout.rs` and `modal.rs` were read for design only; no code was
copied.

## Overlay drawer and bottom sheet

`Drawer::show` and `BottomSheet::show` are modal surfaces attached to a window edge. They are
built on [`egui::Modal`](https://docs.rs/egui/0.36.2/egui/containers/struct.Modal.html), like
[Dialog](dialog.md), and follow the same rules:

- `DialogKind` sets the policy: `Modal` closes on Escape and on a backdrop click, `NonModal` on
  Escape only, `Alert` on neither. The header's Close button (`CLOSE_LABEL`) always closes, as
  `CloseReason::Button("Close")`.
- A popup open before the body owns Escape.
- Focus trap: on opening, focus moves to the surface. Tab and Shift+Tab cycle the Close button,
  the body's widgets and the surface, and never reach the page.
- On close, focus returns to the widget that had it when the surface opened.
- The result is `DialogResult`: `Closed` only on the frame it closes.
- AccessKit role `Dialog` (or `AlertDialog`), modal, labelled by the title.

| Surface | Edge | Extent | Corners |
|---|---|---|---|
| `Drawer`, `DrawerPosition::Start` | left | `DrawerSize`: Small 320, Medium 592, Large 940, Full, or Custom; never wider than the window | square |
| `Drawer`, `DrawerPosition::End` | right | as above | square |
| `BottomSheet` | bottom | `height_fraction` of the window height (0.1 to 1, default 0.5), full width | top corners `borderRadiusXLarge`; grab handle |

## Inline drawer

`Drawer::show_inline(ui, …)` is a side panel inside `ui` that pushes the content beside it. It
does not dim, block or trap focus; Escape does not close it, only the Close button or the app.
AccessKit role `Complementary`, labelled by the title. It has a `colorNeutralStroke2` divider on
the side that faces the page.

## Motion

On opening, the surface slides in from its edge over `durationGentle` on `curveDecelerateMid`.
Under reduced motion (`foundation::motion::set_reduced_motion`) it is in place on the first frame.
Closing is immediate, as for Dialog: the body does not run on the frame the surface closes.
