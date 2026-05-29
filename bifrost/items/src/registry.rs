//! Global item registry — 25 built-in items, extensible via WAC.

use std::collections::BTreeMap;

use crate::item::{ItemDef, ItemEffect, ItemStats, ItemType, Rarity};

// ── ItemRegistry ──────────────────────────────────────────────────────────────

/// Global database of all item types.
///
/// ## Architecture
///
/// ```text
/// WAC LootTableIR  ──►  item_id strings  ──►  ItemRegistry::get()  ──►  ItemDef
/// AuctionHouse     ──►  item_id strings  ──►  ItemRegistry::exists()
/// Player Inventory ──►  item_id strings  ──►  ItemRegistry::get()
/// ```
///
/// All item_ids used in loot tables, auction listings, and inventories are
/// resolved through this registry.  Unknown IDs return `None`/`false`.
pub struct ItemRegistry {
    items: BTreeMap<String, ItemDef>,
}

impl ItemRegistry {
    /// Create a registry with all 25+ built-in items pre-registered.
    pub fn with_builtins() -> Self {
        let mut r = Self { items: BTreeMap::new() };

        // ── Weapons ───────────────────────────────────────────────────────────
        r.reg("iron_sword",    "Iron Sword",       ItemType::Weapon,     Rarity::Common,
              ItemStats::weapon(8),  1, 50,  "⚔",  "A trusty iron blade worn by a hundred soldiers.", vec![]);
        r.reg("steel_sword",   "Steel Sword",      ItemType::Weapon,     Rarity::Uncommon,
              ItemStats::weapon(18), 1, 180, "⚔",  "Forged in Helga's furnace from purified steel.", vec![]);
        r.reg("shadow_blade",  "Shadow Blade",     ItemType::Weapon,     Rarity::Rare,
              ItemStats::weapon(32), 1, 600, "🗡",  "Cuts through darkness as easily as flesh.", vec![]);
        r.reg("crystal_spear", "Crystal Spear",    ItemType::Weapon,     Rarity::Epic,
              ItemStats::weapon(55), 1, 2000, "✨", "Humming with arcane energy from the deep caves.",
              vec![ItemEffect::LightRadius { radius: 4 }]);
        r.reg("void_dagger",   "Void Dagger",      ItemType::Weapon,     Rarity::Legendary,
              ItemStats { atk: 80, speed: 1.4, ..ItemStats::default() }, 1, 8000,
              "🌑", "A blade forged from crystallised void matter.",
              vec![ItemEffect::BuffAttack { flat: 10, duration_ticks: 5 }]);

        // ── Armors ────────────────────────────────────────────────────────────
        r.reg("leather_vest",  "Leather Vest",     ItemType::Armor,      Rarity::Common,
              ItemStats::armor(5, 20),   1, 40,   "🛡", "Basic protection stitched from wolf hide.", vec![]);
        r.reg("chain_mail",    "Chain Mail",       ItemType::Armor,      Rarity::Uncommon,
              ItemStats::armor(12, 50),  1, 160,  "🛡", "Interlocked rings that turn aside glancing blows.", vec![]);
        r.reg("plate_armor",   "Plate Armor",      ItemType::Armor,      Rarity::Rare,
              ItemStats::armor(25, 120), 1, 800,  "🛡", "Heavy, cumbersome, unyielding.", vec![]);
        r.reg("dragon_scale",  "Dragon Scale",     ItemType::Armor,      Rarity::Epic,
              ItemStats::armor(40, 200), 1, 3000, "🐉", "Scales from a fallen wyrm, still warm.", vec![]);

        // ── Consumables ───────────────────────────────────────────────────────
        r.reg("health_potion",  "Health Potion",   ItemType::Consumable, Rarity::Common,
              ItemStats::none(), 10, 20, "🧪", "Restores 50 HP. Tastes like copper coins.",
              vec![ItemEffect::HealOnUse { hp: 50 }]);
        r.reg("great_potion",   "Greater Health",  ItemType::Consumable, Rarity::Uncommon,
              ItemStats::none(), 5, 60, "🧪", "Restores 150 HP.",
              vec![ItemEffect::HealOnUse { hp: 150 }]);
        r.reg("mana_potion",    "Mana Potion",     ItemType::Consumable, Rarity::Common,
              ItemStats::none(), 10, 25, "💧", "Restores 40 MP.",
              vec![ItemEffect::ManaOnUse { mp: 40 }]);
        r.reg("elixir_of_rage", "Elixir of Rage",  ItemType::Consumable, Rarity::Uncommon,
              ItemStats::none(), 5, 80, "⚡", "ATK +20 for 60 ticks.",
              vec![ItemEffect::BuffAttack { flat: 20, duration_ticks: 60 }]);
        r.reg("iron_skin",      "Iron Skin Flask", ItemType::Consumable, Rarity::Uncommon,
              ItemStats::none(), 5, 70, "🛡", "DEF +15 for 45 ticks.",
              vec![ItemEffect::BuffDefense { flat: 15, duration_ticks: 45 }]);
        r.reg("swiftness_brew", "Swiftness Brew",  ItemType::Consumable, Rarity::Uncommon,
              ItemStats::none(), 5, 75, "💨", "Speed ×1.3 for 30 ticks.",
              vec![ItemEffect::BuffSpeed { mult: 1.3, duration_ticks: 30 }]);
        r.reg("antidote",       "Antidote",        ItemType::Consumable, Rarity::Common,
              ItemStats::none(), 5, 15, "💊", "Cures all active status effects.",
              vec![ItemEffect::CureStatus]);

        // ── Materials ─────────────────────────────────────────────────────────
        r.reg("wolf_pelt",     "Wolf Pelt",        ItemType::Material,   Rarity::Common,
              ItemStats::none(), 99, 5,   "🐺", "Soft grey fur, still warm.", vec![]);
        r.reg("iron_ore",      "Iron Ore",         ItemType::Material,   Rarity::Common,
              ItemStats::none(), 99, 3,   "⛏",  "Raw iron waiting for Helga's hammer.", vec![]);
        r.reg("coal",          "Coal",             ItemType::Material,   Rarity::Common,
              ItemStats::none(), 99, 2,   "🪨", "Burns hot and long.", vec![]);
        r.reg("crystal_shard", "Crystal Shard",    ItemType::Material,   Rarity::Uncommon,
              ItemStats::none(), 99, 30,  "💎", "Pulses faintly when held in the dark.", vec![]);
        r.reg("dragon_bone",   "Dragon Bone",      ItemType::Material,   Rarity::Rare,
              ItemStats::none(), 20, 120, "🦴", "Indestructible. Smells of sulphur.", vec![]);
        r.reg("void_dust",     "Void Dust",        ItemType::Material,   Rarity::Epic,
              ItemStats::none(), 50, 500, "✨", "Crystallised Fracture energy.", vec![]);
        r.reg("night_crystal", "Night Crystal",    ItemType::Material,   Rarity::Rare,
              ItemStats::none(), 50, 200, "🌙", "Only forms in absolute darkness.", vec![]);

        // ── Spell Scrolls ─────────────────────────────────────────────────────
        r.reg("scroll_fireball","Scroll: Fireball",ItemType::SpellScroll,Rarity::Uncommon,
              ItemStats::none(), 1, 90, "📜", "Teaches the Fireball spell permanently.",
              vec![ItemEffect::UnlockSpell { spell_id: "fireball".into() }]);
        r.reg("scroll_blink",  "Scroll: Blink",   ItemType::SpellScroll,Rarity::Rare,
              ItemStats::none(), 1, 300, "📜", "Teaches the Blink teleport spell.",
              vec![ItemEffect::UnlockSpell { spell_id: "blink".into() }]);

        // ── Runes ─────────────────────────────────────────────────────────────
        r.reg("rune_strength", "Rune of Strength", ItemType::Rune,       Rarity::Uncommon,
              ItemStats::weapon(5), 1, 100, "🔷", "Socket into a weapon for +5 ATK.", vec![]);
        r.reg("rune_warding",  "Rune of Warding",  ItemType::Rune,       Rarity::Uncommon,
              ItemStats::armor(8, 0), 1, 100, "🔷", "Socket into armor for +8 DEF.", vec![]);
        r.reg("rune_miner",    "Rune of the Miner",ItemType::Rune,       Rarity::Rare,
              ItemStats::none(), 1, 400, "⛏", "Socket for 50% faster tile mining.",
              vec![ItemEffect::MiningBonus { mult: 1.5 }]);

        // ── Quest Items ───────────────────────────────────────────────────────
        r.reg("key_dungeon",   "Dungeon Key",      ItemType::QuestItem,  Rarity::Common,
              ItemStats::none(), 1, 0, "🗝", "Opens the iron gate to the dungeon.",
              vec![ItemEffect::Key { lock_id: "dungeon_gate".into() }]);
        r.reg("elder_tome",    "Elder's Tome",     ItemType::QuestItem,  Rarity::Common,
              ItemStats::none(), 1, 0, "📕", "Elder Mirova's research journal. Handle with care.", vec![]);
        r.reg("dungeon_shard", "Dungeon Shard",    ItemType::QuestItem,  Rarity::Uncommon,
              ItemStats::none(), 3, 0, "💎", "A shard pulsing with dungeon energy.", vec![]);

        r
    }

