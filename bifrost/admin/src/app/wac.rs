//! WAC section — World Asset Compiler + World Director monitor.

use std::collections::BTreeMap;

use wasm_bindgen_futures::spawn_local;
use web_sys::{HtmlInputElement, HtmlSelectElement, HtmlTextAreaElement};
use yew::prelude::*;

use crate::{api, types::{GlobalPressureInput, PressureGraphRequest, WacRequest, ZonePressureInput}};

// ─── Tab ──────────────────────────────────────────────────────────────────────

#[derive(Clone, Copy, PartialEq)]
enum WacTab { Compiler, Director }

// ─── UUID helper ──────────────────────────────────────────────────────────────

fn gen_uuid() -> String {
    let t  = (js_sys::Date::now() as u64).to_le_bytes();
    let r1 = (js_sys::Math::random() * 0xffff_ffff_u64 as f64) as u64;
    let r2 = (js_sys::Math::random() * 0xffff_ffff_u64 as f64) as u64;
    format!("{:08x}-{:04x}-4{:03x}-{:04x}-{:012x}",
        u32::from_le_bytes([t[0],t[1],t[2],t[3]]),
        (r1 >> 16) & 0xffff,
        r1 & 0x0fff,
        (0x8000 | (r1 >> 20 & 0x3fff)) as u16,
        r2 & 0x0000_ffff_ffff_ffff_u64)
}

// ─── WacSection ───────────────────────────────────────────────────────────────

