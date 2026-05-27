# bKG — Skills

> Class skill trees · passives · actives · world manipulation

---

## 1. Overview

Skills are unlocked via **Skill Points** (gained on level-up).
Each class has 3 branches × 4 skills = 12 skills.

| Type | Effect | Unlock |
|---|---|---|
| **Passive** | Permanent stat bonus, applied immediately | Instant |
| **Active** | Hotbar ability with MP cost + cooldown | Adds to hotbar |
| **World** *(future)* | Terrain / biome / economy manipulation | Meta progression |

---

## 2. Warrior

### Branch 1 — Strength

| Skill | Type | Effect |
|---|---|---|
| Iron Muscles | Passive | +5 ATK |
| Battle Rage | Passive | +8 ATK, +5% CRIT |
| Weapon Mastery | Passive | +12 ATK |
| Power Strike | **Active** | 2.3× ATK, 16 MP, 2.2s CD |

### Branch 2 — Defense

| Skill | Type | Effect |
|---|---|---|
| Thick Hide | Passive | +6 DEF |
| Iron Guard | Passive | +8 DEF, +20 HP |
| Fortress | Passive | +10 DEF, −10% damage taken |
| Shield Slam | **Active** | 1.6× ATK + stun, 22 MP, 3s CD |

### Branch 3 — Berserker

| Skill | Type | Effect |
|---|---|---|
| Battle Lust | Passive | Heal 10 HP on kill |
| Blood Rage | Passive | +10 ATK when HP < 30% |
| Undying | Passive | Survive one lethal hit |
| Berserker | **Active** | 3.5× ATK, ignore DEF, costs 10% HP, 4.2s CD |

---

## 3. Mage

### Branch 1 — Fire

| Skill | Type | Effect |
|---|---|---|
| Kindle | Passive | +5 ATK |
| Ignite | Passive | +9 ATK |
| Inferno | Passive | +13 ATK |
| Fireball | **Active** | 3.2× fire damage, 28 MP, 2.6s CD |

### Branch 2 — Arcane

| Skill | Type | Effect |
|---|---|---|
| Mana Font | Passive | +35 max MP |
| Arcane Power | Passive | +7 ATK |
| Spell Haste | Passive | −20% all cooldowns |
| Arcane Burst | **Active** | 4.8× arcane damage, 42 MP, 4.2s CD |

### Branch 3 — Frost

| Skill | Type | Effect |
|---|---|---|
| Frost Touch | Passive | +5 DEF |
| Ice Shield | Passive | +7 DEF, 15% damage reflect |
| Frozen Core | Passive | +25 HP, +5 DEF |
| Frost Nova | **Active** | Freeze enemy 2.2s (stun), 20 MP, 3.2s CD |

---

## 4. Rogue

### Branch 1 — Daggers

| Skill | Type | Effect |
|---|---|---|
| Blade Mastery | Passive | +5 ATK |
| Keen Edge | Passive | +8 ATK, +8% CRIT |
| Assassin | Passive | +12 ATK, +12% CRIT |
| Backstab | **Active** | 2.8× ATK, ignore DEF, 16 MP, 2.1s CD |

### Branch 2 — Shadow

| Skill | Type | Effect |
|---|---|---|
| Shadow Veil | Passive | +12% dodge |
| Evasion | Passive | +20% dodge |
| Phantom | Passive | +28% dodge |
| Vanish | **Active** | Invulnerable 2.2s, 22 MP, 5.5s CD |

### Branch 3 — Poisons

| Skill | Type | Effect |
|---|---|---|
| Venom Blade | Passive | +4 poison / hit |
| Toxic Cloud | Passive | +7 poison / hit |
| Deadly Toxin | Passive | +11 poison / hit |
| Smoke Bomb | **Active** | Enemy misses ×3, 16 MP, 3.2s CD |

---

## 5. Future: World Manipulation Skills

Planned for meta-progression:

```
TERRAIN domain
  - Shape ground tiles in owned zones
  - Place defensive structures via WAC

ECONOMY domain
  - Reduce auction tax in Safe City
  - Influence loot table weights

BIOME domain  (unlock at level 30+)
  - Slow biome corruption in controlled zones
  - Trigger defensive biome evolution

FACTION INFLUENCE
  - Accelerate zone control speed
  - Debuff Synthesis AI agents in adjacent zones
```

---

## See Also

- [`game/players.md`](players.md) — Skill point acquisition
- [`engine/client-runtime.md`](../engine/client-runtime.md) — AnimFSM for skill animations
- [`engine/wac.md`](../engine/wac.md) — World-manipulation skill backend
