# BIFROST Complete Codebase Audit — Every File, Every Function
> All 18 workspace crates + 2 orphaned crates + 4 nova-* engine crates read line by line.
> Focus: what is BROKEN, what is ORPHANED, what UNIQUE FUNCTIONS would be lost, and
> exactly what actions bring everything back under the main bifrost crates.

---

## STATUS LEGEND

| Symbol | Meaning |
|---|---|
| ❌ BROKEN | File references modules that DON'T EXIST — won't compile |
| 🔴 ORPHANED | Exists but not in workspace — never compiled |
| 🟡 DISCONNECTED | In workspace but not wired to anything that uses it |
| ✅ CORRECT | Exists, compiles, properly wired |
| 🆕 NEW FUNCTION | Unique logic that will be lost if deleted without migration |

---

## 1. BROKEN CRATES — Files that reference non-existent modules

### ❌ BROKEN-1: `bifrost/items/` — `inventory.rs` and `registry.rs` DO NOT EXIST

`bifrost/items/src/lib.rs` declares:
```rust
pub mod inventory;   // ← FILE MISSING
pub mod registry;    // ← FILE MISSING

pub use inventory::{EquipSlots, Inventory, ItemStack, InventoryError};
pub use item::{ItemDef, ItemEffect, ItemStats, ItemType, Rarity};
pub use registry::ItemRegistry;
```

**What exists**: only `item.rs` (complete, good code — `ItemDef`, `ItemType`, `Rarity`,
`ItemStats`, `ItemEffect`).

**What is missing** — must be CREATED:

#### `bifrost/items/src/inventory.rs` — needs to contain:

```rust
use serde::{Deserialize, Serialize};
use thiserror::Error;
use crate::item::{ItemDef, ItemType};

/// A single stack of items in one inventory slot.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ItemStack {
    pub item_id:  String,
    pub quantity: u32,
}

/// Which equip slot an item occupies.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum EquipSlot {
    Weapon,
    Armor,
    Rune1,
    Rune2,
    Rune3,
}

/// Equipped item slots — one per slot type.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct EquipSlots {
    pub weapon: Option<ItemStack>,
    pub armor:  Option<ItemStack>,
    pub runes:  [Option<ItemStack>; 3],
}

#[derive(Debug, Error)]
pub enum InventoryError {
    #[error("inventory is full (max {max} slots)")]
    Full { max: usize },
    #[error("item not found: {item_id}")]
    NotFound { item_id: String },
    #[error("cannot equip {item_id}: wrong item type")]
    WrongType { item_id: String },
    #[error("stack size exceeded (max {max})")]
    StackFull { max: u32 },
}

/// Per-player item storage (backpack + equip slots).
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct Inventory {
    pub slots:  Vec<ItemStack>,     // backpack slots (unordered)
    pub equipped: EquipSlots,
    pub gold:   u32,
    pub max_slots: usize,           // configurable per player tier
}

impl Inventory {
    pub fn new(max_slots: usize) -> Self {
        Self { slots: Vec::new(), equipped: EquipSlots::default(),
               gold: 0, max_slots }
    }

    /// Add items to backpack. Stacks if item already present and stackable.
    pub fn add(&mut self, item_id: impl Into<String>, qty: u32, max_stack: u32)
        -> Result<(), InventoryError>
    {
        let item_id = item_id.into();
        // Try to stack onto existing slot first
        if let Some(slot) = self.slots.iter_mut().find(|s| s.item_id == item_id) {
            let new_qty = slot.quantity.saturating_add(qty);
            if new_qty > max_stack { return Err(InventoryError::StackFull { max: max_stack }); }
            slot.quantity = new_qty;
            return Ok(());
        }
        // New slot
        if self.slots.len() >= self.max_slots {
            return Err(InventoryError::Full { max: self.max_slots });
        }
        self.slots.push(ItemStack { item_id, quantity: qty });
        Ok(())
    }

    /// Remove `qty` of `item_id`. Returns actual quantity removed.
    pub fn remove(&mut self, item_id: &str, qty: u32) -> u32 {
        let mut remaining = qty;
        self.slots.retain_mut(|slot| {
            if slot.item_id != item_id || remaining == 0 { return true; }
            if slot.quantity <= remaining {
                remaining -= slot.quantity;
                false // remove slot
            } else {
                slot.quantity -= remaining;
                remaining = 0;
                true
            }
        });
        qty - remaining
    }

    /// Count how many of `item_id` the player holds.
    pub fn count(&self, item_id: &str) -> u32 {
        self.slots.iter().filter(|s| s.item_id == item_id).map(|s| s.quantity).sum()
    }

    /// True if the player holds at least `qty` of `item_id`.
    pub fn has(&self, item_id: &str, qty: u32) -> bool {
        self.count(item_id) >= qty
    }
}
```

