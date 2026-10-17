# Office chrome spec

Status: draft for WI-1555. Implementation items WI-1559 to WI-1570 cite the section numbers
below; keep the numbering. This document contains no implementation. The only code is public
type and function signatures, written as the contract an implementation must meet.

Conventions are fixed by [`docs/ARCHITECTURE.md`](../ARCHITECTURE.md) (rules 1 to 10). Where this
spec adds a rule, it says so under **Decisions**.

## Decisions

These are the choices this spec makes that the item and ARCHITECTURE leave open. Each one is
reversible by editing this file before the implementation item that depends on it starts.

| # | Decision | Why |
|---|---|---|
| D1 | Office components return responses and never send `ViewportCommand`s. The app maps a returned `WindowAction` to a command with a pure function. **Exception:** `window_resize_edges` (§2) sends `BeginResize` itself, because the app never chooses a resize. | ARCHITECTURE rule 5. The mapping is unit-testable without a window. |
| D2 | Chrome dimensions (32, 40, 46, 6 pt and so on) are named constants on a `ChromeMetrics` struct held by `Theme` (`Theme::chrome`). They are not Fluent tokens. | Rule 7 forbids magic numbers outside `theme/`. Fluent has no token for these. **Operator decision, see §11.** |
| D3 | `RibbonState`, `QatState` and `BackstageState` are plain data with public fields. No serde in M1. The app persists them. | Keeps the crate free of a serialization dependency. The app already owns persistence (rule 5). |
| D4 | Control keys are `&'static str`. Duplicate keys within one ribbon, QAT or command bar do not panic. The first key wins and later ones are reported in `conflicts`. | Rule 9 (no panics). Rule 4: keys are caller-chosen constants, never translated labels. |
| D5 | Ribbon tab and group bodies are built by closures every frame into a data model, before any painting. Measurement and scaling run on that model. | Lets §5 size groups before drawing, in immediate mode. |
| D6 | Accent colours for contextual tabs are token choices (`ContextualAccent`), never a `Color32` argument. Each accent maps to a different token. | Rule 7. |
| D7 | KeyTips adapt wordcraft's state machine only (§6.5). Rendering is written here. | The state machine is pure logic and small. Rendering depends on this library's painter. |
| D8 | The CommandBar overflow uses the hysteresis rule of §5.3. | One fitting rule across the chrome. |
| D9 | Ribbon widths are computed in the frame they are needed, from fixed control sizes and text layout (`ctx.fonts`), not from a previous frame's measurement. A cache in `ctx.data` only memoises those results. | Text layout is synchronous, so no first frame is guessed and no step is stepped up speculatively. Answers the cold-cache question for WI-1565. |
| D10 | Resize zones are `egui::Area`s at `Order::Foreground` covering the 6 pt border. They win over the title bar and caption buttons inside that border (§2). | Corners must be reachable. Interior clicks are never intercepted. |

## Shared vocabulary

- **Pt** means logical points (egui units). Every width and height in this spec is in points.
- **Region** is one of: title bar, QAT, ribbon, content, status bar. F6 cycles regions (§9).
- **Control key** is a `&'static str` supplied by the app. It names a control in events and state.
- **Response** means a returned struct. Components never return tuples and never dispatch,
  except where D1 says otherwise.
- **Bound state** is app state a control reads or writes. It is borrowed as `&'a mut` for the
  frame (§4.1).

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
| Caption button | 46 × 32 pt |
| Caption glyphs | Minimize, maximize, restore, close (icon catalogue names; WI-1534) |
| Close hover | background `color.status_danger_background_3`, foreground `color.neutral_foreground_on_brand` |
| Other caption hover | background `color.subtle_background_hover` |
| Caption pressed | background `color.subtle_background_pressed` (close: `status_danger_background_3_pressed`) |
| Inactive window | title and caption glyphs use `color.neutral_foreground_3`; background unchanged |

**Slots.** Left to right: app icon, QAT (§3, when placed in the title bar), centered title or
search box, account area on the right, then the caption buttons.

**Signatures.**

