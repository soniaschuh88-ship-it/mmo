//! # bifrost-items — NOVA Item System
//!
//! Defines every item in the NOVA world:
//! weapons, armor, spell scrolls, consumables, crafting materials,
//! quest items, and runes.
//!
//! ## Architecture
//!
//! ```text
//! WAC AssetBlueprint  ──►  ItemDefinitionIR  ──►  ItemRegistry::register()
//! LootTableIR entries       (item_id strings)        ▲
//! AuctionHouse::Listing  ──► resolves via            │
//!                              ItemRegistry::get()   │
//! Player Inventory  ────────────────────────────────►┘
//! ```
//!
//! All item_ids used in loot tables, auction listings, and inventory
//! are resolved through [`ItemRegistry`].
//!
//! ## Key types
//!
//! - [`ItemDef`] — complete definition of one item type
//! - [`ItemRegistry`] — global database with 35+ built-in items
//! - [`Inventory`] — per-player item storage with equip slots
//! - [`ItemEffect`] — what happens when an item is used or equipped

pub mod inventory;
pub mod item;
pub mod registry;

pub use inventory::{EquipSlots, Inventory, ItemStack, InventoryError};
pub use item::{ItemDef, ItemEffect, ItemStats, ItemType, Rarity};
pub use registry::ItemRegistry;
