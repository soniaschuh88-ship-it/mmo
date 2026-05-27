# bKG — Skills

> Class skill trees · passives · actives · world manipulation

---

## 1. Overview

Each class has 3 branches × 4 skills = 12 skills.
Skill Points are earned on level-up.

| Type | Effect |
|---|---|
| **Passive** | Permanent stat bonus — applied on unlock |
| **Active** | Hotbar ability with MP cost + cooldown |
| **World** *(future)* | Terrain / biome / economy rule manipulation |

---

## 2. Warrior

### Strength
| Skill | Effect |
|---|---|
| Iron Muscles | +5 ATK |
| Battle Rage | +8 ATK, +5% CRIT |
| Weapon Mastery | +12 ATK |
| **Power Strike** | 2.3× ATK · 16 MP · 2.2s CD |

### Defense
| Skill | Effect |
|---|---|
| Thick Hide | +6 DEF |
| Iron Guard | +8 DEF, +20 HP |
| Fortress | +10 DEF, −10% dmg taken |
| **Shield Slam** | 1.6× ATK + stun · 22 MP · 3s CD |

### Berserker
| Skill | Effect |
|---|---|
| Battle Lust | Heal 10 HP on kill |
| Blood Rage | +10 ATK when HP < 30% |
| Undying | Survive one lethal hit |
| **Berserker** | 3.5× ATK, ignore DEF, costs 10% HP · 4.2s CD |

---

## 3. Mage

### Fire
| Skill | Effect |
|---|---|
| Kindle | +5 ATK |
| Ignite | +9 ATK |
| Inferno | +13 ATK |
| **Fireball** | 3.2× fire dmg · 28 MP · 2.6s CD |

### Arcane
| Skill | Effect |
|---|---|
| Mana Font | +35 max MP |
| Arcane Power | +7 ATK |
| Spell Haste | −20% all cooldowns |
| **Arcane Burst** | 4.8× arcane dmg · 42 MP · 4.2s CD |

### Frost
| Skill | Effect |
|---|---|
| Frost Touch | +5 DEF |
| Ice Shield | +7 DEF, 15% reflect |
| Frozen Core | +25 HP, +5 DEF |
| **Frost Nova** | Freeze 2.2s · 20 MP · 3.2s CD |

---

## 4. Rogue

### Daggers
| Skill | Effect |
|---|---|
| Blade Mastery | +5 ATK |
| Keen Edge | +8 ATK, +8% CRIT |
| Assassin | +12 ATK, +12% CRIT |
| **Backstab** | 2.8× ATK, ignore DEF · 16 MP · 2.1s CD |

### Shadow
| Skill | Effect |
|---|---|
| Shadow Veil | +12% dodge |
| Evasion | +20% dodge |
| Phantom | +28% dodge |
| **Vanish** | Invulnerable 2.2s · 22 MP · 5.5s CD |

### Poisons
| Skill | Effect |
|---|---|
| Venom Blade | +4 poison/hit |
| Toxic Cloud | +7 poison/hit |
| Deadly Toxin | +11 poison/hit |
| **Smoke Bomb** | Enemy misses ×3 · 16 MP · 3.2s CD |

---

## 5. Future: World Manipulation Skills

```
TERRAIN        shape ground tiles in owned zones
ECONOMY        reduce auction tax, influence loot weights
BIOME          slow biome corruption, trigger defensive evolution  (level 30+)
FACTION        accelerate zone control, debuff Synthesis agents
```

---

## See Also

- [`game/players.md`](players.md) — skill point acquisition
- [`engine/client-runtime.md`](../engine/client-runtime.md) — AnimFSM for skill visuals
- [`engine/wac.md`](../engine/wac.md) — world-manipulation skill backend
