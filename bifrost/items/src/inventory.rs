//! Player inventory — backpack slots and equip slots.

use serde::{Deserialize, Serialize};
use thiserror::Error;

// ── ItemStack ─────────────────────────────────────────────────────────────────

/// A stack of identical items occupying one inventory slot.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ItemStack {
    /// Stable item ID — resolves through [`crate::registry::ItemRegistry`].
    pub item_id:  String,
    /// Number of items in this stack.
    pub quantity: u32,
}

impl ItemStack {
    pub fn new(item_id: impl Into<String>, quantity: u32) -> Self {
        Self { item_id: item_id.into(), quantity }
    }
}

// ── EquipSlots ────────────────────────────────────────────────────────────────

/// The named equip slots every player has.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct EquipSlots {
    pub weapon: Option<ItemStack>,
    pub armor:  Option<ItemStack>,
    /// Up to 3 socketed runes.
    pub runes:  [Option<ItemStack>; 3],
}

impl EquipSlots {
    /// True if the weapon slot is occupied.
    pub fn has_weapon(&self) -> bool { self.weapon.is_some() }
    /// True if the armor slot is occupied.
    pub fn has_armor(&self) -> bool  { self.armor.is_some() }
    /// Number of rune slots currently filled.
    pub fn rune_count(&self) -> usize {
        self.runes.iter().filter(|r| r.is_some()).count()
    }
}

// ── InventoryError ────────────────────────────────────────────────────────────

#[derive(Debug, Error, PartialEq)]
pub enum InventoryError {
    #[error("inventory is full ({max} slots used)")]
    Full { max: usize },

    #[error("item not found in inventory: {item_id}")]
    NotFound { item_id: String },

    #[error("cannot equip '{item_id}': wrong item type for this slot")]
    WrongType { item_id: String },

    #[error("stack limit exceeded for '{item_id}' (max {max} per slot)")]
    StackFull { item_id: String, max: u32 },

    #[error("all rune slots are occupied")]
    RuneSlotsFull,

    #[error("not enough gold (have {have}, need {need})")]
    InsufficientGold { have: u32, need: u32 },
}

// ── Inventory ─────────────────────────────────────────────────────────────────

/// Per-player item storage: backpack slots + equip slots + gold wallet.
///
/// ## Design notes
///
/// - Backpack slots are unordered; items stack up to their `stack_size`.
/// - All mutations validate against the item's `stack_size` from the registry.
/// - `max_slots` is configurable per player tier (expanded via meta-progression).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Inventory {
    /// Backpack contents (unordered, de-duplicated by item_id for stackables).
    pub slots:    Vec<ItemStack>,
    /// Currently equipped items.
    pub equipped: EquipSlots,
    /// Gold coins carried.
    pub gold:     u32,
    /// Maximum number of distinct backpack slots.
    pub max_slots: usize,
}

impl Default for Inventory {
    fn default() -> Self {
        Self::new(24)
    }
}

impl Inventory {
    /// Create an empty inventory with `max_slots` backpack slots.
    pub fn new(max_slots: usize) -> Self {
        Self { slots: Vec::new(), equipped: EquipSlots::default(), gold: 0, max_slots }
    }

    // ── Gold ──────────────────────────────────────────────────────────────────

    /// Add gold.
    pub fn add_gold(&mut self, amount: u32) {
        self.gold = self.gold.saturating_add(amount);
    }

    /// Spend gold. Returns `Err` if the wallet is too thin.
    pub fn spend_gold(&mut self, amount: u32) -> Result<(), InventoryError> {
        if self.gold < amount {
            return Err(InventoryError::InsufficientGold { have: self.gold, need: amount });
        }
        self.gold -= amount;
        Ok(())
    }

    // ── Backpack ──────────────────────────────────────────────────────────────

    /// Add `qty` of `item_id` to the backpack.
    ///
    /// `max_stack` is the item's `stack_size` from `ItemRegistry`; callers
    /// should look it up before calling.
    pub fn add(&mut self, item_id: impl Into<String>, qty: u32, max_stack: u32)
        -> Result<(), InventoryError>
    {
        let item_id = item_id.into();

        // Try to stack onto an existing slot.
        if let Some(slot) = self.slots.iter_mut().find(|s| s.item_id == item_id) {
            let new_qty = slot.quantity.saturating_add(qty);
            if new_qty > max_stack {
                return Err(InventoryError::StackFull { item_id, max: max_stack });
            }
            slot.quantity = new_qty;
            return Ok(());
        }

        // Open a new slot.
        if self.slots.len() >= self.max_slots {
            return Err(InventoryError::Full { max: self.max_slots });
        }
        if qty > max_stack {
            return Err(InventoryError::StackFull { item_id, max: max_stack });
        }
        self.slots.push(ItemStack::new(item_id, qty));
        Ok(())
    }

    /// Remove up to `qty` of `item_id` from the backpack.
    ///
    /// Returns the actual number removed (may be less if stock is low).
    /// Empty slots are pruned automatically.
    pub fn remove(&mut self, item_id: &str, qty: u32) -> u32 {
        let mut to_remove = qty;
        self.slots.retain_mut(|slot| {
            if slot.item_id != item_id || to_remove == 0 {
                return true;
            }
            if slot.quantity <= to_remove {
                to_remove -= slot.quantity;
                false  // drop the slot
            } else {
                slot.quantity -= to_remove;
                to_remove = 0;
                true
            }
        });
        qty - to_remove
    }

