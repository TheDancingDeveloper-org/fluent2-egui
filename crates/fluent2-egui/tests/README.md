# Component tests

Each control has `tests/<control>.rs` using `egui_kittest` without the `wgpu` feature.
Find the widget through the AccessKit tree by role and label. Check that it renders in
light, dark and high contrast, that a click and a key activate it, and that a disabled
control does not.

Pixel snapshots sit behind the `snapshots` feature and are not part of these tests.
