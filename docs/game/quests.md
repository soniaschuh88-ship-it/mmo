# bKG — Quests

> Quest chains · objectives · rewards · server authority

---

## 1. Quest System Overview

Quests are managed by `bifrost-aigm::QuestRegistry`.
The game client fetches quests from the server at startup — no hardcoded chains.

```
bifrost-aigm QuestRegistry (server)
      ↕  /aigm/quests
app/game.html  (client fallback if offline)
```

---

## 2. Quest States

```rust
pub enum QuestState {
    Available,        // offered by NPC
    Active,           // accepted, objectives in progress
    ReadyToComplete,  // all objectives met, awaiting turn-in
    Completed,        // successfully completed
}
```

---

## 3. Current Quest Chains

### Chain: Guard Captain Aldric

| Stage | Title | Objective | Reward |
|---|---|---|---|
| g1 | Wolf Menace | Kill 5 wolves | 40g · 80 XP |
| g2 | Goblin Raiders | Kill 8 goblins | 70g · 140 XP |
| g3 | The Goblin Chief | Kill the Goblin Chief | 120g · 250 XP |

### Chain: Innkeeper Bram

| Stage | Title | Objective | Reward |
|---|---|---|---|
| i1 | Rat Infestation | Kill 5 rats | 30g · 60 XP |
| i2 | Spider Nest | Kill 6 spiders | 55g · 110 XP |
| i3 | Forest Troll | Kill 2 trolls | 100g · 200 XP |

### Chain: Elder Mirova

| Stage | Title | Objective | Reward |
|---|---|---|---|
| e1 | Ancient Text | Kill 4 skeletons | 60g · 120 XP |
| e2 | Dark Crystals | Kill 3 trolls | 80g · 160 XP |
| e3 | Stop the Ritual | Kill the Dungeon Lich | 150g · 350 XP |

### Chain: Wizard Seraphon

| Stage | Title | Objective | Reward |
|---|---|---|---|
| w1 | Fire Elementals | Kill 4 elementals | 65g · 130 XP |
| w2 | Dungeon Archive | Kill 6 skeletons | 90g · 180 XP |
| w3 | The Dungeon Lich | Kill the Dungeon Lich | 200g · 500 XP |

---

## 4. Quest Object Structure

```rust
pub struct Quest {
    pub id:           Uuid,
    pub state:        QuestState,
    pub giver_npc:    NpcId,
    pub title:        String,
    pub description:  String,
    pub objectives:   Vec<QuestObjective>,
    pub rewards:      QuestReward,
    pub next_quest:   Option<Uuid>,
}

pub struct QuestObjective {
    pub kind:         ObjectiveKind,  // KillCount, Collect, Explore
    pub target:       String,         // monster type / item id / zone id
    pub required:     u32,
    pub current:      u32,
}
```

---

## 5. HTTP API

```bash
GET  /aigm/quests                       # all available quests
GET  /aigm/quests/:id                   # single quest detail
POST /aigm/quests/:id/accept            # player accepts quest
POST /aigm/quests/:id/progress          # update kill/collect count
POST /aigm/quests/:id/complete          # turn in (server validates)
```

*(Routes planned — drift fix PR 3)*

---

## 6. Future Quest Types

| Type | Description |
|---|---|
| Exploration | Discover a zone / landmark |
| Collection | Gather crafting materials |
| Escort | Keep NPC alive during travel |
| Economy | Deliver goods to Safe City auction |
| World event | Respond to Synthesis AI activity |
| Meta | Survive a full run / win a run |

---

## See Also

- [`game/npcs.md`](npcs.md) — Quest-giver NPCs
- [`game/monsters.md`](monsters.md) — Kill targets
- [`game/players.md`](players.md) — Rewards and progression
