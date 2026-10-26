### Changed
- `Combobox<'a, T = &'a str>` gains a generic parameter. Options are borrowed for `'a` and are not copied into the widget. `ComboboxOptions`, `SliceOptions` and `ModelCombobox` are new, and `filter_probe` counts real filter passes for tests (gh:TheDancingDeveloper-org/fluent2-egui#152).
