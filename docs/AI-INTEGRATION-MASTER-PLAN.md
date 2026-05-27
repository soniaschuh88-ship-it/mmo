# AI Integration Master Plan — DRCF MMO

> DELPHOS = deterministic truth kernel  
> BIFROST = event-driven reality bridge  
> AIGM = emergent narrative + NPC intelligence  
> **WHAT'S MISSING**: Intent validation + World Director + Hierarchical Memory

---

## Current State (Audit)

✅ **What exists:**
- `bifrost-aigm` (event-driven, tick-based, budget-enforced)
- `TickBudget` hard caps (50 AI events/tick max)
- 3-layer NPC model (FSM + LLM + Cache)
- `WorldEvent` ledger with BLAKE3 integrity chain
- `AiGmState::ingest()` for event projection

❌ **Critical gaps:**
1. **No intent validation layer** — AI can emit invalid events
2. **No world director** — no global pacing/orchestration
3. **No hierarchical NPC memory** — all memories treated equally
4. **No faction AI** — factions are dumb
5. **No semantic region system** — NPCs don't understand geography
6. **No animation state versioning** — strings are fragile
7. **No reality replay debugger** — hard to diagnose divergences
8. **No emergent history engine** — ledger not used as lore source

---

## PHASE 1: Intent Validation Layer (CRITICAL)

**Goal**: Prevent LLM hallucinations from corrupting world state.

### Problem

Currently:
```rust
AI → WorldEvent (direct emission)
```

If AI decides to:
- Spawn 500 dragons
- Delete all players' inventory
- Break economy with infinite gold
- Create contradictory quests

There's **no safety net**. It just happens.

### Solution: IntentEvent System

**New file: `bifrost/aigm/src/intent.rs`**

```rust
/// AI wants to do something. INTENT is not yet FACT.
#[derive(Debug, Clone)]
pub enum IntentEvent {
    RequestQuestGeneration {
        giver_npc_id: String,
        title: String,
        objectives: Vec<String>,
    },
    RequestNpcDialogue {
        npc_id: String,
        target_player_id: Option<String>,
        trigger: DialogueTrigger,
    },
    RequestMonsterSpawn {
        monster_type: String,
        zone_id: String,
        position: Vec3,
        difficulty: f32,
    },
    RequestWeatherChange {
        zone_id: String,
        new_weather: String,
        duration_ticks: u32,
    },
    RequestFactionAction {
        faction_id: String,
        action_type: String,
        targets: Vec<String>,
    },
}

/// Validation rules that INTENT must pass
pub struct IntentValidator;

impl IntentValidator {
    /// Quest: cooldown, zone rules, economy impact
    pub fn validate_quest_intent(
        &self,
        intent: &IntentEvent,
        state: &AiGmState,
    ) -> Result<(), ValidationError> {
        // Rule 1: Cooldown
        if state.last_quest_at_tick + state.quest_cooldown_ticks > current_tick {
            return Err(ValidationError::Cooldown);
        }

        // Rule 2: Zone rules (no "dark magic" in holy zone)
        if intent.zone_rules.prohibits(intent.action) {
            return Err(ValidationError::ZoneRule);
        }

        // Rule 3: Event budget
        if state.budget_used.ai_events >= state.budget.max_ai_events {
            return Err(ValidationError::BudgetExhausted);
        }

        // Rule 4: Lore consistency
        if !self.is_lore_consistent(intent, &state.story_engine) {
            return Err(ValidationError::LoreViolation);
        }

        // Rule 5: Anti-spam
        if self.is_too_similar_to_recent(intent, &state.recent_events) {
            return Err(ValidationError::TooSimilar);
        }

        Ok(())
    }

    /// Monster: level scaling, spawn pressure, difficulty curve
    pub fn validate_monster_intent(
        &self,
        intent: &IntentEvent,
        state: &AiGmState,
    ) -> Result<(), ValidationError> {
        // Rule 1: Current zone population
        let zone_population = state.get_player_count(intent.zone_id);
        if zone_population < intent.minimum_players {
            return Err(ValidationError::InsufficientPlayers);
        }

        // Rule 2: Difficulty curve (can't spawn boss to lvl 1 players)
        if intent.difficulty > zone_population.max_safe_difficulty() {
            return Err(ValidationError::DifficultyCurveBroken);
        }

        // Rule 3: Spawn pressure (prevent spawn spam)
        let recent_spawns = state.count_recent_spawns(intent.zone_id, last_100_ticks);
        if recent_spawns > SPAWN_PRESSURE_LIMIT {
            return Err(ValidationError::SpawnPressure);
        }

        Ok(())
    }
}
```