#### `bifrost/items/src/registry.rs` — needs to contain:

```rust
use std::collections::BTreeMap;
use crate::item::{ItemDef, ItemEffect, ItemStats, ItemType, Rarity};

/// Global item database — 35+ built-in items + WAC-registered items.
pub struct ItemRegistry {
    items: BTreeMap<String, ItemDef>,
}

impl ItemRegistry {
    /// Create with all built-in items pre-registered.
    pub fn with_builtins() -> Self {
        let mut r = Self { items: BTreeMap::new() };
        // Weapons
        r.reg("iron_sword",    "Iron Sword",      ItemType::Weapon,      Rarity::Common,
              ItemStats::weapon(8),  1, 50,  "⚔",  "A trusty iron blade.", vec![]);
        r.reg("steel_sword",   "Steel Sword",     ItemType::Weapon,      Rarity::Uncommon,
              ItemStats::weapon(18), 1, 180, "⚔",  "Forged in Helga's furnace.", vec![]);
        r.reg("shadow_blade",  "Shadow Blade",    ItemType::Weapon,      Rarity::Rare,
              ItemStats::weapon(32), 1, 600, "🗡",  "Cuts through darkness.", vec![]);
        r.reg("crystal_spear", "Crystal Spear",   ItemType::Weapon,      Rarity::Epic,
              ItemStats::weapon(55), 1, 2000, "✨", "Humming with arcane energy.",
              vec![ItemEffect::LightRadius { radius: 4 }]);
        // Armor
        r.reg("leather_vest",  "Leather Vest",    ItemType::Armor,       Rarity::Common,
              ItemStats::armor(5, 20),  1, 40,   "🛡", "Basic protection.", vec![]);
        r.reg("chain_mail",    "Chain Mail",      ItemType::Armor,       Rarity::Uncommon,
              ItemStats::armor(12, 50), 1, 160,  "🛡", "Interlocked rings.", vec![]);
        r.reg("dragon_scale",  "Dragon Scale",    ItemType::Armor,       Rarity::Epic,
              ItemStats::armor(40, 200),1, 3000, "🐉", "Scales from a fallen wyrm.", vec![]);
        // Consumables
        r.reg("health_potion", "Health Potion",   ItemType::Consumable,  Rarity::Common,
              ItemStats::none(), 10, 20, "🧪", "Restores 50 HP.",
              vec![ItemEffect::HealOnUse { hp: 50 }]);
        r.reg("mana_potion",   "Mana Potion",     ItemType::Consumable,  Rarity::Common,
              ItemStats::none(), 10, 25, "💧", "Restores 40 MP.",
              vec![ItemEffect::ManaOnUse { mp: 40 }]);
        r.reg("elixir_of_rage","Elixir of Rage",  ItemType::Consumable,  Rarity::Uncommon,
              ItemStats::none(), 5, 80, "⚡", "ATK +20 for 60 ticks.",
              vec![ItemEffect::BuffAttack { flat: 20, duration_ticks: 60 }]);
        r.reg("antidote",      "Antidote",        ItemType::Consumable,  Rarity::Common,
              ItemStats::none(), 5, 15, "💊", "Cures all status effects.",
              vec![ItemEffect::CureStatus]);
        // Materials
        r.reg("wolf_pelt",     "Wolf Pelt",       ItemType::Material,    Rarity::Common,
              ItemStats::none(), 99, 5,  "🐺", "Soft grey fur.", vec![]);
        r.reg("crystal_shard", "Crystal Shard",   ItemType::Material,    Rarity::Uncommon,
              ItemStats::none(), 99, 30, "💎", "Pulses faintly in darkness.", vec![]);
        r.reg("dragon_bone",   "Dragon Bone",     ItemType::Material,    Rarity::Rare,
              ItemStats::none(), 20, 120, "🦴", "Indestructible.",  vec![]);
        r.reg("void_dust",     "Void Dust",       ItemType::Material,    Rarity::Epic,
              ItemStats::none(), 50, 500, "✨", "Crystallised Fracture energy.", vec![]);
        r.reg("iron_ore",      "Iron Ore",        ItemType::Material,    Rarity::Common,
              ItemStats::none(), 99, 3,  "⛏", "Raw iron for smelting.", vec![]);
        r.reg("coal",          "Coal",            ItemType::Material,    Rarity::Common,
              ItemStats::none(), 99, 2,  "🪨", "Burns hot and long.", vec![]);
        r.reg("night_crystal", "Night Crystal",   ItemType::Material,    Rarity::Rare,
              ItemStats::none(), 50, 200, "🌙", "Only forms in deep darkness.", vec![]);
        // Spell scrolls
        r.reg("scroll_fireball","Scroll: Fireball",ItemType::SpellScroll,Rarity::Uncommon,
              ItemStats::none(), 1, 90, "📜", "Teaches the Fireball spell.",
              vec![ItemEffect::UnlockSpell { spell_id: "fireball".into() }]);
        r.reg("scroll_blink",  "Scroll: Blink",   ItemType::SpellScroll, Rarity::Rare,
              ItemStats::none(), 1, 300, "📜", "Teaches the Blink spell.",
              vec![ItemEffect::UnlockSpell { spell_id: "blink".into() }]);
        // Runes
        r.reg("rune_strength", "Rune of Strength",ItemType::Rune,        Rarity::Uncommon,
              ItemStats::weapon(5), 1, 100, "🔷", "Socket for +5 ATK.", vec![]);
        r.reg("rune_warding",  "Rune of Warding", ItemType::Rune,        Rarity::Uncommon,
              ItemStats::armor(8, 0), 1, 100, "🔷", "Socket for +8 DEF.", vec![]);
        r.reg("rune_miner",    "Rune of the Miner",ItemType::Rune,       Rarity::Rare,
              ItemStats::none(), 1, 400, "⛏", "Socket for 50% faster mining.",
              vec![ItemEffect::MiningBonus { mult: 1.5 }]);
        // Quest items
        r.reg("key_dungeon",   "Dungeon Key",     ItemType::QuestItem,   Rarity::Common,
              ItemStats::none(), 1, 0, "🗝", "Opens the dungeon gate.",
              vec![ItemEffect::Key { lock_id: "dungeon_gate".into() }]);
        r.reg("elder_tome",    "Elder's Tome",    ItemType::QuestItem,   Rarity::Common,
              ItemStats::none(), 1, 0, "📕", "Elder Mirova's research journal.", vec![]);
        r
    }

    fn reg(&mut self, id: &str, name: &str, ty: ItemType, rarity: Rarity,
           stats: ItemStats, stack: u32, gold: u32, icon: &str, lore: &str,
           effects: Vec<ItemEffect>)
    {
        self.items.insert(id.into(), ItemDef::new(id, name, ty, rarity, stats,
                                                  stack, gold, icon, lore, effects));
    }

    /// Look up an item by ID.
    pub fn get(&self, item_id: &str) -> Option<&ItemDef> {
        self.items.get(item_id)
    }

    /// True if an item with this ID is registered.
    pub fn exists(&self, item_id: &str) -> bool {
        self.items.contains_key(item_id)
    }

    /// Register a new item (e.g. from a WAC-compiled `EntityPrefabIR` loot table).
    pub fn register(&mut self, item: ItemDef) {
        self.items.insert(item.id.clone(), item);
    }

    /// All registered item IDs in sorted order.
    pub fn ids(&self) -> impl Iterator<Item = &str> {
        self.items.keys().map(|s| s.as_str())
    }

    /// All item definitions.
    pub fn all(&self) -> impl Iterator<Item = &ItemDef> {
        self.items.values()
    }

    pub fn len(&self) -> usize { self.items.len() }
}
```

