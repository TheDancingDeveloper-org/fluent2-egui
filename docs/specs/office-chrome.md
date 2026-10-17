# Office chrome spec

Status: draft for WI-1555. Implementation items WI-1559 to WI-1570 cite the section numbers
below; keep the numbering. This document contains no implementation. The only code is public
type and function signatures, written as the contract an implementation must meet.

Conventions are fixed by [`docs/ARCHITECTURE.md`](../ARCHITECTURE.md) (rules 1 to 10). Where this
spec adds a rule, it says so under **Decisions**.

## Decisions

These are the choices this spec makes that the item and ARCHITECTURE leave open. Each one is
reversible by editing this file before WI-1559 starts.

| # | Decision | Why |
|---|---|---|
| D1 | Office components return responses and never send `ViewportCommand`s themselves. The app maps a returned `WindowAction` to a command with a pure function. | ARCHITECTURE rule 5. The mapping is unit-testable without a window. |
| D2 | Chrome dimensions (32, 40, 46, 6 pt and so on) are named constants on a `ChromeMetrics` struct held by `Theme` (`Theme::chrome`). They are not Fluent tokens. | Rule 7 forbids magic numbers outside `theme/`. Fluent has no token for these. **Operator decision, see §11.** |
| D3 | `RibbonState`, `QatState` and `BackstageState` are plain data with public fields. No serde in M1. The app persists them. | Keeps the crate free of a serialisation dependency. The app already owns persistence (rule 5). |
| D4 | Control keys (`ControlKey`) are `&'static str`. Duplicate keys within one ribbon, QAT or command bar do not panic. The first key wins and later ones are reported in the response's `conflicts`. | Rule 9 (no panics). Rule 4 (ids from the caller, not from translated labels): keys are caller-chosen constants. |
| D5 | Ribbon tab and group bodies are built by closures every frame into a data model (`TabModel`), before any painting. Measurement and scaling run on that model. | Lets §5 measure and reduce widths before drawing, in immediate mode. |
| D6 | Accent colours for contextual tabs are token choices (`ContextualAccent`), never a `Color32` argument. | Rule 7. |
| D7 | KeyTips are adapted from wordcraft's state machine only (§6). Rendering is written here. | The state machine is pure logic and small. Rendering depends on this library's painter, so copying it would copy the wrong parts. |
| D8 | The CommandBar overflow algorithm is the one in §8, reusing the hysteresis rule of §5. | One fitting rule across the chrome. |

## Shared vocabulary

- **Pt** means logical points (egui units). Every width and height in this spec is in points.
- **Region** is one of: title bar, QAT, ribbon, content, status bar. F6 cycles regions (§9).
- **Control key** is a `&'static str` supplied by the app. It names a control in events and state.
- **Response** means a returned struct. Components never return tuples and never dispatch.

## 1. TitleBar

**Contract.** The window is undecorated. The app creates it with
`ViewportBuilder::with_decorations(false)`. On macOS it also sets
`with_fullsize_content_view(true)`, and the title bar leaves the traffic-light inset
(`ChromeMetrics::macos_traffic_light_inset`) empty on the left. Windows and Linux use no inset.

**Geometry.**

| Part | Value |
|---|---|
| Height, no QAT and no search | 32 pt |
| Height with QAT or search box | 40 pt |
| Caption button | 46 × 32 pt, close and hover on the same rule |
| Caption glyphs | Minimise, maximise, restore, close (`Icon` names from the icon catalogue; WI-1534) |
| Close hover | background `color.status_danger_background_3`, foreground `color.neutral_foreground_on_brand` |
| Other caption hover | background `color.subtle_background_hover` |
| Caption pressed | background `color.subtle_background_pressed` (close: `status_danger_background_3_pressed`) |
| Inactive window | title and caption glyphs use `color.neutral_foreground_3`; background unchanged |

**Slots.** Left to right: app icon, QAT (§3, when placed in the title bar), centred title or
search box, account area on the right, then the caption buttons.

**Signatures.**

```rust
/// Which window action the user asked for this frame. The app maps it with `window_command`.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum WindowAction {
    Minimize,
    /// Toggle between maximised and restored.
    ToggleMaximize,
    /// Begin a window drag (the user pressed on a drag region and moved).
    StartDrag,
    Close,
}

pub struct TitleBar<'a> { /* builder fields private */ }

pub struct TitleBarSlots<'a, 'b> { /* private; methods below */ }

pub struct TitleBarResponse {
    /// The action requested this frame, if any.
    pub action: Option<WindowAction>,
    /// Whether the title bar drag region was double-clicked.
    pub double_clicked: bool,
    /// The region not covered by slots or caption buttons.
    pub drag_region: egui::Response,
    /// Response for each caption button.
    pub minimize: egui::Response,
    pub maximize: egui::Response,
    pub close: egui::Response,
    /// Whether the window is inactive (`ViewportInfo::focused == Some(false)`).
    pub inactive: bool,
}

impl<'a> TitleBar<'a> {
    /// `id` is caller-supplied. `title` is shown in the centre unless a search box is set.
    pub fn new(id: impl std::hash::Hash, title: impl Into<egui::WidgetText>) -> Self;
    /// Whether the window is maximised. The maximise button shows *restore* when true.
    pub fn maximized(self, maximized: bool) -> Self;
    /// `ViewportInfo::focused`, passed in by the app so the component stays window-agnostic.
    pub fn focused(self, focused: Option<bool>) -> Self;
    /// Adds 8 pt to the height and shows a centred search box slot instead of the title text.
    pub fn with_search(self, enabled: bool) -> Self;
    /// `add` fills the slots. It runs once per frame.
    pub fn show<R>(self, ui: &mut egui::Ui, add: impl FnOnce(&mut TitleBarSlots<'_, '_>) -> R)
        -> (TitleBarResponse, R);
}

impl TitleBarSlots<'_, '_> {
    pub fn app_icon(&mut self, icon: Icon);
    pub fn left(&mut self, add: impl FnOnce(&mut egui::Ui));
    pub fn center(&mut self, add: impl FnOnce(&mut egui::Ui));
    pub fn right(&mut self, add: impl FnOnce(&mut egui::Ui));
}

/// Pure mapping from a request to the viewport command the app sends (D1).
pub fn window_command(action: WindowAction, maximized: bool) -> egui::ViewportCommand;
```

