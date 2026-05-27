# bKG — Quests

> Chains · Objectives · Rewards · Server Authority

---

## 1. System

Quests are managed by `bifrost-aigm::QuestRegistry` on the server.
The game client fetches them at startup — no hardcoded chains.

```
bifrost-aigm QuestRegistry (server)
      ↕  /aigm/quests  (drift fix PR 3)
app/game.html  ← falls back to local data if server offline
```

---

## 2. Quest States

```rust
pub enum QuestState {
    Available,        // offered by NPC
    Active,           // in progress
    ReadyToComplete,  // objectives met, awaiting turn-in
    Completed,
}
```

---

## 3. Current Quest Chains

### Guard Captain Aldric

| Stage | Objective | Reward |
|---|---|---|
| g1 Wolf Menace | Kill 5 wolves | 40g · 80 XP |
| g2 Goblin Raiders | Kill 8 goblins | 70g · 140 XP |
| g3 Goblin Chief | Kill the boss | 120g · 250 XP |

### Innkeeper Bram

| Stage | Objective | Reward |
|---|---|---|
| i1 Rat Infestation | Kill 5 rats | 30g · 60 XP |
| i2 Spider Nest | Kill 6 spiders | 55g · 110 XP |
| i3 Forest Troll | Kill 2 trolls | 100g · 200 XP |

### Elder Mirova

| Stage | Objective | Reward |
|---|---|---|
| e1 Ancient Text | Kill 4 skeletons | 60g · 120 XP |
| e2 Dark Crystals | Kill 3 trolls | 80g · 160 XP |
| e3 Stop the Ritual | Kill the Lich | 150g · 350 XP |

### Wizard Seraphon

| Stage | Objective | Reward |
|---|---|---|
| w1 Fire Elementals | Kill 4 elementals | 65g · 130 XP |
| w2 Dungeon Archive | Kill 6 skeletons | 90g · 180 XP |
| w3 Dungeon Lich | Kill the Lich | 200g · 500 XP |

---

## 4. HTTP API *(drift fix PR 3)*

```bash
GET  /aigm/quests
GET  /aigm/quests/:id
POST /aigm/quests/:id/accept
POST /aigm/quests/:id/progress
POST /aigm/quests/:id/complete
```

---

## 5. Planned Quest Types

| Type | Description |
|---|---|
| Exploration | Discover a zone or landmark |
| Collection | Gather crafting materials |
| Escort | Keep NPC alive during travel |
| Economy | Deliver goods to Safe City auction |
| World event | Respond to Synthesis AI activity |
| Meta | Survive / win a full run |

---

## See Also

- [`game/npcs.md`](npcs.md) — quest-giver NPCs
- [`game/monsters.md`](monsters.md) — kill targets
- [`game/players.md`](players.md) — XP and rewards
