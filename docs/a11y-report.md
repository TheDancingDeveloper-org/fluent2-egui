# Accessibility audit (WI-1586)

This audit covers high contrast, colour contrast, keyboard access, focus visibility, reduced motion and AccessKit semantics for every fluent2-egui component, ahead of 0.1. The tests are the source of truth for this report:

- `crates/fluent2-egui/tests/a11y_contrast.rs` computes the WCAG ratio of every foreground/background token pair the components paint, in light, dark and high contrast (HC). It fails on any new failure, and on any listed failure that has since been fixed. Print the table with `cargo test -p fluent2-egui --test a11y_contrast -- --ignored --nocapture`.
- `crates/fluent2-egui/tests/a11y_keyboard.rs` tabs through a page with every basic control in all three themes. It checks that each stop is a named AccessKit node with the right role and that the Fluent focus ring is painted. It also checks that toasts, the spinner and the indeterminate progress bar stay still under reduced motion.

## 1. Contrast table

Thresholds: text 4.5:1 (WCAG 1.4.3), non-text 3:1 (1.4.11). Disabled pairs are reported but not required. Translucent backgrounds are composed over `neutral_background_1`. **✗** marks a failure.

| component | part | kind | foreground | background | light | dark | hc |
|---|---|---|---|---|---|---|---|
| Theme | body text on window | text 4.5 | `neutral_foreground_1` | `neutral_background_1` | 15.52 | 14.55 | 21.00 |
| Theme | body text on panel | text 4.5 | `neutral_foreground_1` | `neutral_background_2` | 14.87 | 16.48 | 21.00 |
| Button | label | text 4.5 | `neutral_foreground_1` | `neutral_background_1` | 15.52 | 14.55 | 21.00 |
| Button | label, hover | text 4.5 | `neutral_foreground_1_hover` | `neutral_background_1_hover` | 14.24 | 10.86 | 14.37 |
| Button | label, pressed | text 4.5 | `neutral_foreground_1_pressed` | `neutral_background_1_pressed` | 11.76 | 16.48 | 14.37 |
| Button | border | non-text 3 | `neutral_stroke_1` | `neutral_background_1` | **1.53 ✗** | **2.53 ✗** | 21.00 |
| Button | primary label | text 4.5 | `neutral_foreground_on_brand` | `brand_background` | 5.38 | 6.66 | 21.00 |
| Button | primary label, hover | text 4.5 | `neutral_foreground_on_brand` | `brand_background_hover` | 6.66 | 5.38 | 14.37 |
| Button | primary label, pressed | text 4.5 | `neutral_foreground_on_brand` | `brand_background_pressed` | 11.65 | 11.65 | 14.37 |
| Button | primary fill vs surface | non-text 3 | `brand_background` | `neutral_background_1` | 5.38 | **2.18 ✗** | 21.00 |
| Button | subtle label | text 4.5 | `neutral_foreground_2` | `subtle_background` | 10.05 | 10.01 | 21.00 |
| Button | subtle label, hover | text 4.5 | `neutral_foreground_2_hover` | `subtle_background_hover` | 14.24 | 11.73 | 14.37 |
| Button | subtle label, brand hover | text 4.5 | `neutral_foreground_2_brand_hover` | `subtle_background_hover` | 4.94 | **4.18 ✗** | 14.37 |
| Button | disabled label | disabled | `neutral_foreground_disabled` | `neutral_background_disabled` | 1.65 | 2.76 | 13.98 |
| Checkbox | unchecked box | non-text 3 | `neutral_stroke_accessible` | `neutral_background_1` | 6.19 | 6.48 | 21.00 |
| Checkbox | checked fill | non-text 3 | `compound_brand_background` | `neutral_background_1` | 5.38 | 5.18 | 14.37 |
| Checkbox | check mark | non-text 3 | `neutral_foreground_inverted` | `compound_brand_background` | 5.38 | 5.53 | 14.37 |
| Checkbox | label | text 4.5 | `neutral_foreground_1` | `neutral_background_1` | 15.52 | 14.55 | 21.00 |
| Radio | ring | non-text 3 | `neutral_stroke_accessible` | `neutral_background_1` | 6.19 | 6.48 | 21.00 |
| Radio | checked dot | non-text 3 | `compound_brand_foreground_1` | `neutral_background_1` | 5.38 | 5.18 | 14.37 |
| Switch | off track | non-text 3 | `neutral_stroke_accessible` | `neutral_background_1` | 6.19 | 6.48 | 21.00 |
| Switch | on track | non-text 3 | `compound_brand_background` | `neutral_background_1` | 5.38 | 5.18 | 14.37 |
| Switch | on thumb | non-text 3 | `neutral_foreground_inverted` | `compound_brand_background` | 5.38 | 5.53 | 14.37 |
| Switch | off thumb | non-text 3 | `neutral_foreground_3` | `neutral_background_1` | 6.19 | 6.48 | 21.00 |
| Input | value | text 4.5 | `neutral_foreground_1` | `neutral_background_1` | 15.52 | 14.55 | 21.00 |
| Input | placeholder | text 4.5 | `neutral_foreground_4` | `neutral_background_1` | 4.95 | 5.11 | 21.00 |
| Input | underline | non-text 3 | `neutral_stroke_accessible` | `neutral_background_1` | 6.19 | 6.48 | 21.00 |
| Input | focus underline | non-text 3 | `compound_brand_stroke` | `neutral_background_1` | 5.38 | 5.18 | 14.37 |
| Input | filled-darker value | text 4.5 | `neutral_foreground_1` | `neutral_background_3` | 14.24 | 18.42 | 21.00 |
| Field | hint | text 4.5 | `neutral_foreground_3` | `neutral_background_1` | 6.19 | 6.48 | 21.00 |
| Field | error message | text 4.5 | `status_danger_foreground_1` | `neutral_background_1` | 7.12 | **4.16 ✗** | 21.00 |
| Link | text | text 4.5 | `brand_foreground_link` | `neutral_background_1` | 6.66 | 5.18 | 19.56 |
| Link | text, hover | text 4.5 | `brand_foreground_link_hover` | `neutral_background_1` | 7.87 | 6.00 | 19.56 |
| Slider | rail | non-text 3 | `neutral_stroke_accessible` | `neutral_background_1` | 6.19 | 6.48 | 21.00 |
| Slider | thumb and fill | non-text 3 | `compound_brand_background` | `neutral_background_1` | 5.38 | 5.18 | 14.37 |
| ProgressBar | fill vs track | non-text 3 | `compound_brand_background` | `neutral_background_6` | 4.31 | 4.50 | 14.37 |
| Spinner | arc | non-text 3 | `brand_stroke_1` | `neutral_background_1` | 5.38 | 5.18 | 21.00 |
| SpinButton | arrows | text 4.5 | `neutral_foreground_2` | `neutral_background_1` | 10.05 | 10.01 | 21.00 |
| Dropdown / Combobox | placeholder | text 4.5 | `neutral_foreground_4` | `neutral_background_1` | 4.95 | 5.11 | 21.00 |
| Dropdown / Combobox | chevron | text 4.5 | `neutral_foreground_3` | `neutral_background_1` | 6.19 | 6.48 | 21.00 |
| Focus ring | outer ring | non-text 3 | `stroke_focus_2` | `neutral_background_1` | 21.00 | 14.55 | 14.37 |
| Focus ring | outer ring on panel | non-text 3 | `stroke_focus_2` | `neutral_background_2` | 20.12 | 16.48 | 14.37 |
| Focus ring | inner ring on brand | non-text 3 | `stroke_focus_1` | `brand_background` | 5.38 | 3.15 | 21.00 |
| Menu | item | text 4.5 | `neutral_foreground_1` | `neutral_background_1` | 15.52 | 14.55 | 21.00 |
| Menu | item, hover | text 4.5 | `neutral_foreground_1_hover` | `neutral_background_1_hover` | 14.24 | 10.86 | 14.37 |
| Menu | secondary text | text 4.5 | `neutral_foreground_3` | `neutral_background_1` | 6.19 | 6.48 | 21.00 |
| Menu / Popover | surface border | non-text 3 | `neutral_stroke_2` | `neutral_background_1` | **1.32 ✗** | **1.86 ✗** | 21.00 |
| Tooltip | text | text 4.5 | `neutral_foreground_1` | `neutral_background_1` | 15.52 | 14.55 | 21.00 |
| Tooltip | inverted text | text 4.5 | `neutral_foreground_static_inverted` | `neutral_background_static` | 12.63 | 10.86 | 21.00 |
| TabList | selected label | text 4.5 | `neutral_foreground_1` | `neutral_background_1` | 15.52 | 14.55 | 21.00 |
| TabList | label | text 4.5 | `neutral_foreground_2` | `neutral_background_1` | 10.05 | 10.01 | 21.00 |
| TabList | subtle selected label | text 4.5 | `neutral_foreground_1_selected` | `subtle_background_selected` | 13.02 | 12.63 | 14.37 |
| Dropdown / Combobox | option, hover | text 4.5 | `neutral_foreground_1_hover` | `subtle_background_hover` | 14.24 | 11.73 | 14.37 |
| DataGrid | sortable header, hover | text 4.5 | `neutral_foreground_1_hover` | `subtle_background_hover` | 14.24 | 11.73 | 14.37 |
| DataGrid | row, hover | text 4.5 | `neutral_foreground_1_hover` | `subtle_background_hover` | 14.24 | 11.73 | 14.37 |
| Ribbon gallery | scroll button, hover | text 4.5 | `neutral_foreground_2_hover` | `subtle_background_hover` | 14.24 | 11.73 | 14.37 |
| TabList | selected indicator | non-text 3 | `compound_brand_stroke` | `neutral_background_1` | 5.38 | 5.18 | 14.37 |
| Card | text on filled-alternative | text 4.5 | `neutral_foreground_1` | `neutral_background_2` | 14.87 | 16.48 | 21.00 |
| Card | border | non-text 3 | `neutral_stroke_1` | `neutral_background_1` | **1.53 ✗** | **2.53 ✗** | 21.00 |
| Card | content, hover | text 4.5 | `neutral_foreground_1_hover` | `neutral_background_1_hover` | 14.24 | 10.86 | 14.37 |
| Card | content, pressed | text 4.5 | `neutral_foreground_1_pressed` | `neutral_background_1_pressed` | 11.76 | 16.48 | 14.37 |
| Card | content, selected | text 4.5 | `neutral_foreground_1_selected` | `neutral_background_1_selected` | 13.02 | 11.73 | 14.37 |
| Card | filled-alternative content, hover | text 4.5 | `neutral_foreground_1_hover` | `neutral_background_2_hover` | 13.62 | 12.63 | 14.37 |
| Card | filled-alternative content, selected | text 4.5 | `neutral_foreground_1_selected` | `neutral_background_2_selected` | 12.44 | 13.58 | 14.37 |
| Card | subtle content, hover | text 4.5 | `neutral_foreground_1_hover` | `subtle_background_hover` | 14.24 | 11.73 | 14.37 |
| Card | subtle content, selected | text 4.5 | `neutral_foreground_1_selected` | `subtle_background_selected` | 13.02 | 12.63 | 14.37 |
| Ribbon gallery | tile content, hover | text 4.5 | `neutral_foreground_1_hover` | `subtle_background_hover` | 14.24 | 11.73 | 14.37 |
| Ribbon gallery | tile content, pressed | text 4.5 | `neutral_foreground_1_pressed` | `subtle_background_pressed` | 11.76 | 13.58 | 14.37 |
| Ribbon gallery | tile content, selected | text 4.5 | `neutral_foreground_1_selected` | `subtle_background_selected` | 13.02 | 12.63 | 14.37 |
| MessageBar | info text | text 4.5 | `neutral_foreground_1` | `neutral_background_3` | 14.24 | 18.42 | 21.00 |
| MessageBar | success text | text 4.5 | `neutral_foreground_1` | `status_success_background_1` | 14.55 | 16.49 | 21.00 |
| MessageBar | warning text | text 4.5 | `neutral_foreground_1` | `status_warning_background_1` | 14.87 | 14.20 | 21.00 |
| MessageBar | error text | text 4.5 | `neutral_foreground_1` | `status_danger_background_1` | 14.27 | 17.33 | 21.00 |
| MessageBar | success icon | non-text 3 | `status_success_foreground_1` | `status_success_background_1` | 5.89 | 6.06 | 21.00 |
| MessageBar | warning icon | non-text 3 | `status_warning_foreground_1` | `status_warning_background_1` | 4.85 | 6.97 | 21.00 |
| MessageBar | error icon | non-text 3 | `status_danger_foreground_1` | `status_danger_background_1` | 6.55 | 4.96 | 21.00 |
| MessageBar | success border | non-text 3 | `status_success_border_1` | `neutral_background_1` | **1.64 ✗** | **2.71 ✗** | 21.00 |
| MessageBar | error border | non-text 3 | `status_danger_border_1` | `neutral_background_1` | **1.88 ✗** | **2.40 ✗** | 21.00 |
| TitleBar | title | text 4.5 | `neutral_foreground_1` | `neutral_background_1` | 15.52 | 14.55 | 21.00 |
| TitleBar | caption glyph, hover | text 4.5 | `neutral_foreground_1_hover` | `subtle_background_hover` | 14.24 | 11.73 | 14.37 |
| TitleBar | close glyph, hover | text 4.5 | `neutral_foreground_on_brand` | `status_danger_background_3` | 6.07 | 6.07 | 21.00 |
| QAT | overflow glyph | text 4.5 | `neutral_foreground_3` | `neutral_background_1` | 6.19 | 6.48 | 21.00 |
| Ribbon | tab label | text 4.5 | `neutral_foreground_2` | `neutral_background_1` | 10.05 | 10.01 | 21.00 |
| Ribbon | selected tab label | text 4.5 | `brand_foreground_1` | `neutral_background_1` | 5.38 | 5.18 | 21.00 |
| Ribbon | contextual tab, success | text 4.5 | `status_success_foreground_1` | `neutral_background_1` | 6.28 | 5.34 | 21.00 |
| Ribbon | contextual tab, warning | text 4.5 | `status_warning_foreground_1` | `neutral_background_1` | 5.06 | 7.14 | 21.00 |
| Ribbon | contextual tab, danger | text 4.5 | `status_danger_foreground_1` | `neutral_background_1` | 7.12 | **4.16 ✗** | 21.00 |
| Ribbon | File tab label | text 4.5 | `neutral_foreground_on_brand` | `brand_background` | 5.38 | 6.66 | 21.00 |
| Ribbon | group label | text 4.5 | `neutral_foreground_2` | `neutral_background_1` | 10.05 | 10.01 | 21.00 |
| Ribbon | control label, hover | text 4.5 | `neutral_foreground_1_hover` | `subtle_background_hover` | 14.24 | 11.73 | 14.37 |
| Ribbon gallery | item border, selected | non-text 3 | `brand_stroke_1` | `neutral_background_1` | 5.38 | 5.18 | 21.00 |
| KeyTips | badge letter | text 4.5 | `neutral_foreground_1` | `neutral_background_1` | 15.52 | 14.55 | 21.00 |
| KeyTips | typed prefix | text 4.5 | `brand_foreground_1` | `neutral_background_1` | 5.38 | 5.18 | 21.00 |
| KeyTips | badge border | non-text 3 | `neutral_stroke_1` | `neutral_background_1` | **1.53 ✗** | **2.53 ✗** | 21.00 |
| Backstage | item, selected | text 4.5 | `neutral_foreground_1_selected` | `subtle_background_selected` | 13.02 | 12.63 | 14.37 |
| Backstage | selected bar | non-text 3 | `brand_foreground_1` (hc: `neutral_foreground_1_selected`) | `subtle_background_selected` | 4.52 | 4.50 | 14.37 |
| StatusBar | text | text 4.5 | `neutral_foreground_2` | `neutral_background_2` | 9.63 | 11.34 | 21.00 |
| DataGrid | header | text 4.5 | `neutral_foreground_1` | `neutral_background_1` | 15.52 | 14.55 | 21.00 |
| DataGrid | row, selected | text 4.5 | `neutral_foreground_1_selected` | `subtle_background_selected` | 13.02 | 12.63 | 14.37 |
| TreeView / List | row | text 4.5 | `neutral_foreground_2` | `neutral_background_1` | 10.05 | 10.01 | 21.00 |
| TreeView / List | row, hover | text 4.5 | `neutral_foreground_2_hover` | `subtle_background_hover` | 14.24 | 11.73 | 14.37 |
| TreeView / List | row, selected | text 4.5 | `neutral_foreground_1_selected` | `subtle_background_selected` | 13.02 | 12.63 | 14.37 |
| TreeView / List | secondary text | text 4.5 | `neutral_foreground_3` | `neutral_background_1` | 6.19 | 6.48 | 21.00 |
| NavigationView | item | text 4.5 | `neutral_foreground_2` | `neutral_background_4` | 8.82 | 13.62 | 21.00 |
| NavigationView | item, hover | text 4.5 | `neutral_foreground_2_hover` | `neutral_background_4_hover` | 14.87 | 16.48 | 14.37 |
| NavigationView | item, selected | text 4.5 | `neutral_foreground_2_brand_selected` | `neutral_background_4_selected` | 5.38 | 6.20 | 14.37 |
| NavigationView | header | text 4.5 | `neutral_foreground_3` | `neutral_background_4` | 5.43 | 8.82 | 21.00 |
| SplitView | splitter | non-text 3 | `neutral_stroke_1` | `neutral_background_1` | **1.53 ✗** | **2.53 ✗** | 21.00 |
| SplitView | splitter, focused | non-text 3 | `brand_stroke_1` | `neutral_background_1` | 5.38 | 5.18 | 21.00 |

