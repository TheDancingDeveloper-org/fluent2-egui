//! Fluent controls. One file per control; see `docs/ARCHITECTURE.md`.

mod button;
mod slider;
mod spin_button;

pub use button::{Appearance, Button, ButtonSize, Shape};
pub use slider::Slider;
pub use spin_button::SpinButton;