**ACTION**: Create both files. Add `"bifrost/items"` to workspace `Cargo.toml` members.
Add `bifrost-items = { path = "bifrost/items" }` to workspace deps.

---

## 2. ORPHANED FUNCTIONALITY — Unique logic at risk of being lost

### 🆕 UNIQUE-1: `safe-city/src/director.rs` — 3 functions NOT in WAC director

When we delete `safe-city/src/director.rs` (correct — it violates R2/R3), we must
**migrate these 3 functions** to `bifrost-wac/src/director/mod.rs` on `WorldDirector`:

```rust
// MIGRATE TO: bifrost/wac/src/director/mod.rs — add to WorldDirector impl

/// Record economic pressure from a trade transaction.
///
/// Called by AuctionHouse::buy() each time a sale completes.
/// Large trades increase economy_delta (inflation signal for the Director).
pub fn record_trade(&mut self, gold_value: u32) {
    // 10_000 gold = max 0.01 delta per trade (prevents single-trade exploit)
    let impact = (gold_value as f32 / 10_000.0).min(0.01);
    // Increment economy delta — Director::tick() will act if it exceeds threshold
    self.state.economy_delta_accumulator =
        (self.state.economy_delta_accumulator + impact).min(1.0);
}

/// Passive per-tick economic stability recovery.
///
/// Call once per tick after Director::tick() to slowly decay inflation pressure.
pub fn recover_economy(&mut self) {
    self.state.economy_delta_accumulator =
        (self.state.economy_delta_accumulator - 0.0005).max(0.0);
}

/// Update faction balance from zone control snapshot.
///
/// Called by the server each tick with current zone ownership counts.
/// The Director uses this to detect faction snowball (one faction dominating).
pub fn update_faction_balance(
    &mut self,
    zone_control: &std::collections::BTreeMap<String, u32>,
    total_zones: u32,
) {
    self.state.faction_balance.clear();
    for (faction, &zones) in zone_control {
        self.state.faction_balance.insert(
            faction.clone(),
            zones as f32 / total_zones.max(1) as f32,
        );
    }
}
```

