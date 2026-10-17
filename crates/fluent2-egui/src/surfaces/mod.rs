//! Fluent surfaces. One file per surface; see `docs/ARCHITECTURE.md`.

mod card;
mod dialog;
mod menu;
mod popover;
mod tab_list;
mod tooltip;

pub use card::{Card, CardAppearance, CardHeader};
pub use dialog::{CloseReason, Dialog, DialogActions, DialogKind, DialogResult};
pub use menu::{MenuItem, context_menu, divider, group_header, menu_frame, submenu};
pub use popover::Popover;
pub use tab_list::{TabAppearance, TabList, TabSize};
pub(crate) use tab_list::{paint_indicator, tab_widths};
pub use tooltip::{FluentTooltip, TooltipExt, TooltipTone};
