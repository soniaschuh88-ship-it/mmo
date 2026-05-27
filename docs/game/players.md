# bKG — Players

> Identity · Clone System · Meta Progression · Skills

---

## 1. Player Entity

```rust
pub struct PlayerEntity {
    pub id:            PlayerId,
    pub body:          Option<BodyId>,   // None if dead / unclaimed
    pub clone_charges: u32,
    pub memory_core:   MemoryGraph,
}
```

---

## 2. Death & Clone System

### Death Flow

1. Body dies in world
2. Memory snapshot saved to Safe City vault
3. Inventory split — **lost** items drop to world; **secured** items stay in vault
4. Clone spawns if charges available

### Clone Rules

- Same memory graph as the dead body
- Small entropy drift applied (prevents perfect exploit replication)
- Skill decay on death — soft reset, not full wipe

> No true permadeath for progression — but real loss for risk-taking.

---

## 3. Class System

| Class | HP | ATK | DEF | CRIT | Playstyle |
|---|---|---|---|---|---|
| Warrior | 140 | 16 | 12 | 8% | Frontline tank, high survivability |
| Mage | 80 | 24 | 5 | 14% | Burst damage, fragile |
| Rogue | 100 | 19 | 7 | 26% | High crit, dodge, poison |

---

## 4. Voxel Character Model

Each class is an isometric 8×12 voxel model rendered by the Bifrost client runtime.
Bone groups are defined in `nova-anim::VoxelSkeleton::humanoid()`.

See [`engine/client-runtime.md`](../engine/client-runtime.md) for the AnimFSM details.

---

## 5. Meta Progression

| Layer | Scope | Contents |
|---|---|---|
| Run Progression | Resets each run | Skills, gear, bases, territory |
| Meta Progression | Persistent | Archetypes, unlocks, starting perks |

Meta progression lives in the Safe City and persists across all runs.

---

## 6. Skill System

Skills are unlocked via Skill Points (gained on level-up).

| Category | Effect |
|---|---|
| **Passive** | Permanent stat bonus — applied on unlock |
| **Active** | Hotbar ability with MP cost + cooldown |
| **World** *(future)* | Terrain / biome / economy manipulation rules |

See [`game/skills.md`](skills.md) for the full skill trees.

---

## 7. Equipment

Slots: **Weapon · Armor · Accessory**

- Auto-equipped if better than current item
- Crafted via WAC in Safe City
- Traded on the Auction House

---

## See Also

- [`game/skills.md`](skills.md)
- [`game/quests.md`](quests.md)
- [`game/world.md`](world.md)