    fn reg(
        &mut self, id: &str, name: &str, ty: ItemType, rarity: Rarity,
        stats: ItemStats, stack: u32, gold: u32, icon: &str, lore: &str,
        effects: Vec<ItemEffect>,
    ) {
        self.items.insert(id.into(), ItemDef::new(id, name, ty, rarity, stats, stack, gold, icon, lore, effects));
    }

    // ── Queries ───────────────────────────────────────────────────────────────

    /// Look up an item definition by its stable ID.
    pub fn get(&self, item_id: &str) -> Option<&ItemDef> {
        self.items.get(item_id)
    }

    /// True if an item with this ID is registered.
    ///
    /// Use for validating `item_id` fields in auction listings and loot tables.
    pub fn exists(&self, item_id: &str) -> bool {
        self.items.contains_key(item_id)
    }

    /// Register a new item (e.g. from a WAC-compiled `EntityPrefabIR` loot entry).
    ///
    /// Overwrites any existing item with the same ID.
    pub fn register(&mut self, item: ItemDef) {
        self.items.insert(item.id.clone(), item);
    }

    /// All registered item IDs in sorted order.
    pub fn ids(&self) -> impl Iterator<Item = &str> {
        self.items.keys().map(|s| s.as_str())
    }

    /// All item definitions in sorted-by-id order.
    pub fn all(&self) -> impl Iterator<Item = &ItemDef> {
        self.items.values()
    }

