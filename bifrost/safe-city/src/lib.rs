//! # bifrost-safe-city — Safe City, Economy, and Zone Warfare
//!
//! Implements the **Safe City** system from `docs/WORLD.md`:
//!
//! > "Without Safe City your system would be: Chaos + Inflation + AI dominance.
//! >  With Safe City: controlled chaos with stable meta-economy."
//!
//! ## Architecture
//!
//! ```text
//! WORLD SIMULATION LOOP
//! ─────────────────────
//! Player ⇄ Safe City Auction House ⇄ AI Faction ⇄ Economy Graph
//! ```
//!
//! ## Safe City guarantees
//!
//! The Safe City zone enforces these invariants:
//! - No combat events
//! - No territory capture
//! - No biome destruction
//! - Only: trade, crafting, skill progression, AI/player interaction
//!
//! ## Zone architecture
//!
//! ```text
//! SAFE CITY  → stable economy, crafting hub, respawn anchor
//! OUTER ZONES → war economy, faction influence, loot survival
//! DEEP ZONES  → high risk / high reward
//! ```
//!
//! ## Key types
//!
//! - [`SafeCity`] — the central anti-chaos anchor zone
//! - [`AuctionHouse`] — sole global market (all trade gated here)
//! - [`Zone`] — a spatial region with ownership and state
//! - [`ZoneState`] — Safe / Contested / Controlled / Collapsing
//! - [`WorldDirector`] — balances factions, triggers events, manages zone evolution
//! - [`PlayerBase`] — WAC-asset-backed player construction

pub mod auction;
pub mod base;
pub mod city;
pub mod zone;

pub use auction::{AuctionHouse, Listing, ListingStatus, TaxPolicy};
pub use base::{PlayerBase, WacAssetRef};
pub use city::{SafeCity, AllowedAction, CraftingRules, RespawnPolicy};
// ZoneId and FactionId are re-exported from zone (which imports from bifrost-kernel).
pub use zone::{Zone, FactionId, ZoneId, ZoneState, ResourceMap};
