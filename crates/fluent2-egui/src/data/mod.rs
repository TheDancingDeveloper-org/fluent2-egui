//! Data views: DataGrid, TreeView, List, NavigationView, SplitView and the Reorder drag helper
//! (see `docs/specs/data-views.md`), TerminalView behind the `terminal` feature, MarkdownView
//! behind `markdown`, DiffView behind `diff` and CodeView behind `highlight`, and the in-app find
//! bar (`find`).

#[cfg(feature = "highlight")]
pub mod code;
pub mod data_grid;
#[cfg(feature = "diff")]
pub mod diff;
pub mod editable_list;
pub mod find;
#[cfg(feature = "html")]
pub mod html;
pub mod list;
#[cfg(feature = "markdown")]
pub mod markdown;
pub mod nav_view;
pub mod reorder;
pub(crate) mod rows;
pub mod split_view;
#[cfg(feature = "terminal")]
pub mod terminal;
pub mod tree_view;

#[rustfmt::skip]
pub use data_grid::{
    CellCtx,
    CellResponse,
    DataGrid,
    DataGridResponse,
    DataGridSize,
    DataGridState,
    FocusMode,
    RowId,
    SelectionMode,
};
#[rustfmt::skip]
pub use nav_view::{
    NavResponse,
    NavState,
    NavigationView,
    PaneMode,
};
#[rustfmt::skip]
pub use reorder::{
    MoveSource,
    Reorder,
    ReorderAxis,
    ReorderList,
    ReorderMove,
    ReorderResponse,
    Slot,
};
#[rustfmt::skip]
pub use split_view::{
    SplitAxis,
    SplitResponse,
    SplitSize,
    SplitState,
    SplitView,
};
