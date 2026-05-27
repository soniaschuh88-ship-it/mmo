//! # nova-input
//!
//! NOVA Engine unified input abstraction.
//!
//! | Module | What it provides |
//! |---|---|
//! | [`actions`] | [`KeyCode`], [`MouseButton`], [`ActionId`], [`InputMap`], [`InputState`], [`ActionQuery`] |
//!
//! ## Design
//!
//! Raw hardware events (key codes, mouse buttons) are decoupled from game
//! logic through a named-action layer.  The same action can be bound to
//! keyboard, mouse, or gamepad without changing any game code.
//!
//! ```text
//! Browser KeyboardEvent  ──►  InputState::key_down(KeyCode)
//!                                        │
//!                               ActionQuery::held(&actions::move_forward())
//!                                        │
//!                               Game logic reads direction
//! ```
//!
//! ## Integration with bifrost-run
//!
//! Player inputs flow through [`ActionQuery`] → `FactionIntent` → WAC pipeline.
//! The Synthesis AI emits identical `FactionIntent`s, maintaining the
//! symmetry guarantee from `docs/FACTION.md`.

pub mod actions;

pub use actions::{
    ActionId, ActionQuery, Binding, InputMap, InputState, KeyCode, MouseButton,
};
/// Standard MMO action constants (re-exported for convenience).
pub use actions::actions as game_actions;
