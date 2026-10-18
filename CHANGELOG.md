# Changelog

All notable changes are recorded here, following [Keep a Changelog](https://keepachangelog.com/).

## [Unreleased]

### Added
- TerminalView behind the new optional `terminal` feature (`fluent2_egui::data::terminal`): `TerminalGrid`, a `vte` VT grid adapted from rdpapp (cursor movement, erase, insert and delete, scrolling regions, alternate screen, 16/256/24-bit SGR colours, bold, italic, underline and inverse, wide characters, OSC title, BEL, scrollback with stable line numbers), and a view that paints it with ANSI colours from Fluent palette tokens, with wheel and Shift+PageUp/PageDown scrollback, drag selection and copy, highlighted search with next and previous, and Ctrl+click links. The AccessKit node has the Terminal role. Input and PTY transport stay with the app, which resizes to `TerminalResponse::fit`. Gallery page "Terminal" (WI-1791).

### Fixed
- `scripts/upstream-check.py` no longer leaves a source partial when GitHub's compare truncates
  (300 files or 250 commits): it takes the branch head directly and finds watched-path changes
  with per-path commit queries (WI-1867).

### Changed
- craft-libs pin bumped to `ae2f796b673f` after a local `git diff` review: 41 commits, none touch
  the watched paths or the adapted `crates/ui/src/layout` files (WI-1867).

## [0.1.0] - 2026-10-18

### Added
- Release 0.1.0: crate-level quick starts, a README Usage section, a `cargo doc` gate with `RUSTDOCFLAGS=-D warnings`, and `.github/workflows/release.yml` that publishes `fluent2-tokens`, `fluent2-icons` and `fluent2-egui` on a `v*` tag (WI-1587).
- Accessibility audit (`docs/a11y-report.md`): WCAG contrast of every token pair in light, dark and high contrast (`tests/a11y_contrast.rs`), and a Tab-order, focus-ring and reduced-motion check (`tests/a11y_keyboard.rs`). `foundation::motion::reduced_motion(ctx)` (WI-1586).

### Fixed
- High contrast: text on hover, pressed and selected fills uses the matching state foreground across buttons, menus, title bar, ribbon, backstage, DataGrid, TreeView, List, Dropdown, TabList and NavigationView. Dialog, the inverted tooltip and the ProgressBar rail have an edge, and a disabled Primary button keeps its stroke. Checkbox, Switch, Slider and ProgressBar use `compound_brand_background` (dark contrast). The focus ring on Checkbox, Radio and Switch no longer disappears after one frame. Switch has the AccessKit Switch role, and TabList and RadioGroup point their active descendant at the selected item. SpinButton is a single Tab stop. From the Orca walk: window edges, the title-bar drag region and the Combobox chevron are no longer silent Tab stops; TreeView, List and DataGrid report the focused row or cell as the active descendant; Slider exposes its range; Toast is a live region; MenuItem has the MenuItem role; Dialog returns focus to a keyboard opener. `examples/a11y_walk` puts every component on one page for screen-reader walks. Under reduced motion, toasts no longer slide, the indeterminate ProgressBar no longer flashes, and Spinner stops repainting (WI-1586).
- Sample editor (`cargo run -p gallery --example editor`): an undecorated window with the title bar (app icon, QAT, title, search box), resize edges, a ribbon with File, Home, Insert, View and a Picture format contextual tab shown while the placeholder picture is selected, the backstage, a text document and a status bar with word count and zoom. Ribbon and QAT keys map to a local `Command` enum dispatched in one `match`. End-to-end kittest coverage in `crates/gallery/tests/editor_e2e.rs` (WI-1570).
- Ribbon adaptive scaling (spec §5): each group renders at the step `office::ribbon::plan` picks for the available width (Large, Medium, Small, or a collapsed button that opens the group in a popup), with the §5.2 reduction order, the 8 pt hysteresis band of §5.3, a trailing `…` menu for groups that still do not fit, and the per-group width memo of §5.4. `GroupBuilder::steps` and `GroupBuilder::icon` choose a group's steps and collapsed icon. Each group now stacks its label row under its controls (WI-1565).
- KeyTips UI and F6 regions (`office::keytips`): Alt (pressed and released alone) or F10 shows badges over the QAT, File tab and ribbon tabs; a tab letter shows that tab's control badges, a control letter acts as a click and is reported in `RibbonResponse` or `QatResponse`, menu and split keys open their popup, two-letter keys show their typed prefix, Escape steps back, and Alt used as a modifier (Alt+F) stays the app's. `.keytip(key)` on `FileTab`, `RibbonTab`, every ribbon control and `RibbonDialogLauncher`. F6 and Shift+F6 cycle focus through title bar, QAT, ribbon, content (`keytips::mark_region`) and status bar, returning to each region's last focused control. Runs as an egui plugin that the ribbon and QAT install (WI-1567).
- TreeView and List (`fluent2_egui::data::tree_view`, `fluent2_egui::data::list`): a virtualised tree with depth, chevrons that turn with `motion::animate` (none under reduced motion), lazy children asked for once per expanded node, Loading rows, `*` to expand siblings, the spec §5.2 key map, and AccessKit Tree, TreeItem, ListBox and ListBoxOption nodes with level, expanded, position and size. The List uses the DataGrid's selection and roving focus (WI-1584).
- Ribbon gallery (`office::ribbon::Gallery`): an inline row of 48 or 32 pt tiles drawn by a caller closure, with previous-row, next-row and expand buttons at its right edge, and an expanded popup grid with category headers and footer menu items. Arrow keys, Home and End move in the grid, Enter selects, Escape closes and returns focus; the popup lays out only visible rows, so 1,000 items stay under 200 widgets (WI-1564).
- DataGrid selection and keyboard focus: `SelectionMode` (None, Single, Multi) with a checkbox column and select-all, click, Ctrl and Shift selection, the spec §3 key map with scroll-into-view, Enter, F2 and Shift+F10 reported in `DataGridResponse`, Enter moves focus into a cell's focusable child and Escape returns it (the cell closure may return that child's `Response`), and AccessKit Grid, Row, ColumnHeader and GridCell nodes with absolute indices and counts for visible rows (WI-1583).
- Vogt GUI port gap analysis (`docs/vogt-port-gap-analysis.md`): Vogt web screens and patterns mapped to fluent2-egui, with gaps sized and assigned to fluent2 or the app.
- Quick Access Toolbar (`office::qat`): icon-only commands with tooltips, a customise menu to show, hide and reorder items and pick the position, and `QatState` owned by the app with `QatState::apply` (WI-1561).
- DataGrid sorting, column resize and autofit: a header click cycles none, ascending, descending; dragging a column's right edge resizes it within min and max; double-clicking the edge, or the first show of an `Auto` column, autofits it to the widest visible cell. `DataGridResponse` reports `sort_changed` and `column_resized`. Adds the `ArrowSortUp` and `ArrowSortDown` icons (WI-1582).
- NavigationView (`fluent2_egui::data::nav_view`): expanded, compact and minimal modes chosen by width, hamburger, sections and footer items, and TabList's vertical indicator. The pane is one Tab stop with Up, Down, Enter and Space (WI-1585).
- SplitView (`fluent2_egui::data::split_view`): two panes with a draggable divider, minimum sizes, Shift and Home/End keyboard resizing and double-click reset. Adapted from craft-libs (MIT OR Apache-2.0), see NOTICE (WI-1585).
- DataGrid core (`fluent2_egui::data::DataGrid`): columns with fixed, auto and fraction widths clamped to min and max, a virtualised body that calls the cell closure only for visible rows, and a sticky header. `Theme::data` holds the data-view sizes (`DataMetrics`); the optional `serde` feature makes `DataGridState` serialisable (WI-1581).
- Data views spec (`docs/specs/data-views.md`): DataGrid, TreeView, List, NavigationView, SplitView and the docking decision (WI-1580).
- `StatusBar` (bottom strip with left and right slots and a zoom slider) and `CommandBar` (priority overflow into a `More` menu), with `Theme::chrome` holding the office chrome sizes (WI-1569).
- Bundled Inter Regular and SemiBold (OFL-1.1) as the UI font wherever Segoe UI is not available: Linux, macOS, wasm, and Windows without Segoe UI. Segoe UI stays first on Windows. Cargo feature `bundled-font`, on by default (WI-1588).
- KeyTips state machine as pure logic: `office::keytips::state` with `step`, `resolve_keytips` and two-letter keys. No rendering yet (WI-1566).
- Ribbon tab strip: `office::ribbon::Ribbon` with the File tab, content and contextual tabs (with headers), double-click and Ctrl+F1 collapse, and the peek popup. Groups are not built yet (WI-1562).
- Ribbon groups and controls (`TabBuilder::group`, `GroupBuilder`): large and small buttons, toggle, split and menu buttons (`.small()` for the small size), checkbox, dropdown, combobox, a dialog launcher in the group corner and a custom widget slot. Controls report by key through `RibbonResponse`, and the peek popup has a pin button. Full size only: galleries (WI-1564) and scaling (WI-1565) follow (WI-1563).
- Backstage: full-window File view with a navigation column, page closures, Escape and back-arrow close, and focus return to the opener (`office::backstage`, WI-1568).
- Gallery Backstage page: a File button opens the backstage with Home, New, Open, Info and Options placeholder pages (WI-1568).
- `MessageBar` (Info, Success, Warning and Error, with title, actions and dismiss) and the `Toaster` toast host (WI-1553).
- GitHub Pages workflow: the wasm gallery is built with trunk and deployed on push to main (WI-1540).
- Dialog surface: modal, non-modal and alert kinds, focus trap, Escape and default-button policy, focus restoration to the opener (WI-1552).
- `fluent2_egui::platform`: OS accent, high contrast, reduced motion and colour scheme behind the `system-preferences` feature (Windows and the Linux desktop portal; unknown elsewhere, including wasm). `theme_from_system` picks light, dark or high contrast and sets reduced motion (WI-1541).
- Window resize edges for undecorated windows: `office::window_edges::window_resize_edges` (WI-1560).
- Office `TitleBar` for undecorated windows: caption buttons, drag, double-click maximise and slots. `Theme::chrome` holds the chrome sizes (`ChromeMetrics`). Gallery example `chrome` (WI-1559).
- Caption glyphs in the `fluent2-icons` catalogue: Subtract (minimise), Maximize and Square Multiple (restore) (WI-1559).
- Tooltip (`Response::fluent_tooltip`) and Popover surfaces (WI-1551).
- TabList and Card surfaces (WI-1554).
- Dropdown and Combobox controls (WI-1549). An open Dropdown takes Escape, so an enclosing dialog stays open (WI-1552).
- Display controls: `Link`, `Badge`, `Divider`, `Field`, `ProgressBar` and `Spinner` (WI-1550).
- ToggleButton and CompoundButton controls (WI-1543).
- SplitButton and MenuButton controls (WI-1544).
- Menu and ContextMenu: Fluent frame, items with icon, shortcut, checkmark, submenu, divider and group header (WI-1545).
- Checkbox, RadioGroup and Switch controls (WI-1546).
- Input, SearchBox and Textarea: Fluent frame around egui `TextEdit` (WI-1547).
- `SpinButton` and `Slider` controls (WI-1548).
- Button control: primary, secondary, outline, subtle and transparent; small, medium and large; icon and icon-only (WI-1542).
- Gallery app (`cargo run -p gallery`): theme switcher, accent seed, alias-colour swatches, type ramp and icon grid. Wasm entry via trunk (WI-1539).
- `Icon` widget and `fluent2_egui::install`: Fluent System Icons at 16/20/24 px, tinted from the theme (WI-1536).
- Foundation helpers: keyboard focus ring, elevation shadows and motion curves (`foundation::{focus, elevation, motion}`).
- `fluent2_egui::theme`: `Theme`, `ThemeKind` and `theme::install`, mapping Fluent alias tokens onto egui visuals.
- Fluent type ramp as egui text styles, and runtime loading of the Windows system UI font (WI-1538). No font file is added.
- `fluent2_tokens` re-exports `AliasColors` and `Shadows`.
- Brand ramps: `BrandRamp`, `FLUENT`, `from_seed`, and brand-aware `light` / `dark` token sets.
- `cargo xtask gen-tokens` generates the `fluent2-tokens` Rust modules from the JSON snapshot.
- Fluent 2 token snapshot (`crates/fluent2-tokens/data/*.json`) from `@fluentui/tokens@1.0.0-alpha.24`.
- Workspace scaffold: `fluent2-tokens`, `fluent2-icons`, `fluent2-egui`.
- `fluent2-icons`: committed Fluent System Icons subset and `cargo xtask gen-icons`.
- Upstream tracking: `upstream.toml`, `UPSTREAM.md`, `scripts/upstream-check.py` and a weekly
  drift workflow.
