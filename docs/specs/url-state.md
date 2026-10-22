# URL state (`platform::hash_state`)

Status: implemented for issue #119. Module: `crates/fluent2-egui/src/platform/hash_state.rs`.

## Purpose

A web view's tab, filter, sort or search should survive a reload and be shareable as a link.
This module keeps that state in the URL fragment (`#tab=controls&q=button`). It is part of
`platform` because it is the one piece of the library that depends on the browser. It is
always compiled and needs no cargo feature.

## Format

- The fragment is `key=value` pairs joined by `&`, with no leading `#` in the API.
- Keys and values are percent-encoded (UTF-8). The unreserved characters `A-Z a-z 0-9 - . _ ~`
  are written as they are; everything else, including `&`, `=`, `#`, `%` and space, is
  escaped as `%XX`.
- Decoding never fails. An empty piece is skipped, a key without `=` has an empty value, an
  invalid `%` escape is kept as written, and invalid UTF-8 is replaced. Only the first pair
  with a given key is used by `value`.
- Pairs keep their order when encoded. The app decides the order.

## API

| item | wasm | native |
|---|---|---|
| `encode(pairs)` / `decode(fragment)` / `value(pairs, key)` | pure | pure |
| `HashState::read_external()` | returns the browser fragment when it differs from the last one seen; the first call returns it even when empty | always `None` |
| `HashState::write(fragment)` | `history.replaceState`, keeping path and query; skipped when unchanged | no-op |

## Behaviour

- **Writes use `replaceState`.** A filter typed one key at a time does not add a history
  entry per key. Back and forward do not undo filter changes.
- **Reads poll.** The app calls `read_external` once per frame. It is a cheap read of
  `location.hash`. A back or forward move to another entry, a pasted link with a new
  fragment, or a hand-edited address changes the fragment and is applied on the next frame.
  No `popstate` or `hashchange` closure is held.
- **Native has no URL.** Apps keep their defaults there, so one code path serves both
  targets.

## App pattern

```rust
if let Some(fragment) = state.read_external() {
    apply(&hash_state::decode(&fragment)); // missing values fall back to defaults
}
// ... draw ...
state.write(&hash_state::encode(current_pairs));
```

Apply every key on an external change, and fall back to the default for a missing one, so
the view always matches the address bar.

The gallery's "URL state" page is the reference: a tab list and a filter round-trip through
`#tab=…&q=…`.

## Not covered

- Pushing history entries for discrete navigation (a future option, if a flow needs Back to
  step through views).
- Encoding structured values beyond flat pairs, such as lists or nested state. Use a
  comma-separated value, escaped by the app, if that is needed.
