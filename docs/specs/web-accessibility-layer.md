# Web accessibility layer: a hidden DOM mirror of the AccessKit tree

Status: design and prototype for GitHub issue #108, 2026-10-22. Branch `issue-108-a11y-dom`.
Prototype: `crates/gallery/src/a11y_dom.rs`, behind the gallery's `web-a11y-dom` feature.

## Problem

On wasm, eframe draws everything into one `<canvas>`. The browser sees no text, so its own
find (Ctrl+F), screen readers, translation, password managers and other extensions have
nothing to work on. Native builds don't have this problem: eframe hands egui's AccessKit tree
to the platform adapter (UIA, AT-SPI, NSAccessibility).

## Survey (pinned versions: egui/eframe 0.36.2, accesskit 0.24.1)

**eframe web.** `eframe-0.36.2/src/web/app_runner.rs` destructures `PlatformOutput` with
`accesskit_update: _, // not currently implemented`. eframe's web runner also never calls
`Context::enable_accesskit`, so egui doesn't build the tree on the web unless the app asks.
What eframe does do on the web is a hidden `<input>` for text entry and IME, which gives
virtual keyboards and IME a target but carries no semantics.

**egui.** AccessKit is a non-optional dependency of egui 0.36 (there is no `accesskit`
feature to turn on). When enabled, `Context::end_pass` builds a **full** `TreeUpdate` every
pass: every node, `Tree::new(root)`, and the focused node. The root node carries a
`scale(pixels_per_point)` transform, so node bounds are in egui points. On the web eframe maps
one point to one CSS pixel (times `zoom_factor`), so bounds are CSS pixels relative to the
canvas. `egui::Plugin::output_hook` runs on the `FullOutput` before the integration sees it,
and `Plugin::input_hook` can add `Event::AccessKitActionRequest` events to `RawInput`, which
egui already understands (focus, click, set value, text selection). That pair is enough to
build the layer outside eframe, with no fork.

**AccessKit.** AccessKit ships platform adapters for Windows, macOS, Unix (AT-SPI) and
Android, plus `accesskit_winit`. There is no released web adapter in the 0.24 line; a
browser adapter has been discussed upstream, but nothing is published that we can depend on.
`accesskit_consumer` (already in our lock file through the native adapters) gives a tree with
parent links, filtered children and transformed bounds; it compiles on wasm and is the
natural base for an adapter.

**Flutter web.** Flutter renders to canvas too (CanvasKit / skwasm) and solves this with its
semantics tree:

- Off by default. A single offscreen "Enable accessibility" placeholder button
  (`flt-semantics-placeholder`) turns it on when a screen reader activates it; apps can force
  it on with `SemanticsBinding.ensureSemantics()`.
- When on, each semantics node becomes an `<flt-semantics>` element in a host
  (`flt-semantics-host`) layered over the canvas, nested to mirror the tree, positioned with
  absolute CSS transforms relative to its parent, and given ARIA roles and labels.
- Updates arrive as diffs (`SemanticsUpdate`) and only changed nodes are touched.
- Text fields get real `<input>`/`<textarea>` elements so the browser's own editing, IME and
  password managers work. Taps and focus on the DOM are sent back to the framework as
  semantics actions.
- Plain text is exposed as labels; Flutter's DOM text is not normally searchable with
  Ctrl+F, and keeping the visual canvas and the DOM text in sync is one of its known costs.

We copy that shape: a host element, nested nodes, roles and names from the tree, diffs, real
inputs for text fields, and actions routed back.

## Architecture

### Where it lives

A new optional module in the library, `fluent2_egui::web_a11y` (feature `web-a11y`, wasm
only), exposing one `egui::Plugin`. Apps add it with `ctx.add_plugin(WebA11y::default())`.
It depends on `web-sys` and `wasm-bindgen` only under `cfg(target_arch = "wasm32")`. The
gallery prototype is the same code in the gallery crate so the library API isn't committed
yet.

### Tree mirroring

- `setup` calls `ctx.enable_accesskit()` and creates the host `<div>` as a sibling of the
  canvas: `position: fixed`, covering the canvas rect, `pointer-events: none`,
  `color: transparent`, no background.
