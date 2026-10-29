### Fixed

- Installing the UI font (Segoe UI Variable or Segoe UI on Windows, Inter elsewhere) no longer drops egui's emoji fallback faces. egui 0.37 panics when a glyph such as U+26A0 has no face, which the Windows CI job hit in the combobox tests.
