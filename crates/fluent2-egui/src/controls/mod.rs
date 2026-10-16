//! Fluent controls. One file per control; see `docs/ARCHITECTURE.md`.

mod button;
mod input;
mod search_box;
mod slider;
mod spin_button;
mod textarea;

pub use button::{Appearance, Button, ButtonSize, Shape};
pub use input::{Input, InputAppearance, accent_open_for_test};
pub use search_box::SearchBox;
pub use slider::Slider;
pub use spin_button::SpinButton;
pub use textarea::{Textarea, TextareaResize};