**Behaviour.**

- Drag: press on `drag_region`, then movement sends `StartDrag`. Use `drag_started` so a click
  without movement does nothing.
- Double-click on `drag_region` gives `double_clicked` and `ToggleMaximize`.
- `window_command` maps `Minimize` to `ViewportCommand::Minimized(true)`, `ToggleMaximize` to
  `ViewportCommand::Maximized(!maximized)`, `StartDrag` to `ViewportCommand::StartDrag`, and
  `Close` to `ViewportCommand::Close`.
- Caption buttons are keyboard reachable (§9). Each has an accessible label: "Minimise",
  "Maximise", "Restore" (when maximised), "Close".

## 2. Window resize edges

For undecorated windows only. Edges and corners are hit zones; the app gets a direction and
starts the native resize.

**Geometry.** A 6 pt hit zone along each of the four edges and a 6 × 6 pt corner square.
Corners win over edges. Zones sit inside the window rectangle, so they do not overlap the
caption buttons (which are at the top-right, §1). The top edge zone is 6 pt high and is not
drawn; the title bar drag region takes precedence in its own rectangle.

**Cursors.** Edges use the horizontal, vertical or diagonal resize cursor for their direction.
Diagonal corners use the matching diagonal cursor.

**Suppression.** When the window is maximised, no zones are active and no cursors change.

**Signatures.**

```rust
/// Returns the resize direction under the pointer, if any, and sets the cursor (a UI concern,
/// not a window command). The app sends `ViewportCommand::BeginResize(direction)` on press.
pub fn resize_zone(ui: &egui::Ui, window_rect: egui::Rect, maximized: bool)
    -> Option<egui::viewport::ResizeDirection>;
```

`egui::viewport::ResizeDirection` is the type `ViewportCommand::BeginResize` takes. WI-1560
confirms the path against the pinned egui 0.36.

**Behaviour.** The zone is checked before the title bar drag region on press, so resizing from
the top corners works. Pointer events inside a zone are not passed to content underneath.

## 3. Quick Access Toolbar (QAT)

Icon-only buttons. Each is a `Button::icon_only` (WI-1542) with a tooltip (WI-1551). The QAT
is placed in the title bar (§1, height 40 pt) or below or above the ribbon, chosen by the app.

**Model.** The app owns the item order and visibility. The QAT never reorders by itself.

```rust
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct QatState {
    /// Keys of visible items, in display order.
    pub items: Vec<&'static str>,
    pub position: QatPosition,
    /// Whether the customise menu is open.
    pub customise_open: bool,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Default)]
pub enum QatPosition {
    /// Inside the title bar (default).
    #[default]
    TitleBar,
    /// Row above the ribbon tab strip.
    AboveRibbon,
    /// Row below the ribbon.
    BelowRibbon,
}

/// One entry the app offers. `key` is stable across sessions.
pub struct QatItem<'a> {
    pub key: &'static str,
    pub icon: Icon,
    pub label: &'a str,
    pub tooltip: &'a str,
}

pub struct Qat<'a> { /* private */ }

pub struct QatResponse {
    /// The key of the item clicked this frame.
    pub clicked: Option<&'static str>,
    /// Customise-menu actions this frame.
    pub customise: Option<QatCustomise>,
    /// Keys that appear in `items` but not in the offered list, and were dropped.
    pub conflicts: Vec<&'static str>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum QatCustomise {
    Show(&'static str, bool),
    MoveUp(&'static str),
    MoveDown(&'static str),
    Position(QatPosition),
}

impl<'a> Qat<'a> {
    pub fn new(id: impl std::hash::Hash, state: &'a mut QatState, offered: &'a [QatItem<'a>]) -> Self;
    pub fn show(self, ui: &mut egui::Ui) -> QatResponse;
}
```

**Customise menu.** Opened from a trailing `…` icon button. Lists every offered item with a
checkbox (`Show`), and up and down buttons on each row (`MoveUp`, `MoveDown`). Its last section
is the position choice (`Position`). It is a `Menu` (WI-1545) with `MenuItem::checked`.

**Rules.**
- The app applies `QatCustomise` to `QatState`. The component mutates nothing except
  `customise_open`.
- A click on an item returns its key; the app decides what it does (rule 5).
- An item the app stops offering is removed from `items` on the next frame and reported in
  `conflicts`.

## 4. Ribbon model and API

### 4.1 Model

Closures run once per frame and fill a `TabModel` (D5). No drawing happens in the closure.

