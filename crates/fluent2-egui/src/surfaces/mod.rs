//! Fluent surfaces. One file per surface; see `docs/ARCHITECTURE.md`.
//!
//! A message bar and a card, shown together:
//!
//! ```
//! use fluent2_egui::surfaces::{Card, CardHeader, Intent, MessageBar};
//!
//! # fn demo(ui: &mut egui::Ui) {
//! let bar = MessageBar::new(Intent::Warning, "The file changed on disk.")
//!     .action("Reload")
//!     .dismissible()
//!     .show(ui);
//! if bar.action_clicked == Some(0) {
//!     // reload
//! }
//!
//! Card::new().show(ui, |ui| {
//!     CardHeader::new("Quarterly report").description("Edited 2 hours ago").show(ui);
//! });
//! # }
//! ```

mod accordion;
mod bottom_sheet;
mod card;
mod command_palette;
mod dialog;
#[cfg(feature = "docking")]
mod dock;
mod drawer;
mod menu;
mod message_bar;
mod popover;
mod shortcut_overlay;
mod tab_list;
mod tab_strip;
mod toast;
mod tooltip;

pub use accordion::{Accordion, AccordionItem, AccordionResponse, AccordionSize, ExpandIconPosition};
pub use bottom_sheet::BottomSheet;
#[rustfmt::skip]
pub use card::{
    Card,
    CardAppearance,
    CardHeader,
};
#[rustfmt::skip]
pub use command_palette::{
    CommandPalette,
    PaletteCommand,
    PaletteMatch,
    rank,
};
#[rustfmt::skip]
pub use dialog::{
    CloseReason,
    Dialog,
    DialogActions,
    DialogKind,
    DialogResult,
};
#[cfg(feature = "docking")]
#[rustfmt::skip]
pub use dock::{
    DockArea,
    DockEdge,
    DockNode,
    DockResponse,
    DockSide,
    DockTree,
    DockViewer,
    DropTarget,
    Pane,
    PaneId,
    SideDock,
    SideDockResponse,
    SideDockState,
};
#[rustfmt::skip]
pub use drawer::{
    CLOSE_LABEL,
    Drawer,
    DrawerPosition,
    DrawerSize,
};
#[rustfmt::skip]
pub use menu::{
    MenuItem,
    context_menu,
    divider,
    group_header,
    menu_frame,
    submenu,
};
#[rustfmt::skip]
pub use message_bar::{
    Intent,
    MessageBar,
    MessageBarResponse,
};
pub use popover::Popover;
pub use shortcut_overlay::ShortcutOverlay;
#[rustfmt::skip]
pub use tab_list::{
    TabAppearance,
    TabList,
    TabSize,
};
pub(crate) use tab_list::{paint_indicator, tab_widths};
#[rustfmt::skip]
pub use tab_strip::{
    StripTab,
    TabMove,
    TabStrip,
    TabStripResponse,
};
#[rustfmt::skip]
pub use toast::{
    Toast,
    ToastId,
    ToastPosition,
    Toaster,
};
#[rustfmt::skip]
pub use tooltip::{
    FluentTooltip,
    TooltipExt,
    TooltipTone,
};
