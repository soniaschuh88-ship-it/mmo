//! Core item types: [`ItemType`], [`Rarity`], [`ItemStats`], [`ItemEffect`], [`ItemDef`].

use serde::{Deserialize, Serialize};

// ── ItemType ─────────────────────────────────────────────────────────────────

/// Category of an item — drives UI, stacking, and auction filter behaviour.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ItemType {
    /// Melee or ranged weapon — equips in the weapon slot.
    Weapon,
    /// Body protection — equips in the armor slot.
    Armor,
    /// Single-use scroll that teaches the player a named spell.
    SpellScroll,
    /// Single-use consumable (potion, food, elixir).
    Consumable,
    /// Crafting component — stacks up to 99, no equip slot.
    Material,
    /// Cannot be sold or discarded; required for a specific quest stage.
    QuestItem,
    /// Socketed into a weapon or armor slot for a passive bonus.
    Rune,
}

// ── Rarity ───────────────────────────────────────────────────────────────────

/// Rarity tier — controls drop rates, UI colour, and base value multiplier.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Rarity {
    Common,
    Uncommon,
    Rare,
    Epic,
    Legendary,
}

impl Rarity {
    /// Gold value multiplier relative to Common (1.0).
    pub fn value_mult(self) -> f32 {
        match self {
            Rarity::Common    => 1.0,
            Rarity::Uncommon  => 2.5,
            Rarity::Rare      => 8.0,
            Rarity::Epic      => 25.0,
            Rarity::Legendary => 100.0,
        }
    }

    /// CSS hex colour for the rarity badge in game.html.
    pub fn color(self) -> &'static str {
        match self {
            Rarity::Common    => "#9d9d9d",
            Rarity::Uncommon  => "#1eff00",
            Rarity::Rare      => "#0070dd",
            Rarity::Epic      => "#a335ee",
            Rarity::Legendary => "#ff8000",
        }
    }

    pub fn label(self) -> &'static str {
        match self {
            Rarity::Common    => "Common",
            Rarity::Uncommon  => "Uncommon",
            Rarity::Rare      => "Rare",
            Rarity::Epic      => "Epic",
            Rarity::Legendary => "Legendary",
        }
    }
}

// ── ItemStats ─────────────────────────────────────────────────────────────────

/// Flat stat bonuses granted while an item is equipped.
///
/// Additive on top of the entity's base stats.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ItemStats {
    /// Attack bonus.
    pub atk:   i32,
    /// Defense bonus.
    pub def:   i32,
    /// Max HP bonus.
    pub hp:    i32,
    /// Max mana bonus.
    pub mana:  i32,
    /// Speed multiplier (1.0 = no change).
    pub speed: f32,
}

impl Default for ItemStats {
    fn default() -> Self { Self { atk: 0, def: 0, hp: 0, mana: 0, speed: 1.0 } }
}

impl ItemStats {
    pub fn weapon(atk: i32)          -> Self { Self { atk, ..Self::default() } }
    pub fn armor(def: i32, hp: i32)  -> Self { Self { def, hp, ..Self::default() } }
    pub fn none()                    -> Self { Self::default() }

    /// Combine two stat blocks (additive, speed multiplicative).
    pub fn add(&self, other: &Self) -> Self {
        Self {
            atk:   self.atk  + other.atk,
            def:   self.def  + other.def,
            hp:    self.hp   + other.hp,
            mana:  self.mana + other.mana,
            speed: self.speed * other.speed,
        }
    }
}

// ── ItemEffect ────────────────────────────────────────────────────────────────

/// What happens when an item is used (consumable/scroll) or passively applied
/// while equipped (rune/weapon/armor).
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum ItemEffect {
    /// Immediately restore HP (consumable).
    HealOnUse { hp: u32 },
    /// Immediately restore mana (consumable).
    ManaOnUse { mp: u32 },
    /// Temporary attack buff for `duration_ticks` ticks.
    BuffAttack { flat: i32, duration_ticks: u32 },
    /// Temporary defense buff.
    BuffDefense { flat: i32, duration_ticks: u32 },
    /// Temporary speed boost.
    BuffSpeed { mult: f32, duration_ticks: u32 },
    /// Cure all active debuffs / status effects.
    CureStatus,
    /// Learn the named spell (scroll — consumed on use).
    UnlockSpell { spell_id: String },
    /// Passive voxel-mining speed bonus while equipped (Terraria-style digging).
    MiningBonus { mult: f32 },
    /// Required key for a specific door or chest in the world.
    Key { lock_id: String },
    /// Passive light radius while equipped (great for dungeon exploration).
    LightRadius { radius: u8 },
}

// ── ItemDef ───────────────────────────────────────────────────────────────────

/// Complete, immutable definition of one item type.
///
/// An `ItemDef` describes what an item *is* — stats, effects, rarity.
/// The actual item instances in player inventories are [`crate::inventory::ItemStack`]
/// which reference the `id` field of this struct.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ItemDef {
    /// Stable kebab-case ID.  Used in loot tables, inventory, and events.
    pub id:           String,
    /// Human-readable display name.
    pub display_name: String,
    /// Classification.
    pub item_type:    ItemType,
    /// Rarity tier.
    pub rarity:       Rarity,
    /// Flat stat bonuses while equipped (0 for consumables/materials).
    pub stats:        ItemStats,
    /// Maximum stack size in one inventory slot.
    /// Weapons/armor/scrolls: 1.  Consumables: 10.  Materials: 99.
    pub stack_size:   u32,
    /// Base gold value before auction / economy modifiers.
    pub value_gold:   u32,
    /// Short emoji or ASCII icon for game.html UI.
    pub icon:         String,
    /// Flavour lore text (1–2 sentences).
    pub lore:         String,
    /// Effects triggered on use or applied passively while equipped.
    pub effects:      Vec<ItemEffect>,
}

impl ItemDef {
    #[allow(clippy::too_many_arguments)]
    pub fn new(
        id:           impl Into<String>,
        display_name: impl Into<String>,
        item_type:    ItemType,
        rarity:       Rarity,
        stats:        ItemStats,
        stack_size:   u32,
        value_gold:   u32,
        icon:         impl Into<String>,
        lore:         impl Into<String>,
        effects:      Vec<ItemEffect>,
    ) -> Self {
        Self {
            id: id.into(), display_name: display_name.into(), item_type, rarity,
            stats, stack_size, value_gold, icon: icon.into(), lore: lore.into(),
            effects,
        }
    }
}

// ── Tests ─────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn rarity_ordering() {
        assert!(Rarity::Common < Rarity::Legendary);
        assert!(Rarity::Rare   < Rarity::Epic);
    }

    #[test]
    fn stats_add() {
        let a = ItemStats::weapon(5);
        let b = ItemStats::armor(3, 20);
        let c = a.add(&b);
        assert_eq!(c.atk, 5);
        assert_eq!(c.def, 3);
        assert_eq!(c.hp,  20);
    }

    #[test]
    fn item_def_serde() {
        let def = ItemDef::new(
            "iron_sword", "Iron Sword", ItemType::Weapon, Rarity::Common,
            ItemStats::weapon(8), 1, 50, "⚔", "A trusty iron blade.", vec![],
        );
        let json = serde_json::to_string(&def).unwrap();
        let back: ItemDef = serde_json::from_str(&json).unwrap();
        assert_eq!(def, back);
    }
}
