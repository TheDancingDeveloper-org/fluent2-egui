### Changed

- `DialogKind::NonModal` is a real non-modal dialog: no backdrop, and clicks reach the page behind it. Escape still closes it; a click outside does not. `DialogKind::Dismissible` is now a deprecated alias of `NonModal`. `Modal` and `Alert` still block the page (gh:TheDancingDeveloper-org/fluent2-egui#148).
