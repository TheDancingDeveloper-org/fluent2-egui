//! Fluent controls. One file per control; see `docs/ARCHITECTURE.md`.

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

pub use avatar::{
    AVATAR_SIZES, Avatar, AvatarColor, AvatarGroup, AvatarGroupLayout, AvatarShape, Persona, Presence,
    initials, palette_index,
};
pub use badge::{Badge, BadgeAppearance, BadgeColor, BadgeSize};
pub use breadcrumb::{
    Breadcrumb, BreadcrumbDivider, BreadcrumbItemStyle, BreadcrumbResponse, BreadcrumbSize,
};
pub(crate) use button::ensure_type_ramp;
pub use button::{Appearance, Button, ButtonSize, Shape};
pub use calendar::{
    Calendar, CalendarDate, CalendarResponse, CalendarStrings, DayContent, MAX_YEAR, MIN_YEAR, TimeOfDay,
    days_in_month,
};
pub use checkbox::{CheckState, Checkbox};
pub use combobox::Combobox;
pub use compound_button::CompoundButton;
pub use date_picker::{DatePicker, PickerResponse, TimePicker};
pub use divider::Divider;
pub use dropdown::Dropdown;
pub use field::{Field, ValidationState};
pub use file_card::{FileCard, FileCardResponse, file_card_row, file_icon, format_size};
pub use input::{Input, InputAppearance, accent_open_for_test};
pub use link::Link;
pub use menu_button::MenuButton;
pub use mini_bars::MiniBars;
pub use pager::{PageItem, Pager, PagerResponse, page_count, page_window};
pub use progress_bar::{ProgressBar, ProgressBarSize};
pub use radio::RadioGroup;
pub use search_box::SearchBox;
pub use skeleton::{SKELETON_SIZES, Skeleton, SkeletonAnimation, SkeletonItem};
pub use slider::Slider;
pub use sparkline::{SeriesSummary, Sparkline};
pub use spin_button::SpinButton;
pub use spinner::{Spinner, SpinnerSize};
pub use split_button::{SplitButton, SplitButtonResponse};
pub use switch::Switch;
pub use tag::{InteractionTag, Tag, TagAppearance, TagGroup, TagGroupResponse, TagResponse, TagSize};
pub use tag_picker::{TagPicker, TagPickerResponse, TagSuggestion};
pub use textarea::{Textarea, TextareaResize};
pub use toggle_button::ToggleButton;