    /// Total count of `item_id` across all backpack slots.
    pub fn count(&self, item_id: &str) -> u32 {
        self.slots.iter()
            .filter(|s| s.item_id == item_id)
            .map(|s| s.quantity)
            .sum()
    }

    /// True if the player holds at least `qty` of `item_id`.
    pub fn has(&self, item_id: &str, qty: u32) -> bool {
        self.count(item_id) >= qty
    }

    // ── Equip ─────────────────────────────────────────────────────────────────

    /// Equip a weapon (moves from backpack if present; replaces any current weapon).
    ///
    /// If a weapon was already equipped, it is returned to the backpack.
    pub fn equip_weapon(&mut self, item_id: impl Into<String>, max_stack: u32)
        -> Result<(), InventoryError>
    {
        let item_id = item_id.into();

        // Move current weapon back to backpack first.
        if let Some(old) = self.equipped.weapon.take() {
            // Attempt to add old weapon back; if pack is full, re-slot and fail.
            if let Err(e) = self.add(old.item_id.clone(), old.quantity, max_stack) {
                self.equipped.weapon = Some(old);
                return Err(e);
            }
        }

        // Remove from backpack and equip.
        let removed = self.remove(&item_id, 1);
        if removed == 0 {
            return Err(InventoryError::NotFound { item_id });
        }
        self.equipped.weapon = Some(ItemStack::new(item_id, 1));
        Ok(())
    }

    /// Unequip weapon and return it to the backpack.
    pub fn unequip_weapon(&mut self, max_stack: u32) -> Result<(), InventoryError> {
        if let Some(item) = self.equipped.weapon.take() {
            self.add(item.item_id.clone(), item.quantity, max_stack)
                .map_err(|_| { self.equipped.weapon = Some(item); InventoryError::Full { max: self.max_slots } })
        } else {
            Ok(())
        }
    }

    /// Socket a rune into the first open rune slot.
    pub fn socket_rune(&mut self, item_id: impl Into<String>) -> Result<(), InventoryError> {
        let item_id = item_id.into();
        let removed = self.remove(&item_id, 1);
        if removed == 0 {
            return Err(InventoryError::NotFound { item_id });
        }
        let slot = self.equipped.runes.iter_mut().find(|r| r.is_none())
            .ok_or(InventoryError::RuneSlotsFull)?;
        *slot = Some(ItemStack::new(item_id, 1));
        Ok(())
    }
}

// ── Tests ─────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn add_and_count() {
        let mut inv = Inventory::new(10);
        inv.add("wolf_pelt", 3, 99).unwrap();
        assert_eq!(inv.count("wolf_pelt"), 3);
    }

    #[test]
    fn stacking() {
        let mut inv = Inventory::new(10);
        inv.add("wolf_pelt", 50, 99).unwrap();
        inv.add("wolf_pelt", 30, 99).unwrap();
        assert_eq!(inv.count("wolf_pelt"), 80);
        assert_eq!(inv.slots.len(), 1, "should still be one slot");
    }

    #[test]
    fn stack_overflow_rejected() {
        let mut inv = Inventory::new(10);
        inv.add("scroll", 1, 1).unwrap();
        let err = inv.add("scroll", 1, 1).unwrap_err();
        assert!(matches!(err, InventoryError::StackFull { .. }));
    }

    #[test]
    fn inventory_full_rejected() {
        let mut inv = Inventory::new(2);
        inv.add("item_a", 1, 1).unwrap();
        inv.add("item_b", 1, 1).unwrap();
        let err = inv.add("item_c", 1, 1).unwrap_err();
        assert!(matches!(err, InventoryError::Full { .. }));
    }

    #[test]
    fn remove_partial() {
        let mut inv = Inventory::new(10);
        inv.add("coal", 5, 99).unwrap();
        let removed = inv.remove("coal", 3);
        assert_eq!(removed, 3);
        assert_eq!(inv.count("coal"), 2);
    }

    #[test]
    fn remove_empties_slot() {
        let mut inv = Inventory::new(10);
        inv.add("coal", 3, 99).unwrap();
        let removed = inv.remove("coal", 3);
        assert_eq!(removed, 3);
        assert!(inv.slots.is_empty());
    }

    #[test]
    fn has_enough() {
        let mut inv = Inventory::new(10);
        inv.add("iron_ore", 5, 99).unwrap();
        assert!(inv.has("iron_ore", 5));
        assert!(!inv.has("iron_ore", 6));
    }

    #[test]
    fn gold_operations() {
        let mut inv = Inventory::new(10);
        inv.add_gold(100);
        inv.spend_gold(40).unwrap();
        assert_eq!(inv.gold, 60);
        let err = inv.spend_gold(100).unwrap_err();
        assert!(matches!(err, InventoryError::InsufficientGold { .. }));
    }

    #[test]
    fn equip_weapon() {
        let mut inv = Inventory::new(10);
        inv.add("iron_sword", 1, 1).unwrap();
        inv.equip_weapon("iron_sword", 1).unwrap();
        assert!(inv.equipped.has_weapon());
        assert_eq!(inv.count("iron_sword"), 0);
    }

    #[test]
    fn socket_rune() {
        let mut inv = Inventory::new(10);
        inv.add("rune_strength", 1, 1).unwrap();
        inv.socket_rune("rune_strength").unwrap();
        assert_eq!(inv.equipped.rune_count(), 1);
        assert_eq!(inv.count("rune_strength"), 0);
    }
}
