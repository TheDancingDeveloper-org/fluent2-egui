# Web accessibility DOM layer

Status: first implementation for GitHub issue #108.

## Problem

On wasm, eframe paints the application into one canvas. Browser find, screen readers,
translation tools and extensions cannot inspect the text and controls inside that canvas.
Eframe 0.36 produces no browser accessibility DOM from egui's AccessKit output.

## Design

The optional `fluent2-egui/web-a11y` feature exposes `WebAccessibilityPlugin` on wasm. An
application registers the plugin on its egui context before its first frame. The plugin enables
AccessKit, reads each full `TreeUpdate` in `Plugin::output_hook`, and mirrors supported nodes into
a transparent fixed-position DOM layer next to the canvas.

Each mirrored node records a signature containing its role, accessible text and value, state,
focusability and bounds. A frame only mutates elements whose signature changed and removes nodes
that disappeared. The layer has no pointer events, background, border or visible text, so canvas
painting and mouse input are unchanged. Text remains present in the DOM for browser find and
translation.

Common text, button, link, selection, input, menu, tab, tree and grid roles map to ARIA roles.
Names, values, disabled/selected/expanded/checked states and tab stops come from the AccessKit
node. Egui focus moves browser focus to the corresponding element; browser focus queues an
AccessKit `Focus` action for egui's next frame.

The gallery enables the feature on wasm. The plugin creates its host lazily and requires no
markup changes, so changes to `crates/gallery/index.html` remain independent.

## Deferred

- DOM activation actions such as click, toggle and increment/decrement.
- Real browser inputs for editing, IME and password-manager integration.
- Resolving names expressed only through AccessKit relationships.
- Nested DOM structure, live regions and richer text semantics.
- Off-screen virtualized rows, which do not exist in egui's current tree.
- Browser and screen-reader walkthroughs plus large-grid frame-time measurement.
