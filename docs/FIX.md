Next Steps — Zusammenführung in Bifrost
Was bereits da ist (nach PR #3):

bifrost-server hat run_director, synthesis, safe_city, 17 neue HTTP-Routen
Zones werden mit Biome-IDs "forest", "desert", "dungeon", "plains" geseeded
bifrost-wac hat TileMapIR + vollständigen BiomeIR-Typen
nova-anim hat AnimStateMachine, nova-core hat Vec3/Quat/Mat4
Schritt 1 — Noise-Smoothstep-Fix (1 Zeile, game.html)
Problem: JS verwendet kubisches Smoothstep t*(3-2t), Rust quintic t³(t(6t-15)+10) — selber Seed, andere Welt.
Fix: game.html Zeile 224: ux=xfxf(3-2xf) → ux=xfxfxf(xf*(xf*6-15)+10)

Schritt 2 — Kanonische Biome-Liste (bifrost-wac)
Problem: SimState seeded mit "forest"/"desert"/"dungeon", WAC-Palette hat 11 Einträge, JS hat 14 IDs — alle unterschiedlich.
Fix: bifrost/wac/src/biomes.rs mit einer einzigen Wahrheitsquelle:

pub const CANONICAL_BIOMES: &[BiomeKey] = &[
"deep_water", "water", "sand", "grass", "dark_forest",
"crimson_forest", "rock", "mountain", "snow",
"dungeon", "village", "building", "swamp", "volcanic"
];
Dann WAC-TileMap-Palette + SimState-Zones + game.html BIOME-Enum alle dagegen synchronisieren.

Schritt 3 — BiomeIR Duplikat entfernen
Problem: bifrost/wac/types.rs:148 und nexus/bridge/wac_adapter.rs:188 definieren beide BiomeIR — zwei verschiedene Structs.
Fix: nexus/voxel-kernel/Cargo.toml bekommt bifrost-wac als dep, wac_adapter.rs importiert bifrost_wac::types::BiomeIR statt eigene Struct. Kein circular dep (bifrost-wac kennt nexus nicht).

Schritt 4 — AnimationGraphIR → nova-anim verbinden
Problem: bifrost/wac/compile/animation.rs produziert AnimationGraphIR, nova-anim hat AnimStateMachine — parallel, nicht verbunden.
Fix: AnimationGraphIR::to_nova_fsm(&self, skeleton: VoxelSkeleton) -> AnimStateMachine als Methode in bifrost-wac (braucht nova-anim als workspace dep). WAC kann dann direkt FSMs für alle Entities ausgeben.

Schritt 5 — Quest-Routen + game.html Fetch
Problem: bifrost-aigm hat vollständiges Quest-System mit QuestRegistry, game.html hat 12 hardcodierte Quests in QCHAINS.
Fix:

bifrost-server bekommt /aigm/quests → GET (Liste) + POST (accept/turn-in)
game.html ersetzt const QCHAINS = {...} durch let QCHAINS = {}; fetchQuests() beim Start
QCHAINS wird aus API-Response befüllt; fällt auf Fallback zurück wenn Server offline
Schritt 6 — NPC-Routen + game.html Fetch
Problem: bifrost-aigm hat NpcRegistry mit AI-verhaltenem NPC-Tick, game.html hat 6 hardcodierte NPC-Objekte.
Fix:

bifrost-server bekommt /aigm/npcs → GET Liste der NPCs im Startbereich
game.html ersetzt hartcodierten NPC-Array durch API-Fetch beim Start
NpcBehavior-Tick-Ergebnisse landen über WebSocket in game.html
Schritt 7 — WorldRunDirector ↔ WorldDirector verdrahten
Problem: Run endet → Welt soll neu generiert werden → WorldDirector soll neue Assets emittieren → aber beide in SimState sind nicht verbunden.
Fix: In /run/end-Handler:

let run_result = s.run_director.end_run(...);
let pressure = PressureGraph::from_run_result(&run_result);
let new_blueprints = s.director.tick(&pressure);
// → WAC compiled → nexus_rt generates new world
Schritt 8 — Vec3Payload aus aigm/event.rs → nova_core::Vec3
Problem: bifrost/aigm/src/event.rs:186 definiert ein eigenes Vec3Payload { x, y, z: f32 }.
Fix: Ersetzen durch pub type Vec3Payload = nova_core::Vec3; — nova-core als dep zu bifrost-aigm.

Zusammenfassung
Schritt PR-Titel Dateien berührt
───────────────────────────────────────────────────────────────────
1 fix: noise smoothstep JS=Rust app/game.html (1 line)
2 feat: canonical biome registry bifrost/wac, SimState, game.html
3 refactor: unify BiomeIR nexus/bridge, bifrost/wac
4 feat: AnimIR→nova-anim bridge bifrost/wac, nova/anim
5 feat: quest HTTP routes + fetch bifrost/server/api, game.html
6 feat: NPC HTTP routes + fetch bifrost/server/api, game.html
7 feat: wire run→wac director bifrost/server/api (run/end)
8 refactor: Vec3Payload→nova::Vec3 bifrost/aigm/event.rs
Ja. Schritt 1+2 sofort zusammen in EINEM PR machen.
Das ist gerade der wichtigste Stabilitäts-Fix im gesamten Stack.

Nicht flashy. Aber absolut kritisch.

🧠 WARUM DAS SO WICHTIG IST
Du bist jetzt an dem Punkt, wo:

same seed
same world
nicht mehr optional ist.

Das ist die Grundlage für:

replayability
deterministic networking
AI simulation consistency
server/client sync
run validation
future federation
🚨 SCHRITT 1 IST KRITISCHER ALS ER AUSSIEHT
Smoothstep mismatch
Aktuell:

JS world ≠ Rust world
trotz gleichem Seed.

Das zerstört langfristig:

replay systems
chunk authority
AI planning
biome prediction
path consistency
Richtige Entscheidung
ux = xfxfxf*(xf*(xf*6-15)+10)
Das ist exakt die richtige Perlin/quintic interpolation.

🧬 SCHRITT 2 IST NOCH WICHTIGER
Das hier:

single canonical biome registry
muss absolut zentralisiert werden.

🚨 AKTUELL HAST DU:
JS biome IDs
≠
WAC biome IDs
≠
seeded zone IDs
Das ist später ein kompletter Nightmare:

biome desync
wrong palette loads
invalid AI assumptions
broken procedural generation
corrupted replay states
✅ DIE RICHTIGE LÖSUNG
bifrost-wac wird:
🧠 “WORLD TYPE AUTHORITY”
NICHT:
game.html
nexus
bifrost-server
sondern:

bifrost-wac::biomes
🔥 ICH WÜRDE DAS NOCH STÄRKER MACHEN
Nicht nur:

pub const CANONICAL_BIOMES
Sondern:

👉 Voller Registry-Typ
Beispiel
pub struct BiomeDefinition {
pub id: &'static str,
pub display_name: &'static str,

pub temperature: f32,
pub humidity: f32,

pub voxel_palette: &'static [&'static str],

pub ambient_fx: AmbientFx,

pub risk_level: u8,
}
🧠 WARUM DAS BESSER IST
Dann wird Biome nicht nur:

