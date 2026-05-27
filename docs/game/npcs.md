# bKG — NPCs

> AI Game Master · dialogue · quests · behavior layers

---

## 1. Three Behavior Layers

| Layer | System | Description |
|---|---|---|
| 1 | `NpcBehavior` FSM | Reactive — idle, patrol, speak, flee |
| 2 | `DialogueQueue` | LLM-backed dialogue with cooldown |
| 3 | `AiContext` | Long-term memory, goals, relationships |

---

## 2. Village NPCs

| Name | Role | Quest Chain |
|---|---|---|
| Guard Captain Aldric | Zone defense | `chain_guard` |
| Innkeeper Bram | Village problems | `chain_inn` |
| Elder Mirova | Ancient knowledge | `chain_elder` |
| Wizard Seraphon | Dungeon | `chain_wiz` |
| Blacksmith Helga | Vendor — weapons | — |
| Healer Lyris | Vendor — potions | — |

---

## 3. Behavior FSM (Layer 1)

```
idle ──(player_near)──► speak ──(done)──► idle
idle ──(timeout)──────► patrol ──────────► idle
patrol ──(threat)──────► flee
speak ──(quest)────────► quest_dialogue
```

---

## 4. Dialogue (Layer 2)

```rust
pub struct NpcLlmRequest {
    pub npc_id:               NpcId,
    pub player_context:       PlayerDialogueContext,
    pub world_state:          WorldStateSnapshot,
    pub conversation_history: Vec<DialogueTurn>,
}
```

LLM responses are validated before display. Short-term memory prevents repetition.

---

## 5. HTTP API *(drift fix PR 3)*

```bash
GET  /aigm/npcs          # list NPCs in active zone
GET  /aigm/npcs/:id      # single NPC state
POST /aigm/npcs/:id/speak
POST /aigm/tick          # advance all NPC FSMs
```

---

## See Also

- [`game/quests.md`](quests.md)
- [`game/world.md`](world.md)
- [`engine/wac.md`](../engine/wac.md)