```rust
pub struct Ribbon<'a> { /* private: tab specs with boxed closures */ }

pub struct TabBuilder<'t> { /* private */ }
pub struct GroupBuilder<'g> { /* private */ }

impl<'a> Ribbon<'a> {
    pub fn new(id: impl std::hash::Hash) -> Self;
    /// The File tab. It opens the backstage (§7) and is never a content tab.
    pub fn file_tab(self, label: &str) -> Self;
    /// A content tab. `key` is stable; `build` fills the groups.
    pub fn tab(self, key: &'static str, label: &str, build: impl FnOnce(&mut TabBuilder<'_>) + 'a) -> Self;
    /// A contextual tab set. Its tabs appear only while `visible` is true, with `accent`.
    pub fn contextual(self, set: &'static str, label: &str, accent: ContextualAccent, visible: bool,
        build: impl FnOnce(&mut TabBuilder<'_>) + 'a) -> Self;
    pub fn show(self, ui: &mut egui::Ui, state: &mut RibbonState) -> RibbonResponse;
}

impl TabBuilder<'_> {
    /// A group. `label` is shown under the group. `build` fills the controls.
    pub fn group(&mut self, key: &'static str, label: &str, build: impl FnOnce(&mut GroupBuilder<'_>));
}

impl GroupBuilder<'_> {
    /// Icon above, label below. Large is the default scale step (§5).
    pub fn large(&mut self, key: &'static str, icon: Icon, label: &str);
    /// Icon and label side by side.
    pub fn small(&mut self, key: &'static str, icon: Icon, label: &str);
    /// A toggle. `value` is the app's state, passed by `&mut`.
    pub fn toggle(&mut self, key: &'static str, icon: Icon, label: &str, value: &mut bool);
    /// A main action and a menu. `menu` fills a popup as in `SplitButton::show`.
    pub fn split(&mut self, key: &'static str, icon: Icon, label: &str,
        menu: impl FnOnce(&mut egui::Ui) + 'a);
    /// A button that opens a menu (`MenuButton`).
    pub fn menu(&mut self, key: &'static str, icon: Icon, label: &str,
        menu: impl FnOnce(&mut egui::Ui) + 'a);
    pub fn gallery(&mut self, key: &'static str, label: &str, items: &'a [GalleryItem<'a>],
        selected: &'a mut Option<usize>);
    /// A combobox (`Combobox`) with the given options, bound to `selected`.
    pub fn combobox(&mut self, key: &'static str, label: &str, options: &'a [&'a str],
        text: &'a mut String);
    /// A dropdown (`Dropdown`) with the given options, bound to `selected`.
    pub fn dropdown(&mut self, key: &'static str, label: &str, options: &'a [&'a str],
        selected: &'a mut usize);
    /// A checkbox row, bound to `value`.
    pub fn checkbox(&mut self, key: &'static str, label: &str, value: &mut bool);
    /// The dialog-launcher button at the group's bottom-right corner.
    pub fn dialog_launcher(&mut self, key: &'static str);
    /// Any widget. Its `Response` is returned to the app through `RibbonResponse::custom`.
    pub fn custom(&mut self, key: &'static str, add: impl FnMut(&mut egui::Ui) -> egui::Response + 'a);
}

pub struct GalleryItem<'a> {
    pub key: &'static str,
    pub label: &'a str,
    pub icon: Icon,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ContextualAccent {
    Brand,
    Success,
    Warning,
    Danger,
    Important,
}
```

Notes on the signatures:
- Lifetimes: `build` closures and bound state are borrowed for `'a`, the frame. Apps build the
  ribbon each frame, as with any egui widget.
- `combobox` and `dropdown` bind to app state directly. They are the existing `Combobox` and
  `Dropdown` controls (WI-1549); the ribbon adds only the group layout.
- `checkbox` is the existing `Checkbox` control (WI-1546).

### 4.2 Control kinds and what they map to

| Kind | Built on | Notes |
|---|---|---|
| Large | Own painter in `office/ribbon/group.rs`, using `Button` colours and tokens | Icon 32 pt, label below. Two lines maximum; then truncates with ellipsis. |
| Small | `Button` (WI-1542), `ButtonSize::Small`, `Appearance::Subtle` | Icon 16 pt, label beside. |
| Toggle | `ToggleButton` (WI-1543) | Small or Large form, selected state follows `value`. |
| Split | `SplitButton` (WI-1544) | Returns `SplitButtonResponse`; the ribbon reports `main` and `menu` as events. |
| Menu | `MenuButton` (WI-1544) | Popup content is `surfaces::menu` (WI-1545). |
| Gallery | Own painter, §4.4 | Inline row plus expanded grid. |
| Combobox, Dropdown | WI-1549 | Width from the group's scale step. |
| Checkbox | WI-1546 | Label beside. |
| Custom | Caller's widget | Returned as a `Response` in `RibbonResponse::custom`. |

### 4.3 Contextual tabs

- Contextual tab sets are added after the content tabs and are visible only while `visible` is
  true. When they appear, the app may set `RibbonState::active` to one of them.
- A contextual set has a header label above its tabs, drawn in its accent colour (D6). The
  accent colour is a theme token: `Brand` → `color.brand_foreground_1`, `Success` →
  `color.status_success_foreground_1`, `Warning` → `color.status_warning_foreground_1`,
  `Danger` → `color.status_danger_foreground_1`, `Important` → `color.brand_foreground_1` with
  the set's own label.
- When a contextual set disappears while one of its tabs is active, the active tab becomes the
  first content tab. This is a state change, not a response.

### 4.4 Galleries

