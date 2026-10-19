# Backlog index

The backlog lives in Vogt, initiative **"fluent2-egui: Fluent 2 component library for egui"**.
Each work item is self-contained (goal, files, API, pinned references, acceptance, gates). This
page is only an index: **the Vogt item is authoritative.** Items marked *opus* are design/spec
work; everything else is sized for a small-model worker.

Order: a row can start when everything in its "after" column is merged (or its PR is open and
the item says it may stack).

## M0 Foundations

| ref | item | size | model | after |
|---|---|---|---|---|
| WI-1531 | Fluent token data snapshot (JSON) | S | grok | — |
| WI-1532 | `xtask gen-tokens`: token Rust modules | S | grok | 1531 |
| WI-1533 | Brand ramp, `from_seed`, brand-aware tokens | S | grok | 1532 |
| WI-1534 | `xtask gen-icons`: Fluent System Icons subset | S | grok | 1532 |
| WI-1535 | Theme module and egui Visuals mapping | S | grok | 1533 |
| WI-1536 | Icon widget, `fluent2_egui::install` | S | grok | 1534, 1535 |
| WI-1537 | Focus ring, elevation, motion | XS | grok | 1535 |
| WI-1538 | Type ramp, runtime system font | S | grok | 1535 |
| WI-1539 | Gallery (native + wasm) | S | grok | 1536, 1538 |
| WI-1540 | Gallery on GitHub Pages (operator enables Pages) | XS | grok | 1539 |
| WI-1541 | OS accent / HC / reduced-motion detection | S | grok | 1533, 1537 |
| WI-1588 | Question: Linux/web font fallback | XS | opus | 1538 |

## M2 Core controls (built before M1: the ribbon uses them)

| ref | item | size | model | after |
|---|---|---|---|---|
| WI-1542 | **Button — the reference component** | S | grok | 1536, 1537, 1538 |
| WI-1543 | ToggleButton, CompoundButton | XS | grok | 1542 |
| WI-1545 | Menu, ContextMenu | S | grok | 1536, 1537 |
| WI-1544 | SplitButton, MenuButton | S | grok | 1542, 1545 |
| WI-1546 | Checkbox, RadioGroup, Switch | S | grok | 1542 |
| WI-1547 | Input, SearchBox, Textarea | S | grok | 1542 |
| WI-1548 | SpinButton, Slider | S | grok | 1547 |
| WI-1549 | Dropdown, Combobox | S | grok | 1547, 1545 |
| WI-1550 | Link, Badge, Divider, Field, ProgressBar, Spinner | S | grok | 1542 |
| WI-1551 | Tooltip, Popover | S | grok | 1537, 1538 |
| WI-1552 | Dialog (spec + implementation) | S | opus | 1542, 1551 |
| WI-1553 | MessageBar, Toast | S | grok | 1550 |
| WI-1554 | TabList, Card | S | grok | 1542 |

## M1 Office chrome

| ref | item | size | model | after |
|---|---|---|---|---|
| WI-1555 | **Office chrome spec** (`docs/specs/office-chrome.md`) | S | opus | 1544, 1554 |
| WI-1559 | TitleBar (spec §1) | S | grok | 1555 |
| WI-1560 | Window resize edges (§2) | XS | grok | 1559 |
| WI-1561 | Quick Access Toolbar (§3) | XS | grok | 1559, 1545 |
| WI-1562 | Ribbon tab strip, contextual tabs, collapse (§4) | S | grok | 1555 |
| WI-1563 | Ribbon groups and controls (§4) | S | grok | 1562, 1546, 1549 |
| WI-1564 | Ribbon galleries (§4) | S | grok | 1563 |
| WI-1565 | Ribbon adaptive scaling (§5) | M | grok | 1564 |
| WI-1566 | KeyTips state machine (§6) | S | grok | 1555 |
| WI-1567 | KeyTips UI, F6 regions (§6, §9) | S | grok | 1566, 1563, 1561 |
| WI-1568 | Backstage (§7) | S | grok | 1562 |
| WI-1569 | StatusBar, CommandBar (§8) | S | grok | 1555, 1548 |
| WI-1570 | Sample editor + end-to-end tests (M1 exit) | S | grok | 1560, 1565, 1567, 1568, 1569 |

## M3 Data and layout

| ref | item | size | model | after |
|---|---|---|---|---|
| WI-1580 | **Data views spec** (`docs/specs/data-views.md`) | S | opus | 1545, 1546 |
| WI-1581 | DataGrid core (§1) | S | grok | 1580 |
| WI-1582 | DataGrid sort, resize, autofit (§2) | S | grok | 1581 |
| WI-1583 | DataGrid selection, keyboard, a11y (§3–4) | S | grok | 1582 |
| WI-1584 | TreeView, List (§5–6) | S | grok | 1583 |
| WI-1585 | NavigationView, SplitView (§7–8) | S | grok | 1580, 1554 |

## M4 Release

| ref | item | size | model | after |
|---|---|---|---|---|
| WI-1586 | High-contrast and accessibility audit | S | opus | 1570, 1584, 1585, 1552, 1553 |
| WI-1587 | 0.1.0 release (operator tags and publishes) | S | grok | 1586 |

## Vogt port wave (after 0.1)

Filed from `docs/vogt-port-gap-analysis.md` §4. All merged; the PR is in the last column.

| ref | item | gap | PR |
|---|---|---|---|
| WI-1791 | TerminalView, `terminal` feature | G13 | #50 |
| WI-1792 | CommandPalette | G21 | #52 |
| WI-1793 | TabStrip | G1 | #54 |
| WI-1794 | Docking, `docking` feature | G2 | #57 |
| WI-1795 | File dialog, `file-dialog` feature | G28 | #56 |
| WI-1796 | MarkdownView, `markdown` feature | G14 | #59 |
| WI-1797 | DiffView, `diff` feature | G16 | #61 |
| WI-1798 | CodeView and highlighter, `highlight` feature | G15 | #62 |
| WI-1799 | Highlighted text helper | G17 | #55 |
| WI-1800 | Variable-height List rows | G11 | #63 |
| WI-1801 | Reorder helper | G12 | #60 |
| WI-1802 | Drawer, BottomSheet | G5 | #58 |
| WI-1803 | Pager | G10 | #65 |
| WI-1804 | Sparkline, MiniBars | G18 | #64 |
| WI-1805 | Density: compact, medium, touch | G4 | #66 |
| WI-1886 | CI: merge queue, path-scoped checks | — | #53 |
| WI-1910 | Docs overhaul and README gallery | — | open |

## Recurring

| ref | item |
|---|---|
| WI-1589 | Weekly upstream review (UPSTREAM.md procedure) |