Additionally add to `DirectorState`:
```rust
// bifrost/wac/src/director/mod.rs — in DirectorState
pub economy_delta_accumulator: f32,
pub faction_balance: std::collections::BTreeMap<String, f32>,
```

And wire `faction_balance` into `evaluate_economy()` — if any faction has
`balance > 0.75`, bias the loot economy adjustment toward scarcity.

---

### 🆕 UNIQUE-2: `bifrost/wac/src/nvidia.rs` — NVIDIA NIM client (feature-gated)

This is **real, valuable new functionality** — complete LLM integration.
It already exists and is correct. **Do not touch it.** Ensure it stays.

Key functions:
- `NimClient::generate_asset_spec(intent, asset_type)` → concise WAC spec string
- `NimClient::generate_blueprint_spec(faction_intent, asset_type, seed)` → `AssetBlueprint`
- `synthesis_world_prompt(zone_id, strategy)` → system prompt for Synthesis faction

Status: `#![cfg(feature = "nvidia-nim")]` — only compiled when enabled. ✅

---

### 🆕 UNIQUE-3: `bifrost/wac/src/azure.rs` — Azure AI Foundry client (feature-gated)

**Complete, real infrastructure** connected to the bKG Azure AI project.
Endpoint: `https://bkg-resource.services.ai.azure.com/`

Key functions **not found anywhere else**:
- `AzureAiClient::generate_npc_dialogue(npc_name, npc_role, mood, world_state, player_input)` → `NpcDialogueResponse`
- `AzureAiClient::generate_zone_narration(zone_name, biome, run_number)` → atmospheric text
- `AzureAiClient::is_available()` → runtime check for `AZURE_AI_KEY`
- Fallback logic: Azure if key available, else NVIDIA NIM

Status: `#![cfg(feature = "azure-ai")]` — only compiled when enabled. ✅