- Inline row: shows up to the number of items that fit the group's width at its current step.
  Two arrow buttons scroll one row at a time. Each item is a 48 × 48 pt tile.
- Expanded grid: a drop-down button at the row's right edge opens a popup grid. Its width is the
  group width. The popup closes on selection or on Escape, and returns focus to the gallery.
- A gallery item click gives `GalleryPicked(index)` and sets `selected`.

### 4.5 Collapse and pin

`RibbonState` holds the display state, which is a table (see also §9 for keys):

```rust
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct RibbonState {
    /// The active content or contextual tab, by key. `None` means the first content tab.
    pub active: Option<&'static str>,
    pub display: RibbonDisplay,
    /// Whether the open ribbon stays open after a click outside it (pinned).
    pub pinned: bool,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Default)]
pub enum RibbonDisplay {
    /// Tab strip and ribbon body visible.
    #[default]
    Expanded,
    /// Only the tab strip is visible.
    Collapsed,
    /// Collapsed, with the ribbon body open as a popup over the content, for one tab.
    /// The popup closes on click outside unless `pinned` is set.
    Peek,
}
```

| From | Event | To | Notes |
|---|---|---|---|
| Expanded | Ctrl+F1, or the collapse button | Collapsed | `pinned` cleared |
| Collapsed | Ctrl+F1 | Expanded | |
| Collapsed | Click a content tab | Peek | `active` set to the tab |
| Peek | Click outside, or Escape | Collapsed | |
| Peek | Pin button | Expanded | `pinned` set |
| Expanded | Click outside | Expanded | unchanged |

- Clicking a tab in Expanded state only changes `active`.
- A pinned ribbon in Peek state is the same as Expanded.

### 4.6 RibbonResponse

```rust
pub struct RibbonResponse {
    /// Events from controls this frame, in visual order.
    pub events: Vec<RibbonEvent>,
    /// The tab that became active this frame, if it changed.
    pub tab_changed: Option<&'static str>,
    /// Keys used more than once in this ribbon. The first occurrence keeps its events (D4).
    pub conflicts: Vec<&'static str>,
    /// Responses of `custom` widgets, keyed by their key.
    pub custom: Vec<(&'static str, egui::Response)>,
}

#[derive(Clone, Debug, PartialEq)]
pub struct RibbonEvent {
    pub key: &'static str,
    pub kind: RibbonEventKind,
}

#[derive(Clone, Debug, PartialEq)]
pub enum RibbonEventKind {
    Clicked,
    /// A split button's menu item or a menu item was clicked (the item's own key).
    MenuItem(&'static str),
    /// A toggle or checkbox changed; the new value is also in the bound `&mut bool`.
    Changed(bool),
    /// A combobox, dropdown or gallery changed; the index or text is in the bound state.
    Selected(usize),
    DialogLauncher,
}
```

**Rules.**
- `RibbonResponse` only reports. The app maps `key` to its command (rule 5).
- Events for a control that is disabled (`ui.add_enabled(false, …)`) are never produced.
- Keys in `conflicts` are duplicates within the ribbon. The first occurrence keeps its events.

## 5. Ribbon layout and adaptive scaling

### 5.1 Scale steps

Each group renders at one of four steps, from widest to narrowest:

| Step | Rendering | Example group |
|---|---|---|
| `Large` | Large buttons with labels below icons | Clipboard: Paste large |
| `Medium` | Small buttons with labels beside icons | Clipboard: Paste small with label |
| `Small` | Icon only, labels in tooltips | Clipboard: Paste icon only |
| `Collapsed` | One button showing the group's dialog icon or first control; opens a popup with the full group | Any group |

```rust
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub enum ScalingStep { Large, Medium, Small, Collapsed }

/// The steps a group may use, in order. Groups are never stepped beyond `Collapsed`.
pub struct ScalingSteps { /* the four steps, always in this order */ }
```

Every group has all four steps. The group declares its widths from the controls it holds; the
ribbon measures them.

### 5.2 Reduction order

Input: available width `W` (the tab body width minus margins), the active tab's groups in
visual order, and each group's current step and cached width at each step.

1. Start every group at `Large`.
2. While the sum of group widths (plus 8 pt separators between groups) is greater than `W`:
   - Take the **rightmost** group that is not yet at `Collapsed`.
   - Step it one level down (`Large` → `Medium` → `Small` → `Collapsed`).
3. The tie-break: the reduction always goes to the rightmost group. Two groups at the same step
   never change in any other order.
4. Stop when the sum fits, or when every group is `Collapsed`.

Growth (when space returns) runs the other way: the **leftmost** group that is not at `Large`
steps up one level, and only when the available width is at least the new sum plus the
hysteresis margin (§5.3).

Overflow: if every group is `Collapsed` and the sum still exceeds `W`, the rightmost groups are
moved into a trailing `…` menu button. Its popup lists the moved groups as sub-menus, each
containing that group's controls. The ribbon height never changes.

### 5.3 Hysteresis

- A group steps up only when `W` is at least the new total plus **8 pt**.
- A group steps down as soon as the total exceeds `W`.
- Within an 8 pt band, the state does not change. This prevents oscillation at a resize edge.

### 5.4 Measurement

- Each group's width at each step is measured on the frame it is rendered at that step, and
  stored in `ctx.data` keyed by the ribbon id and tab key, then the group key and step. The
  store entry is `(width: f32, content_hash: u64)`.
- `content_hash` is a hash of the group's control keys and labels. A change in contents
  invalidates the entry; the group's widths are re-measured on the next frame that renders it.