```rust
/// The window action the user asked for this frame. The app maps it with `window_command`.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum WindowAction {
    Minimize,
    /// Toggle between maximized and restored.
    ToggleMaximize,
    /// Begin a window drag (the user pressed on a drag region and moved).
    StartDrag,
    Close,
}

pub struct TitleBar<'a> { /* builder fields private */ }

pub struct TitleBarSlots<'s> { /* private; methods below */ }

/// What [`TitleBar::show`] returns. `inner` is whatever the slot closure returned.
pub struct TitleBarResponse<R> {
    /// The value returned by the slot closure.
    pub inner: R,
    /// The action requested this frame, if any.
    pub action: Option<WindowAction>,
    /// Whether the drag region was double-clicked.
    pub double_clicked: bool,
    /// The region not covered by slots or caption buttons.
    pub drag_region: egui::Response,
    /// One response per caption button.
    pub minimize: egui::Response,
    pub maximize: egui::Response,
    pub close: egui::Response,
    /// Whether the window is inactive (`ViewportInfo::focused == Some(false)`).
    pub inactive: bool,
}

impl<'a> TitleBar<'a> {
    /// `id` is caller-supplied. `title` is shown in the center unless a search box is set.
    pub fn new(id: impl std::hash::Hash, title: impl Into<egui::WidgetText>) -> Self;
    /// Whether the window is maximized. The maximize button shows *restore* when true.
    pub fn maximized(self, maximized: bool) -> Self;
    /// `ViewportInfo::focused`, passed in so the component stays window-agnostic.
    pub fn focused(self, focused: Option<bool>) -> Self;
    /// Adds 8 pt to the height and shows a centered search box slot instead of the title text.
    pub fn with_search(self, enabled: bool) -> Self;
    /// `add` fills the slots. It runs once per frame.
    pub fn show<R>(
        self,
        ui: &mut egui::Ui,
        add: impl FnOnce(&mut TitleBarSlots<'_>) -> R,
    ) -> TitleBarResponse<R>;
}

impl TitleBarSlots<'_> {
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
- Caption buttons are keyboard reachable (§9). Accessible labels: "Minimize", "Maximize"
  (or "Restore" when maximized), "Close".
- The top 6 pt of the title bar belongs to the resize zone, not the drag region (§2, D10).

## 2. Window resize edges

For undecorated windows only. Implementation item: WI-1560, which uses these names.

**Zones.** Eight zones along the window's inner rect: four edges and four corners. Thickness is
6 pt (`ChromeMetrics::resize_zone`). A corner zone is a 6 × 6 pt square at each corner.

**Precedence (D10).** Zones are `egui::Area`s at `Order::Foreground`, one per zone, sized to the
zone. Inside the 6 pt border:

- The top edge zone covers the top 6 pt of the title bar. A press there starts a resize, not a
  drag. Below 6 pt, the drag region works as normal.
- The top-right corner zone covers the top-right 6 × 6 pt of the Close caption button. A press
  there starts a resize. The rest of Close works.
- Other caption buttons lose only their top 6 pt to the top edge zone.
- No zone covers the window interior, so nothing there is intercepted.

**Cursors.** Hover over a zone sets `CursorIcon::ResizeHorizontal` (left and right edges),
`ResizeVertical` (top and bottom), or `ResizeNwSe` and `ResizeNeSw` (corners).

**Suppression.** When `ctx.input(|i| i.viewport().maximized)` or `.fullscreen` is `Some(true)`,
no zones are created and no cursor changes. The component reads nothing else from the app.

**Signatures.**

```rust
/// Call once per frame in an undecorated window. Sets resize cursors on hover and sends
/// `ViewportCommand::BeginResize(direction)` on a primary press in a zone (D1 exception).
pub fn window_resize_edges(ctx: &egui::Context);

/// Pure hit test for WI-1560's unit tests: the direction for `pos` in the 6 pt border of
/// `screen_rect`, corners first, or `None` for the interior and outside the window.
pub fn zone_at(
    pos: egui::Pos2,
    screen_rect: egui::Rect,
    thickness: f32,
) -> Option<egui::viewport::ResizeDirection>;
```

`egui::viewport::ResizeDirection` is the type `ViewportCommand::BeginResize` takes. WI-1560
confirms the path against the pinned egui 0.36.

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
    /// Whether the customize menu is open.
    pub customize_open: bool,
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
    /// Keytip for §6. `None` means auto-assigned.
    pub keytip: Option<&'static str>,
}

pub struct Qat<'a> { /* private */ }

pub struct QatResponse {
    /// The key of the item clicked this frame.
    pub clicked: Option<&'static str>,
    /// Customize-menu actions this frame.
    pub customize: Option<QatCustomize>,
    /// Keys that appear in `items` but not in the offered list, and were dropped.
    pub conflicts: Vec<&'static str>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum QatCustomize {
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

**Customize menu.** Opened from a trailing `…` icon button. Lists every offered item with a
checkbox (`Show`), and up and down buttons on each row (`MoveUp`, `MoveDown`). Its last section
is the position choice (`Position`). It is a `Menu` (WI-1545) with `MenuItem::checked`.

**Rules.**
- The app applies `QatCustomize` to `QatState`. The component mutates nothing except
  `customize_open`.
- A click on an item returns its key; the app decides what it does (rule 5).
- An item the app stops offering is removed from `items` on the next frame and reported in
  `conflicts`.

## 4. Ribbon model and API

### 4.1 Lifetimes and the model

There is one lifetime model for the whole ribbon.

- `'a` is the frame. Every piece of bound app state is borrowed for `'a` as `&'a mut T`, or as
  `&'a [T]` for read-only option lists. The ribbon never owns app state.
- The ribbon stores those borrows only in the per-frame model (D5). `show` drops them when it
  returns.
- `RibbonState` is the only state the ribbon owns across frames. It is passed to `show` as
  `&mut`.
- Build closures run once per frame, before painting. They add controls to the model. They do
  not draw.