### Integration into AiGmState

```rust
pub struct AiGmTick {
    pub events_out: Vec<WorldEvent>,
    pub pending_intents: Vec<IntentEvent>,  // NEW: unvalidated intents
    pub pending_dialogues: Vec<PendingDialogue>,
    pub validation_failures: Vec<ValidationError>,  // diagnostic
    pub budget_used: BudgetUsage,
}

impl AiGmState {
    pub fn tick(
        &mut self,
        incoming_events: &[WorldEvent],
        npc_input: &NpcTickInput,
        budget: TickBudget,
        now_ms: u64,
        current_tick: u64,
    ) -> AiGmTick {
        // 1. Ingest
        self.ingest(incoming_events);

        // 2. NPC FSM → intents (not direct events)
        let all_intents = self.npc_registry.tick(npc_input, now_ms);

        // 3. VALIDATE each intent
        let mut validated_intents = Vec::new();
        let mut validation_failures = Vec::new();

        for intent in all_intents {
            match self.validator.validate(&intent, self) {
                Ok(()) => validated_intents.push(intent),
                Err(e) => validation_failures.push(e),  // log, don't emit
            }
        }

        // 4. Compile validated intents → events
        let events_out = self.intent_compiler.compile(validated_intents);

        // 5. Story beats (already validated by StoryEngine)
        // ...
    }
}
```

**Deliverables for Phase 1:**
- [ ] `bifrost/aigm/src/intent.rs` — `IntentEvent` enum + `IntentValidator`
- [ ] `bifrost/aigm/src/compiler.rs` — `IntentCompiler` (intents → events)
- [ ] Update `AiGmState::tick()` to use intent pipeline
- [ ] Tests for validation rules
- [ ] Diagnostic logging for rejected intents

---

## PHASE 2: World Director

**Goal**: Global orchestration. Prevent cascading AI events. Maintain world pacing.

### Problem

Currently NPCs and AI are **completely decoupled**. No global awareness of:
- "Are we in a war?"
- "Is the economy inflating?"
- "Are players frustrated?"
- "Are we spawning too many monsters?"

This leads to:
- Runaway spawning
- Contradictory story beats
- Economy collapse
- Player burnout

### Solution: WorldDirector Service

**New file: `bifrost/aigm/src/director.rs`**

