### Added

- `DataGrid::drag_source` drags the selected rows (or the row under the pointer) out of the grid, with a ghost that counts them; `DataGrid::drag_label` sets its text ("3 torrents") and `DataGridResponse::dragging` reports the drag.
- `data::row_drag::DropTarget` makes any `Response`, such as a sidebar label, a drop target for grid rows: a brand highlight while a drag hovers it, the not-allowed cursor when it refuses, and a `RowDrop` with the row ids on release. Escape cancels.
- `data::row_drag::drop_menu` and `drop_choices` list the same targets in a context menu, the keyboard alternative to dragging.
- Gallery "Row drag" page: torrents dragged onto sidebar categories, a lockable category and a "Move to" menu.