```rust
pub struct Ribbon<'a> { /* private: file tab and RibbonTab<'a> list */ }

impl<'a> Ribbon<'a> {
    pub fn new(id: impl std::hash::Hash) -> Self;
    /// The File tab. It opens the backstage (§7) and is never a content tab.
    pub fn file_tab(self, tab: FileTab) -> Self;
    /// A content or contextual tab (§4.3).
    pub fn tab(self, tab: RibbonTab<'a>) -> Self;
    pub fn show(self, ui: &mut egui::Ui, state: &mut RibbonState) -> RibbonResponse;
}

pub struct FileTab { /* private */ }
impl FileTab {
    pub fn new(label: &str) -> Self;
    pub fn keytip(self, key: &'static str) -> Self;
}

pub struct RibbonTab<'a> { /* private */ }
impl<'a> RibbonTab<'a> {
    /// `build` runs once per frame and adds groups to the tab.
    pub fn new(key: &'static str, label: &str, build: impl FnOnce(&mut TabBuilder<'a>) + 'a) -> Self;
    pub fn keytip(self, key: &'static str) -> Self;
    /// Makes this tab part of a contextual set. It shows only while `visible` is true (§4.3).
    pub fn contextual(self, set: &'static str, accent: ContextualAccent, visible: bool) -> Self;
}

pub struct TabBuilder<'a> { /* private */ }
impl<'a> TabBuilder<'a> {
    /// A group. `build` adds its controls. `key` is stable and unique within the ribbon.
    pub fn group(&mut self, key: &'static str, label: &str, build: impl FnOnce(&mut GroupBuilder<'a>));
}

pub struct GroupBuilder<'a> { /* private */ }
impl<'a> GroupBuilder<'a> {
    /// Which scale steps this group may use (§5.1). Default: all four.
    pub fn steps(&mut self, steps: ScalingSteps);
    /// Icon of the collapsed button (§5.1). Default: the first control's icon.
    pub fn icon(&mut self, icon: Icon);
    pub fn add(&mut self, control: impl Into<RibbonControl<'a>>);
}

/// Every control kind is a builder. Each has `.keytip(key)` (§6.2).
pub enum RibbonControl<'a> { /* one variant per kind below; private */ }

/// Large button: icon above the label. Event: `Clicked`.
pub struct RibbonLarge<'a> { /* private */ }
impl<'a> RibbonLarge<'a> {
    pub fn new(key: &'static str, icon: Icon, label: &str) -> Self;
    pub fn keytip(self, key: &'static str) -> Self;
}

/// Small button: icon beside the label. Event: `Clicked`.
pub struct RibbonSmall<'a> { /* private */ }
impl<'a> RibbonSmall<'a> {
    pub fn new(key: &'static str, icon: Icon, label: &str) -> Self;
    pub fn keytip(self, key: &'static str) -> Self;
}

/// Toggle button bound to `value`. Event: `Changed` (`value` is already updated).
pub struct RibbonToggle<'a> { /* private */ }
impl<'a> RibbonToggle<'a> {
    pub fn new(key: &'static str, icon: Icon, label: &str, value: &'a mut bool) -> Self;
    pub fn keytip(self, key: &'static str) -> Self;
}

/// Split button. `Clicked` on the main action. `menu` draws the popup; items in it write app
/// state, and the app reads that state (`SplitButton::show`, WI-1544).
pub struct RibbonSplit<'a> { /* private */ }
impl<'a> RibbonSplit<'a> {
    pub fn new(key: &'static str, icon: Icon, label: &str, menu: impl FnOnce(&mut egui::Ui) + 'a) -> Self;
    pub fn keytip(self, key: &'static str) -> Self;
}

/// Menu button. `Clicked` when it is pressed to open. `menu` as for `RibbonSplit`.
pub struct RibbonMenu<'a> { /* private */ }
impl<'a> RibbonMenu<'a> {
    pub fn new(key: &'static str, icon: Icon, label: &str, menu: impl FnOnce(&mut egui::Ui) + 'a) -> Self;
    pub fn keytip(self, key: &'static str) -> Self;
}

pub struct GalleryItem<'a> {
    pub key: &'static str,
    pub label: &'a str,
    pub icon: Icon,
}

/// Gallery bound to `selected`. Event: `Changed` when the selection changes.
pub struct RibbonGallery<'a> { /* private */ }
impl<'a> RibbonGallery<'a> {
    pub fn new(
        key: &'static str,
        label: &str,
        items: &'a [GalleryItem<'a>],
        selected: &'a mut Option<usize>,
    ) -> Self;
    pub fn keytip(self, key: &'static str) -> Self;
}

/// Combobox bound to `text`. Picking an option copies it into `text`. Event: `Changed`.
pub struct RibbonCombobox<'a> { /* private */ }
impl<'a> RibbonCombobox<'a> {
    pub fn new(key: &'static str, label: &str, options: &'a [&'a str], text: &'a mut String) -> Self;
    pub fn keytip(self, key: &'static str) -> Self;
}

/// Dropdown bound to the selected index. Event: `Changed`.
pub struct RibbonDropdown<'a> { /* private */ }
impl<'a> RibbonDropdown<'a> {
    pub fn new(key: &'static str, label: &str, options: &'a [&'a str], selected: &'a mut usize) -> Self;
    pub fn keytip(self, key: &'static str) -> Self;
}

/// Checkbox bound to `value`. Event: `Changed`.
pub struct RibbonCheckbox<'a> { /* private */ }
impl<'a> RibbonCheckbox<'a> {
    pub fn new(key: &'static str, label: &str, value: &'a mut bool) -> Self;
    pub fn keytip(self, key: &'static str) -> Self;
}

/// Dialog launcher at the group's bottom-right corner. Event: `Clicked`.
pub struct RibbonDialogLauncher { /* private */ }
impl RibbonDialogLauncher {
    pub fn new(key: &'static str) -> Self;
    pub fn keytip(self, key: &'static str) -> Self;
}

/// Caller-drawn widget. `add` receives the scale step it must draw at (§5.1) and returns its
/// response, which appears in `RibbonResponse::custom`. Widths are declared, not measured.
pub struct RibbonCustom<'a> { /* private */ }
impl<'a> RibbonCustom<'a> {
    pub fn new(
        key: &'static str,
        add: impl FnMut(&mut egui::Ui, ScalingStep) -> egui::Response + 'a,
    ) -> Self;
    /// Width in points at each step (Large, Medium, Small).
    pub fn widths(self, large: f32, medium: f32, small: f32) -> Self;
    pub fn keytip(self, key: &'static str) -> Self;
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ContextualAccent {
    Brand,
    Success,
    Warning,
    Danger,
    Neutral,
}
```