- On the first frame of a tab, with no cache, every group is laid out at `Collapsed` and a
  repaint is requested. The groups grow in the following frames. The first frame is therefore
  never wider than `W`.
- Steps the group does not currently render keep their previous cached width.

### 5.5 Worked example

Available width `W = 900 pt`. Four groups, widths per step (`L`, `M`, `S`, `C`):

| Group | L | M | S | C |
|---|---|---|---|---|
| Clipboard | 200 | 160 | 120 | 36 |
| Font | 320 | 260 | 230 | 36 |
| Paragraph | 280 | 240 | 200 | 36 |
| Styles | 360 | 300 | 260 | 36 |

Separators: 3 × 8 = 24 pt.

1. All `Large`: 200 + 320 + 280 + 360 + 24 = **1184 pt**. Above 900, so reduce.
2. Rightmost not `Collapsed`: Styles. `L → M`: 1184 − (360 − 300) = **1124**. Still above 900.
3. Styles `M → S`: 1124 − (300 − 260) = **1084**. Still above 900.
4. Styles `S → C`: 1084 − (260 − 36) = **860**. Fits (860 ≤ 900).

Result: Clipboard L, Font L, Paragraph L, Styles C. Total 860 pt.

Growth with the same state:

- If `W` becomes 1000 pt, Styles would need `S` at 1084 + 8 = 1092. 1000 is less, so it stays
  `C`.
- If `W` becomes 1100 pt, Styles would step to `S` at a total of 1084; 1100 ≥ 1084 + 8 = 1092.
  It steps to `S`.
- At 1090 pt, it stays `C`: 1090 is in the hysteresis band (1084 to 1092). This is the
  oscillation case the band exists to stop.

## 6. KeyTips

### 6.1 Activation and display

- Alt, pressed and released with no other key in between, activates KeyTips at the root level.
- F10 does the same and also toggles KeyTips off when already active.
- Badges are small rounded rectangles (`ChromeMetrics::keytip_badge`, 16 × 16 pt for one
  letter, grown to fit two letters) placed at the top-left of the target control's rectangle,
  offset 2 pt. Tab badges go under the tab label.
- Badge text is the `Caption1` ramp, medium weight. Colours: background
  `color.neutral_background_1`, text `color.neutral_foreground_1`, border
  `color.neutral_stroke_1`. The active sequence's prefix is shown in `color.brand_foreground_1`.

### 6.2 Keys

- **App-supplied**: a control may carry `.keytip("H")` (in the builder), or the app passes keys
  in its QAT item and in its own keytip list. Supplied keys are used as given, uppercased.
- **Auto-assigned**: the first letter of the label, uppercased, that is not already used at the
  same level. If none is free, the next letters of the label are tried, then `A` to `Z`, then
  digits `1` to `9`.
- **Multi-letter**: when a level has more than 26 targets, keys become two letters. Two-letter
  keys are assigned in order, so the first letter selects a group of targets and the second
  selects one. Such keys are rare. The spec does not require them in M1 beyond the state
  machine supporting them.
- **Conflicts**: two targets at the same level with the same key. The first keeps the key. The
  second is re-assigned by the auto rule. Conflicts are reported in `KeyTipsReport::conflicts`.
  Nothing panics.

### 6.3 State machine

```rust
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum KeyTipState {
    /// KeyTips are not shown. Alt or F10 leaves this state.
    Idle,
    /// Root level: tabs, QAT and app-supplied keys are shown.
    Root,
    /// A content tab was chosen. Its controls are shown. `tab` is its key.
    Tab { tab: &'static str },
    /// A menu or gallery popup opened from a control is shown. `control` is its key.
    Popup { tab: &'static str, control: &'static str },
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum KeyTipInput {
    /// A letter or digit key, or a two-letter sequence's second key.
    Char(char),
    Escape,
    Backspace,
    /// Alt or F10 pressed.
    Activate,
    /// Mouse click anywhere, or focus moved to a region outside the chrome.
    Dismiss,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum KeyTipAction {
    None,
    /// Show the keytips for a level. The app shows the target list.
    Show,
    /// Select a tab (switches to it and shows its controls).
    SelectTab(&'static str),
    /// Invoke a control. The app receives this as a normal click for the key.
    Invoke(&'static str),
    /// Open a control's popup (menu, gallery).
    OpenPopup(&'static str),
}

/// Pure transition. Returns the next state and the action for the app.
pub fn step(state: &KeyTipState, input: KeyTipInput, targets: &KeyTipTargets)
    -> (KeyTipState, KeyTipAction);
```

Transitions:

| State | Input | Next state | Action |
|---|---|---|---|
| Idle | Activate | Root | Show |
| Idle | other | Idle | None |
| Root | Char matching a tab | Tab(tab) | SelectTab |
| Root | Char matching a QAT or app key | Idle | Invoke |
| Root | Char no match | Root | None |
| Root | Escape | Idle | None |
| Root | Backspace | Idle | None |
| Root | Activate | Idle | None |
| Root | Dismiss | Idle | None |
| Tab(t) | Char matching a control | Idle | Invoke |
| Tab(t) | Char matching a menu or gallery control | Popup(t, c) | OpenPopup |
| Tab(t) | Char no match | Tab(t) | None |
| Tab(t) | Escape or Backspace | Root | Show |
| Tab(t) | Activate or Dismiss | Idle | None |
| Popup(t, c) | Char matching an item | Idle | Invoke (the item key) |
| Popup(t, c) | Escape or Backspace | Tab(t) | Show |
| Popup(t, c) | Activate or Dismiss | Idle | None |

