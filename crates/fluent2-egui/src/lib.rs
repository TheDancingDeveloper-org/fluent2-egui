//! Fluent 2 themes, controls and Office-style application chrome for egui.
//!
//! Not affiliated with or endorsed by Microsoft. See `docs/ARCHITECTURE.md` for the API
//! conventions every component follows.
//!
//! # Quick start
//!
//! Install the theme once, then use the controls like any egui widget:
//!
//! ```
//! use fluent2_egui::controls::{Appearance, Button};
//! use fluent2_egui::Theme;
//!
//! # fn demo(ctx: &egui::Context, ui: &mut egui::Ui) {
//! fluent2_egui::install(ctx, &Theme::default());
//!
//! if ui.add(Button::new("Save").appearance(Appearance::Primary)).clicked() {
//!     // save
//! }
//! # }
//! ```
//!
//! The Office chrome lives in [`office`]. A ribbon reports clicks by control key:
//!
//! ```
//! use fluent2_egui::office::ribbon::{FileTab, Ribbon, RibbonLarge, RibbonState, RibbonTab};
//! use fluent2_egui::IconName;
//!
//! # fn demo(ui: &mut egui::Ui, state: &mut RibbonState) {
//! let response = Ribbon::new("main")
//!     .file_tab(FileTab::new("file", "File"))
//!     .tab(RibbonTab::new("home", "Home", |tab| {
//!         tab.group("clipboard", "Clipboard", |group| {
//!             group.add(RibbonLarge::new("paste", IconName::ClipboardPaste, "Paste"));
//!         });
//!     }))
//!     .show(ui, state);
//! for event in response.events {
//!     println!("{} {:?}", event.key, event.kind);
//! }
//! # }
//! ```

#![warn(missing_docs)]

pub use fluent2_tokens as tokens;

pub mod controls;
pub mod data;
pub mod foundation;
pub mod office;
pub mod platform;
pub mod surfaces;
pub mod theme;

pub use controls::{
    Badge, BadgeAppearance, BadgeColor, BadgeSize, CheckState, Checkbox, Combobox, CompoundButton, Divider,
    Dropdown, Field, Input, InputAppearance, Link, MenuButton, ProgressBar, ProgressBarSize, RadioGroup,
    SearchBox, Slider, SpinButton, Spinner, SpinnerSize, SplitButton, SplitButtonResponse, Switch, Textarea,
    TextareaResize, ToggleButton, ValidationState,
};
pub use data::{
    CellCtx, DataGrid, DataGridResponse, DataGridSize, DataGridState, FocusMode, RowId, SelectionMode,
};
pub use platform::{SystemPreferences, system_preferences, theme_from_system};
pub use surfaces::{
    Card, CardAppearance, CardHeader, CloseReason, Dialog, DialogActions, DialogKind, DialogResult,
    FluentTooltip, MenuItem, Popover, TabAppearance, TabList, TabSize, TooltipExt, TooltipTone, context_menu,
    divider, group_header, menu_frame, submenu,
};
pub use theme::{Theme, ThemeKind};

/// Names a caller usually wants in scope.
pub mod prelude {
    pub use crate::foundation::icon::{Icon, IconName, IconSize, IconStyle};
    pub use crate::foundation::text::{TypeRamp, font_id, rich, style};
    pub use crate::{
        Badge, BadgeAppearance, BadgeColor, BadgeSize, Card, CardHeader, CheckState, Checkbox, Combobox,
        CompoundButton, Dialog, DialogKind, Divider, Dropdown, Field, FluentTooltip, Input, Link, MenuButton,
        ProgressBar, ProgressBarSize, RadioGroup, SearchBox, Slider, SpinButton, Spinner, SpinnerSize,
        SplitButton, Switch, TabList, Textarea, Theme, ThemeKind, ToggleButton, TooltipExt, ValidationState,
    };
}

/// Install image loaders and `theme`.
///
/// Apps call this instead of [`theme::install`]. It is safe to call again: image loaders are
/// installed once, and the theme and text styles are written in place.
pub fn install(ctx: &egui::Context, theme: &Theme) {
    install_image_loaders(ctx);
    theme::install(ctx, theme);
}

fn install_image_loaders(ctx: &egui::Context) {
    let id = egui::Id::NULL.with("fluent2_image_loaders");
    let already = ctx.data(|data| data.get_temp::<bool>(id).unwrap_or(false));
    if already {
        return;
    }
    egui_extras::install_image_loaders(ctx);
    ctx.data_mut(|data| data.insert_temp(id, true));
}

pub use foundation::icon::{Icon, IconName, IconSize, IconStyle, paint_icon};
pub use foundation::text::{TypeRamp, font_id, install_system_fonts, install_text_styles, rich, style};