    /// Number of registered items.
    pub fn len(&self) -> usize { self.items.len() }

    /// True if the registry is empty.
    pub fn is_empty(&self) -> bool { self.items.is_empty() }
}

// ── Tests ─────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn builtins_registered() {
        let r = ItemRegistry::with_builtins();
        assert!(r.len() >= 25);
        assert!(r.exists("iron_sword"));
        assert!(r.exists("health_potion"));
        assert!(r.exists("wolf_pelt"));
        assert!(r.exists("scroll_fireball"));
        assert!(r.exists("rune_strength"));
        assert!(r.exists("key_dungeon"));
    }

    #[test]
    fn unknown_item_returns_none() {
        let r = ItemRegistry::with_builtins();
        assert!(!r.exists("fake_item_xyz_999"));
        assert!(r.get("fake_item_xyz_999").is_none());
    }

    #[test]
    fn custom_registration() {
        let mut r = ItemRegistry::with_builtins();
        let before = r.len();
        r.register(ItemDef::new(
            "alien_ore", "Alien Ore", ItemType::Material, Rarity::Rare,
            ItemStats::none(), 50, 300, "🌀", "Not from this world.", vec![],
        ));
        assert_eq!(r.len(), before + 1);
        assert!(r.exists("alien_ore"));
    }

    #[test]
    fn all_items_have_non_empty_ids() {
        let r = ItemRegistry::with_builtins();
        for def in r.all() {
            assert!(!def.id.is_empty(), "item has empty id");
            assert!(!def.display_name.is_empty(), "item '{}' has no display name", def.id);
        }
    }

    #[test]
    fn legendary_items_have_high_value() {
        let r = ItemRegistry::with_builtins();
        for def in r.all().filter(|d| d.rarity == Rarity::Legendary) {
            assert!(def.value_gold >= 1000,
                "legendary '{}' should have value >= 1000 gold", def.id);
        }
    }

    #[test]
    fn quest_items_have_zero_gold_value() {
        let r = ItemRegistry::with_builtins();
        for def in r.all().filter(|d| d.item_type == ItemType::QuestItem) {
            assert_eq!(def.value_gold, 0,
                "quest item '{}' should have 0 gold value", def.id);
        }
    }
}