Two-letter sequences: a first letter that is a prefix of a two-letter key moves the state to a
sub-state inside the same level. The next `Char` completes it. Escape cancels a partial
sequence to the level it started from. The table above shows single-letter transitions.

`KeyTipTargets` is the list of `(key, level, target)` the app built this frame, derived from the
ribbon model (§4) and the QAT (§3), so the state machine has no egui dependency.

### 6.4 Which keytips the app supplies

The app supplies keys for: QAT items (they are its commands), File tab, Save and Undo in the
QAT, and any control whose auto key would be misleading. Everything else is auto-assigned.

### 6.5 Upstream: adapt or write

**Recommendation: adapt the state machine from wordcraft.** `crates/ui-egui/src/keytips.rs`
at the rev pinned in `upstream.toml` (`wordcraft`, currently `cf8aea6`) is the reference. The
state machine is pure logic under MIT OR Apache-2.0. The rendering is rewritten (D7).

If WI-1566 adapts it, WI-1566 follows `UPSTREAM.md`: it adds the file header, an
`[[adaptation]]` entry in `upstream.toml`, and changes wordcraft's role to `adapted`. Its
`ours` path is `crates/fluent2-egui/src/office/keytips.rs`.

This spec did not compare the wordcraft code line by line. WI-1566 checks the transition table
in §6.3 against that file and records any difference.

## 7. Backstage

A full-window view that replaces the ribbon and the content area while open.

**Layout.** Left navigation column, 240 pt wide. At its top, a back arrow (`Button::icon_only`,
accessible label "Back"). Below it, page entries, each a 40 pt tall row (`MenuItem`-style,
selected row uses `color.subtle_background_selected` and a 3 pt brand bar at the left). At the
bottom, bottom-group entries (Options, Account), separated by a `Divider`. The page area fills
the rest, with 24 pt padding.

```rust
#[derive(Clone, Debug, PartialEq, Eq, Default)]
pub struct BackstageState {
    pub open: bool,
    /// Active page key. `None` means the first page.
    pub page: Option<&'static str>,
}

pub struct Backstage<'a> { /* private */ }

pub struct BackstageResponse {
    /// True on the frame the backstage was closed by the back arrow or Escape (§7).
    pub closed: bool,
    /// The page that became active this frame, if it changed.
    pub page_changed: Option<&'static str>,
}

impl<'a> Backstage<'a> {
    pub fn new(state: &'a mut BackstageState) -> Self;
    /// A page in the main group. `add` draws its content. Returns self for chaining.
    pub fn page(self, key: &'static str, label: &str, add: impl FnOnce(&mut egui::Ui) + 'a) -> Self;
    /// A page in the bottom group.
    pub fn bottom_page(self, key: &'static str, label: &str, add: impl FnOnce(&mut egui::Ui) + 'a) -> Self;
    pub fn show(self, ui: &mut egui::Ui) -> BackstageResponse;
}
```

**Behaviour.**

- Opened by the File tab (`Ribbon::file_tab`, §4.1) through the app: the File tab click sets
  `BackstageState::open = true`. The component does not open itself.
- Escape closes the backstage, but only when no text field in the page has keyboard focus. A
  focused field consumes Escape first.
- The back arrow closes it and returns focus to the control that opened it (the File tab), or
  to the ribbon's tab strip if that is gone.
- Page contents are drawn with the normal controls (§4 onwards); pages do not change the
  backstage's own keys.
- A page closure cannot close the backstage directly. It writes to app state it captures; the
  app clears `open` on its next frame (rule 5).
- `page_changed` lets the app react to navigation. It is set only when the user changes page.

## 8. StatusBar and CommandBar

### 8.1 StatusBar

A 24 pt tall strip at the bottom of the window, `color.neutral_background_2`, text
`Caption1` in `color.neutral_foreground_2`.

- Slots: left items, right items, and a zoom slider at the far right.
- Zoom slider: `Slider` (WI-1548) of 10 to 400 percent, with a percentage label. The label
  is a button that resets to 100 percent.

```rust
pub struct StatusBar<'a> { /* private */ }

pub struct StatusBarResponse {
    /// The zoom slider's response, if a zoom was set.
    pub zoom: Option<egui::Response>,
    /// The zoom reset button was clicked.
    pub zoom_reset: bool,
}

impl<'a> StatusBar<'a> {
    pub fn new(id: impl std::hash::Hash) -> Self;
    pub fn left(self, add: impl FnOnce(&mut egui::Ui) + 'a) -> Self;
    pub fn right(self, add: impl FnOnce(&mut egui::Ui) + 'a) -> Self;
    /// Shows the zoom control bound to `percent`. Clamped to 10..=400 when drawn.
    pub fn zoom(self, percent: &'a mut f32) -> Self;
    pub fn show(self, ui: &mut egui::Ui) -> StatusBarResponse;
}
```

Clamping and non-finite values: `percent` is clamped, and a NaN is replaced with 100.0 (rule 9).

### 8.2 CommandBar

A horizontal bar of commands with priority-based overflow. Each item has a priority. Higher
priority stays visible longer.

