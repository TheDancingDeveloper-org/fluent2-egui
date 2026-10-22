### Added

- `platform::LayoutStore` (feature `serde`): saves split sizes, side docks, dock trees, DataGrid
  columns and density under keys as one JSON document, loaded from and saved to any string
  storage such as eframe's (#118). The gallery now restores its layout and density on restart.
- `DataGridSize` derives serde under the `serde` feature.
