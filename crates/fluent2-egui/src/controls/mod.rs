//! Fluent controls. One file per control; see `docs/ARCHITECTURE.md`.
//!
//! A few controls together, bound to app state:
//!
//! ```
//! use fluent2_egui::controls::{Appearance, Badge, Button, Checkbox, ProgressBar, Switch};
//!
//! # fn demo(ui: &mut egui::Ui, notify: &mut bool, agree: &mut bool, progress: f32) {
//! ui.add(Switch::new(notify, "Notifications"));
//! ui.add(Checkbox::bool(agree, "I agree to the terms"));
//! ui.add(ProgressBar::new(Some(progress)));
//! ui.horizontal(|ui| {
//!     if ui.add(Button::new("Save").appearance(Appearance::Primary)).clicked() {
//!         // save
//!     }
//!     ui.add(Badge::new("3 new"));
//! });
//! # }
//! ```

mod avatar;
mod badge;
mod breadcrumb;
pub(crate) mod button;
mod calendar;
mod checkbox;
mod combobox;
mod compound_button;
mod date_picker;
mod divider;
mod dropdown;
mod field;
mod file_card;
mod input;
mod link;
mod menu_button;
mod mini_bars;
mod pager;
mod progress_bar;
mod radio;
mod rich_text_editor;
mod search_box;
mod skeleton;
mod slider;
mod sparkline;
mod spin_button;
mod spinner;
pub(crate) mod split_button;
mod switch;
mod tag;
mod tag_picker;
mod textarea;
mod toggle_button;

#[rustfmt::skip]
pub use avatar::{
    AVATAR_SIZES,
    Avatar,
    AvatarColor,
    AvatarGroup,
    AvatarGroupLayout,
    AvatarShape,
    Persona,
    Presence,
    initials,
    palette_index,
};
#[rustfmt::skip]
pub use badge::{
    Badge,
    BadgeAppearance,
    BadgeColor,
    BadgeSize,
};
#[rustfmt::skip]
pub use breadcrumb::{
    Breadcrumb,
    BreadcrumbDivider,
    BreadcrumbItemStyle,
    BreadcrumbResponse,
    BreadcrumbSize,
};
pub(crate) use button::ensure_type_ramp;
#[rustfmt::skip]
pub use button::{
    Appearance,
    Button,
    ButtonSize,
    Shape,
};
#[rustfmt::skip]
pub use calendar::{
    Calendar,
    CalendarDate,
    CalendarResponse,
    CalendarStrings,
    DayContent,
    MAX_YEAR,
    MIN_YEAR,
    TimeOfDay,
    days_in_month,
};
#[rustfmt::skip]
pub use checkbox::{
    CheckState,
    Checkbox,
};
pub use combobox::Combobox;
pub use compound_button::CompoundButton;
#[rustfmt::skip]
pub use date_picker::{
    DatePicker,
    PickerResponse,
    TimePicker,
};
pub use divider::Divider;
pub use dropdown::Dropdown;
#[rustfmt::skip]
pub use field::{
    Field,
    ValidationState,
};
#[rustfmt::skip]
pub use file_card::{
    FileCard,
    FileCardResponse,
    file_card_row,
    file_icon,
    format_size,
};
#[rustfmt::skip]
pub use input::{
    Input,
    InputAppearance,
    accent_open_for_test,
};
pub use link::Link;
pub use menu_button::MenuButton;
pub use mini_bars::MiniBars;
#[rustfmt::skip]
pub use pager::{
    PageItem,
    Pager,
    PagerResponse,
    page_count,
    page_window,
};
#[rustfmt::skip]
pub use progress_bar::{
    ProgressBar,
    ProgressBarSize,
};
pub use radio::RadioGroup;
#[rustfmt::skip]
pub use rich_text_editor::{
    BlockKind,
    CharStyle,
    InlineFormat,
    RichText,
    RichTextEditor,
    RichTextEditorResponse,
};
pub use search_box::SearchBox;
#[rustfmt::skip]
pub use skeleton::{
    SKELETON_SIZES,
    Skeleton,
    SkeletonAnimation,
    SkeletonItem,
};
pub use slider::Slider;
#[rustfmt::skip]
pub use sparkline::{
    SeriesSummary,
    Sparkline,
};
pub use spin_button::SpinButton;
#[rustfmt::skip]
pub use spinner::{
    Spinner,
    SpinnerSize,
};
#[rustfmt::skip]
pub use split_button::{
    SplitButton,
    SplitButtonResponse,
};
pub use switch::Switch;
#[rustfmt::skip]
pub use tag::{
    InteractionTag,
    Tag,
    TagAppearance,
    TagGroup,
    TagGroupResponse,
    TagResponse,
    TagSize,
};
#[rustfmt::skip]
pub use tag_picker::{
    TagPicker,
    TagPickerResponse,
    TagSuggestion,
};
#[rustfmt::skip]
pub use textarea::{
    Textarea,
    TextareaResize,
};
pub use toggle_button::ToggleButton;
