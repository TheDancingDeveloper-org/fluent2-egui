### Added

- DataGrid column layout: `ColumnLayout { order, hidden }` in `DataGridState` (serde-gated), `Column::required`, header right-click and Shift+F10 reported as `DataGridResponse::header_context_menu`, a `column_menu` chooser (show/hide, resize to fit, reset), and column reorder by dragging a header or Alt+Left/Right on a focused one (`columns_reordered`). The gallery grid now has twenty columns and the chooser.
