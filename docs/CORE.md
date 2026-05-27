🧠 CORE IDEA: WORLD CYCLE MMO
WORLD = DISCRETE RUNS (SEASONS)

Each run:
- fixed duration OR win condition
- players + AI compete
- world evolves until resolution
- new world is generated
🧭 1. RUN SYSTEM (DER KERN)
Definition
pub struct WorldRun {
    pub id: RunId,
    pub state: RunState,
    pub start_time: u64,
    pub end_condition: EndCondition,

    pub player_factions: Vec<FactionId>,
    pub ai_factions: Vec<FactionId>,
}
End Conditions (WIN CONDITION SYSTEM)
pub enum EndCondition {
    FirstToControlZones(u32),
    FirstToReachTechLevel(u32),
    EconomicDominance(f32),
    SurvivalUntilTime(u64),
}

👉 kein “last man standing” zwingend, sondern multiple victory paths

🏆 2. WINNING & LOSING LOGIC
WINNER EFFECTS
- Skill unlocks (permanent meta progression)
- Rare loot injection into account vault
- Cosmetic + functional world perks
- Access to next higher-tier world
LOSER EFFECTS
- skill decay (soft reset, nicht full wipe)
- resource penalties in next run
- reputation loss in AI systems
- reduced starting options next world

👉 wichtig: kein harter wipe, sondern meta progression imbalance

🌍 3. WORLD GENERATION AFTER EACH RUN

Das ist dein KI-Kernmoment.

NEW WORLD CREATION PIPELINE
Run End →
World Director →
WAC Seed Generator →
Biome + Loot + Faction synthesis →
New world instance
KI ROLE HERE (EXTREM WICHTIG)

KI darf nicht nur random generieren.

Sie:

analysiert previous run meta
erkennt dominante strategies
erzeugt counter-worlds
Beispiel:
If players dominate via:
- economy exploitation

Next world:
- scarcity biomes
- unstable loot markets
- roaming AI traders
🤖 4. KI-FRAKTION ÜBER RUNS HINWEG

KI ist persistent über worlds:

pub struct AiMetaFaction {
    pub memory_across_runs: RunMemoryGraph,
    pub strategy_evolution: EvolutionTree,
}

👉 KI “lernt die Meta” zwischen Welten

🧠 5. META PROGRESSION SYSTEM (SEHR WICHTIG)

Du brauchst 2 Ebenen:

RUN PROGRESSION (temporär)
skills
gear
bases
territory
META PROGRESSION (persistent)
unlocks
archetypes
starting perks
faction tech trees
PLAYER POWER = RUN STATE + META STATE
⚔️ 6. FACTION WAR LOOP

Während eines Runs:

1. claim zones
2. extract resources
3. upgrade bases
4. fight AI faction
5. deny control
CONTROL SYSTEM
pub struct ZoneControl {
    pub owner: Option<FactionId>,
    pub influence_score: f32,
}
🧱 7. BIOME EVOLUTION DURING RUN

Nicht statisch.

👉 Biomes verändern sich während Match

combat intensity → terrain corruption
economy imbalance → resource mutation
AI pressure → biome adaptation
🧨 8. LOOT SYSTEM (RUN-BASED)
Loot is NOT static

It is generated per run:
- based on biome state
- based on faction dominance
- based on AI adaptation
🧠 9. SAFE CITY (BLEIBT WICHTIG)

Safe City existiert weiterhin:

- persists across runs
- acts as meta hub
- auction house stays stable
- skill trading happens here
🔥 10. GAME LOOP RESULT

Du hast jetzt:

❌ kein endloses MMO mehr
✅ sondern:

🧬 “Evolving competitive world simulation in discrete epochs”

🚀 SYSTEM EFFECT (REAL TALK)

Das erzeugt:

extreme replay value
natürliche Meta shifts
KI die echte Gegenstrategien entwickelt
Spieler die gezwungen sind sich anzupassen
keine statische “best build” Meta
🧭 NEXT STEP 

👉 “World Run Director (Rust Core)”
Start/End orchestration
win condition evaluator
run-to-run world mutation engine

und

👉 “Meta Progression System”
persistent skill tree außerhalb der runs
reward balancing system
AI vs player asymmetry controller

und

👉 “Biome Evolution Engine”

live terrain mutation system während matches