```rust
/// Monitors world state. Issues directives to AI systems.
#[derive(Debug)]
pub struct WorldDirector {
    /// How many monsters are actively spawned?
    pub active_combat_density: f32,

    /// Player frustration (inferred from death rate, quest fail rate, log-outs)
    pub player_frustration: f32,

    /// Quest completion rate vs. creation rate
    pub quest_velocity: f32,

    /// Economy inflation index
    pub inflation_index: f32,

    /// Zone activity (players per zone)
    pub zone_activity: HashMap<String, u32>,

    /// AI event pressure (events emitted per tick, rolling avg)
    pub ai_event_pressure: f32,

    /// Story intensity (0.0 = calm, 1.0 = apocalypse)
    pub story_intensity: f32,

    /// Current world mood (affects NPC behavior, quest selection, spawn rates)
    pub world_mood: WorldMood,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum WorldMood {
    Peaceful,
    Tense,
    WarTime,
    Celebration,
    Plague,
}

impl WorldDirector {
    /// Compute world state from ledger
    pub fn update(&mut self, state: &AiGmState, current_tick: u64) {
        // Combat density: count active CombatDamage events in last 100 ticks
        self.active_combat_density = state
            .recent_events
            .iter()
            .filter(|e| matches!(e.event_type, EventType::CombatDamage))
            .count() as f32 / 100.0;

        // Player frustration: (deaths + quest_fails) / active_players
        let deaths = state.recent_events.iter()
            .filter(|e| matches!(e.event_type, EventType::CombatDeath))
            .count();
        let frustration_score = deaths as f32 / state.active_player_count.max(1) as f32;
        self.player_frustration = frustration_score.clamp(0.0, 1.0);

        // Quest velocity
        let created = state.recent_events.iter()
            .filter(|e| matches!(e.event_type, EventType::AigmQuestCreate))
            .count();
        let completed = state.recent_events.iter()
            .filter(|e| matches!(e.event_type, EventType::AigmQuestComplete))
            .count();
        self.quest_velocity = (completed as f32) / (created.max(1) as f32);

        // Inflation: gold dropped vs. gold spent (simplified)
        let loot_gold = state.recent_events.iter()
            .filter_map(|e| {
                if let EventPayload::EconomyLootDrop(loot) = &e.payload {
                    Some(loot.gold_value)
                } else {
                    None
                }
            })
            .sum::<u32>();
        let spent_gold = state.recent_events.iter()
            .filter_map(|e| {
                if let EventPayload::EconomyTrade(trade) = &e.payload {
                    Some(trade.gold)
                } else {
                    None
                }
            })
            .sum::<u32>();
        self.inflation_index = (loot_gold as f32) / (spent_gold.max(1) as f32);

        // Update mood based on metrics
        self.update_mood();
    }

    /// Update world mood based on current metrics
    fn update_mood(&mut self) {
        if self.active_combat_density > 0.8 || self.story_intensity > 0.7 {
            self.world_mood = WorldMood::WarTime;
        } else if self.player_frustration > 0.6 {
            self.world_mood = WorldMood::Tense;
        } else if self.active_combat_density < 0.1 && self.player_frustration < 0.2 {
            self.world_mood = WorldMood::Peaceful;
        } else {
            self.world_mood = WorldMood::Celebration;
        }
    }

    /// Issue directives to regulate AI behavior
    pub fn compute_directives(&self) -> DirectorDirectives {
        DirectorDirectives {
            // If too much combat, tell AI to reduce spawn rate
            spawn_rate_multiplier: if self.active_combat_density > 0.7 {
                0.5
            } else if self.active_combat_density < 0.2 {
                1.5
            } else {
                1.0
            },

            // If players frustrated, create easier quests
            quest_difficulty_delta: if self.player_frustration > 0.6 {
                -1.0
            } else {
                0.0
            },

            // If economy inflating, reduce loot drops
            loot_multiplier: if self.inflation_index > 2.0 {
                0.7
            } else {
                1.0
            },

            // If story intensity high, increase dramatic events
            story_beat_rate: self.story_intensity,

            // Faction aggression scales with war intensity
            faction_aggression_multiplier: match self.world_mood {
                WorldMood::WarTime => 2.0,
                WorldMood::Tense => 1.5,
                WorldMood::Peaceful => 0.5,
                _ => 1.0,
            },
        }
    }
}

pub struct DirectorDirectives {
    pub spawn_rate_multiplier: f32,
    pub quest_difficulty_delta: f32,
    pub loot_multiplier: f32,
    pub story_beat_rate: f32,
    pub faction_aggression_multiplier: f32,
}
```

### Integration

```rust
pub struct AiGmState {
    // ... existing fields ...
    pub director: WorldDirector,
}

impl AiGmState {
    pub fn tick(&mut self, ...) -> AiGmTick {
        // 1. Update director with new information
        self.director.update(self, current_tick);

        // 2. Get directives
        let directives = self.director.compute_directives();

        // 3. Apply directives when spawning/quest-creating
        for intent in &validated_intents {
            match intent {
                IntentEvent::RequestMonsterSpawn { .. } => {
                    // Scale spawn by multiplier
                    if rand::random::<f32>() > directives.spawn_rate_multiplier {
                        continue;  // skip this spawn
                    }
                },
                IntentEvent::RequestQuestGeneration { .. } => {
                    // Adjust difficulty
                    let adjusted_difficulty = difficulty + directives.quest_difficulty_delta;
                    // ...
                },
                _ => {}
            }
        }
    }
}
```

**Deliverables for Phase 2:**
- [ ] `bifrost/aigm/src/director.rs` — `WorldDirector` + `WorldMood`
- [ ] Director metrics computation (from event ledger)
- [ ] Directive issuance system
- [ ] Integration into `AiGmState::tick()`
- [ ] Tests for mood transitions + directive correctness

