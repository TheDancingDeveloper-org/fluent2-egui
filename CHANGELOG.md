# Changelog

All notable changes are recorded here, following [Keep a Changelog](https://keepachangelog.com/).

## [Unreleased]

Entry format and the per-PR fragment workflow are in [`changelog.d/README.md`](changelog.d/README.md).

## [0.4.2] - 2026-10-30

### Fixed

- Builds with `RUSTFLAGS=-D warnings` under `--no-default-features` and single optional features (including `html`), and on `wasm32-unknown-unknown`. Colour emoji still installs when `bundled-font` is on.
- `HtmlView` link clicks reach the link again. The view's drag-selection widget covers the whole document; registered after a link, egui drops the click underneath it. The drag is registered first, using the previous frame's block rectangles, and each text block only senses hover.
- A `Dropdown`, `Combobox` or `TagPicker` inside a modal `Dialog` can be picked again (regression in 0.4.1). The listbox layer is now registered in egui's area order directly above the layer it opens from, so the dialog's modal no longer blocks clicks on its rows.

## [0.4.1] - 2026-10-29

### Added

- Colour emoji fallback (Noto Emoji COLR/CPAL, OFL-1.1) and `MissingGlyphPolicy::Tofu` on install, so production text never panics on a missing glyph. The face is the optional `color-emoji` feature (off by default, and not compiled into wasm32), so the gallery wasm stays inside its compressed-size budget. egui's monochrome emoji faces still cover the glyphs when the feature is off.
- `EmojiPicker`: search, categories, recent emoji, keyboard movement and AccessKit names.
- `GroupedList`: a virtualised list of expandable groups. Only the rows in view are built, the current group's header sticks to the top while its rows scroll, and Left, Right, Enter and a header click expand and collapse. Headers are AccessKit groups; rows are list-box options.
- Inter-pane drag and drop: `data::pane_drag` carries a typed payload from a `DragSource` to a `DropTarget` in another pane, with hover and accept feedback, a keyboard "Move to" menu, and a polite AccessKit live region. The gallery's Pane drag page demonstrates it.
- `HtmlView` selects text across blocks (Copy places the range on the clipboard), highlights a case-insensitive find query and steps through its matches, reports the link under the pointer, and exposes the message as an AccessKit document of headings, paragraphs and links.
- `Textarea` and `RichTextEditor` take an optional `SpellChecker` the app supplies. The library ships no dictionary. Misspellings are underlined, a secondary click or Shift+F10 opens suggestions (replace or ignore), and the checker runs only when the text changes. `WordListChecker` is the small stand-in the gallery and tests use.

### Fixed

- A dialog whose body fits no longer wraps that body in a `ScrollArea`. egui 0.37 clips scroll children to the viewport even when the content fits, and on the first frame that viewport is shorter than the body, so a button in the body never received the click, was missing from AccessKit, and the click closed the dialog through the backdrop. The body scrolls only once its measured height is taller than the room left under the title and actions.
- `Input` inside a `Field` copies the field's label onto the text box's AccessKit node, so a field with no input label of its own still has an accessible name.

## [0.4.0] - 2026-10-29

### Breaking

- Requires egui, eframe, egui_extras and egui_kittest 0.37 (was 0.36). egui's MSRV stays 1.95, matching this crate.
- Callers that name `egui::WidgetType` must use `egui::Role` (a re-export of `accesskit::Role`). Checkboxes are `Role::CheckBox`. Selectable rows and nav rows are `Role::ListBoxOption`, not `Role::Button`.
- `Id::new` is deprecated. New ids in this crate use `Id::unique`. `FontId::new` and `LayerId::new` are unchanged.
- `egui::Event::MouseWheel` now carries `source: egui::MouseWheelSource`.
- Scroll `State::offset` is private. Read `unclamped_offset()` instead.
- `Ui::id` is `Ui::scope_id`.
- egui_kittest panics after settle when a visible input widget (including `ListBoxOption`, `TextInput` and `MultilineTextInput`) has no accessible name. Set a label, placeholder, or AccessKit label.
- `DialogKind::NonModal` is a real non-modal dialog: no backdrop, and clicks reach the page behind it. Escape still closes it; a click outside does not. `DialogKind::Dismissible` is now a deprecated alias of `NonModal`. `Modal` and `Alert` still block the page. Code that treated `NonModal` as an alias of `Dismissible` (a modal surface that only Escape dismisses) changes behaviour.

### Changed

- Accessible names are set on gallery inputs that egui_kittest 0.37 now rejects when a visible input role has no label or placeholder (the cause of issue #146).
- The slider rail, brand fill and thumb stroke follow Fluent hover and pressed alias colours (`neutral_stroke_accessible_hover` / `_pressed`, `compound_brand_background_hover` / `_pressed`, `neutral_stroke_1_hover` / `_pressed`). Disabled still uses the disabled aliases and wins over hover and pressed. Sizes stay on `Theme.slider`.
- Workspace `repository` and the gallery terminal demo link now point at `TheDancingDeveloper-org/fluent2-egui`.

### Fixed

- Installing the UI font (Segoe UI Variable or Segoe UI on Windows, Inter elsewhere) no longer drops egui's emoji fallback faces. egui 0.37 panics when a glyph such as U+26A0 has no face, which the Windows CI job hit in the combobox tests.
- Regenerated the Fluent System Icons subset from microsoft/fluentui-system-icons @ 64e5aa757d1b70c24a283256ddcb8269d0f58773 (was a347af95478f989038cccbf085978865264c048a). The committed SVGs and catalogue are unchanged. The MIT notice now names the new revision.
- Recorded the bundled Inter Regular and SemiBold faces against the v4.1 release commit e3a3d4c57d5ecc01453a575621882a384c1995a3. The shipped TTF files and OFL-1.1 text are unchanged and match Inter-4.1.zip (extras/ttf and LICENSE.txt).

## [0.3.0] - 2026-10-29

### Upgrading from 0.2

Breaking changes in 0.3.0:

- `DialogActions::primary` and `secondary` take a stable id and a display text. `CloseReason::Button` carries that id.

  Before:

  ```rust
  actions.primary("Save");
  actions.secondary("Cancel");
  match reason {
      CloseReason::Button(text) if text == "Save" => save(),
      _ => {}
  }
  ```

  After:

  ```rust
  actions.primary("save", "Save");
  actions.secondary("cancel", "Cancel");
  match reason {
      CloseReason::Button(id) if id == "save" => save(),
      _ => {}
  }
  ```

- `CloseReason` is no longer `Copy`. Match by reference, or clone it when you need to keep the value.

  Before:

  ```rust
  let reason: CloseReason = dialog.close_reason();
  let again = reason;
  ```

  After:

  ```rust
  // CloseReason is not Copy. Borrow it while matching, or clone it to keep it.
  match dialog.show(ctx, &mut open, |ui| body(ui)) {
      DialogResult::Open(_) => {}
      DialogResult::Closed(reason) => {
          let _kept = reason.clone();
          match &reason {
              CloseReason::Escape => {}
              CloseReason::Backdrop => {}
              CloseReason::Button(id) if id == "save" => save(),
              CloseReason::Button(_) | CloseReason::Programmatic => {}
          }
      }
  }
  ```

- `DialogKind::NonModal` is deprecated. Dialogs are always modal. `DialogKind` only chooses what dismisses the surface: `Modal` (backdrop click and Escape), `Dismissible` (Escape only), `Alert` (neither). `NonModal` is an alias of `Dismissible`.

  Before:

  ```rust
  Dialog::new(DialogKind::NonModal)
  ```

  After:

  ```rust
  Dialog::new(DialogKind::Dismissible)
  ```

- `Combobox<'a, T = &'a str>` gains a type parameter with default `T = &'a str`. Code that names the type with an explicit lifetime (`Combobox<'a>`) or a type list no longer matches; name `Combobox` or `Combobox<'a, &'a str>`. Options are borrowed for `'a` and are not copied into the widget.

  Before:

  ```rust
  let combo: Combobox = Combobox::new(id, &text);
  ```

  After (unchanged for the default, or name the option type):

  ```rust
  let combo: Combobox<'_, &str> = Combobox::new(id, &text);
  ```

- `web-sys` is no longer an optional dependency on wasm32, and it gains the `History`, `Location` and `Window` features. `wasm-bindgen` is now a direct dependency (it was already in the tree through egui). The `file-dialog` feature no longer enables `web-sys`; it still enables `rfd`, `pollster` and `wasm-bindgen-futures`.

  Before:

  ```toml
  fluent2-egui = { version = "0.2", features = ["file-dialog"] }
  # file-dialog turned on optional web-sys
  ```

  After:

  ```toml
  fluent2-egui = { version = "0.3", features = ["file-dialog"] }
  # web-sys is always linked on wasm32; file-dialog does not enable it
  ```

### Added
- Skeleton (`controls::Skeleton`, `SkeletonItem`, `SkeletonAnimation`, `SKELETON_SIZES`): a loading placeholder for lists and cards. Items are a full-width rectangle, a circle or a text line (a fraction of the width), stacked with `spacingVerticalS` and snapped to Fluent's size ramp (16 to 128). The fill is `neutral_stencil_1`. The default wave sweeps `neutral_stencil_2` across the items; `Pulse` fades between the two; `Static` does not move. Reduced motion holds still and repaints nothing. High contrast draws an outline in `neutral_stroke_1`. The whole skeleton is one busy ProgressIndicator named "Loading" (or the label given); the items are hidden from AccessKit. Gallery page "Skeleton" (WI-1935).
- Tag, InteractionTag and TagGroup (`controls::Tag`, `InteractionTag`, `TagGroup`, `TagAppearance`, `TagSize`, `TagResponse`, `TagGroupResponse`), for filter chips and labels. A tag is filled, outline or brand, extra-small (20), small (24) or medium (32), with an optional icon or `Avatar`, secondary text (under the primary at medium size), and an optional dismiss button named "Remove <text>"; Delete or Backspace on a focused tag dismisses it and reports `dismissed`, and the app removes it. An InteractionTag's body is a Button with a separate dismiss target, and `.selected(true)` shows it in the brand style, AccessKit-toggled. A TagGroup is an AccessKit List of ListItems with one Tab stop: the arrow keys, Home and End move between tags, and after a dismiss focus moves to the next tag (or the previous, for the last). High contrast adds an accessible outline. Gallery: "Tags" on Basic controls (WI-1934).
- Breadcrumb (`controls::Breadcrumb`, `BreadcrumbSize`, `BreadcrumbDivider`, `BreadcrumbItemStyle`, `BreadcrumbResponse`): a path of items for file browsers and folder trees (rdpapp SFTP, WI-1936). Items are buttons or links; `.current(i)` picks the current item, which is the last one by default. It is plain text, not interactive, and marked `aria-current=page`. Dividers are chevrons or slashes, in small, medium (default) and large sizes. When the row is too narrow, the fewest middle items that fit fold into a "…" menu, and the current item is never folded. `show` reports the item clicked, including items in the menu. The row is an AccessKit Navigation landmark named "Breadcrumb". Gallery page "Breadcrumb".
- FileCard (`controls::FileCard`, `file_card_row`, `format_size`, `file_icon`): an attachment card for compose and reading panes (email-rs L7). The icon comes from the MIME type or the file extension (image, document, spreadsheet, archive, mail, calendar, generic). The name is cut with an ellipsis, with the full name as the tooltip, and the size reads like "1.2 MB" (1024-based). While a transfer runs, a progress bar (determinate or not) replaces the size. Remove and download are icon buttons named "Remove <name>" and "Download <name>"; activating an openable card reports `open`, and a click on an action never also opens it. The card is an AccessKit Button named "<name>, <size>" or "<name>, 40% transferred". `file_card_row` wraps cards with Fluent gaps. Gallery: "Attachments" on the Surfaces page (WI-1945).
- FileDropZone (`platform::file_drop`, behind `file-dialog`): a drop target for files dragged in from the OS, for attachments in compose (email-rs L6). It wraps any content. While files are dragged over the window, it shows a Fluent overlay (tinted fill, dashed brand border, icon and label). It reads dropped files off the UI thread: a worker thread over egui's `DroppedFile::bytes` on native, and `bytes_async` on wasm, where eframe already turns the browser `DataTransfer` into dropped files. Results arrive as `PickedFile`s in `received`, with read errors in `failed`. egui gives no drop position, so a zone takes a drop when the pointer was last inside it, or always with `window_wide(true)`. Each drop is read by one zone only: the first that takes it that frame. Files over `max_bytes` (default 50 MiB, `DEFAULT_MAX_BYTES`) are refused without being read; their size comes from file metadata on native and the browser `File` on wasm. A drop's files are read one at a time by a single worker. Gallery: a drop box on the File dialog page (WI-1944).
- TagPicker (`controls::TagPicker`, `TagSuggestion`, `TagPickerResponse`): a Fluent token input, for email recipients (email-rs L3). Tags have remove buttons (Tab stops named "Remove …"; Delete or Backspace on one removes it). Free text can be typed, and `,`, `;` or a line break, typed or pasted, adds each piece; a paste is read before the single-line field so line breaks still separate. Backspace in an empty field removes the last tag, and duplicates are skipped, ignoring case. Suggestions are app-owned and async: `query_changed` asks the app to fetch, it passes them back with `.suggestions_for(query, ..)` (or `.suggestions(..)` for a synchronous list) and shows a spinner with `.loading(true)`. Results for an older query, results while loading, and results on the frame the query changed are never offered, so Enter adds the typed text instead of a stale suggestion; Down, Up and Enter pick one, and a click picks too. `.validate(..)` marks tags in the danger style and as AccessKit-invalid. The field is an AccessKit ComboBox named by `.label` or its Field, with the tags as its value. The gallery shows a "To" field on Basic controls (WI-1941).
- Calendar, DatePicker and TimePicker (`controls`), for snooze, calendar and search dates (email-rs L5).
  - `CalendarDate` and `TimeOfDay` are plain, always-valid types with their own arithmetic (days since the epoch, `add_days`, `add_months` clamped to month ends, weekday) and parsing (`YYYY-MM-DD`, `D/M/YYYY` or `M/D/YYYY`; `14:30`, `1430`, `2:30 pm`). The new optional `chrono` feature converts them to and from `NaiveDate` and `NaiveTime`. Nothing reads the clock: the app passes "today".
  - `Calendar` is a six-week month grid: one Tab stop, arrows by day or week, Page Up and Page Down by month, Home and End to the week ends, Enter, Space or a click to pick. It takes a min/max range, a first weekday, localisable `CalendarStrings`, and a `day_content` hook for app-drawn marks such as event dots. It is an AccessKit Grid whose GridCells are named with the full date; the focused day is the active descendant.
  - `DatePicker` is an editable date field with a calendar button (and Alt+Down) that opens a Calendar popup. Text that does not parse keeps the old value and marks the field invalid.
  - `TimePicker` is a Combobox of times at a step (default 30 minutes), 24- or 12-hour, that also accepts typed times. Typed text stays as typed and is parsed only on Enter, on leaving the field, or when it matches a listed time.
  - Date arithmetic cannot overflow: `add_months` and `add_days` work in i64 and clamp to `MIN_YEAR`–`MAX_YEAR` (chrono's range).
  - Gallery page "Date and time" (WI-1943).
- README "Components" section: every public component by module (`controls`, `surfaces`, `data`, `office`, `platform`) with its Fluent 2 counterpart (or "egui-only"), its feature flag and its gallery page, plus the Fluent 2 components not yet provided and their Vogt items. `tests/readme_index.rs` fails when a component re-exported from `controls`, `surfaces` or `data` has no README row, unless the test lists it as a supporting type (WI-1933).
- HtmlView behind the new optional `html` feature (`fluent2_egui::data::html`), for HTML mail (email-rs L1). `HtmlDocument::parse` tokenises with `html5gum` (MIT, no dependencies, no unsafe) into a lossy block model. It keeps text, headings, paragraphs, lists, blockquotes, `pre`, rules, tables as layout grids (`colspan`, widths, cell and table backgrounds), links, images, and inline colour, highlight, weight, italic, underline, strike-through, size and alignment. It drops scripts, styles, `head`, forms, embedded content, tracking pixels and hidden preheaders. `HtmlView` paints it with the Fluent type ramp. Each text block is a selectable label, and every link is a focusable AccessKit Link reported through a `LinkPolicy`. Images go through an `ImagePolicy` (`Show`, `Embedded` for decoded `data:` images, `Blocked` with a placeholder and a count, `Hidden`); by default `data:` images show and remote ones are blocked. Colours are kept in light, remapped by lightness in dark until text reads at 4.5:1, and dropped in high contrast. `to_plain_text` is the text fallback. Builds for wasm32. Spec `docs/specs/html-view.md`; gallery page "HTML"; synthetic fixtures in `tests/fixtures/html` (WI-1939).
- Gallery: a "Basic controls" page (every button kind, link, checkbox including mixed, radio group, switch, slider, spin button, input, search box, textarea, dropdown, combobox, fields with hint and validation, badges, spinner, progress bars, dividers) and a "Surfaces" page (modal, non-modal, alert and tall dialogs; popover; context menu; tooltips; a toast and a message bar for each intent; cards; transparent and subtle tab lists). Every control on them has an accessible name, so they double as screen-reader walk pages, and both appear in the README gallery images. Found by the 2026-10-20 audit (WI-1930).
- Avatar, AvatarGroup and Persona (`controls`). Avatar: Fluent sizes 16–128 pt (`AVATAR_SIZES`; other sizes snap to the nearest); initials from the name (`initials`: first and last word, bracketed words skipped; an unbalanced bracket is ignored rather than hiding the rest of the name), or a person icon, or an app-loaded texture; circular or square; neutral, brand or colourful colours. Colourful picks one of Fluent's 30 named palette colours with a stable FNV-1a hash of the name (`palette_index`), so a name's colour may differ from Fluent web's. A presence badge (available, busy, away, do not disturb, offline). In high contrast every avatar, including a picture, gets an edge. Each avatar is an AccessKit Image named "Name, presence". AvatarGroup: spread or stacked, with a "+N" overflow whose name and tooltip list the hidden people, in a Group named "N people" or by `.label`. Persona: an avatar with up to three lines of text. All 30 palette pairs pass 4.5:1 in light and dark (`a11y_contrast`). Gallery page "Avatars" (WI-1932).
- KeyTips for screen readers: each ribbon tab, ribbon control, the File tab and each QAT item carries its full KeyTip sequence as its AccessKit access key ("Alt, H, V"). A polite live region (`keytips::live_region_id()`) says "KeyTips on" and "KeyTips off". `keytips::register_for` registers a target with the widget it runs, so app-registered targets get the access key too (WI-1849).
- `RadioGroup::label` and `Slider::label` set the accessible name (not drawn). Inside a `Field`, SpinButton, RadioGroup and Slider without a label of their own take the Field's label. The Slider still defaults to "Slider" outside a Field. `tests/a11y_keyboard.rs` now checks that every Tab stop has a name (WI-1843).
- `Dialog::description` sets a plain-text accessible description on the dialog node (WI-1850).
- Accessible text for data views: `DataGrid::cell_text(|row, column_id| ..)` labels each GridCell, and each Row reads its cells joined with ", "; `List::row_label(|row| ..)` labels each option. AccessKit derives no name from these nodes' children, so Orca was silent as focus moved through a grid or list. `Column::id()` returns a column's id. Used in the gallery DataGrid and List pages and in `examples/a11y_walk.rs` (WI-1848).
- `SpinButton::label` sets the field's accessible name (AccessKit label), like `Textarea::label`. It is not drawn. Found by the rdpapp port (WI-1815) (WI-1918).
- `DataGrid::drag_source` drags the selected rows (or the row under the pointer) out of the grid, with a ghost that counts them; `DataGrid::drag_label` sets its text ("3 torrents") and `DataGridResponse::dragging` reports the drag.
- `data::row_drag::DropTarget` makes any `Response`, such as a sidebar label, a drop target for grid rows: a brand highlight while a drag hovers it, the not-allowed cursor when it refuses, and a `RowDrop` with the row ids on release. Escape cancels.
- `data::row_drag::drop_menu` and `drop_choices` list the same targets in a context menu, the keyboard alternative to dragging.
- Gallery "Row drag" page: torrents dragged onto sidebar categories, a lockable category and a "Move to" menu.
- EditableList (`data::editable_list::EditableList`, `EditableListState`, `EditableListEvent`, `EditableListResponse`, `EditableKind`): a column of text or number rows edited in place. Enter or a click edits a row, Enter commits, Escape cancels, and F2 and Delete edit or remove the focused row. An add field and Add button append rows, and each row has a remove button. The widget holds only the draft text and emits `Added`, `Edited` and `Removed` events for the app to apply, so it never mutates its rows. Number rows refuse values that do not parse and keep the editor open with a message. The list is an AccessKit List and invalid fields are marked invalid. Gallery page "Editable list" (GitHub #117).
- URL state (`platform::hash_state`): `encode`, `decode` and `value` turn `key=value` pairs into a percent-encoded URL fragment and back, and `HashState` reads and writes the browser's `location.hash` on wasm (`replaceState`, so filters add no history entries). Native builds have no URL: reads return `None` and writes do nothing. The app polls `read_external` each frame, which picks up back and forward and pasted links. Spec: `docs/specs/url-state.md`. Gallery page "URL state" round-trips a tab and a filter through the address bar (#119).
- `Toaster::push_keyed` coalesces toasts: while a toast with the same key is showing, a push replaces its content, moves it to the newest slot and restarts its timeout, so a bulk action reads as one toast ("Paused 12 torrents").
- A collapsed "+N more" row above the toast stack counts toasts waiting behind the visible ones.
- `Toaster::set_max_visible` sets how many toasts the stack draws (default 3).
- `Toaster::history` returns the last 100 pushed toasts, oldest first, and `Toaster::clear_history` forgets them, for a status-bar history list.
- Gallery toasts page: a keyed "Pause 12 torrents" button, a "Push 50 toasts" button and a collapsed history list.
- `cargo bench -p fluent2-egui --bench large_views` prints frame time for a 10,000-row list, a 10,000-row data grid and a combobox of 10,000 options. It is not a required check (gh:TheDancingDeveloper-org/fluent2-egui#153).
- Downstream build fixtures check fluent2-egui with default features off, each optional feature on its own, a wasm consumer, and the declared MSRV (gh:TheDancingDeveloper-org/fluent2-egui#154).
- Gallery pages for NavigationView and SplitView, so both data views are reachable from the page list.
- DataGrid copy (issue #105): Ctrl+C on a focused grid copies the focused cell's text, or the selected rows (else the focused row) as TSV, to the clipboard. Works natively and on wasm through the browser Clipboard API. `DataGridResponse::copied` reports the text. Needs `DataGrid::cell_text`. `data_grid::{tsv_field, tsv_rows, rows_to_copy}` build the same text for a context menu. Gallery DataGrid page: right-click "Copy cell" and "Copy row(s)".
- Read-only text can be selected across labels with the mouse or Shift+arrows, and Ctrl+C copies it. Labels in cards, detail panes, and the markdown and HTML views are covered. Gallery page "Text selection" (issue #106).
- In-app find (`data::find`): a reusable `FindBar` on `SearchBox` that searches any number of `FindProvider`s (string slices, `TextLines`, `FindItems` for grids and other computed content, and `TerminalGrid` with the `terminal` feature). Ctrl+F (Cmd+F) opens it and focuses the field, Escape closes it, Enter and Shift+Enter step through the matches with wrap-around, and a counter reads "3 of 12" or "No results"; an optional "Aa" toggle makes it case-sensitive. `FindState::layout_job` highlights matches in palette yellow and the current one in marigold (the highlight colour and underlines in high contrast), and `FindState::take_reveal` reports the match to scroll to. Gallery page "Find" over a data grid, a list, text and a log (issue #107).
- `List::scroll_to_row` and `DataGrid::scroll_to_row`: scroll a virtualised row into the middle of the view, for example to show a find match.
- Feature `web-find`: `data::find::capture_browser_find` stops the browser's own find from taking Ctrl+F on wasm.
- DataGrid column layout: `ColumnLayout { order, hidden }` in `DataGridState` (serde-gated), `Column::required`, header right-click and Shift+F10 reported as `DataGridResponse::header_context_menu`, a `column_menu` chooser (show/hide, resize to fit, reset), and column reorder by dragging a header or Alt+Left/Right on a focused one (`columns_reordered`). The gallery grid now has twenty columns and the chooser.
- TreeView checks and cells: `TreeNode::checked(CheckState)` draws a tri-state checkbox per node, and a click on it or Space on the focused node reports `TreeViewResponse::check_toggled` (the app owns the states and their propagation). `TreeView::columns(&[TreeColumn], cell)` adds fixed-width trailing cell columns with a header row, drawn for visible rows only. Checked tree items expose AccessKit `toggled` (checked, not checked, partially checked). Gallery page "Tree view": 5,000 torrent files with size, progress and priority columns (issue #110).
- LineChart (`controls::LineChart`, `ChartSeries`, `TimeRange`, `ChartHover`,
  `LineChartResponse`, `time_ago`, `bytes_per_second`) behind the new `chart` feature: a
  multi-series time-series chart with a value axis, a time axis, a legend, a hover cursor with
  a value read-out and 5 min / 1 h / 6 h range chips (#111). `ChartSeries` is a fixed-capacity
  ring buffer, series take Fluent palette tokens (alternating colours and dashes in high
  contrast), an optional area fill, and each frame keeps at most two points per pixel column.
  The AccessKit node summarises the visible samples. Gallery page "Charts".
- Shortcut registry (`foundation::shortcuts::Shortcuts`, `Shortcut`, `ShortcutScope`, `ShortcutError`, `ShortcutsExt`): one source for key handling, menu captions, tooltips and help. `register` refuses a repeated id or keys another binding holds (Cmd and Ctrl count as one). `ctx.shortcut_pressed(id)` consumes the press once, lets Ctrl+Shift+S beat Ctrl+S, and never fires a binding without Ctrl, Cmd or Alt while a text field has focus. `shortcut_caption` and `shortcut_tooltip` use egui's platform formatting (⌘ on macOS). `MenuItem::bound(id)` draws the caption from the registry, and `surfaces::ShortcutOverlay` is a dialog listing every binding by group. Gallery page "Shortcuts" (#113).
- `PieceBar`: a strip over piece states, a BitTorrent bitfield or availability counts, bucketed
  to one segment per pixel, with an AccessKit label carrying the percent done.
- `PieceMap`: a grid with one cell per piece, a hover read-out and a legend with counts.
- `PieceState` (`Missing`, `Downloading`, `Have`) for both widgets.
- MenuBar (`office::menu_bar::MenuBar`, `MenuBarUi`): the classic File / Edit / View / Tools / Help bar of top-level menus holding Fluent `MenuItem`s, submenus and dividers. Titles mark mnemonics with `&` (`"&File"`). Alt pressed alone or F10 focuses the bar and underlines mnemonics; Alt+letter opens a menu from anywhere; Left and Right move between menus and switch the open one; Down, Enter or Space open the focused menu with focus on its first item; Esc closes the menu, then leaves the bar. With a menu open, hovering another title switches to it. AccessKit roles are MenuBar, MenuItem (with has-popup, expanded and access key) and Menu. It is a plain widget row, so it fits in a `TitleBar` slot. Shortcut captions come from `MenuItem::shortcut`; macOS OS-menu forwarding is left to the app. Gallery page "Menu bar" (#115).
- `platform::LayoutStore` (feature `serde`): saves split sizes, side docks, dock trees, DataGrid
  columns and density under keys as one JSON document, loaded from and saved to any string
  storage such as eframe's (#118). The gallery now restores its layout and density on restart.
- `DataGridSize` derives serde under the `serde` feature.
- Accordion (`surfaces::Accordion`, `AccordionItem`, `AccordionSize`, `ExpandIconPosition`, `AccordionResponse`): Fluent headers that show or hide panels, for settings panes and grouped options. Small to extra-large sizes, the chevron at the start or end, single or multiple open, and `collapsible` to let the last open item close (off by default, as in Fluent). The app owns the open indices. Panel height animates with `durationNormal` on `curveEasyEase` and snaps under reduced motion. Headers are buttons (Enter or Space toggles) and AccessKit Buttons with expanded state that control their panel Region. Gallery page "Accordion" (WI-1937).
- RichTextEditor (`controls::RichTextEditor`, `RichText`, `CharStyle`, `BlockKind`,
  `InlineFormat`, `RichTextEditorResponse`): a compose control for mail (email-rs, WI-1940).
  A toolbar toggles bold, italic and underline (also Ctrl+B, Ctrl+I, Ctrl+U), links, bulleted
  and numbered lists and quotes over egui's multiline text edit. With no selection, a toggled
  format applies to the next characters typed. `RichText::to_html` writes a small escaped
  HTML subset (`p`, `ul`/`ol`/`li`, `blockquote`, `strong`, `em`, `u`, and `a` for `http`,
  `https` and `mailto` links only); `RichText::to_plain` writes the plain-text alternative.
  Gallery page "Rich text editor".
- Add an optional wasm accessibility plugin that mirrors common AccessKit roles, names, states and focus into a hidden DOM layer for browser find, screen readers, translation and extensions.

### Changed
- Dependencies (wasm32 only): `web-sys` is no longer optional and gains the `History`, `Location` and `Window` features, and `wasm-bindgen` is a new direct dependency. Both were already in the tree through egui. The `file-dialog` feature no longer enables `web-sys`.
- `fluent2-icons` is dual-licensed `MIT OR Apache-2.0` (it was `MIT`). Every published crate ships `LICENSE-MIT`, `LICENSE-APACHE` and `NOTICE` (gh:TheDancingDeveloper-org/fluent2-egui#161).
- Dark theme: two Fluent tokens failed WCAG AA for text. `Theme::dark` now sets `status_danger_foreground_1` to `status_danger_foreground_3` (danger text was 4.16:1; Field errors, the danger ribbon tab, danger badges and message bars) and `neutral_foreground_2_brand_hover` to `neutral_foreground_2_hover` (subtle button hover label was 4.18:1). Light and high contrast keep Fluent's values (WI-1844).
- Weekly upstream review (#100, 2026-10-24): re-reviewed every upstream pin. No watched-path drift for rdpapp, vte, pulldown-cmark, egui_commonmark, imara-diff, syntect, bat, egui (still 0.36), fluentui tokens, selawik, inter, winui-gallery, laststudio-fluent-egui and fluent-gpui, so only their review dates moved. html5gum, the TypeScript grammar, Fluent System Icons, craft-libs and the wordcraft, gridcraft and deckcraft references moved on a watched path and stay pinned until that review is finished.
- Code view's TypeScript grammar is the 2024 TypeScript-TmLanguage conversion (satisfies, const type parameters, import attributes, regexp `v`), published by aemeath and regenerated with SublimeSyntaxConvertor. TSX stays on the previous grammar (WI-2433).
- Dialog actions take a stable id and a display text (`DialogActions::primary(id, text)` and `secondary`). `CloseReason::Button` carries that id, and the button's widget id is derived from it, so two actions may share a label and a translated label still closes as the same action. This is a breaking change: call sites that passed only a label now pass an id as well, and `CloseReason` is no longer `Copy` (gh:TheDancingDeveloper-org/fluent2-egui#147).
- Dialogs are always modal. `DialogKind` only chooses what dismisses the surface: `Modal` closes on a backdrop click and on Escape, `Dismissible` on Escape only, `Alert` on neither. `DialogKind::NonModal` remains as a deprecated alias of `Dismissible`; it never let background input through (gh:TheDancingDeveloper-org/fluent2-egui#148).
- Light and dark theme strokes, status borders and the dark primary fill now meet WCAG 2.2 contrast (3:1 non-text, 4.5:1 text) instead of keeping Fluent colours that failed those pairs (issue #150).
- Slider rail, thumb and inner-dot sizes come from `Theme.slider` (`SliderMetrics`). A disabled slider paints the disabled alias colours for the rail, fill and thumb instead of the resting brand colours (issue #151).
- `Combobox<'a, T = &'a str>` gains a generic parameter. Options are borrowed for `'a` and are not copied into the widget. `ComboboxOptions`, `SliceOptions` and `ModelCombobox` are new, and `filter_probe` counts real filter passes for tests (gh:TheDancingDeveloper-org/fluent2-egui#152).
- The gallery wasm size budget is enforced. `gallery-wasm` runs `scripts/wasm-size-check.sh` against `ci/wasm-budget.txt` (gzip and brotli, each 4,500,000 bytes) and uploads the size report. Raise the budget in that file when a change needs the room (gh:TheDancingDeveloper-org/fluent2-egui#153).
- `install` turns egui's text selection on in both egui themes, so an app that had turned it off gets it back for Fluent text.
- Docs: a runnable `Toaster` example in `surfaces::toast`, and short examples in the `controls` and `surfaces` module docs that show a few widgets together. They run as doctests (WI-1938).
- CI on a pull request now checks only the packages its diff touches, with a shared,
  SHA-pinned Rust cache and `cargo-nextest`; the merge queue and `main` still run the full
  Linux and Windows matrix. The module, gallery page, README and supporting-type lists are
  sorted one entry per line, and changelog entries are per-PR fragments under `changelog.d/`,
  so parallel PRs no longer conflict on shared lists. The gallery screenshot job now names
  the command to refresh a changed image (WI-2071).

### Fixed
- Slider thumb and rail painted the value from before the pointer press, and the value reported on release was that pre-input value. Both now follow the pointer (gh:TheDancingDeveloper-org/fluent2-egui#143, b94bf46).
- Extreme slider ranges no longer produce NaN geometry (gh:TheDancingDeveloper-org/fluent2-egui#144, b6508a5).
- A freeform combobox ignored the arrow-key selection when Enter was pressed. Enter now commits the highlighted option (gh:TheDancingDeveloper-org/fluent2-egui#145, c1610c0).
- Tooltip text was only painted, so screen readers never read it. `fluent_tooltip` now sets it as the widget's AccessKit description (Fluent's `aria-describedby`), with no hover needed. A tooltip that only repeats the label adds nothing, and ribbon tooltips ("Paste\nCtrl+V") describe the control by its shortcut alone. Found by the 2026-10-20 audit (WI-1931).
- KeyTips: once KeyTips are on, letters typed with Alt still held were dropped. They now continue the sequence, as in Office, and releasing Alt afterwards no longer toggles KeyTips off. Found in the 2026-10-20 upstream review (gridcraft 4ed93c3b) (WI-1927).
- Opening Backstage from the keyboard left focus nowhere. Focus now moves to the selected entry, repeating the request until egui accepts it. A Dialog node is now described by its body (AccessKit `described_by`), so the body is announced on open while focus is on the primary button. Found by the WI-1586 Orca walk (WI-1850).
- High contrast: Card header text and icon on a hovered, pressed or selected Card, and caller-drawn ribbon gallery tile content, kept the resting foreground on the highlight fill (about 1.46:1). They now take `neutral_foreground_1_{hover,pressed,selected}`; gallery draw closures see it through `ui.visuals().text_color()`. The subtle TabList and NavigationView selected indicator takes the selected foreground in high contrast, where the brand stroke matched the fill (WI-1845).
- Input, SearchBox, Textarea, SpinButton and Combobox painted their opaque frame fill after the text on the same layer, hiding the typed text, the placeholder and the Combobox chevron. Each field now reserves the fill's slot before its text paints. The gallery's controls images are regenerated (WI-1913).
- Dialog bodies taller than the screen scroll inside the surface. The title and actions stay visible, every control stays inside the surface (so clicks no longer land on the backdrop and close a Modal), and a control that gains focus is scrolled into view. Found by the rdpapp port (WI-1815) (WI-1912).
- `Card` paints its elevation shadow behind its fill. A resting card had looked grey, because the shadow was drawn over the surface colour.
- The foreground for a filled danger `Badge` is white in light and dark themes. It was unreadable in dark (the fill's own red, about 1.2:1) and low-contrast in light (about 1.7:1). It is now about 6:1 on the fill in both.
- HTML data URIs with a declared decoded length over 8 MiB are refused before a decode buffer is allocated (gh:TheDancingDeveloper-org/fluent2-egui#141).
- File drops stop reading at the size limit. An openable path is cut off by a bounded reader. A pathless drop, or a path that cannot be opened, is copied through the same reader so only `max + 1` bytes are kept (gh:TheDancingDeveloper-org/fluent2-egui#142).
- Combobox filter cache for a plain slice (`Combobox::new`) was keyed only on the query, so a caller that rebuilt the option slice while the popup stayed open reused stale indexes and could panic. The cache now fingerprints the slice, and a cached index past the list is dropped. Closed popups still do not filter (gh:TheDancingDeveloper-org/fluent2-egui#152).
- Code view highlights TSX as TypeScript, including JSX tags, instead of as JavaScript (WI-2424).
- Docs now match the changelog-fragment rule, the Dialog `show` signature, Field parent naming, and the README: Accordion is listed only as shipped, the egui table includes 0.2, and the hosted gallery is marked unavailable because GitHub Pages is not enabled.

## [0.2.0] - 2026-10-19

Minor release: 0.1.0 was prepared but never tagged or published, and a large additive widget wave has landed since (TabStrip, docking, DiffView, CodeView, MarkdownView, TerminalView, CommandPalette, Drawer, Pager, charts, density). Pre-1.0, new public API takes a minor bump.

### Added
- README gallery images: `crates/gallery/tests/gallery_snapshots.rs` renders every gallery page (light and dark), a controls sheet and the editor sample (light, dark and high contrast) to `docs/images/gallery/` through egui_kittest and wgpu (gallery feature `snapshots`). `scripts/gallery-screenshots.sh` runs it in a pinned `rust:1.95-trixie` container with Mesa lavapipe; `--update` rewrites and optimises the images. The `gallery-screenshots` CI job runs the same check and fails when an image is stale (WI-1910).
- Density (`theme::Density`: Compact, Medium, Touch; `Theme::with_density`, `DataMetrics::for_density`, `TOUCH_TARGET`). Compact steps controls one Fluent size down (32 → 24, 40 → 32); Touch makes every interactive control at least 44 × 44 (icon buttons square, spin button steppers side by side); spacing comes from the XS/S/M spacing tokens. Buttons, toggle, menu and split buttons, dropdown and combobox (fields and options), input, search box, spin button, checkbox, radio, switch, slider, menu items, command palette rows, tab list, tab strip and the data-view rows follow it; Office chrome keeps its fixed metrics. Medium is unchanged. Spec `docs/specs/density.md`; the gallery's top bar has a Density switch (WI-1805).
- List variable-height rows (`List::variable_height(estimate)`, `List::forget_heights`): rows are laid out top to bottom, measured when shown and cached by `RowId`; the viewport is anchored to its top row so inserts or height changes above it do not move the view, and keyboard moves repeat their reveal until the measured row is in view. Shared maths (`HeightCache`, prefix offsets, binary-searched visible range, `ScrollAnchor`) in `data/rows.rs`. Spec note `docs/specs/data-views.md` §6.1 (amends D2 for List only). Gallery page "List rows" with 10,000 mixed rows (WI-1800).
- Pager (`controls::Pager`): First, Previous, page numbers with ellipses, Next and Last, and an optional page-size Dropdown (`page_sizes`); `show` returns `PagerResponse { page, page_size }` with the new page, clamps the page into range, and keeps the first item of the old page in view when the size changes. The window maths is the pure `controls::page_window` (a constant `2 * siblings + 5` slots, first and last always shown, an ellipsis only for two or more hidden pages) with `controls::page_count`. `Button::min_width` overrides the size's minimum width, for the compact page buttons. Gallery page "Pager" (WI-1803).
- Sparkline and MiniBars (`controls::{Sparkline, MiniBars}`): small axis-free charts for table cells and dashboard tiles. Sparkline draws a `compoundBrandForeground1` line with a dot on the last value, and breaks the line at non-finite values. MiniBars draws `compoundBrandBackground` bars from a zero line, so negatives hang below it. An optional fixed `range` or `max` lets charts be compared. Empty, NaN-only, one-point, flat and all-zero series render without panicking. Each widget is an AccessKit Image labelled with a `SeriesSummary` ("CPU: 3 values, min 3, max 41, last 17.50"). Gallery page "Charts" (WI-1804).
- Docking behind the new optional `docking` feature (`surfaces::DockTree`, `DockArea`, `SideDock`), per the data-views spec §9.1 amendment: a serde-able pane split tree of the app's tab values (split, close with collapse, move, reorder), painted with a TabStrip per pane and a SplitView per split; drag a tab onto another pane's strip or body to move it there, or near a pane body's edge to split it; a loaded layout is normalised (empty panes, duplicate ids, id counter); collapsible left, right and bottom side docks. TabStrip's response gains `dragging`, `dropped_outside` and `tab_rects`. Gallery page "Docking" (WI-1794).
- DiffView behind the new optional `diff` feature (`fluent2_egui::data::diff`): `DiffModel` diffs two texts once with `imara-diff` (Histogram) into hunks with configurable context (`@@ -a,b +c,d @@` headers; nearby changes share a hunk), inline and side-by-side row layouts (removed and added lines paired, the shorter side padded), and word-level changes in paired lines (in display columns, tabs expanded, wide characters counted). `DiffView` paints only the rows in the viewport through `rows::visible_rows`, scrolls both ways, and takes its colours from Fluent palette tokens (`DiffColors`); in high contrast, changed lines get a bar and changed words an outline. Gallery page "Diff" with a 10,000-line file (WI-1797).
- CodeView and a shared syntax highlighter behind the new optional `highlight` feature (`fluent2_egui::data::code`). `highlight(language, text)` is a pure function that returns per-line `TokenSpan` runs of `SyntaxToken` kinds (keyword, string, comment, number, constant, type, function, key, attribute, punctuation, inserted, deleted, diff meta, heading). `syntax_color` maps each kind to a Fluent palette token at 4.5:1 or better in light and dark themes, and to the plain foreground in high contrast. `line_job` builds a LayoutJob for diff and markdown views. `CodeView` is read-only with a line-number gutter, two-way scrolling that lays out only the visible rows, drag selection, Ctrl+A, copy and Escape, and a read-only MultilineTextInput AccessKit node. Built on syntect (MIT) with fancy-regex, so it runs native and on wasm. Rust, TypeScript (a vendored grammar from Microsoft's TypeScript-Sublime-Plugin as converted by bat, Apache-2.0, see NOTICE), Python, JSON, YAML and diff are tested. `CodeView::revision` keys the highlight cache without hashing the text. A spike rejected tree-sitter because its grammars do not build for wasm32 (`docs/specs/code-view.md`). Gallery page "Code view" (WI-1798).
- TabStrip (`surfaces::TabStrip`): a document tab strip next to TabList. Tabs (`StripTab`) carry an optional icon, a close button and a dirty dot that the close button replaces on hover. Drag to reorder, with a drop marker; tabs that do not fit sit behind a "More tabs" overflow menu listing every tab, and the selected tab is kept in view. Keyboard: one tab stop; arrows and Home/End select, Ctrl+Shift+arrows move, Ctrl+Tab/Ctrl+Shift+Tab (and Ctrl+PageDown/PageUp) cycle, Ctrl+W closes, middle click closes; `.shortcuts(true)` answers the cycle and close keys without focus. Returns `TabStripResponse { selected, closed, moved }`, and `apply` updates the app's list and selection. AccessKit TabList with Tab children (Modified in the description) and labelled close buttons. Gallery page "Tab strip". fluent-gpui's TabStrip (close and select only, no reorder or overflow) was a behaviour reference; nothing copied (WI-1793).
- File dialogs behind the new optional `file-dialog` feature (`fluent2_egui::platform::file_dialog`): `FileDialog::open` and `FileDialog::save` start an `rfd` async dialog (worker thread on native, browser event loop on wasm) and `FileDialog::poll` returns the `FileDialogOutcome` once, from the frame, with no blocking calls. Opened files arrive as `PickedFile` (name, native path, bytes); `open_many` picks several. For streaming transfers such as SFTP, native-only `pick_paths` and `pick_save_path` return paths without reading or writing. Gallery page "File dialog" (WI-1795).
- Drawer and BottomSheet (`surfaces::{Drawer, BottomSheet}`): an overlay drawer from the start or end edge (Fluent sizes Small 320, Medium 592, Large 940, Full, or custom), an inline drawer (`Drawer::show_inline`, a side panel with the Complementary role that traps nothing), and a bottom sheet for narrow windows. The overlay surfaces share Dialog's `DialogKind` Escape and backdrop policy, focus trap and focus return, and slide in on `durationGentle`/`curveDecelerateMid`, snapping under reduced motion. Spec `docs/specs/drawer.md`; gallery page "Drawer" (WI-1802).
- `foundation::text::highlighted(text, ranges, font, theme)` and `highlighted_with(.., color, accent)`: a `LayoutJob` with caller-supplied byte ranges (for example fuzzy-match indices) drawn in `brandForeground1` and underlined. Ranges may overlap, be empty, unsorted or past the end; they are clamped, snapped to `char` boundaries and merged, never panicking. `char_ranges(text, indices)` turns fuzzy-match `char` positions into those byte ranges (WI-1799).
- Reorder drag helper (`data::reorder`): `Reorder` sessions with `container`s of `item`s report a `ReorderMove` (from and final slot, pointer or keyboard) and never change app data; `ReorderMove::apply` / `apply_between` do the `Vec` case. Drag shows the item under the pointer and a `compound_brand_stroke` drop bar; Escape cancels; Alt+arrow keys move the focused item within its container or to the neighbouring one; a disabled `Ui` makes it read-only. Items are AccessKit ListItems. Gallery page "Reorder" (WI-1801).
- MarkdownView behind the new optional `markdown` feature (`fluent2_egui::data::markdown`): CommonMark plus GFM tables, strikethrough and task lists parsed with `pulldown-cmark` into a `Document` block tree (`Document::outline` gives a text snapshot), painted with the Fluent type ramp (headings `title2` to `caption1`) and colour tokens. Links never act on their own: the app's `LinkPolicy` classifies each destination as `Web`, `File` or `Blocked` (`default_link_policy` allows only `http`, `https` and `mailto`), blocked links are plain text, and an activated link comes back in `MarkdownResponse::clicked`. Code blocks go through an optional `Highlighter` hook. Raw HTML stays literal, images show alt text, and nesting past `MAX_DEPTH` is flattened. Headings and links have the AccessKit Heading and Link roles. Gallery page "Markdown" (WI-1796).
- TerminalView behind the new optional `terminal` feature (`fluent2_egui::data::terminal`): `TerminalGrid`, a `vte` VT grid adapted from rdpapp (cursor movement, erase, insert and delete, scrolling regions, alternate screen, 16/256/24-bit SGR colours, bold, italic, underline and inverse, wide characters, OSC title, BEL, scrollback with stable line numbers), and a view that paints it with ANSI colours from Fluent palette tokens, with wheel and Shift+PageUp/PageDown scrollback, drag selection and copy, highlighted search with next and previous, and Ctrl+click links. The AccessKit node has the Terminal role. Input and PTY transport stay with the app, which resizes to `TerminalResponse::fit`. Gallery page "Terminal" (WI-1791).
- CommandPalette (`surfaces::CommandPalette`): a modal surface near the top of the window with a SearchBox and a ranked list of the app's `PaletteCommand`s. Ranking is the pure `surfaces::rank` (exact, prefix, substring and fuzzy label matches above subtitle, keyword and group matches, recents first on an empty query and as a tie-break). Matched label characters are highlighted and underlined. Up and Down skip disabled rows, Enter or a click returns the chosen id, Escape or the backdrop closes. The surface is an AccessKit Dialog whose ListBox is the parent of its ListBoxOption rows, and the search box points its active descendant at the selected row. The gallery has a Command palette page. Scoring adapted from fluent-gpui, relicensed MIT OR Apache-2.0 by its owner, see NOTICE (WI-1792).

### Fixed
- `scripts/upstream-check.py` no longer leaves a source partial when GitHub's compare truncates
  (300 files or 250 commits): it takes the branch head directly and finds watched-path changes
  with per-path commit queries (WI-1867).

### Changed
- `release.yml` no longer publishes to crates.io on a tag push: publishing needs a manual run on the tag with the `publish_crates` input (default false) (WI-1911).
- CI: merge-queue support (`merge_group`), path-scoped PR checks (Windows leg in the queue and on `main` only), cancel-in-progress per ref, and a single `ci` gate job to require. See `docs/ci.md` (WI-1886).
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