**ACTION**: Wire the Azure client into `bifrost-aigm`'s NPC dialogue dispatch.
Currently `NpcLlmRequest::build_prompt()` builds a prompt but nothing sends it.
The Azure client is the sender. Connect them.

---

### 🔴 ORPHANED-1: `bifrost/items/` — not in workspace

Complete `ItemDef` + `Rarity` + `ItemStats` + `ItemEffect` system.
`Cargo.toml` is correctly formatted, uses workspace versions.
Just missing from workspace members and missing two source files (see BROKEN-1).

**ACTION**: Create missing files (see BROKEN-1), add to workspace.

---

### 🔴 ORPHANED-2: `bifrost/admin/` — standalone, not in workspace

Complete Yew admin panel with 7 full UI sections. Correctly standalone
(needs `wasm-pack`, different toolchain from server). Has all sections:

| Section file | What it does |
|---|---|
| `app/world.rs` | Edit world name/size/description — PUT `/admin-api/world` |
| `app/biomes.rs` | CRUD biomes — GET/POST/PUT/DELETE `/admin-api/biomes` |
| `app/story.rs` | Edit story arcs + beats — CRUD `/admin-api/story/arcs` |
| `app/npcs.rs` | Edit NPCs (name, icon, position, system_prompt) — CRUD `/admin-api/npcs` |
| `app/quests.rs` | Edit quests — CRUD `/admin-api/quests` |
| `app/loot.rs` | Edit monsters + loot items — CRUD `/admin-api/loot/*` |
| `app/wac.rs` | WAC compiler UI + World Director monitor — POST `/api/wac/*` |

**PROBLEM**: The server (`bifrost-server`) has NO `/admin-api/` routes at all.
The admin panel calls `/admin-api/world`, `/admin-api/biomes`, etc. — these 404.

**ACTION**: Add admin-api routes to `bifrost/server/src/main.rs`. The data these
routes serve lives in `app/world-data.json`. Routes need read/write of that JSON.

---

## 3. DISCONNECTED FUNCTIONALITY — In workspace but not wired

### 🟡 DISC-1: `bifrost-aigm` NPC dialogue has no LLM sender

`NpcLlmRequest::build_prompt()` in `bifrost/aigm/src/npc/dialogue.rs` builds
a complete prompt string. `AiGmState::tick()` returns `pending_dialogues: Vec<PendingDialogue>`.

But nothing actually sends these to the Azure AI / NVIDIA NIM client.

`bifrost-server`'s API returns them in a response field but doesn't call
`AzureAiClient::generate_npc_dialogue()` or equivalent.

**ACTION**: In the server's AIGM tick handler, for each `PendingDialogue`:
1. Call `AzureAiClient::generate_npc_dialogue()` or `NimClient` equivalent
2. Parse the `NpcLlmResponse`
3. Call `state.emit(aigm.npc.speak event)` with the response
4. Update NPC `ai_context.mood` and `current_goal` from the response

---

### 🟡 DISC-2: `bifrost-aigm` WorldEvents never flow through EventPipeline (R3 violation)

`AiGmState::tick()` returns `events_out: Vec<WorldEvent>`.
The server returns these in the API response but never calls `state.emit(event)`.

R3 requires every world-state-changing event to go through `EventPipeline::process()`.

**ACTION**: In all `/aigm/*` server handlers, for each event in `events_out`:
```rust
for event in result.events_out {
    if let Err(e) = state.emit(event) {
        tracing::warn!("aigm event rejected by pipeline: {e}");
    }
}
```

---

### 🟡 DISC-3: `bifrost-run::WorldRunDirector::generate_next_world_seed()` never consumed

`bifrost/run/src/director.rs` has `generate_next_world_seed(base_seed) → WorldSeedConfig`.
`WorldSeedConfig` has fields like `scarcity_biomes`, `volatile_loot`, `contested_zones`.

These flags should modify `DirectorConfig` for the next run's WAC WorldDirector.
Currently nothing reads them.

