# fluent2-egui

Fluent 2 themes, controls and Office-style application chrome for [egui](https://github.com/emilk/egui):
light, dark and high-contrast themes generated from Fluent 2 design tokens, Fluent System Icons, and
the pieces desktop productivity apps need: title bar, quick access toolbar, ribbon with keytips,
backstage/File view, command bar, status bar, data grid and tree view. It runs natively on
Windows and Linux, and on the web through wasm.

**Live gallery:** https://thedancingdeveloper.github.io/fluent2-egui/

**Status: 0.2.0.** Milestones:

| milestone | contents |
|---|---|
| M0 Foundations | token and icon generators, theme installation, fonts, focus ring, gallery (native and wasm), CI |
| M1 Office chrome | title bar, QAT, ribbon, keytips, backstage, status bar |
| M2 Core controls | buttons, inputs, selection controls, menus, dialog, message bar, toast, tab list |
| M3 Data and layout | data grid, tree view, navigation view, split view, docking |
| M4 0.1 release | docs, crates.io, hosted gallery |

## Usage

```toml
[dependencies]
egui = "0.36"
fluent2-egui = "0.2"
```

Install the theme once (it is safe to call again, for example when the user switches theme),
then use the controls like any egui widget:

```rust
use fluent2_egui::controls::{Appearance, Button};
use fluent2_egui::Theme;

fn ui(ctx: &egui::Context, ui: &mut egui::Ui) {
    fluent2_egui::install(ctx, &Theme::default());

    if ui.add(Button::new("Save").appearance(Appearance::Primary)).clicked() {
        // save
    }
}
```

A ribbon is built each frame from tabs, groups and controls, and reports clicks by key:

```rust
use fluent2_egui::office::ribbon::{FileTab, Ribbon, RibbonLarge, RibbonState, RibbonTab};
use fluent2_egui::IconName;

fn ribbon(ui: &mut egui::Ui, state: &mut RibbonState) {
    let response = Ribbon::new("main")
        .file_tab(FileTab::new("file", "File"))
        .tab(RibbonTab::new("home", "Home", |tab| {
            tab.group("clipboard", "Clipboard", |group| {
                group.add(RibbonLarge::new("paste", IconName::ClipboardPaste, "Paste"));
            });
        }))
        .show(ui, state);
    for event in response.events {
        println!("{} {:?}", event.key, event.kind);
    }
}
```

`cargo run -p gallery --example editor` runs a sample editor with the full Office chrome.

## Components

What the crate offers, by module, mapped to Fluent 2. Rows are sorted by type name and kept one per line: add a new row in sorted position so two PRs adding components do not conflict. "Gallery" is the page in `cargo run -p gallery`. `tests/readme_index.rs` fails when a public widget is missing from these tables.

### `controls`

| type | Fluent 2 | feature | gallery |
|---|---|---|---|
| `Avatar` | Avatar | | Avatars |
| `AvatarGroup` | AvatarGroup | | Avatars |
| `Badge` | Badge, CounterBadge | | Basic controls |
| `Breadcrumb` (`BreadcrumbSize`, `BreadcrumbDivider`, `BreadcrumbItemStyle`, `BreadcrumbResponse`) | Breadcrumb | | Breadcrumb |
| `Button` | Button | | Basic controls |
| `Calendar` | Calendar | | Date and time |
| `Checkbox` | Checkbox | | Basic controls |
| `Combobox` | Combobox | | Basic controls |
| `CompoundButton` | CompoundButton | | Basic controls |
| `DatePicker` | DatePicker | | Date and time |
| `Divider` | Divider | | Basic controls |
| `Dropdown` | Dropdown | | Basic controls |
| `Field` | Field (and Label) | | Basic controls |
| `FileCard` | Attachment (file card) | | Surfaces |
| `Input` | Input | | Basic controls |
| `InteractionTag` | InteractionTag | | Basic controls |
| `LineChart` (`ChartSeries`, `TimeRange`, `ChartHover`, `LineChartResponse`) | egui-only (charts, `chart` feature) | | Charts |
| `Link` | Link | | Basic controls |
| `MenuButton` | MenuButton | | Basic controls |
| `Pager` | egui-only (pagination) | | Pager |
| `Persona` | Persona | | Avatars |
| `PieceBar`, `PieceMap` | egui-only (charts) | | Charts |
| `ProgressBar` | ProgressBar | | Basic controls |
| `RadioGroup` | RadioGroup | | Basic controls |
| `RichTextEditor` (`RichText`, `CharStyle`, `BlockKind`, `InlineFormat`, `RichTextEditorResponse`) | egui-only (rich text compose) | | Rich text editor |
| `SearchBox` | SearchBox | | Basic controls |
| `Skeleton`, `SkeletonItem` | Skeleton (SkeletonItem) | | Skeleton |
| `Slider` | Slider | | Basic controls |
| `Sparkline`, `MiniBars` | egui-only (charts) | | Charts |
| `SpinButton` | SpinButton | | Basic controls |
| `Spinner` | Spinner | | Basic controls |
| `SplitButton` | SplitButton | | Basic controls |
| `Switch` | Switch | | Basic controls |
| `Tag` | Tag | | Basic controls |
| `TagGroup` | TagGroup | | Basic controls |
| `TagPicker` | TagPicker | | Basic controls |
| `Textarea` | Textarea | | Basic controls |
| `TimePicker` | TimePicker | | Date and time |
| `ToggleButton` | ToggleButton | | Basic controls |

### `surfaces`

| type | Fluent 2 | feature | gallery |
|---|---|---|---|
| `Accordion`, `AccordionItem`, `AccordionSize`, `ExpandIconPosition`, `AccordionResponse` | Accordion | | Accordion |
| `Card`, `CardHeader` | Card | | Surfaces |
| `CommandPalette` | egui-only (command palette) | | Command palette |
| `Dialog` | Dialog | | Surfaces |
| `DockArea`, `DockTree`, `SideDock` | egui-only (docking) | `docking` | Docking |
| `Drawer`, `BottomSheet` | Drawer (overlay and inline) | | Drawer |
| `FluentTooltip` (`TooltipExt::fluent_tooltip`) | Tooltip | | Surfaces |
| `MenuItem`, `context_menu`, `submenu` | Menu | | Surfaces |
| `MessageBar` | MessageBar | | Surfaces |
| `Popover` | Popover | | Surfaces |
| `ShortcutOverlay`, `foundation::shortcuts::Shortcuts` | egui-only (shortcut registry and help) | | Shortcuts |
| `TabList` | TabList | | Surfaces |
| `TabStrip` | egui-only (document tabs) | | Tab strip |
| `Toaster` | Toast | | Surfaces |

### `data`

| type | Fluent 2 | feature | gallery |
|---|---|---|---|
| `code::CodeView` | egui-only (code view) | `highlight` | Code view |
| `DataGrid` | DataGrid, Table | | DataGrid |
| `diff::DiffView` | egui-only (diff) | `diff` | Diff |
| `editable_list::EditableList` | egui-only (inline list editing) | | Editable list |
| `html::HtmlView` | egui-only (sanitised HTML, for mail) | `html` | HTML |
| `list::List` | List | | List rows |
| `markdown::MarkdownView` | egui-only (Markdown) | `markdown` | Markdown |
| `NavigationView` | Nav | | — |
| `Reorder` | egui-only (drag reorder) | | Reorder |
| `row_drag::DropTarget` | egui-only (drag rows onto a target) | | Row drag |
| `SplitView` | egui-only (split panes) | | — |
| `terminal::TerminalView` | egui-only (terminal) | `terminal` | Terminal |
| `tree_view::TreeView` | Tree | | — (`examples/a11y_walk.rs`) |

### `office` (Office chrome) and `platform`

| type | what | feature | gallery |
|---|---|---|---|
| `office::backstage::Backstage` | File page | | Backstage |
| `office::command_bar::CommandBar` | command bar | | — |
| `office::keytips` | KeyTips and F6 region cycling | | Ribbon |
| `office::menu_bar::MenuBar` | classic menu bar (File, Edit, …) with Alt, F10 and mnemonics | | Menu bar |
| `office::qat::Qat` | quick access toolbar | | Ribbon |
| `office::ribbon::Ribbon` | ribbon with groups, galleries and adaptive scaling | | Ribbon |
| `platform::file_dialog::FileDialog` | async open and save dialogs | `file-dialog` | File dialog |
| `platform::file_drop::FileDropZone` | drop target for files from the OS, read off the UI thread | `file-dialog` | File dialog |
| `TitleBar`, `office::status_bar::StatusBar` | title bar and status bar | | — (`examples/editor.rs`, `examples/chrome.rs`) |

### Not yet provided

| Fluent 2 | status |
|---|---|
| Accordion | WI-1937 |
| Toolbar, Rating, InfoLabel, Image, Select, SwatchPicker, Carousel, TeachingPopover | not planned yet |

## Crates

| crate | purpose |
|---|---|
| `fluent2-tokens` | Fluent 2 design tokens as plain Rust data (no UI dependency) |
| `fluent2-icons` | Fluent System Icons subset (MIT) |
| `fluent2-egui` | the egui library |

## egui compatibility

| fluent2-egui | egui |
|---|---|
| 0.1.x | 0.36 |

## CI

Pull requests scope their jobs to the packages the diff touches. `main` and the merge queue run
the full suite on Linux and Windows. The single required check is `ci`. See
[`docs/ci.md`](docs/ci.md).

## Upstream sources

This project generates from, adapts and follows several fast-moving upstreams (egui, Fluent UI
tokens, Fluent System Icons, the storytold Craft apps). They are pinned and tracked in
[`upstream.toml`](upstream.toml); see [`UPSTREAM.md`](UPSTREAM.md).

## Licence

Dual-licensed under [MIT](LICENSE-MIT) or [Apache-2.0](LICENSE-APACHE), at your option, except
`fluent2-icons`, which is MIT only. Notices are in [NOTICE](NOTICE) and per-asset attribution is
in [ATTRIBUTION.md](ATTRIBUTION.md).

Not affiliated with or endorsed by Microsoft. "Fluent" refers to Microsoft's public Fluent 2
design system, whose open-source tokens and icons this project uses under their MIT licences.
Microsoft fonts are never bundled. On Windows the system's Segoe UI is used at runtime.