- `output_hook` takes `platform_output.accesskit_update`. Each AccessKit node maps to one
  element, keyed by `NodeId` in a `HashMap<NodeId, Mirrored>`.
- Role mapping (first cut): `Label` → `<span>` (plain text, findable); `Button` →
  `<button>`; `CheckBox` → `role=checkbox` + `aria-checked`; `RadioButton`, `Switch`,
  `Slider` (`aria-valuenow/min/max`), `ComboBox`, `Tab`/`TabList`, `Menu`/`MenuItem`,
  `Tree`/`TreeItem`, `Grid`/`Row`/`Cell` (`aria-rowcount`, `aria-rowindex`), `Dialog`
  (`aria-modal`), `Heading` (`<h1..6>` from `level`), `Link` → `<a>`; `TextInput` /
  `MultilineTextInput` → see "Text fields". Anything unmapped becomes a `<div>` with
  `role=generic` and its label, so text is still found.
- Phase 1 is flat (every element a child of the host, absolutely positioned in canvas
  coordinates). Phase 2 nests elements to follow the tree, so screen readers get groups,
  lists and grids with the right structure; positions are then relative to the parent's
  box.

### Diffing per frame

egui sends the full tree each pass, so the layer does the diff:

1. For each node, compute a small `Signature` (role, text, the state the mapping uses, the
   rounded rect). Equal to last frame → skip. Changed → patch only that element's text,
   attributes or style.
2. Nodes not in this update are removed (`Element::remove`), and their listeners dropped.
3. Phase 2 also diffs the child list and moves elements only when order changes.

Most frames change nothing in the DOM, so the steady-state cost is the walk and the
signature comparison, not DOM work. A later optimisation hashes each node's signature in
egui order and skips the walk when the whole-tree hash is unchanged. We could instead
upstream "send a diff" to egui, but that is an egui change and stays out of this work.

### Positioning

- Bounds are points relative to the canvas; add the canvas's
  `getBoundingClientRect()` origin once per frame (one layout read, never per node).
- Write `left/top/width/height` in CSS px, rounded to whole pixels so sub-pixel jitter
  doesn't count as a change. Phase 2 can switch to `transform: translate()` to keep layout
  work off the main path.
- Text gets `font-size` from the box height, `white-space: pre`, `line-height: 1`, so the
  browser's find highlight and scroll-into-view land on the drawn text.
- Clipped content: nodes outside the canvas rect, or with `is_hidden`, are not mirrored.
  Content inside a `ScrollArea` that is off screen isn't in egui's tree anyway (egui only
  builds what it lays out), so find can't reach rows of a virtualised grid until they scroll
  in. That is the main limit of this design; see Risks.

### Focus and action routing

- Mouse: the host has `pointer-events: none`, so the canvas keeps all mouse input and egui's
  hover, press and drag work as today.
- Keyboard and assistive tech: mirrored interactive elements are focusable. A DOM `click`
  (Enter/Space, or a screen reader's activate) queues `ActionRequest { action: Click, .. }`
  and requests a repaint; `input_hook` drains the queue into `RawInput::events`.
- Focus: a DOM `focus` event on a mirrored element queues `Action::Focus` so egui moves its
  focus. Going the other way, when `TreeUpdate::focus` names a node, the layer calls
  `.focus({ preventScroll: true })` on its element so the screen reader follows egui's own
  Tab order. A guard (the id last focused by the layer) stops the two from bouncing.
- eframe's hidden text agent `<input>` takes focus while egui edits text. Phase 3 has to
  agree with it on who owns DOM focus (see Text fields).
- Other actions: `SetValue` from slider/spin `aria-valuenow` changes, `Expand`/`Collapse`
  for tree items and accordions, `ScrollIntoView` when find or a reader moves to an element
  whose node is partly visible.

### Text fields

Phase 3. A mirrored `TextInput` becomes a real `<input>` (or `<textarea>`) with `value`,
`aria-label` and selection taken from the node. Two options: (a) keep eframe's text agent and
make the mirrored input a read-only proxy that forwards focus to it, or (b) replace the text
agent with the mirrored input so password managers can fill the right field. (b) is the
better result but needs eframe changes; (a) first.

## Per-frame cost budget

Targets, measured on the gallery's data-grid page with a 1,000-row grid (about 40 visible
rows × 8 columns, ~600 AccessKit nodes in the tree), release build, mid-range laptop Chrome:

| Item | Budget |
|---|---|
| Building the AccessKit tree (egui, once `enable_accesskit` is on) | ≤ 0.5 ms |
| Layer walk + signature compare, no changes | ≤ 0.3 ms |
| DOM patch while scrolling (every visible row changes) | ≤ 1.5 ms |
| Total added frame time, steady state | ≤ 1 ms (of a 16.7 ms frame) |
| Total added frame time, worst case (scrolling) | ≤ 2.5 ms |
| Extra memory | ≤ 2 KB per mirrored node |

Measure with `performance.now()` around `output_hook` and the Chrome performance panel
(Style/Layout under the layer's mutations). If scrolling blows the budget, rate-limit
position-only updates to every other frame while a scroll is in progress.

## Risks

- **Virtualised content is not findable.** Find only sees what egui laid out this frame. A
  full answer needs an egui-side "find" or app-supplied off-screen text; the in-app find item
  covers that, and this layer stays a mirror.
- **Double interaction.** If a DOM element ever took pointer events, clicks would hit both it
  and the canvas. Keep the host at `pointer-events: none`; test it.
- **Focus fights** between the layer, eframe's text agent and egui focus. Mitigated by the
  focus guard and by phasing text fields last.
- **Visual leaks.** Transparent text can still show a find highlight box, selection
  highlight, or a focus ring. Find highlight is wanted; selection and focus rings must be
  suppressed (`::selection { background: transparent }`, `outline: none`) because egui draws
  its own.
- **Text drift.** egui wraps and lays out text itself; the browser wraps the mirrored string
  its own way, so the highlight box can differ from the drawn glyphs on multi-line labels.
  Acceptable for find; screen readers don't care.
- **Tree quality.** The DOM is only as good as our AccessKit nodes. Components that set no
  role or label (ARCHITECTURE rule on `accesskit_node_builder`) show up as unnamed `generic`
  elements. The screen-reader walkthroughs (#101, #102) are the check.
- **Cost of `enable_accesskit`.** Building the tree has a cost even with no reader. Flutter
  makes it opt-in; we start always-on for find, and measure. If it is too dear, offer an
  "enable accessibility" placeholder like Flutter and turn on find support per app.
- **egui upgrades.** The plugin depends on `Plugin::output_hook`/`input_hook` and the full-tree
  behaviour; both are public API but may change across egui minors. Covered by a wasm test in
  CI once the library module exists.
- **Upstream overlap.** If eframe or AccessKit ships a web adapter, this layer should give way
  to it. Keep the module small and adapter-shaped (TreeUpdate in, ActionRequest out) so the
  swap is easy, and consider offering it upstream.

## Phased plan

1. **Prototype (this branch).** Gallery feature `web-a11y-dom`: flat layer, labels and
   buttons, diffing, click routing. Check Ctrl+F finds visible text.
2. **Library module, flat.** `fluent2_egui::web_a11y` behind feature `web-a11y`; full role
   mapping table above; focus routing both ways; selection/focus-ring suppression; a
   `wasm-bindgen-test` that mounts the gallery and checks the DOM has the expected roles and
   names. Measure the budget on the data grid (issue step 4).
3. **Nested tree.** Mirror parent/child structure (lists, grids, menus, dialogs, tab lists)
   with relative positioning and child-order diffing. NVDA (Chrome/Firefox, Windows) and
   VoiceOver (Safari, macOS) walkthroughs of the gallery, recorded against #101/#102.
4. **Text fields.** Real inputs, option (a) then (b); password manager check.
5. **Upstream.** Offer the adapter to eframe (or as an AccessKit web adapter) once stable.

## Acceptance (from the issue)

- Browser Ctrl+F finds visible text: phase 1 (prototype checks this).
- NVDA and VoiceOver can read and navigate the gallery on wasm: phase 3.
- No visible change, and frame time within budget: phase 2 measures, every phase keeps it.
