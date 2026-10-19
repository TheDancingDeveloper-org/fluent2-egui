//! Fluent surfaces. One file per surface; see `docs/ARCHITECTURE.md`.

mod bottom_sheet;
mod card;
mod command_palette;
mod dialog;
mod drawer;
#[cfg(feature = "docking")]
mod dock;
mod menu;
mod message_bar;
mod popover;
mod tab_list;
mod tab_strip;
mod toast;
mod tooltip;

pub use bottom_sheet::BottomSheet;
pub use card::{Card, CardAppearance, CardHeader};
pub use command_palette::{CommandPalette, PaletteCommand, PaletteMatch, rank};
pub use dialog::{CloseReason, Dialog, DialogActions, DialogKind, DialogResult};
pub use drawer::{CLOSE_LABEL, Drawer, DrawerPosition, DrawerSize};
#[cfg(feature = "docking")]
pub use dock::{
    DockArea, DockEdge, DockNode, DockResponse, DockSide, DockTree, DockViewer, DropTarget, Pane, PaneId,
    SideDock, SideDockResponse, SideDockState,
};
pub use menu::{MenuItem, context_menu, divider, group_header, menu_frame, submenu};
pub use message_bar::{Intent, MessageBar, MessageBarResponse};
pub use popover::Popover;
pub use tab_list::{TabAppearance, TabList, TabSize};
pub(crate) use tab_list::{paint_indicator, tab_widths};
pub use tab_strip::{StripTab, TabMove, TabStrip, TabStripResponse};
pub use toast::{Toast, ToastId, ToastPosition, Toaster};
pub use tooltip::{FluentTooltip, TooltipExt, TooltipTone};