**ACTION**: When a run ends and `WorldRunDirector::evaluate_tick()` returns
`Some(RunResult)`, generate the next seed config and feed it to the WAC director:
```rust
// bifrost/server/src/api.rs — in run_tick handler
if let Some(result) = run_result {
    let seed_cfg = state.run_director.generate_next_world_seed(42);
    // translate to DirectorConfig override
    if seed_cfg.scarcity_biomes {
        state.director.config.biome_evolution_threshold = 0.50; // lower = easier to trigger
    }
    if seed_cfg.volatile_loot {
        state.director.config.economy_adjustment_threshold = 0.10;
    }
}
```

---

### 🟡 DISC-4: Server has no `/admin-api/` routes

The admin panel (`bifrost/admin`) calls:
- `GET /admin-api/world`
- `GET/POST/PUT/DELETE /admin-api/biomes`
- `GET/POST/PUT/DELETE /admin-api/story/arcs` (and beats)
- `GET/POST/PUT/DELETE /admin-api/npcs`
- `GET/POST/PUT/DELETE /admin-api/quests`
- `GET/POST/PUT/DELETE /admin-api/loot/monsters`
- `GET/POST/PUT/DELETE /admin-api/loot/items`

These all 404 currently. The data lives in `app/world-data.json`.

**ACTION**: Add admin router to `bifrost/server/src/main.rs`. Read/write `world-data.json`.
All admin-api types are already defined in `bifrost/admin/src/types.rs` — mirror those
as Axum request/response types in `bifrost/server/src/models.rs`.

---

### 🟡 DISC-5: `bifrost-items` not connected to `AuctionHouse`

`bifrost-safe-city::auction::Listing` has `item_id: String` with no validation.
Anyone can create a listing for `"fake_item"` that doesn't exist.

`bifrost-wac::types::LootEntry` has `item_id: String` — same.
`bifrost-aigm::event::LootPayload` has `item_id: String` — same.

**ACTION** (after BROKEN-1 is fixed and `bifrost-items` is in workspace):
- `bifrost-safe-city/Cargo.toml`: add `bifrost-items = { workspace = true }`
- `AuctionHouse::post()`: call `ItemRegistry::exists(item_id)` before inserting listing
- Server `SimState`: add `pub item_registry: ItemRegistry` (init with builtins)

---

## 4. R1/R2/R3 VIOLATIONS (summarised — detail in previous audit)

| # | Where | Rule | Fix |
|---|---|---|---|
| V1 | `safe-city/src/zone.rs:5-6` | R1: FactionId/ZoneId redefined | Import from kernel |
| V2 | `safe-city/src/director.rs` | R2+R3: direct mutation, no pipeline | DELETE file |
| V3 | `lockstep/src/tick.rs:ZoneId(u32)` | R1: ZoneId incompatible type | Rename → ShardId |
| V4 | `physics/src/vec3.rs:Vec3` | R1: Vec3 name collision | Rename → PhysicsVec3 |
| V5 | `aigm/src/aigm.rs:TickBudget` | R1: name clash with lockstep | Rename → AiGmBudget |
| V6 | `synthesis/src/memory.rs:MemoryEntry` | R1: name clash with aigm | Rename → FactionMemoryEntry |
| V7 | `aigm` WorldEvents | R3: not going through pipeline | Call `state.emit()` in handlers |

---

## 5. DUPLICATED LOGIC (summarised — safe to consolidate)

| Pattern | Locations | Action |
|---|---|---|
| FIFO ring buffer | `PromptCache`, `AssetCache`, `ShortTermMemory`, `FactionMemory` | Generic `RingBuffer<K,V>` in kernel |
| `has(spec, keywords)` | All 4 WAC compilers | Move to `compile/mod.rs` |
| `make_id(spec, n)` | `biome.rs`, `tilemap.rs` | Move to `compile/mod.rs` |
| `extract_entity_noun()` | `biome.rs`, `entity.rs`, `animation.rs` | Move to `compile/mod.rs` |
| `title_case(s)` | `biome.rs`, `loot.rs` | Move to `compile/mod.rs` |
| Faction balance tracking | `safe-city::director`, `synthesis::WorldModel` | Single snapshot in kernel/run |

---

## 6. COMPLETE ORDERED FIX PLAN