String-ID
sondern:

vollständige Weltregeldefinition
🧩 DANN KÖNNEN AUTOMATISCH:
WAC
biome compile
nova-render
ambient fx
synthesis AI
strategic evaluation
loot system
drop weighting
world director
mutation rules
…alles dieselbe Quelle benutzen.

🚀 WICHTIGER EXTRA MOVE
BiomeKey als enum statt &str
JETZT.

Nicht später.

Statt:
"forest"
Lieber:
pub enum BiomeKey {
DeepWater,
Water,
Sand,
Grass,
DarkForest,
CrimsonForest,
Rock,
Mountain,
Snow,
Dungeon,
Village,
Building,
Swamp,
Volcanic,
}
🧠 WARUM?
Das verhindert später:

typo worlds
invalid packets
broken serialization
impossible biome states
🚨 SEHR WICHTIG
Schritt 3 direkt danach.
Das doppelte BiomeIR MUSS sterben.

Sonst bekommst du später:

invalid conversions
duplicated compile logic
silent desyncs
broken WAC contracts
🧬 ARCHITEKTUR-REGEL AB JETZT
Every world concept exists ONCE.
Nicht:

duplicated structs
mirrored enums
copied IDs
🔥 SCHRITT 4 IST HEIMLICH RIESIG
Das hier:

AnimationGraphIR → nova-anim FSM
ist extrem stark.

Warum?

Dann kann WAC direkt:

creatures
bosses
AI entities
player classes
als komplette Runtime FSMs erzeugen.

🧠 DANN WIRD WAC:
Nicht nur:

asset compiler
sondern:

behavior compiler
🚨 SCHRITT 5+6 SIND DER BEGINN VON “ECHTER WELT”
Das Entfernen der hardcoded JS arrays ist riesig wichtig.

Denn dann:

world authority = server
Nicht browser state.

Das ist DER Übergang zum echten MMO.

🧨 SCHRITT 7 IST DER WAHRE TURNING POINT
Das hier:

run end
→ pressure graph
→ world director
→ WAC
→ new world
…ist die Geburt deiner:

🧬 selbst-evolvierenden Weltpipeline
🚀 MEINE EMPFOHLENE REIHENFOLGE
PR 1
✔ Schritt 1 + 2

PR 2
✔ Schritt 3 + 8

(shared core type cleanup)

PR 3
✔ Schritt 5 + 6

(server-authoritative world data)

PR 4
✔ Schritt 7

(self-evolving world loop)

PR 5
✔ Schritt 4

(WAC behavioral runtime compiler)

🧠 FAZIT
Ja. Sofort mit Schritt 1+2 anfangen.

Denn genau diese beiden Dinge machen aus:

"voxel game with systems"
endlich:

deterministic shared reality simulation

Project Organisation Rules
Single source of truth. One module, one location.

Core Principles
One concept, one crate — no concept may be split across crates
Single mutation path — all state changes via StateTransitionFn only
EventPipeline required — all events must pass through EventPipeline.process()
Replay-safe — same ledger + same reducers = same final state, always
No SystemTime::now() — use clock SequencedInstant for all ordering