```rust
pub struct CommandItem<'a> {
    pub key: &'static str,
    pub icon: Icon,
    pub label: &'a str,
    /// Higher stays visible longer. 0 to 255.
    pub priority: u8,
}

pub struct CommandBar<'a> { /* private */ }

pub struct CommandBarResponse {
    /// The key of the command clicked this frame (visible or in the overflow menu).
    pub activated: Option<&'static str>,
    /// Whether the overflow menu is open this frame.
    pub more_open: bool,
}

impl<'a> CommandBar<'a> {
    pub fn new(id: impl std::hash::Hash, items: &'a [CommandItem<'a>]) -> Self;
    pub fn show(self, ui: &mut egui::Ui) -> CommandBarResponse;
}
```

**Overflow algorithm.** This is the rule from craft-libs' toolbar fitting
(`crates/ui/src/layout/toolbar.rs` at the pinned rev in `upstream.toml`), restated here so
WI-1569 does not need to read it:

1. Sort items by `priority`, highest first. Ties keep their given order.
2. Keep the highest-priority items while their total width (plus 4 pt gaps) fits the bar.
3. The rest go into a trailing `More` `MenuButton` (WI-1544), in the same priority order, as
   `MenuItem`s with their icons and labels.
4. The `More` button appears only when at least one item overflows. Its width is reserved
   before the fit, so the bar does not flicker.
5. Hysteresis: an item comes back out of overflow only when the bar has at least 8 pt spare
   beyond its width (§5.3).

Item widths are measured the frame they render and cached in `ctx.data`, as in §5.4.

## 9. Accessibility and keyboard map

### 9.1 AccessKit roles

Roles are set through `ctx.accesskit_node_builder` where egui's `WidgetType` has no match
(ARCHITECTURE rule 8).

| Part | Role | Label |
|---|---|---|
| Title bar | `TitleBar` if the pinned accesskit has it, else `Group` | "Title bar" |
| Caption buttons | `Button` | "Minimise", "Maximise" or "Restore", "Close" |
| QAT | `Toolbar` | "Quick Access Toolbar" |
| QAT item | `Button` | The item's label |
| Ribbon tab strip | `TabList` | "Ribbon tabs" |
| Ribbon tab | `Tab`, with selected state | The tab's label |
| Ribbon body | `TabPanel` | The tab's label |
| Group | `Group` | The group's label |
| Large, small, custom | `Button` | The control's label |
| Toggle | `Button` with toggled state | The label |
| Split | `Button` for the main action, `Button` with `has_popup` for the arrow | "<label>", "<label> menu" |
| Menu | `MenuButton` with `has_popup` | The label |
| Gallery item | `Button` | The item's label |
| Dialog launcher | `Button` | "<group label> dialog" |
| Backstage | `Dialog`-like region | "Backstage" |
| Status bar | `Group` | "Status bar" |
| Zoom slider | `Slider` | "Zoom" with the value |
| CommandBar | `Toolbar` | The bar's label |

### 9.2 Tab order

Within the window, Tab moves through, in order: app icon, QAT items, title text or search box,
account area, caption buttons; then the ribbon tab strip, the active tab's controls; then the
content area; then the status bar. Shift+Tab reverses.

Arrow keys inside the tab strip move between tabs (roving focus). Enter or Space selects a tab.

### 9.3 F6 region cycling

F6 moves focus to the next region, Shift+F6 to the previous. Order: title bar, QAT (if
placed in the ribbon area), ribbon, content, status bar. A region with no focusable content is
skipped. Focus lands on the region's first focusable control, or its last focused control if
the region is visited again.

### 9.4 Keyboard map

| Keys | Action | Owner |
|---|---|---|
| Alt, F10 | KeyTips on (§6) | Chrome |
| Ctrl+F1 | Collapse or expand ribbon (§4.5) | Chrome |
| F6, Shift+F6 | Cycle regions (§9.3) | Chrome |
| Escape | Close KeyTips step, close popup, close Peek, close backstage | Chrome, by innermost open thing |
| Enter, Space | Activate focused control | Control |
| Arrow keys | Move within tab strip, gallery, grid | Control |

Application shortcuts (Ctrl+S and so on) are not in this map. The app owns them (rule 5).

## 10. Tokens

Every colour, size and font the chrome uses, by token. Names are the `fluent2-tokens` names,
which mirror Fluent's own (ARCHITECTURE, Theme). `color.` is `Theme::color`.

**Colours.**

| Use | Token |
|---|---|
| Title bar, ribbon and tab body background | `color.neutral_background_1` |
| Tab strip and status bar background | `color.neutral_background_2` |
| Caption button hover (not close) | `color.subtle_background_hover` |
| Caption button pressed | `color.subtle_background_pressed` |
| Close hover | `color.status_danger_background_3` |
| Close hover pressed | `color.status_danger_background_3_pressed` |
| Close hover foreground | `color.neutral_foreground_on_brand` |
| Primary text, active tab | `color.neutral_foreground_1` |
| Secondary text, status bar | `color.neutral_foreground_2` |
| Inactive window text, disabled-looking caption | `color.neutral_foreground_3` |
| Selected tab underline, KeyTip prefix, Backstage selected bar | `color.brand_foreground_1` |
| Backstage selected row background | `color.subtle_background_selected` |
| Border, separators | `color.neutral_stroke_1` |
| Strong border (menus, popups) | `color.neutral_stroke_2` |
| Focus ring | `color.stroke_focus_2` |
| Contextual accent: Success, Warning, Danger | `color.status_success_foreground_1`, `status_warning_foreground_1`, `status_danger_foreground_1` |

**Sizes and spacing.** These are `ChromeMetrics` fields (D2), not Fluent tokens. Values are
fixed by this spec.