## 2. Failures

### 2.1 Decorative 1 px edges (light and dark), accepted

Fluent's `neutral_stroke_1`, `neutral_stroke_2` and `status_*_border_1` measure 1.3 to 2.7:1 against the surface. These are the borders of Button, Card, Menu/Popover, MessageBar, KeyTip badges and the SplitView splitter. WCAG 1.4.11 does not require them: each control is identified by its label, fill or icon, and the controls whose state depends on an edge (Checkbox, Radio, Switch, Input, Slider) use `neutral_stroke_accessible` (≥ 6:1). In HC all of these strokes are white (21:1). Listed in `KNOWN_FAILURES`.

### 2.2 Dark-theme token values: decision open (WI-1844)

- `status_danger_foreground_1` text on the surface is **4.16:1**. It is used for the Field error message and the danger contextual ribbon tab.
- `neutral_foreground_2_brand_hover` on the subtle hover fill is **4.18:1** (subtle button, hover only).
- The primary button fill against the dark surface is 2.18:1. It is not required, because the white label on the fill is 6.66:1.

### 2.3 Fixed in this PR

| problem | themes | fix |
|---|---|---|
| Text on hover, pressed and selected fills kept the resting foreground. In HC those fills are the highlight colour, so text was white on cyan, **1.46:1** (Button, SplitButton, Menu, TitleBar caption buttons, ribbon controls and groups, gallery scroll buttons, Backstage, DataGrid rows and sortable headers, TreeView, List, Dropdown options, subtle TabList, NavigationView) | HC | Use `neutral_foreground_{1,2}_{hover,pressed,selected}`. These equal the resting tokens in light and dark, so nothing changes there. DataGrid and List set `override_text_color` for caller-drawn cells. |
| NavigationView selected item: `neutral_foreground_2_brand_selected` with no fill, **1.00:1** in HC | HC | Paint `neutral_background_4_selected` under the selected item, as Fluent's nav item does |
| Backstage selected bar matched the highlight fill (1.46:1) | HC | The bar uses `neutral_foreground_1_selected` in HC |
| Checkbox, Switch, Slider and ProgressBar filled with `brand_background`: 2.18:1 against the dark surface, check mark and thumb 2.33:1 | dark | Use `compound_brand_background*`, the token Fluent uses for these controls (5.2:1 or better) |