---

## PHASE 3: Hierarchical NPC Memory

**Goal**: NPCs remember relevant things, forget irrelevant things.

### Problem

Current memory:
```rust
vec![event_id_1, event_id_2, event_id_3, ...]
```

All events equally important. This means:
- NPC remembers trivial chat from 1000 ticks ago
- NPC forgets important boss death
- NPCs can't build narratives

### Solution: Memory Stack

**New file: `bifrost/aigm/src/memory.rs`**

```rust
/// Hierarchical memory for NPCs
#[derive(Debug, Clone)]
pub struct MemoryStack {
    /// Immediate sensory input (current tick, max 10 entries)
    /// Auto-cleared each tick
    pub sensory: RingBuffer<EventId>,

    /// Episodes: "Player defeated Lich King", "Wizard destroyed the bridge"
    /// Long-term episodic memory (weeks of game time)
    /// Max 50 episodes per NPC
    pub episodic: Vec<Episode>,

    /// Facts: "Humans are mortal", "The king is wise", "Dragon fears water"
    /// Semantic knowledge (never changes)
    pub semantic: Vec<Fact>,

    /// Emotional state: "angry_at_player_1", "mourning_for_aldric", "in_love_with_npc_2"
    /// Affects dialogue tone and behavior
    pub emotional: EmotionalState,

    /// Shared faction memory (accessible to all faction members)
    pub faction_knowledge: Option<SharedFactionMemory>,
}

#[derive(Debug, Clone)]
pub struct Episode {
    pub id: String,
    pub title: String,
    pub description: String,
    pub participants: Vec<String>,
    pub significance: f32,  // 0.1 = minor, 1.0 = apocalyptic
    pub timestamp: u64,
    pub affected_goals: Vec<String>,  // quest IDs affected
}

#[derive(Debug, Clone)]
pub struct Fact {
    pub statement: String,  // "humans are mortal"
    pub confidence: f32,    // 0.5 = heard rumor, 1.0 = witnessed
    pub source_npc: Option<String>,
}

#[derive(Debug, Clone)]
pub struct EmotionalState {
    pub feelings: HashMap<String, f32>,  // "happy_about_quest_123" → 0.8
    pub relationships: HashMap<String, Relationship>,  // NPC ID → relationship
}

#[derive(Debug, Clone)]
pub struct Relationship {
    pub npc_id: String,
    pub attitude: f32,  // -1.0 = hates, 0.0 = neutral, 1.0 = loves
    pub history: Vec<String>,  // key events with this person
}

impl MemoryStack {
    /// Extract relevant memories for dialogue generation
    pub fn relevant_memories_for_dialogue(&self, context: &str) -> Vec<String> {
        let mut memories = Vec::new();

        // Add recent episodes (most recent first)
        for episode in self.episodic.iter().rev().take(5) {
            if episode.significance > 0.5 {
                memories.push(format!(
                    "Remember when: {} (significance: {})",
                    episode.description, episode.significance
                ));
            }
        }

        // Add relevant facts
        for fact in &self.semantic {
            if context.contains(&fact.statement) || fact.confidence > 0.8 {
                memories.push(fact.statement.clone());
            }
        }

        // Add emotional state
        for (feeling, intensity) in &self.emotional.feelings {
            if intensity > &0.5 {
                memories.push(format!("Feeling: {}", feeling));
            }
        }

        memories
    }

    /// Update memory from incoming event
    pub fn apply_event(&mut self, event: &WorldEvent, npc_id: &str) {
        match &event.payload {
            EventPayload::CombatDeath { killer_id, .. } => {
                // Create episode: "NPC X died"
                if event.author.is_npc(npc_id) {
                    self.episodic.push(Episode {
                        title: "I died".into(),
                        significance: 1.0,
                        ..
                    });
                    // Add emotional: grief for allies
                    self.emotional.feelings.insert(
                        format!("grieving_npcs"),
                        0.9,
                    );
                }
            },
            EventPayload::PlayerSpeak { text, player_id, .. } => {
                // Check if it's directed at us
                if text.contains(npc_id) {
                    // Sensory: someone talked about us
                    self.sensory.push(event.seq);

                    // Update relationship
                    if let Some(rel) = self.emotional.relationships.get_mut(player_id) {
                        // Positive dialogue?
                        if text.contains("good") || text.contains("help") {
                            rel.attitude += 0.1;
                        }
                    }
                }
            },
            EventPayload::AigmQuestComplete { quest_id, player_id, .. } => {
                // Player completed a quest we gave!
                self.episodic.push(Episode {
                    title: format!("Player {} completed {}", player_id, quest_id),
                    significance: 0.6,
                    participants: vec![player_id.clone()],
                    ..
                });
                // Update relationship
                if let Some(rel) = self.emotional.relationships.get_mut(player_id) {
                    rel.attitude += 0.2;
                }
            },
            _ => {}
        }
    }
}
```