| Field | Value |
|---|---|
| `title_bar_height` | 32 pt |
| `title_bar_height_extended` | 40 pt |
| `caption_button_width` | 46 pt |
| `caption_button_height` | 32 pt |
| `resize_zone` | 6 pt |
| `resize_corner` | 6 pt |
| `tab_strip_height` | 32 pt |
| `ribbon_body_height` | 96 pt, including group labels |
| `group_separator` | 8 pt |
| `command_bar_gap` | 4 pt |
| `hysteresis` | 8 pt |
| `status_bar_height` | 24 pt |
| `backstage_nav_width` | 240 pt |
| `keytip_badge` | 16 pt square for one letter |

Spacing between items inside a group uses the Fluent spacing constants
`spacing_horizontal::S` (8 pt) and `spacing_horizontal::XS` (4 pt) from
`fluent2_tokens::generated::global`. Corner radii: `border_radius::MEDIUM` (4 pt) for buttons
and tabs, `border_radius::LARGE` (6 pt) for popups.

**Fonts.** The `TypeRamp` (WI-1538) names used:

| Use | Ramp |
|---|---|
| Title text in the title bar | `Body1Strong` |
| Ribbon tab labels | `Body1` |
| Group labels and small ribbon labels | `Caption1` |
| Large ribbon button labels | `Caption1` |
| KeyTip badges | `Caption2` with medium weight |
| Status bar text | `Caption1` |
| Backstage page headings | `Title2` |

**Motion.** Tab underline change and Peek open use `duration::FAST` (150 ms) and the
`curve::` constants from `fluent2_tokens::generated::global`, through `foundation::motion`
(WI-1537). Reduced motion (WI-1541) snaps them to their end state.

## 11. Item breakdown check

Checked against the Vogt titles of WI-1559 to WI-1570 (read 2026-10-17) and the index in
`docs/BACKLOG.md`. This is a check of titles and section citations. I did not re-read each
item's body, so a body that says something different than its title is not covered by this
check.

| Section | Covered by | Result |
|---|---|---|
| §1 TitleBar | WI-1559 | Covered |
| §2 Resize edges | WI-1560 | Covered |
| §3 QAT | WI-1561 | Covered |
| §4 Ribbon model and API | WI-1562 (tabs, contextual, collapse), WI-1563 (groups, controls), WI-1564 (galleries) | Covered, split three ways |
| §5 Scaling | WI-1565 | Covered |
| §6 KeyTips state machine | WI-1566 | Covered |
| §6 KeyTips UI, §9 F6 | WI-1567 | Covered |
| §7 Backstage | WI-1568 | Covered |
| §8 StatusBar, CommandBar | WI-1569 | Covered |
| §9 Accessibility and keyboard map | WI-1567 (F6 only) | **Gap.** No item owns the AccessKit roles in §9.1, the Tab order in §9.2, or Ctrl+F1 and Escape (§4.5, §9.4). |
| §10 Tokens | none | **Gap.** No item cites §10. |

**Corrections to make before implementation starts:**

1. Add §9.1 and §9.2 to WI-1559 (title bar and caption roles) and to WI-1569 (status bar and
   CommandBar roles). Add Ctrl+F1 and the Escape order to WI-1562. This gives each part an
   owner.
2. Add §10 to every M1 item it applies to, so the tokens are used and not re-chosen. This
   affects WI-1559, WI-1561, WI-1562, WI-1566, WI-1568 and WI-1569.
3. WI-1562 depends on WI-1555 only. Its tab strip uses `TabList` (WI-1554), which is still
   open. Add WI-1554 as a direct relation of WI-1562 so the dependency is visible.
4. WI-1563 uses `Checkbox` (WI-1546), `Combobox` and `Dropdown` (WI-1549). Its backlog row lists
   these. Confirm the relations in Vogt include WI-1546 and WI-1549.

**Operator decision (see D2).** Chrome metrics live in `ChromeMetrics`, held by `Theme`. That
adds a field to `Theme`, which WI-1535 owns. The alternative is a private `office/metrics.rs`.
Recommendation: `Theme::chrome`, because rule 7 then holds without exception.

## Acceptance

An implementation item author can implement each section without asking a design question.
The signatures are consistent with ARCHITECTURE rules 1 to 10: builders implement the widget
pattern, containers take closures, responses are named structs, ids come from the caller,
nothing dispatches commands, and no colour or size is a literal outside `theme/`.

Verified against `main` at 72dbc87: `Button::icon_only`, `ToggleButton::icon_only`,
`SplitButton::show` and `SplitButtonResponse { main, menu, menu_open }`, `MenuButton::show`,
`Dropdown::new`, `surfaces::MenuItem`, and `surfaces::{submenu, divider, context_menu}`. The
`Checkbox`, `Combobox` and `Dropdown` controls the ribbon uses are on `main`. Not on `main`:
`TabList` (WI-1554, still open). §4 and §8 depend on it or on nothing that is missing. Token
names in §10 were checked against `crates/fluent2-tokens/src/generated/` (`colors.rs`,
`web_light.rs`, `global.rs`).

Not verified here, and checked by the implementation item that cites it: the exact AccessKit
role names in the pinned accesskit version (§9.1), the `egui::viewport::ResizeDirection` path
(§2), and the wordcraft and craft-libs algorithms (§6.5, §8.2).

Deliver: branch `wi-1555-office-chrome-spec`, PR against `main`, naming WI-1555. Do not merge.
WI-1555 is marked done only after WI-1554 lands.