## 3. High contrast: visible boundaries

Fluent HC draws boundaries with strokes, not fills. Every component was checked:

- **Fixed:** Dialog had no edge on a black page, and the inverted Tooltip had none either. Both now carry `transparent_stroke`, which is clear in light and dark and white in HC, matching Fluent's surface border. The ProgressBar rail was black on black, so an empty bar was invisible; it now has a `transparent_stroke` outline. A disabled Primary button lost its edge in HC; it now keeps `neutral_stroke_disabled`, as SplitButton already did.
- **Already fine:** Input, Dropdown, Combobox, SpinButton, Checkbox, Radio, Switch, Slider rail, Card (Filled uses `transparent_stroke`, Outline uses `neutral_stroke_1`), Menu, Popover, normal Tooltip, MessageBar, Toast, KeyTips, TitleBar, Ribbon, StatusBar, DataGrid, TreeView, List, NavigationView, SplitView. Subtle and Transparent buttons have no edge at rest, as in Fluent.
- **Fixed (WI-1845):** CardHeader text and icon on a hovered, pressed or selected Card, and caller-drawn ribbon gallery tile content (through `override_text_color`), take `neutral_foreground_1_{hover,pressed,selected}`. The TabList (subtle) and NavigationView selected indicator takes `neutral_foreground_1_selected` in HC, where `compound_brand_stroke` matched the highlight fill.