### Prompt Generation with Memory

```rust
fn generate_npc_prompt(
    npc: &NpcState,
    context: &str,
    memory: &MemoryStack,
) -> String {
    let relevant_memories = memory.relevant_memories_for_dialogue(context);

    format!(
        r#"You are {}. {}

Personality: {}

Recent memories:
{}

Current situation: {}

Respond authentically based on your memories and personality. Keep responses brief (1-2 sentences)."#,
        npc.name,
        npc.background,
        npc.personality,
        relevant_memories.join("\n"),
        context,
    )
}
```

**Deliverables for Phase 3:**
- [ ] `bifrost/aigm/src/memory.rs` — `MemoryStack` + related types
- [ ] Memory application logic in NPC tick
- [ ] Prompt generation using hierarchical memory
- [ ] Memory decay (episodes fade over time)
- [ ] Tests for memory recall

---

## PHASE 4: Faction AI (Optional but Powerful)

**Goal**: Factions become first-class agents.

```rust
pub struct FactionMind {
    faction_id: String,
    goals: Vec<Goal>,
    resources: ResourceMap,
    enemies: Vec<FactionId>,
    allies: Vec<FactionId>,
    territory: TerritoryGraph,
    economy: FactionEconomy,
    public_opinion: f32,
}

impl FactionMind {
    pub fn tick(&mut self, world_state: &WorldDirector) -> Vec<IntentEvent> {
        let mut intents = Vec::new();

        // If resources low, conduct raids
        if self.resources.gold < 1000 {
            intents.push(IntentEvent::RequestFactionAction {
                faction_id: self.faction_id.clone(),
                action_type: "raid_merchants".into(),
                targets: self.find_merchant_caravans(),
            });
        }

        // If territory threatened, mobilize
        if self.territory.under_attack() {
            intents.push(IntentEvent::RequestFactionAction {
                faction_id: self.faction_id.clone(),
                action_type: "defend_territory".into(),
                targets: self.identify_invaders(),
            });
        }

        intents
    }
}
```

---

## PHASE 5: Semantic Region System

NPCs understand geography, not just coordinates:

```rust
pub struct SemanticRegion {
    region_id: String,
    tags: Vec<String>,  // ["slums", "criminal", "poor"]
    danger_level: f32,
    controlling_faction: Option<FactionId>,
    narrative_state: String,  // "oppressed", "thriving", "under_siege"
}

// NPC knows: "I'm in the Slums. It's controlled by the Black Market. People here are poor and desperate."
// Not just: "x=500, y=200, z=50"
```

---

## PHASE 6: Animation State IDs (Deterministic)

Replace strings:

```rust
#[repr(u16)]
pub enum AnimStateId {
    Idle = 0,
    Walk = 1,
    Run = 2,
    Attack = 3,
    Dead = 4,
    // ...
}

// Event
pub enum VoxelInstruction {
    ANIMATE(entity_id, AnimStateId, duration_ms),
}
```

---

## PHASE 7: Reality Replay Tool

Diagnostic debugger:

```rust
pub struct RealityReplayer {
    ledger: Vec<WorldEvent>,
    breakpoints: Vec<u64>,
}

impl RealityReplayer {
    pub fn replay(&self, until_tick: u64) -> ReplayResult {
        // Re-execute entire history
        // Compare NPC states, combat results, etc.
        // Report divergences
    }

    pub fn diff_npc_state(
        &self,
        npc_id: &str,
        tick_a: u64,
        tick_b: u64,
    ) -> NpcStateDiff {
        // What changed about this NPC between two ticks?
    }
}
```