Each control struct converts into `RibbonControl<'a>` (`From`), so `GroupBuilder::add` takes
any of them.

**Example.** Illustrative only; `Icon` names are not final.

```rust
Ribbon::new("main")
    .file_tab(FileTab::new("File").keytip("F"))
    .tab(RibbonTab::new("home", "Home", |t| {
        t.group("clipboard", "Clipboard", |g| {
            g.add(RibbonLarge::new("paste", Icon::Paste, "Paste").keytip("V"));
            g.add(RibbonSmall::new("cut", Icon::Cut, "Cut"));
        });
        t.group("font", "Font", |g| {
            g.add(RibbonToggle::new("bold", Icon::Bold, "Bold", &mut app.bold).keytip("1"));
        });
    }).keytip("H"))
    .show(ui, &mut app.ribbon_state);
```

Here `app.bold` is borrowed for the frame and `app.ribbon_state` separately; the closures
capture disjoint fields.

### 4.2 Control kinds and what they map to

| Kind | Built on | Notes |
|---|---|---|
| Large, Small | Own painter in `office/ribbon/group.rs`, using `Button` colours and tokens | Scaling per §5.1. |
| Toggle | `ToggleButton` (WI-1543) | Selected state follows `value`. |
| Split | `SplitButton` (WI-1544) | Its `SplitButtonResponse` maps to `Clicked` on `key`. |
| Menu | `MenuButton` (WI-1544) | Popup content is `surfaces::menu` (WI-1545). |
| Gallery | Own painter, §4.4 | Inline row plus expanded grid. |
| Combobox, Dropdown | WI-1549 | Width from §5.1. |
| Checkbox | WI-1546 | Label beside. |
| Dialog launcher | `Button::icon_only` (WI-1542) | Fixed 16 pt icon. |
| Custom | Caller's widget | Response in `RibbonResponse::custom`. |

### 4.3 Contextual tabs

- A tab with `.contextual(set, accent, visible)` is added after the content tabs. It shows only
  while `visible` is true. When it appears, the app may set `RibbonState::active` to it.
- Tabs with the same `set` share a header label above them. The header label is drawn in the
  accent colour (D6). Token per accent:

| Accent | Token |
|---|---|
| `Brand` | `color.brand_foreground_1` |
| `Success` | `color.status_success_foreground_1` |
| `Warning` | `color.status_warning_foreground_1` |
| `Danger` | `color.status_danger_foreground_1` |
| `Neutral` | `color.neutral_foreground_2` |

- When a contextual tab disappears while it is active, the active tab becomes the first content
  tab. This is a state change in `RibbonState`, not a response.

### 4.4 Galleries

- Inline row: shows the number of tiles for the group's current step (§5.1). Two arrow buttons
  scroll one row at a time. Each item is a 48 × 48 pt tile at Large, 32 × 32 pt at Medium.
- Expanded grid: a drop-down button at the row's right edge opens a popup grid. Its width is the
  group width. The popup closes on selection or on Escape, and returns focus to the gallery.
- Selecting an item writes its index to `selected` and gives `Changed` on `key`.

### 4.5 Collapse and pin

`RibbonState` holds the display state:

```rust
#[derive(Clone, Debug, PartialEq, Eq, Default)]
pub struct RibbonState {
    /// The active tab, by key. `None` means the first content tab.
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
- A pinned ribbon in Peek state behaves as Expanded.

### 4.6 RibbonResponse

```rust
pub struct RibbonResponse {
    /// Events from controls this frame, in visual order.
    pub events: Vec<RibbonEvent>,
    /// The tab that became active this frame, if it changed.
    pub tab_changed: Option<&'static str>,
    /// Keys used more than once in this ribbon. The first occurrence keeps its events (D4).
    pub conflicts: Vec<&'static str>,
    /// Responses of `RibbonCustom` widgets, keyed by their key.
    pub custom: Vec<(&'static str, egui::Response)>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct RibbonEvent {
    pub key: &'static str,
    pub kind: RibbonEventKind,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum RibbonEventKind {
    /// A button, split, menu, or dialog launcher was pressed.
    Clicked,
    /// The bound state changed this frame. The new value is in the state the control was
    /// given, not in the event.
    Changed,
}
```

**Rules.**
- `RibbonResponse` only reports. The app maps `key` to its command (rule 5). Items inside a
  menu or split popup write app state and do not produce events of their own.
- Events for a disabled control (`ui.add_enabled(false, …)`) are never produced.
- Keys in `conflicts` are duplicates within the ribbon. The first occurrence keeps its events.

## 5. Ribbon layout and adaptive scaling

### 5.1 Scale steps

Each group renders at one of four steps, from widest to narrowest:

| Step | Rendering |
|---|---|
| `Large` | Large controls, with labels under icons where the control has them |
| `Medium` | Small controls, with labels beside icons |
| `Small` | Icon only; labels move into tooltips |
| `Collapsed` | One 36 pt button, showing the group's icon (`GroupBuilder::icon`, or the first control's icon). Opens a popup with the full group |

```rust
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub enum ScalingStep { Large, Medium, Small, Collapsed }

/// The steps a group may use, widest first. Always ends with `Collapsed`.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ScalingSteps { /* private */ }

impl ScalingSteps {
    /// All four steps. The default.
    pub fn all() -> Self;
    /// The given steps, sorted widest first, duplicates removed. `Collapsed` is always added.
    /// Example: `ScalingSteps::new(&[ScalingStep::Large, ScalingStep::Small])` skips Medium.
    pub fn new(steps: &[ScalingStep]) -> Self;
}
```

Set on a group with `GroupBuilder::steps`. A group that omits Medium still reduces from Large
to Small.

**Per-control rendering.** Each control has a rendering at each step the group allows.

| Control | Large | Medium | Small |
|---|---|---|---|
| `RibbonLarge` | 32 pt icon above a label of up to two lines | 16 pt icon beside a one-line label | 16 pt icon only |
| `RibbonSmall` | 16 pt icon beside its label | same as Large | 16 pt icon only |
| `RibbonToggle` | as `RibbonLarge` | as `RibbonSmall` | as Small |
| `RibbonSplit` | 32 pt icon above label, chevron below the label | 16 pt icon beside label, chevron after | 16 pt icon with chevron; no label |
| `RibbonMenu` | as `RibbonLarge`, with chevron after label | as `RibbonSmall`, with chevron | 16 pt icon only |
| `RibbonGallery` | 6 tiles of 48 pt, with arrows | 3 tiles of 32 pt, with arrows | one 36 pt button showing the selected item's icon; opens the grid |
| `RibbonCombobox` | 120 pt wide | 96 pt wide | 64 pt wide |
| `RibbonDropdown` | 120 pt wide | 96 pt wide | 64 pt wide |
| `RibbonCheckbox` | checkbox and label | checkbox and label | checkbox only; label in tooltip |
| `RibbonDialogLauncher` | 16 pt icon | 16 pt icon | 16 pt icon |
| `RibbonCustom` | `add` draws at `Large` | `add` draws at `Medium` | `add` draws at `Small` |

Label text is measured as described in §5.4. Widths in the table are fixed sizes.

### 5.2 Reduction order

Input: available width `W` (the tab body width minus margins), the active tab's groups in
visual order, and each group's steps.

1. Start every group at its widest allowed step.
2. While the sum of group widths (plus 8 pt separators between groups) is greater than `W`:
   - Take the **rightmost** group that is not yet at `Collapsed`.
   - Step it to the next step it allows (`Large` → `Medium` → `Small` → `Collapsed`, skipping
     steps the group does not allow).
3. The tie-break: the reduction always goes to the rightmost group. Two groups at the same step
   never change in any other order.
4. Stop when the sum fits, or when every group is `Collapsed`.

Growth (when space returns) runs the other way: the **leftmost** group that is not at its
widest step steps up one allowed level, and only when `W` is at least the new sum plus the
hysteresis margin (§5.3).

Overflow: if every group is `Collapsed` and the sum still exceeds `W`, the rightmost groups are
moved into a trailing `…` menu button. Its popup lists the moved groups as sub-menus, each
containing that group's controls. The ribbon height never changes.

### 5.3 Hysteresis

- A group steps up only when `W` is at least the new total plus **8 pt**.
- A group steps down as soon as the total exceeds `W`.
- Within an 8 pt band, the state does not change. This prevents oscillation at a resize edge.

### 5.4 Measurement

- Widths are computed in the frame they are needed, from the table in §5.1 and the label text.
  Label width comes from `ctx.fonts(|f| f.layout_no_wrap(…))`, which is synchronous. There is no
  first-frame guess (D9).
- A `RibbonCustom` has declared widths, so it is never measured.
- `ctx.data` memoises the computed widths per (ribbon id, tab key, group key), with a content
  hash over the keys, labels and allowed steps. A hash change recomputes them. The memo is an
  optimisation only; dropping it changes nothing visible.
- Padding: 4 pt on each side of a control, and 4 pt between icon and label
  (`spacing_horizontal::XS`).
- A label is capped at `ChromeMetrics::large_label_max` (72 pt) for Large controls, and wraps to
  two lines before it is capped.

### 5.5 Worked example

Available width `W = 900 pt`. Four groups, widths per step (`L`, `M`, `S`, `C`):

| Group | L | M | S | C |
|---|---|---|---|---|
| Clipboard | 200 | 160 | 120 | 36 |
| Font | 320 | 260 | 230 | 36 |
| Paragraph | 280 | 240 | 200 | 36 |
| Styles | 360 | 300 | 260 | 36 |

Separators: 3 × 8 = 24 pt. All four groups allow all steps.

1. All `Large`: 200 + 320 + 280 + 360 + 24 = **1184 pt**. Above 900, so reduce.
2. Rightmost not `Collapsed`: Styles. `L → M`: 1184 − (360 − 300) = **1124**. Still above 900.
3. Styles `M → S`: 1124 − (300 − 260) = **1084**. Still above 900.
4. Styles `S → C`: 1084 − (260 − 36) = **860**. Fits (860 ≤ 900).

Result: Clipboard L, Font L, Paragraph L, Styles C. Total 860 pt.

Growth with the same state:

- If `W` becomes 1000 pt, Styles would need `S` at 1084 + 8 = 1092. 1000 is less, so it stays
  `C`.
- If `W` becomes 1100 pt, Styles steps to `S`, because 1100 ≥ 1084 + 8 = 1092.
- At 1090 pt, it stays `C`: 1090 is in the hysteresis band (1084 to 1092). This is the
  oscillation case the band exists to stop.

These widths are the same on the first frame, because they come from text layout (§5.4).

## 6. KeyTips

### 6.1 Activation and display

- Alt, pressed and released with no other key in between, activates KeyTips at the root level.
- F10 does the same and also toggles KeyTips off when already active.
- Badges are small rounded rectangles (`ChromeMetrics::keytip_badge`, 16 × 16 pt for one
  letter, grown to fit two letters) placed at the top-left of the target's rectangle, offset
  2 pt. Tab badges go under the tab label.
- Badge text is the `Caption2` ramp, medium weight. Colours: background
  `color.neutral_background_1`, text `color.neutral_foreground_1`, border
  `color.neutral_stroke_1`. The active sequence's prefix is shown in `color.brand_foreground_1`.

### 6.2 Keys

**App-supplied.** The app sets a key with `.keytip(key)` on: `FileTab`, `RibbonTab`, every
control struct in §4.1, `RibbonDialogLauncher`, and `QatItem::keytip`. A supplied key is
uppercased, and must be one ASCII letter, two ASCII letters, or one digit. Anything else is
invalid: it is listed in `KeyTipsReport::invalid`, and the target is auto-assigned.

**Auto-assigned.** The first letter of the label, uppercased, that is not already used at the
same level. If none is free, the next letters of the label are tried, then `A` to `Z`, then
digits `1` to `9`.

**Multi-letter.** When a level has more than 26 targets, keys become two letters. The first
letter selects a group of targets and the second selects one. The state machine in §6.3
supports this. M1 does not need it in practice.

**Conflicts.** Two targets at the same level with the same key. The first keeps the key. The
second is re-assigned by the auto rule and listed in `KeyTipsReport::conflicts`. Nothing panics.

**Types.**

```rust
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum KeyTipLevel {
    /// Tabs, the File tab, QAT items and app-level targets.
    Root,
    /// The controls of one tab, by tab key.
    Tab(&'static str),
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum TargetKind { Tab, QatItem, Control, MenuItem }

/// One target as the chrome sees it this frame. Built from the ribbon model (§4) and the
/// QAT (§3). `requested` is the app's `.keytip(…)`.
pub struct KeyTipCandidate {
    pub target: &'static str,
    pub level: KeyTipLevel,
    pub kind: TargetKind,
    pub requested: Option<&'static str>,
    pub label: String,
    pub rect: egui::Rect,
}

/// One target with its final key.
#[derive(Clone, Debug, PartialEq)]
pub struct KeyTipTarget {
    pub target: &'static str,
    pub level: KeyTipLevel,
    pub kind: TargetKind,
    /// The key the user types, uppercased.
    pub key: String,
    /// Where the badge is drawn.
    pub rect: egui::Rect,
}

/// Every target with its final key, for one frame.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct KeyTipTargets { /* private: Vec<KeyTipTarget> */ }

impl KeyTipTargets {
    pub fn iter(&self) -> impl Iterator<Item = &KeyTipTarget>;
    /// The target at `level` with `key`, if any.
    pub fn find(&self, level: KeyTipLevel, key: &str) -> Option<&KeyTipTarget>;
}

/// What assignment did, for the app's tests and debug overlay.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct KeyTipsReport {
    /// Targets whose requested key was already taken at their level.
    pub conflicts: Vec<&'static str>,
    /// Requested keys that were not one or two letters or one digit.
    pub invalid: Vec<&'static str>,
    /// Targets that had no requested key.
    pub auto_assigned: Vec<&'static str>,
}

pub struct KeyTipResolution {
    pub targets: KeyTipTargets,
    pub report: KeyTipsReport,
}

/// Assigns keys to `candidates` by the rules above.
pub fn resolve_keytips(candidates: &[KeyTipCandidate]) -> KeyTipResolution;
```

### 6.3 State machine

```rust
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum KeyTipState {
    /// KeyTips are not shown. Alt or F10 leaves this state.
    Idle,
    /// Root level: tabs, QAT and app-level targets are shown.
    Root,
    /// A content tab was chosen. Its controls are shown.
    Tab { tab: &'static str },
    /// A menu or gallery popup opened from a control is shown.
    Popup { tab: &'static str, control: &'static str },
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum KeyTipInput {
    /// A letter or digit key.
    Char(char),
    Escape,
    Backspace,
    /// Alt or F10 pressed.
    Activate,
    /// Mouse click anywhere, or focus moved outside the chrome.
    Dismiss,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum KeyTipAction {
    None,
    /// Show the targets for the new state's level.
    Show,
    /// Switch to a tab and show its controls.
    SelectTab(&'static str),
    /// Invoke a target as a normal click or activation.
    Invoke(&'static str),
    /// Open a control's popup (menu, gallery).
    OpenPopup(&'static str),
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct KeyTipStep {
    pub state: KeyTipState,
    pub action: KeyTipAction,
}

/// Pure transition. No egui input is read here, so the table is unit-testable.
pub fn step(state: &KeyTipState, input: KeyTipInput, targets: &KeyTipTargets) -> KeyTipStep;
```

Transitions (single-letter; two-letter sequences are a sub-state of the same level, completed by
the next `Char`, and cancelled to their level by Escape):

| State | Input | Next state | Action |
|---|---|---|---|
| Idle | Activate | Root | Show |
| Idle | other | Idle | None |
| Root | Char matching a tab | Tab(tab) | SelectTab |
| Root | Char matching a QAT or app-level target | Idle | Invoke |
| Root | Char no match | Root | None |
| Root | Escape, Backspace, Activate, Dismiss | Idle | None |
| Tab(t) | Char matching a control | Idle | Invoke |
| Tab(t) | Char matching a menu or gallery control | Popup(t, c) | OpenPopup |
| Tab(t) | Char no match | Tab(t) | None |
| Tab(t) | Escape or Backspace | Root | Show |
| Tab(t) | Activate or Dismiss | Idle | None |
| Popup(t, c) | Char matching an item | Idle | Invoke (the item's key) |
| Popup(t, c) | Escape or Backspace | Tab(t) | Show |
| Popup(t, c) | Activate or Dismiss | Idle | None |

### 6.4 Which keytips the app supplies

The app supplies keys for: QAT items (they are its commands), the File tab, Save and Undo in the
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
accessible label "Back"). Below it, page entries, each a 40 pt tall row. The selected row uses
`color.subtle_background_selected` and a 3 pt brand bar at the left. At the bottom, bottom-group
entries (Options, Account), separated by a `Divider`. The page area fills the rest, with 24 pt
padding.

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

- Opened by the File tab through the app: the File tab click sets `BackstageState::open = true`.
  The component does not open itself.
- Escape closes the backstage, but only when no text field in the page has keyboard focus. A
  focused field consumes Escape first.
- The back arrow closes it and returns focus to the control that opened it (the File tab), or
  to the ribbon's tab strip if that is gone.
- Page contents are drawn with the normal controls (§4 onwards). Pages do not change the
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
    /// Shows the zoom control bound to `percent`.
    pub fn zoom(self, percent: &'a mut f32) -> Self;
    pub fn show(self, ui: &mut egui::Ui) -> StatusBarResponse;
}
```

`percent` is clamped to 10..=400 when drawn. A NaN is replaced with 100.0 (rule 9).

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

Item widths come from text layout (§5.4), as in the ribbon.

## 9. Accessibility and keyboard map

### 9.1 AccessKit roles

Roles are set through `ctx.accesskit_node_builder` where egui's `WidgetType` has no match
(ARCHITECTURE rule 8).

| Part | Role | Label |
|---|---|---|
| Title bar | `TitleBar` if the pinned accesskit has it, else `Group` | "Title bar" |
| Caption buttons | `Button` | "Minimize", "Maximize" or "Restore", "Close" |
| QAT | `Toolbar` | "Quick Access Toolbar" |
| QAT item | `Button` | The item's label |
| Ribbon tab strip | `TabList` | "Ribbon tabs" |
| Ribbon tab | `Tab`, with selected state | The tab's label |
| Ribbon body | `TabPanel` | The tab's label |
| Group | `Group` | The group's label |
| Large, Small, Custom | `Button` | The control's label |
| Toggle | `Button` with toggled state | The label |
| Split | `Button` for the main action, `Button` with `has_popup` for the arrow | "<label>", "<label> menu" |
| Menu | `Button` with `has_popup` | The label |
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
| Inactive window text | `color.neutral_foreground_3` |
| Selected tab underline, KeyTip prefix, Backstage selected bar | `color.brand_foreground_1` |
| Backstage selected row background | `color.subtle_background_selected` |
| Border, separators | `color.neutral_stroke_1` |
| Strong border (menus, popups) | `color.neutral_stroke_2` |
| Focus ring | `color.stroke_focus_2` |
| Contextual accents | see §4.3 |

**Sizes and spacing.** These are `ChromeMetrics` fields (D2), not Fluent tokens. Values are
fixed by this spec.

| Field | Value |
|---|---|
| `title_bar_height` | 32 pt |
| `title_bar_height_extended` | 40 pt |
| `caption_button_width` | 46 pt |
| `caption_button_height` | 32 pt |
| `resize_zone` | 6 pt |
| `tab_strip_height` | 32 pt |
| `ribbon_body_height` | 96 pt, including group labels |
| `group_separator` | 8 pt |
| `command_bar_gap` | 4 pt |
| `hysteresis` | 8 pt |
| `status_bar_height` | 24 pt |
| `backstage_nav_width` | 240 pt |
| `keytip_badge` | 16 pt square for one letter |
| `icon_small` | 16 pt |
| `icon_large` | 32 pt |
| `large_label_max` | 72 pt |
| `collapsed_button` | 36 pt |

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

Checked against the Vogt titles and bodies of WI-1559, WI-1560 and the other M1 items (read
2026-10-17) and the index in `docs/BACKLOG.md`.

| Section | Covered by | Result |
|---|---|---|
| §1 TitleBar | WI-1559 | Covered. Its labels and `ViewportCommand` mapping match §1. |
| §2 Resize edges | WI-1560 | Covered. Its `window_resize_edges(ctx)` and `zone_at(pos, screen_rect, thickness)` match §2. No correction needed. |
| §3 QAT | WI-1561 | Covered |
| §4 Ribbon model and API | WI-1562 (tabs, contextual, collapse), WI-1563 (groups, controls), WI-1564 (galleries) | Covered, split three ways |
| §5 Scaling | WI-1565 | Covered, with one correction below |
| §6 KeyTips state machine | WI-1566 | Covered |
| §6 KeyTips UI, §9 F6 | WI-1567 | Covered |
| §7 Backstage | WI-1568 | Covered |
| §8 StatusBar, CommandBar | WI-1569 | Covered |
| §9 Accessibility and keyboard map | WI-1567 (F6 only) | **Gap.** No item owns the AccessKit roles in §9.1, the tab order in §9.2, or Ctrl+F1 and Escape (§4.5, §9.4). |
| §10 Tokens | none | **Gap.** No item cites §10. |

**Corrections to make before implementation starts:**

1. Add §9.1 and §9.2 to WI-1559 (title bar and caption roles) and to WI-1569 (status bar and
   CommandBar roles). Add Ctrl+F1 and the Escape order to WI-1562. This gives each part an
   owner.
2. Add §10 to every M1 item it applies to, so the tokens are used and not re-chosen. This
   affects WI-1559, WI-1561, WI-1562, WI-1566, WI-1568 and WI-1569.
3. WI-1565 says to cache previous-frame group widths in `ctx.data`. This spec replaces that
   with same-frame text measurement and a memo (§5.4, D9). WI-1565 should cite §5.4.
4. WI-1562 depends on WI-1555 only. Its tab strip uses `TabList` (WI-1554), which is still
   open. Add WI-1554 as a direct relation of WI-1562 so the dependency is visible.
5. WI-1563 uses `Checkbox` (WI-1546), `Combobox` and `Dropdown` (WI-1549). Confirm the Vogt
   relations include WI-1546 and WI-1549.

**Operator decision (see D2).** Chrome metrics live in `ChromeMetrics`, held by `Theme`. That
adds a field to `Theme`, which WI-1535 owns. The alternative is a private `office/metrics.rs`.
Recommendation: `Theme::chrome`, because rule 7 then holds without exception. This must be
settled before WI-1559 starts.

## Acceptance

An implementation item author can implement each section without asking a design question.
The signatures are consistent with ARCHITECTURE rules 1 to 10: builders implement the widget
pattern, containers take closures, responses are named structs (no tuples), ids come from the
caller, nothing dispatches commands except `window_resize_edges` (D1), and no colour or size is
a literal outside `theme/`.

Verified against `main` at 72dbc87: `Button::icon_only`, `ToggleButton::icon_only`,
`SplitButton::show` and `SplitButtonResponse { main, menu, menu_open }`, `MenuButton::show`,
`Dropdown::new`, `surfaces::MenuItem`, and `surfaces::{submenu, divider, context_menu}`. The
`Checkbox`, `Combobox` and `Dropdown` controls the ribbon uses are on `main`. Not on `main`:
`TabList` (WI-1554, still open). Token names in §10 were checked against
`crates/fluent2-tokens/src/generated/` (`colors.rs`, `web_light.rs`, `global.rs`). The accent
tokens in §4.3 were checked the same way; no `status_important` or `status_severe` token exists,
which is why `Important` was replaced by `Neutral`.

Not verified here, and checked by the implementation item that cites it: the exact AccessKit
role names in the pinned accesskit version (§9.1), the `egui::viewport::ResizeDirection` path
(§2), the `ctx.fonts` text-layout call used in §5.4, and the wordcraft and craft-libs algorithms
(§6.5, §8.2).

Deliver: branch `wi-1555-office-chrome-spec`, PR against `main`, naming WI-1555. Do not merge.
WI-1555 is marked done only after WI-1554 lands.
