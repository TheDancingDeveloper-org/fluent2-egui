//! KeyTips: Alt or F10 shows letter badges over the ribbon, and typing a key runs its target.
//!
//! [`state`] is the pure state machine (spec §6). It has no egui types. Rendering is added
//! in a later item.

pub mod state;