## 4. Screen reader

### 4.1 Orca (Linux): done

**Setup:** Orca (Ubuntu 26.04 packages) with at-spi2-core on Xvfb, in a throwaway container. The release binaries talk to AT-SPI through eframe's AccessKit bridge. Keys were driven with `xdotool`, and the announcements below are Orca's `SPEECH OUTPUT` lines from `orca --debug-file`, grouped per key press. Targets: `cargo run -p gallery` (every page), `cargo run -p gallery --example editor`, and the new `cargo run -p gallery --example a11y_walk`. That example puts every control, surface and data view on one page, because the gallery has no controls page.

What Orca announces at the PR head:

| component | Orca says (Tab or arrow onto it) | verdict |
|---|---|---|
| Button | "Save, button" | ok |
| ToggleButton | "Bold, toggle button not pressed" | ok |
| Checkbox | "Agree to the terms, check box not checked" | ok |
| RadioGroup | "Small, selected radio button" (active descendant) | ok; the group has no name (WI-1843) |
| Switch | "Wi-Fi, toggle button not pressed" | ok. AccessKit's AT-SPI bridge maps the Switch role to toggle button (upstream) |
| Input | "Name, entry" (from the placeholder) | ok |
| Textarea | "entry" | **gap:** no name (WI-1843) |
| Slider | "Slider, slider, 100, 25 percent" | value and percent ok after the fix; the name is generic (WI-1843) |
| SpinButton | "spin button, 1.00" | **gap:** no name (WI-1843) |
| Dropdown / Combobox | "Colour, combo box" / "Font, combo box" | ok |
| TabList | "Home, page tab" | ok |
| Link | "Help, link" | ok |
| Menu (MenuItem) | "New." / "Close." | ok |
| Dialog | open: "Save, button"; Tab: "Save changes?, dialog"; Escape: "Open dialog, button" (focus restored) | fixed (WI-1850): the dialog is described by its body (`described_by`), and `Dialog::description` sets a plain-text description; not re-walked with Orca |
| Toast | "Saved" spoken on push, with no focus move | ok |
| MessageBar, ProgressBar | not Tab stops; present as "notification 'Storage. Disk is almost full'" and "progress bar 40%" | ok |
| TreeView | "Files, tree" then "Documents", "Report" on the arrow keys | ok; level and expanded state not spoken |
| List | "Folders, list box"; arrows silent | **gap:** rows have no name (WI-1848) |
| DataGrid | focus enters the grid, arrows silent | **gap:** cells and rows have no name (WI-1848) |
| Title bar / QAT | "Save, button", "Undo", "Redo", "Customize", "Search, entry", "Minimize", "Maximize", "Close" | ok |
| Ribbon | "File, button", "Home, page tab", "Paste, button", … "Font, combo box", "Size, combo box", "Bold, toggle button not pressed", "More, button", "Font dialog, button" | ok |
| F6 regions | Save → Close → Select all → document entry → Zoom out | ok |
| KeyTips (Alt) | nothing | **gap** (WI-1849) |
| Backstage | Space on File: "frame", focus lost | fixed (WI-1850): focus moves to the selected entry on open, covered by `a11y_keyboard.rs`; not re-walked with Orca |
| Status bar | "100%, button", "Zoom in", "Zoom, slider, 100, 25 percent", "Zoom out" | ok |

