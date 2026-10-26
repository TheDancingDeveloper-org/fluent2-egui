### Changed
- `Combobox<'a, T = &'a str>` gains a generic parameter. Options are borrowed for `'a` and are not copied into the widget. `ComboboxOptions`, `SliceOptions` and `ModelCombobox` are new, and `filter_probe` counts real filter passes for tests (gh:TheDancingDeveloper-org/fluent2-egui#152).

### Fixed
- Combobox filter cache for a plain slice (`Combobox::new`) was keyed only on the query, so a caller that rebuilt the option slice while the popup stayed open reused stale indexes and could panic. The cache now fingerprints the slice, and a cached index past the list is dropped. Closed popups still do not filter (gh:TheDancingDeveloper-org/fluent2-egui#152).