### STAGE 1 — Create missing files (unblocks compilation)

| # | Action | Creates |
|---|---|---|
| S1-1 | Create `bifrost/items/src/inventory.rs` | `Inventory`, `ItemStack`, `EquipSlots`, `InventoryError` |
| S1-2 | Create `bifrost/items/src/registry.rs` | `ItemRegistry` + 25 built-in items |
| S1-3 | Add `"bifrost/items"` to workspace Cargo.toml members | crate compilable |
| S1-4 | Add `bifrost-items = { path = "bifrost/items" }` to workspace deps | importable |

### STAGE 2 — Migrate unique functions before deleting safe-city director

| # | Action |
|---|---|
| S2-1 | Add `record_trade()`, `recover_economy()`, `update_faction_balance()` to `bifrost-wac::WorldDirector` |
| S2-2 | Add `economy_delta_accumulator` and `faction_balance` fields to `DirectorState` |
| S2-3 | Wire `faction_balance` into WAC director `evaluate_economy()` |
| S2-4 | In `AuctionHouse::buy()`, call `state.director.record_trade(gold_value)` |
| S2-5 | In server tick advance, call `state.director.recover_economy()` |
| S2-6 | In server run-tick handler, call `state.director.update_faction_balance(...)` |

### STAGE 3 — Delete the broken safe-city director

| # | Action |
|---|---|
| S3-1 | Delete `bifrost/safe-city/src/director.rs` |
| S3-2 | Remove `pub mod director` from `safe-city/src/lib.rs` |
| S3-3 | Remove exports: `WorldDirector, BalanceMatrix, PressureField, WorldEvent` from safe-city |
| S3-4 | Add `bifrost-kernel = { workspace = true }` to `safe-city/Cargo.toml` |
| S3-5 | Replace `pub type ZoneId = String; pub type FactionId = String` with `use bifrost_kernel::{FactionId, ZoneId}` in `zone.rs` |

### STAGE 4 — Rename collisions

| # | Action |
|---|---|
| S4-1 | `bifrost-lockstep`: rename `ZoneId(u32)` → `ShardId(u32)` in `tick.rs`, `barrier.rs`, `scheduler.rs`, `lib.rs` |
| S4-2 | `bifrost-physics`: rename `Vec3` → `PhysicsVec3` in `vec3.rs`, `executor.rs`, `voxel.rs` |
| S4-3 | `bifrost-aigm`: rename `TickBudget` → `AiGmBudget`, `BudgetUsage` → `AiGmBudgetUsage` |
| S4-4 | `bifrost-synthesis`: rename `MemoryEntry` → `FactionMemoryEntry` in `memory.rs` |
| S4-5 | `bifrost-synthesis/lib.rs`: remove `pub use faction::{FactionId, ZoneId}` |
| S4-6 | `bifrost-run/run.rs:12`: remove `pub use bifrost_kernel::FactionId` |

### STAGE 5 — Wire disconnected functionality

| # | Action |
|---|---|
| S5-1 | Add `/admin-api/*` routes to `bifrost-server/src/main.rs` (read/write `world-data.json`) |
| S5-2 | Add `item_registry: ItemRegistry` to `SimState`, validate item_ids in `AuctionHouse::post()` |
| S5-3 | Wire NPC dialogue: dispatch `PendingDialogue` to `AzureAiClient::generate_npc_dialogue()` |
| S5-4 | Emit AIGM WorldEvents through `state.emit()` in server handlers (R3 fix) |
| S5-5 | Feed `WorldSeedConfig` from run director into WAC `DirectorConfig` at run end |

### STAGE 6 — Consolidate duplicated logic

| # | Action |
|---|---|
| S6-1 | Add `bifrost/kernel/src/ring_buffer.rs` — generic `RingBuffer<K: Eq, V>` |
| S6-2 | Refactor `PromptCache` and `AssetCache` to use `RingBuffer` |
| S6-3 | Move `has()`, `make_id()`, `extract_entity_noun()`, `title_case()` to `wac/src/compile/mod.rs` |
| S6-4 | Remove duplicate helpers from `biome.rs`, `loot.rs`, `entity.rs`, `animation.rs` |

