### Changed
- Dialogs are always modal. `DialogKind` only chooses what dismisses the surface: `Modal` closes on a backdrop click and on Escape, `Dismissible` on Escape only, `Alert` on neither. `DialogKind::NonModal` remains as a deprecated alias of `Dismissible`; it never let background input through (gh:TheDancingDeveloper-org/fluent2-egui#148).
