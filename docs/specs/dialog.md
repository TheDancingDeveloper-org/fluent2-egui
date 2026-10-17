# Dialog

A modal or non-modal surface built on [`egui::Modal`](https://docs.rs/egui/0.36.2/egui/containers/struct.Modal.html).
The egui modal already paints a full-screen backdrop, blocks input to the UI behind it, tracks the
topmost modal layer, and reports whether any popup was open at the start of the frame. Fluent
chrome, focus, keyboard policy and accessibility sit on top of that. A custom area is not used.

## Kinds

| Kind | Backdrop click | Escape | Use |
|---|---|---|---|
| `Modal` (default) | closes (`CloseReason::Backdrop`) | closes (`CloseReason::Escape`) when this dialog is the top modal and no popup is open | ordinary dialog |
| `NonModal` | does not close | same as Modal | a surface that stays open while the user clicks the dimmed page |
| `Alert` | ignored | ignored | a choice the user must make with a button |

`Dialog::show` returns `DialogResult::Closed` only on the frame the dialog closes. While it stays
open the body closure runs and its value is `DialogResult::Open`. Setting `open` to `false` from
outside, or calling `DialogActions::close`, is `CloseReason::Programmatic`.

## Focus trap

On the frame the dialog opens, focus moves to the primary action, or to the dialog surface when
there is no primary action. Tab and Shift+Tab then cycle only among the widgets inside the dialog:
from the last widget Tab wraps to the first, and from the first Shift+Tab wraps to the last. The
surface itself is one stop in that cycle, so a dialog with no other widgets still has a focused
node. Widgets outside the dialog are not reachable while it is open.

When the dialog closes, focus returns to the widget that was focused on the frame it opened (the
opener).

## Enter and the default button

Enter activates the primary action, unless:

- focus is in a text field (`ctx.text_edit_focused()`). The spec asks for multiline fields
  only, but egui does not expose whether a `TextEdit` is multiline, so single-line fields also
  keep Enter. Narrowing this needs a multiline marker from `Textarea`, a separate item, or
  operator sign-off on the broader rule; or
- focus is on another button (that button takes Enter and Space).

Escape follows the kind table above and is also swallowed when a popup is open, so a Dropdown
inside the dialog can close itself first. The next Escape, with no popup open, closes a Modal or
NonModal dialog. Alert never closes on Escape.

## Nested popups

Popups (Dropdown, Menu) opened from inside the dialog use egui's popup layer, which is above the
modal. `egui::Modal` records `any_popup_open` before the body runs. Escape is handled by the
popup and is not treated as a dialog close on that frame. Backdrop clicks do not fall through the
dialog surface to the page.

Egui clears focus on Escape unless the focused widget's event filter asks for it. An open Dropdown
sets `escape` on its focus lock filter and consumes the press, so the Dropdown closes and the dialog
does not see the key. This is the one change this item makes to a control, in `controls/dropdown.rs`.

## Surface

| Part | Token |
|---|---|
| Surface fill | `neutral_background_1` |
| Radius | `borderRadiusXLarge` |
| Shadow | `shadow64` |
| Padding | 24 (`spacingHorizontalXXL` / `spacingVerticalXXL`) |
| Max width | 600 by default (`Dialog::width`) |
| Title | `subtitle1`, `neutral_foreground_1` |
| Actions | right-aligned, gap 8 (`spacingHorizontalS`); below 480 pt of screen width the actions stack vertically, full width |
| Backdrop | `backgroundOverlay` — black at alpha 102, which is 0.4 (`web_light.background_overlay`). High contrast uses the same alias from the high-contrast token set. |

Primary actions use `Button` `Appearance::Primary`. Secondary actions use `Appearance::Secondary`.

## Accessibility

The surface sets AccessKit role `Dialog`, or `AlertDialog` for `DialogKind::Alert`. The accessible
name is the title, via the node's label and a labelled-by relation to the title widget.