#[function_component(WacSection)]
pub fn wac_section() -> Html {
    let tab = use_state(|| WacTab::Compiler);

    // ── Compiler state ────────────────────────────────────────────────────────
    let asset_type  = use_state(|| "biome_definition".to_string());
    let spec        = use_state(String::new);
    let constraints = use_state(String::new);
    let seed        = use_state(|| 42u64);
    let compiling   = use_state(|| false);
    let comp_result = use_state(|| None::<String>);  // formatted JSON
    let comp_error  = use_state(|| None::<String>);

    // ── Director state ────────────────────────────────────────────────────────
    let zone_id    = use_state(|| "forest".to_string());
    let p_density  = use_state(|| 5.0f32);
    let kill_rate  = use_state(|| 10.0f32);
    let eco_delta  = use_state(|| 0.0f32);
    let narrative  = use_state(|| 0.5f32);
    let tot_players = use_state(|| 10u32);
    let at_tick    = use_state(|| 1000u64);
    let dir_running = use_state(|| false);
    let dir_result  = use_state(|| None::<String>);
    let dir_error   = use_state(|| None::<String>);
    let dir_history = use_state(|| None::<String>);

    // Tab callbacks
    let to_compiler = { let tab = tab.clone(); Callback::from(move |_: MouseEvent| tab.set(WacTab::Compiler)) };
    let to_director = { let tab = tab.clone(); Callback::from(move |_: MouseEvent| tab.set(WacTab::Director)) };

    // ── Compiler callbacks ────────────────────────────────────────────────────

    let on_type = {
        let asset_type = asset_type.clone();
        Callback::from(move |e: Event| {
            let el: HtmlSelectElement = e.target_unchecked_into();
            asset_type.set(el.value());
        })
    };
    let on_spec = {
        let spec = spec.clone();
        Callback::from(move |e: InputEvent| {
            let el: HtmlTextAreaElement = e.target_unchecked_into();
            spec.set(el.value());
        })
    };
    let on_constraints = {
        let constraints = constraints.clone();
        Callback::from(move |e: InputEvent| {
            let el: HtmlTextAreaElement = e.target_unchecked_into();
            constraints.set(el.value());
        })
    };
    let on_seed = {
        let seed = seed.clone();
        Callback::from(move |e: InputEvent| {
            let el: HtmlInputElement = e.target_unchecked_into();
            if let Ok(v) = el.value().parse::<u64>() { seed.set(v); }
        })
    };

    let on_compile = {
        let spec = spec.clone(); let constraints = constraints.clone();
        let seed = seed.clone(); let asset_type = asset_type.clone();
        let compiling = compiling.clone(); let comp_result = comp_result.clone();
        let comp_error = comp_error.clone();
        Callback::from(move |_: MouseEvent| {
            let req = WacRequest {
                id:                    gen_uuid(),
                asset_type:            (*asset_type).clone(),
                natural_language_spec: (*spec).clone(),
                constraints: (*constraints).lines()
                    .map(|l| l.trim().to_owned()).filter(|l| !l.is_empty()).collect(),
                seed: *seed,
            };
            let compiling = compiling.clone();
            let comp_result = comp_result.clone();
            let comp_error = comp_error.clone();
            spawn_local(async move {
                compiling.set(true); comp_error.set(None);
                match api::wac_compile(&req).await {
                    Ok(v) => {
                        comp_result.set(Some(
                            serde_json::to_string_pretty(&v).unwrap_or_else(|_| v.to_string())
                        ));
                    }
                    Err(e) => comp_error.set(Some(e)),
                }
                compiling.set(false);
            });
        })
    };

    // ── Director callbacks ────────────────────────────────────────────────────

    macro_rules! f32_cb {
        ($state:ident) => {{
            let $state = $state.clone();
            Callback::from(move |e: InputEvent| {
                let el: HtmlInputElement = e.target_unchecked_into();
                if let Ok(v) = el.value().parse::<f32>() { $state.set(v); }
            })
        }};
    }

    let on_zone_id = {
        let zone_id = zone_id.clone();
        Callback::from(move |e: InputEvent| {
            let el: HtmlInputElement = e.target_unchecked_into();
            zone_id.set(el.value());
        })
    };
    let on_density   = f32_cb!(p_density);
    let on_kill_rate = f32_cb!(kill_rate);
    let on_eco       = f32_cb!(eco_delta);
    let on_narrative = f32_cb!(narrative);
    let on_tick      = {
        let at_tick = at_tick.clone();
        Callback::from(move |e: InputEvent| {
            let el: HtmlInputElement = e.target_unchecked_into();
            if let Ok(v) = el.value().parse::<u64>() { at_tick.set(v); }
        })
    };
    let on_players   = {
        let tot_players = tot_players.clone();
        Callback::from(move |e: InputEvent| {
            let el: HtmlInputElement = e.target_unchecked_into();
            if let Ok(v) = el.value().parse::<u32>() { tot_players.set(v); }
        })
    };

    let on_director_tick = {
        let zone_id = zone_id.clone(); let p_density = p_density.clone();
        let kill_rate = kill_rate.clone(); let eco_delta = eco_delta.clone();
        let narrative = narrative.clone(); let tot_players = tot_players.clone();
        let at_tick = at_tick.clone();
        let dir_running = dir_running.clone(); let dir_result = dir_result.clone();
        let dir_error = dir_error.clone();
        Callback::from(move |_: MouseEvent| {
            let mut zones = BTreeMap::new();
            zones.insert((*zone_id).clone(), ZonePressureInput {
                zone_id:        (*zone_id).clone(),
                player_density: *p_density,
                kill_rate:      *kill_rate,
                loot_flow:      0.0,
                quest_rate:     0.0,
                contention:     0.0,
            });
            let req = PressureGraphRequest {
                zones,
                global: GlobalPressureInput {
                    total_players:      *tot_players,
                    economy_delta:      *eco_delta,
                    player_trend:       0.0,
                    narrative_momentum: *narrative,
                    quest_throughput:   0.0,
                },
                at_tick: *at_tick,
            };
            let dir_running = dir_running.clone();
            let dir_result  = dir_result.clone();
            let dir_error   = dir_error.clone();
            spawn_local(async move {
                dir_running.set(true); dir_error.set(None);
                match api::director_tick(&req).await {
                    Ok(v) => dir_result.set(Some(
                        serde_json::to_string_pretty(&v).unwrap_or_else(|_| v.to_string())
                    )),
                    Err(e) => dir_error.set(Some(e)),
                }
                dir_running.set(false);
            });
        })
    };

    let on_load_history = {
        let dir_history = dir_history.clone(); let dir_error = dir_error.clone();
        Callback::from(move |_: MouseEvent| {
            let dh = dir_history.clone(); let de = dir_error.clone();
            spawn_local(async move {
                match api::director_history().await {
                    Ok(v) => dh.set(Some(serde_json::to_string_pretty(&v).unwrap_or_else(|_| v.to_string()))),
                    Err(e) => de.set(Some(e)),
                }
            });
        })
    };

    // ── Asset type options ────────────────────────────────────────────────────

    const ASSET_TYPES: &[(&str, &str)] = &[
        ("biome_definition", "🌿 Biome Definition"),
        ("loot_table",       "💎 Loot Table"),
        ("animation_graph",  "🎬 Animation Graph"),
        ("entity_prefab",    "🧙 Entity Prefab"),
        ("voxel_structure",  "🧱 Voxel Structure"),
    ];

    html! {
        <div class="section">
            <div class="section-header">
                <div>
                    <h1 class="section-title">{"⚡ WAC + World Director"}</h1>
                    <p class="section-sub">{"World Asset Compiler: text → validated, deterministic IR."}</p>
                </div>
                <div class="tab-bar">
                    <button class={classes!("btn", (*tab == WacTab::Compiler).then_some("btn-primary").unwrap_or("btn-ghost"))}
                        onclick={to_compiler}>{"Compiler"}</button>
                    <button class={classes!("btn", (*tab == WacTab::Director).then_some("btn-primary").unwrap_or("btn-ghost"))}
                        onclick={to_director}>{"Director"}</button>
                </div>
            </div>

            if *tab == WacTab::Compiler {
                // ── Compiler panel ────────────────────────────────────────────────
                <div class="wac-panel">
                    <div class="wac-rule-box">
                        <strong>{"Hard rule: "}</strong>
                        {"LLM / designer describes rules. WAC compiles deterministically. Engine never receives raw text."}
                    </div>

                    <div class="form-grid">
                        <div class="field">
                            <label>{"Asset Type"}</label>
                            <select class="input" onchange={on_type}>
                                { for ASSET_TYPES.iter().map(|(val, label)| html! {
                                    <option value={*val} selected={*asset_type == *val}>{*label}</option>
                                }) }
                            </select>
                        </div>
                        <div class="field">
                            <label>{"Seed"}<span class="hint">{"(non-zero, same seed = same output)"}</span></label>
                            <input type="number" class="input input-sm" value={seed.to_string()}
                                min="1" oninput={on_seed} />
                        </div>
                        <div class="field field-full">
                            <label>{"Natural Language Spec"}</label>
                            <textarea class="input textarea wac-spec" rows="4"
                                value={(*spec).clone()} oninput={on_spec}
                                placeholder={match asset_type.as_str() {
                                    "biome_definition" => "crystal forest with glowing trees that emit light at night and aggressive bats",
                                    "loot_table"       => "rare crystals drop from glowing bats at night in the crystal forest",
                                    "animation_graph"  => "idle search attack flee die spawn",
                                    "entity_prefab"    => "hostile boss dungeon overlord with high hp and powerful atk",
                                    _                  => "describe the asset…",
                                }}
                            />
                        </div>
                        <div class="field field-full">
                            <label>{"Constraints"}<span class="hint">{"(one per line)"}</span></label>
                            <textarea class="input textarea" rows="3"
                                value={(*constraints).clone()} oninput={on_constraints}
                                placeholder={"no floating voxels\nnavmesh must remain connected\nmax_drop_rate <= 0.05"} />
                        </div>
                    </div>

                    <div class="form-actions">
                        <button class="btn btn-primary" disabled={*compiling} onclick={on_compile}>
                            { if *compiling { "Compiling…" } else { "⚡ Compile" } }
                        </button>
                    </div>

                    if let Some(err) = (*comp_error).clone() {
                        <div class="wac-error">{ format!("Error: {err}") }</div>
                    }
                    if let Some(result) = (*comp_result).clone() {
                        <div class="wac-result">
                            <div class="wac-result-label">{"Compiled AssetIR"}</div>
                            <pre class="wac-json">{ result }</pre>
                        </div>
                    }
                </div>
            }

            if *tab == WacTab::Director {
                // ── Director panel ────────────────────────────────────────────────
                <div class="wac-panel">
                    <div class="wac-rule-box">
                        <strong>{"World Director: "}</strong>
                        {"Reads pressure graph → emits AssetBlueprints → WAC compiles them → BIFROST applies."}
                    </div>

                    <h3 class="wac-sub-title">{"Simulate Pressure Tick"}</h3>
                    <div class="form-grid">
                        <div class="field"><label>{"Zone ID"}</label>
                            <input type="text" class="input" value={(*zone_id).clone()} oninput={on_zone_id} />
                        </div>
                        <div class="field"><label>{"At Tick"}</label>
                            <input type="number" class="input input-sm" value={at_tick.to_string()}
                                min="1" oninput={on_tick} />
                        </div>
                        <div class="field"><label>{"Player Density"}<span class="hint">{"players in zone"}</span></label>
                            <input type="number" class="input input-sm" value={p_density.to_string()}
                                min="0" step="0.5" oninput={on_density} />
                        </div>
                        <div class="field"><label>{"Kill Rate"}<span class="hint">{"kills/tick"}</span></label>
                            <input type="number" class="input input-sm" value={kill_rate.to_string()}
                                min="0" step="0.5" oninput={on_kill_rate} />
                        </div>
                        <div class="field"><label>{"Economy Delta"}<span class="hint">{"+0.3 = inflating"}</span></label>
                            <input type="number" class="input input-sm" value={eco_delta.to_string()}
                                step="0.05" oninput={on_eco} />
                        </div>
                        <div class="field"><label>{"Narrative Momentum"}<span class="hint">{"< 0.1 = stalled"}</span></label>
                            <input type="number" class="input input-sm" value={narrative.to_string()}
                                min="0" max="1" step="0.05" oninput={on_narrative} />
                        </div>
                        <div class="field"><label>{"Total Players"}</label>
                            <input type="number" class="input input-sm" value={tot_players.to_string()}
                                min="0" oninput={on_players} />
                        </div>
                    </div>
                    <div class="form-actions">
                        <button class="btn btn-primary" disabled={*dir_running} onclick={on_director_tick}>
                            { if *dir_running { "Running…" } else { "▶ Run Director Tick" } }
                        </button>
                        <button class="btn btn-ghost" onclick={on_load_history}>{"📋 Load History"}</button>
                    </div>

                    if let Some(err) = (*dir_error).clone() {
                        <div class="wac-error">{ format!("Error: {err}") }</div>
                    }
                    if let Some(result) = (*dir_result).clone() {
                        <div class="wac-result">
                            <div class="wac-result-label">{"Director Decisions"}</div>
                            <pre class="wac-json">{ result }</pre>
                        </div>
                    }
                    if let Some(hist) = (*dir_history).clone() {
                        <div class="wac-result">
                            <div class="wac-result-label">{"Decision History"}</div>
                            <pre class="wac-json">{ hist }</pre>
                        </div>
                    }
                </div>
            }
        </div>
    }
}