**Fixed in this PR because of the walk**, each re-walked afterwards:

- **Window edges and title-bar drag region:** these were 9 silent Tab stops ahead of the QAT in the editor. They are now pointer-only (`Sense::CLICK | Sense::DRAG`).
- **Combobox chevron:** it was a silent extra Tab stop. It is now pointer-only; Alt+Down opens the list.
- **TreeView, List, DataGrid:** arrow keys moved focus with no event for screen readers. The focused row or cell is now the active descendant. TreeView rows are announced. DataGrid and List cell content now hangs under its cell or option node (`UiBuilder::accessibility_parent`), which WI-1848 builds on.
- **Slider:** the AccessKit node now carries min, max and step. Orca read a zoom slider at 100 (on a 10..=400 range) as "0 percent".
- **Toast:** it now sets a live region (polite, or assertive for warning and error), so it is spoken.
- **MenuItem:** it set a selected state even when not checkable, so Orca read every item as "toggle button not pressed". It now has the MenuItem role, or MenuItemCheckBox when checkable.
- **Dialog:** Escape left focus nowhere. egui drops a focus request for a widget that was blocked by the modal on the previous frame, so the request is repeated on the first frame after the close. The old `focus_returns_to_opener` test hid this, because it re-requested the opener's focus every frame. `tests/a11y_keyboard.rs::escape_returns_focus_to_a_keyboard_opener` covers it now.
- **Gallery:** the Accent swatch was read as "button"; it is now labelled by its label. The pages panel had a focusable resize handle with no name; the panel is no longer resizable.
- Found before the walk, and confirmed by it: Switch role, TabList and RadioGroup active descendants, SpinButton as a single Tab stop.

