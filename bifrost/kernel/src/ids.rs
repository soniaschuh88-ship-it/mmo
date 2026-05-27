//! Canonical shared identifier type aliases.
//!
//! **Rule 1 — One concept, one crate.**
//!
//! `FactionId` and `ZoneId` may only be defined here.
//! Every other crate that needs them must import from `bifrost_kernel`.
//!
//! Using distinct type aliases (instead of raw `String` everywhere) lets
//! the compiler catch cross-concept assignment mistakes at zero runtime cost.

/// Stable identifier for a faction — human player guild or AI sub-faction.
///
/// Opaque `String` wrapper.  Formatting is `"<kind>:<slug>"`,
/// e.g. `"player:guild-ironforge"` or `"synthesis:alpha"`.
pub type FactionId = String;

/// Stable identifier for a spatial zone within a world run.
///
/// Opaque `String` wrapper.  Formatting is `"<tier>:<slug>"`,
/// e.g. `"safe:hub-1"`, `"outer:east-ridge"`, `"deep:dungeon-b4"`.
pub type ZoneId = String;
