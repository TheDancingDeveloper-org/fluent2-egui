//! Fluent 2 themes, controls and Office-style application chrome for egui.
//!
//! Not affiliated with or endorsed by Microsoft. See `docs/ARCHITECTURE.md` for the API
//! conventions every component follows.

#![warn(missing_docs)]

pub use fluent2_tokens as tokens;

pub mod controls;
pub mod foundation;
pub mod theme;

pub use controls::{
    CheckState, Checkbox, Input, InputAppearance, RadioGroup, SearchBox, Slider, SpinButton, Switch,
    Textarea, TextareaResize,
};
pub use theme::{Theme, ThemeKind};

/// Names a caller usually wants in scope.
pub mod prelude {
    pub use crate::foundation::icon::{Icon, IconName, IconSize, IconStyle};
    pub use crate::foundation::text::{TypeRamp, font_id, rich, style};
    pub use crate::{
        CheckState, Checkbox, Input, RadioGroup, SearchBox, Slider, SpinButton, Switch, Textarea, Theme,
        ThemeKind,
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