### 4.2 Narrator / NVDA (Windows): **blocked**

The walk on the estate Windows box (desktop-tlk1gff) has not been done. Reaching the box needs its SSH key (Infisical `100.109.218.11_SSH`), and this session's permission policy refused that fetch. The session reported the block in Vogt. Narrator and NVDA also need a logged-in interactive desktop on the box. The same three targets and key sequences apply. UIA receives the same AccessKit tree as AT-SPI, so roles and names should match §4.1, apart from Switch, which UIA maps natively.

### 4.3 AccessKit tree (kittest)

`tests/a11y_keyboard.rs` checks roles, names and focus in-process, as described in §5.

## 5. Keyboard and focus

- **Fixed:** Checkbox, Radio and Switch drew the focus ring only on the frame that had a key event, so it vanished one frame after Tab. They now use the shared sticky `foundation::focus::focus_visible`.
- `tests/a11y_keyboard.rs` checks that Tab reaches Button, ToggleButton, Checkbox, RadioGroup, Switch, Input, Slider, SpinButton, Dropdown, TabList and Link in light, dark and HC, with a ring on each stop. Office chrome and data views are covered by their own tests: F6 regions and KeyTips (`keytips.rs`), ribbon (`ribbon*.rs`), Backstage, Dialog focus trap, DataGrid, TreeView, List and NavigationView key maps.

## 6. Reduced motion

- Added `foundation::motion::reduced_motion(ctx)` for components that run their own clock.
- **Fixed:** toasts still slid in and out, because the slide used its own clock and only the opacity went through `animate`. They are now static.
- **Fixed:** the indeterminate ProgressBar jumped between the two ends of the rail every frame, so it flashed. It now shows a still centred segment and stops asking for repaints.
- **Fixed:** the Spinner asked for a repaint every frame while drawing a static arc.
- Already correct, since everything else goes through `motion::animate`: Switch thumb, Input accent, TreeView chevron, NavigationView pane and indicator, TabList indicator.
