//! Fluent controls. One file per control; see `docs/ARCHITECTURE.md`.

mod badge;
mod button;
mod checkbox;
mod compound_button;
mod divider;
mod field;
mod input;
mod link;
mod progress_bar;
mod radio;
mod search_box;
mod slider;
mod spin_button;
mod spinner;
mod switch;
mod textarea;
mod toggle_button;

pub use badge::{Badge, BadgeAppearance, BadgeColor, BadgeSize};
pub use button::{Appearance, Button, ButtonSize, Shape};
pub use checkbox::{CheckState, Checkbox};
pub use compound_button::CompoundButton;
pub use divider::Divider;
pub use field::{Field, ValidationState};
pub use input::{Input, InputAppearance, accent_open_for_test};
pub use link::Link;
pub use progress_bar::{ProgressBar, ProgressBarSize};
pub use radio::RadioGroup;
pub use search_box::SearchBox;
pub use slider::Slider;
pub use spin_button::SpinButton;
pub use spinner::{Spinner, SpinnerSize};
pub use switch::Switch;
pub use textarea::{Textarea, TextareaResize};
pub use toggle_button::ToggleButton;