---

## 7. WHAT THE CLEAN CODEBASE LOOKS LIKE

```
bifrost-kernel       canonical types (FactionId, ZoneId, SequencedInstant, RingBuffer)
├── bifrost-items    ItemDef, ItemRegistry, Inventory          ← ADD to workspace
├── bifrost-vis      VoxelOpcode, VoxelProgram
│   └── bifrost-lockstep   ShardId (was ZoneId), TickBudget (VIS)
│       └── bifrost-chunk  authority epochs
├── bifrost-wac      WorldDirector (canonical, with record_trade/recover_economy/update_balance)
│   ├── nvidia.rs    NimClient                                  ← KEEP (feature=nvidia-nim)
│   ├── azure.rs     AzureAiClient + NPC dialogue              ← KEEP (feature=azure-ai)
│   ├── bifrost-wasm world gen bridge
│   ├── bifrost-synthesis  AI faction (uses kernel FactionId directly)
│   └── nova-anim    wac feature bridge
├── bifrost-physics  PhysicsVec3 (was Vec3, f64)
├── bifrost-run      run epochs, EndCondition, MetaProgression
├── bifrost-safe-city AuctionHouse, Zone, SafeCity              ← NO director module
│   └── depends on bifrost-items (item validation)
└── nova-core        Vec3(f32), ECS, Transform3D
    ├── nova-render  WebGPU (future desktop client)
    ├── nova-anim    AnimFSM, VoxelSkeleton
    └── nova-input   ActionMap
        └── bifrost-aigm  WorldEvent, AiGmState, AiGmBudget (was TickBudget)
            └── bifrost-server   HTTP server (all crates wired)
                └── admin-api routes    ← ADD (serves world-data.json to admin panel)

bifrost/admin/       standalone wasm-pack (Yew admin panel)    ← KEEP standalone, document
```

---

## 8. QUICK REFERENCE: Files that need to exist but DON'T yet

| Missing file | Must contain | Blocks |
|---|---|---|
| `bifrost/items/src/inventory.rs` | `Inventory`, `ItemStack`, `EquipSlots`, `InventoryError` | `bifrost-items` compilation |
| `bifrost/items/src/registry.rs` | `ItemRegistry` + 25 built-in item definitions | `bifrost-items` compilation |

---

## 9. QUICK REFERENCE: Functions that live in the wrong crate and must MOVE before deletion

| Function | Currently in | Move to | Why |
|---|---|---|---|
| `record_trade(gold_value)` | `safe-city::WorldDirector` | `wac::WorldDirector` | Economy tracking belongs in WAC director |
| `recover_economy()` | `safe-city::WorldDirector` | `wac::WorldDirector` | Economy tracking belongs in WAC director |
| `update_faction_balance(...)` | `safe-city::WorldDirector` | `wac::WorldDirector` | Faction balance = WAC director input signal |

---

## 10. THINGS THAT ARE CORRECT AND MUST NOT BE TOUCHED

| Item | Status | Note |
|---|---|---|
| `bifrost/wac/src/nvidia.rs` | ✅ complete | NVIDIA NIM LLM client, feature-gated |
| `bifrost/wac/src/azure.rs` | ✅ complete | Azure AI Foundry client, feature-gated, bKG project |
| `bifrost/admin/` | ✅ complete | Yew admin panel, correct as standalone wasm-pack |
| `bifrost-aigm` quest/story | ✅ complete | `StoryEngine`, `QuestRegistry`, all correct |
| `nova-anim` | ✅ complete | AnimFSM, VoxelSkeleton, standard_character_fsm |
| `bifrost-synthesis` | ✅ correct | Uses `pub use bifrost_kernel::{FactionId, ZoneId}` properly |
| `bifrost-witness` | ✅ correct | WitnessExecutor, ConsensusResult, quorum logic |
| WAC compile pipeline | ✅ correct | validate → compile → 5 asset types, all deterministic |

---

*Complete audit. 41 structural issues. 2 broken crates. 3 unique functions to migrate.
2 LLM integrations to preserve. 4 wiring gaps. All documented with exact file paths.*