---

## PHASE 8: Emergent History Engine (THE KILLER FEATURE)

Since you have an immutable event ledger, you can:

1. **Compress history into lore**
   ```
   500 battle events → "The Great Battle of Sector-9"
   ```

2. **NPCs reference real past events**
   ```
   Player: "Tell me a story"
   NPC: "Do you remember the Flood of Sector-9?
         That was when 100 players drowned fighting the Leviathan."
   ```

3. **History affects current world state**
   ```
   If many players died in Zone-A before,
   that zone's creatures are now more aggressive (they remember).
   ```

```rust
pub struct HistoryCompressor {
    ledger: &[WorldEvent],
}

impl HistoryCompressor {
    pub fn compress_into_lore(&self, zone_id: &str) -> Vec<LoreBeat> {
        // Group similar events
        // Attach narrative weight
        // Create named story arcs

        vec![
            LoreBeat {
                title: "The Goblin Incursion".into(),
                description: "In Season 3, goblins invaded from the Whispering Caves...".into(),
                affected_npcs: vec!["aldric", "thorin"],
                significance: 0.8,
            },
        ]
    }

    pub fn get_npc_role_in_history(&self, npc_id: &str) -> Vec<String> {
        // Find all events authored by this NPC
        // Determine if they were hero, villain, victim, witness
        vec!["The Great Betrayal (victim)", "The Goblin Incursion (hero)"]
    }
}
```

---

## Implementation Roadmap

### Week 1-2: Phase 1 (Intent Validation)
- Implement `IntentEvent` enum
- Implement `IntentValidator`
- Implement `IntentCompiler`
- Test all validation rules

### Week 2-3: Phase 2 (World Director)
- Implement `WorldDirector`
- Compute all metrics from ledger
- Directive issuance
- Integration into tick

### Week 3-4: Phase 3 (Hierarchical Memory)
- Implement `MemoryStack` types
- Memory application in NPC tick
- Prompt generation with memory
- Memory decay

### Week 4+: Phases 4-8 (Optional)
- Faction AI
- Semantic regions
- Animation IDs
- Replay tool
- History compression

---

## Testing Strategy

Each phase needs:
1. **Unit tests** — validation rules, memory logic, director metrics
2. **Integration tests** — intent → event pipeline end-to-end
3. **Ledger audit tests** — verify budget not exceeded, events are valid
4. **Chaos tests** — spawn 1000 intents, verify no crashes

---

## Monitoring & Observability

Add telemetry:
```rust
struct AiGmTelemetry {
    intents_generated: u32,
    intents_validated: u32,
    intents_rejected: u32,
    rejection_reasons: HashMap<String, u32>,
    budget_exhaustion_count: u32,
    director_mood: WorldMood,
    average_event_pressure: f32,
}
```

---

## Summary

| Phase | Component | Impact | Effort |
|-------|-----------|--------|--------|
| 1 | Intent Validation | **Critical** — prevents corruption | 1-2 weeks |
| 2 | World Director | **Critical** — global pacing | 1 week |
| 3 | Hierarchical Memory | **High** — better NPC narratives | 1-2 weeks |
| 4 | Faction AI | Medium — emergent faction wars | 1 week |
| 5 | Semantic Regions | Medium — geographical meaning | 3 days |
| 6 | Animation IDs | Low — determinism + perf | 2 days |
| 7 | Replay Tool | Low — diagnostic only | 1 week |
| 8 | History Engine | **High** — lore from ledger | 1 week |

---

## The Killer Insight

This isn't a "game engine with AI sprinkles."

This is an **event-sourced persistent world** where:
- Physics is deterministic (Bifrost)
- Narrative is emergent (AIGM)
- History is immutable (ledger)
- Truth is centralized (DELPHOS)
- Reality is distributed (swarm)

That's closer to:
- **Dwarf Fortress** (emergent narratives)
- **EVE Online** (large-scale faction wars)
- **Rimworld** (storytelling engine)
- **SpatialOS** (distributed simulation)

mashed together.

The AIGM isn't the star. **The ledger is**. Everything grows from it.
