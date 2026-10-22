### Added

- `PieceBar`: a strip over piece states, a BitTorrent bitfield or availability counts, bucketed
  to one segment per pixel, with an AccessKit label carrying the percent done.
- `PieceMap`: a grid with one cell per piece, a hover read-out and a legend with counts.
- `PieceState` (`Missing`, `Downloading`, `Have`) for both widgets.
