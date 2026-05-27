//! NPCs section — CRUD for NPC definitions with full AI context.

use wasm_bindgen_futures::spawn_local;
use web_sys::{HtmlInputElement, HtmlSelectElement, HtmlTextAreaElement};
use yew::prelude::*;

use crate::{api, types::Npc};

// ─── NPC form component ───────────────────────────────────────────────────────

#[derive(Properties, PartialEq)]
struct NpcFormProps {
    pub npc:    Npc,
    pub is_new: bool,
    pub on_change: Callback<Npc>,
    pub on_save:   Callback<MouseEvent>,
    pub on_cancel: Callback<MouseEvent>,
}

#[function_component(NpcForm)]
fn npc_form(props: &NpcFormProps) -> Html {
    let n = props.npc.clone();

    macro_rules! text_cb {
        ($field:ident) => {{
            let cb = props.on_change.clone();
            let n2 = n.clone();
            Callback::from(move |e: InputEvent| {
                let el: HtmlInputElement = e.target_unchecked_into();
                let mut npc = n2.clone(); npc.$field = el.value(); cb.emit(npc);
            })
        }};
    }
    macro_rules! num_cb {
        ($field:ident, $t:ty) => {{
            let cb = props.on_change.clone();
            let n2 = n.clone();
            Callback::from(move |e: InputEvent| {
                let el: HtmlInputElement = e.target_unchecked_into();
                if let Ok(v) = el.value().parse::<$t>() {
                    let mut npc = n2.clone(); npc.$field = v; cb.emit(npc);
                }
            })
        }};
    }

    let on_id     = text_cb!(id);
    let on_name   = text_cb!(name);
    let on_icon   = text_cb!(icon);
    let on_color  = text_cb!(color);
    let on_model  = text_cb!(model);
    let on_goal   = text_cb!(current_goal);
    let on_x      = num_cb!(x, i32);
    let on_y      = num_cb!(y, i32);
    let on_cd     = num_cb!(cooldown_ms, u64);

    let on_faction = {
        let cb = props.on_change.clone();
        let n2 = n.clone();
        Callback::from(move |e: Event| {
            let el: HtmlSelectElement = e.target_unchecked_into();
            let mut npc = n2.clone(); npc.faction = el.value(); cb.emit(npc);
        })
    };

    let on_quest_id = {
        let cb = props.on_change.clone();
        let n2 = n.clone();
        Callback::from(move |e: InputEvent| {
            let el: HtmlInputElement = e.target_unchecked_into();
            let mut npc = n2.clone();
            let v = el.value();
            npc.quest_id = if v.is_empty() { None } else { Some(v) };
            cb.emit(npc);
        })
    };

    let on_prompt = {
        let cb = props.on_change.clone();
        let n2 = n.clone();
        Callback::from(move |e: InputEvent| {
            let el: HtmlTextAreaElement = e.target_unchecked_into();
            let mut npc = n2.clone(); npc.system_prompt = el.value(); cb.emit(npc);
        })
    };

    // Lines: one dialogue line per textarea row
    let on_lines = {
        let cb = props.on_change.clone();
        let n2 = n.clone();
        Callback::from(move |e: InputEvent| {
            let el: HtmlTextAreaElement = e.target_unchecked_into();
            let mut npc = n2.clone();
            npc.lines = el.value().lines().filter(|l| !l.trim().is_empty())
                .map(str::to_owned).collect();
            cb.emit(npc);
        })
    };

    let lines_text = n.lines.join("\n");
    let quest_val  = n.quest_id.clone().unwrap_or_default();

    html! {
        <div class="form-panel">
            <h2 class="form-panel-title">
                { n.icon.clone() }{" "}{ if props.is_new { "New NPC" } else { "Edit NPC" } }
            </h2>
            <div class="form-grid">
                <div class="field">
                    <label>{"ID"}<span class="hint">{"(slug)"}</span></label>
                    <input type="text" class="input" value={n.id.clone()} oninput={on_id}
                        readonly={!props.is_new} />
                </div>
                <div class="field">
                    <label>{"Name"}</label>
                    <input type="text" class="input" value={n.name.clone()} oninput={on_name} />
                </div>
                <div class="field">
                    <label>{"Icon"}<span class="hint">{"(emoji)"}</span></label>
                    <input type="text" class="input input-sm" value={n.icon.clone()} oninput={on_icon} />
                </div>
                <div class="field">
                    <label>{"Color"}</label>
                    <div class="color-row">
                        <input type="color" class="color-swatch" value={n.color.clone()}
                            oninput={on_color.clone()} />
                        <input type="text" class="input input-sm" value={n.color.clone()}
                            oninput={on_color} />
                    </div>
                </div>
                <div class="field">
                    <label>{"Map X"}</label>
                    <input type="number" class="input input-sm" value={n.x.to_string()}
                        min="0" max="59" oninput={on_x} />
                </div>
                <div class="field">
                    <label>{"Map Y"}</label>
                    <input type="number" class="input input-sm" value={n.y.to_string()}
                        min="0" max="59" oninput={on_y} />
                </div>
                <div class="field">
                    <label>{"Faction"}</label>
                    <select class="input" onchange={on_faction}>
                        { for ["neutral","friendly","hostile"].iter().map(|&f| html! {
                            <option value={f} selected={n.faction == f}>{ f }</option>
                        }) }
                    </select>
                </div>
                <div class="field">
                    <label>{"Quest ID"}<span class="hint">{"(optional)"}</span></label>
                    <input type="text" class="input" value={quest_val} oninput={on_quest_id}
                        placeholder="wolves / rats / goblins / tome / …" />
                </div>
                <div class="field">
                    <label>{"LLM Model"}</label>
                    <input type="text" class="input" value={n.model.clone()} oninput={on_model}
                        placeholder="ollama/llama3-8b" />
                </div>
                <div class="field">
                    <label>{"Cooldown (ms)"}</label>
                    <input type="number" class="input input-sm" value={n.cooldown_ms.to_string()}
                        min="1000" step="1000" oninput={on_cd} />
                </div>
                <div class="field field-full">
                    <label>{"Current Goal"}</label>
                    <input type="text" class="input" value={n.current_goal.clone()} oninput={on_goal} />
                </div>
                <div class="field field-full">
                    <label>{"System Prompt"}<span class="hint">{"(NPC personality)"}</span></label>
                    <textarea class="input textarea" rows="3"
                        value={n.system_prompt.clone()} oninput={on_prompt} />
                </div>
                <div class="field field-full">
                    <label>{"Dialogue Lines"}<span class="hint">{"(one per line)"}</span></label>
                    <textarea class="input textarea" rows="3"
                        value={lines_text} oninput={on_lines}
                        placeholder={"Line one\nLine two\nLine three"} />
                </div>
            </div>
            <div class="form-actions">
                <button class="btn btn-primary" onclick={props.on_save.clone()}>{"Save"}</button>
                <button class="btn btn-ghost"   onclick={props.on_cancel.clone()}>{"Cancel"}</button>
            </div>
        </div>
    }
}

// ─── NPCs section ─────────────────────────────────────────────────────────────

#[function_component(NpcsSection)]
pub fn npcs_section() -> Html {
    let npcs    = use_state(Vec::<Npc>::new);
    let loading = use_state(|| true);
    let editing = use_state(|| None::<Npc>);
    let is_new  = use_state(|| false);
    let toast   = use_state(|| None::<String>);

    // Load on mount
    {
        let npcs    = npcs.clone();
        let loading = loading.clone();
        let toast   = toast.clone();
        use_effect_with((), move |_| {
            spawn_local(async move {
                match api::get_npcs().await {
                    Ok(list) => { npcs.set(list); loading.set(false); }
                    Err(e)   => { toast.set(Some(format!("Load failed: {e}"))); loading.set(false); }
                }
            });
            || ()
        });
    }

    let on_new = {
        let editing = editing.clone();
        let is_new  = is_new.clone();
        Callback::from(move |_: MouseEvent| { editing.set(Some(Npc::default())); is_new.set(true); })
    };

    let on_cancel = {
        let editing = editing.clone();
        Callback::from(move |_: MouseEvent| editing.set(None))
    };

    let on_change = {
        let editing = editing.clone();
        Callback::from(move |n: Npc| editing.set(Some(n)))
    };

    let on_save = {
        let editing = editing.clone();
        let is_new  = is_new.clone();
        let npcs    = npcs.clone();
        let toast   = toast.clone();
        Callback::from(move |_: MouseEvent| {
            let n       = match (*editing).clone() { Some(n) => n, None => return };
            let editing = editing.clone();
            let is_new  = is_new.clone();
            let npcs    = npcs.clone();
            let toast   = toast.clone();
            spawn_local(async move {
                let result = if *is_new { api::create_npc(&n).await } else { api::update_npc(&n).await };
                match result {
                    Ok(_) => {
                        if let Ok(list) = api::get_npcs().await { npcs.set(list); }
                        toast.set(Some("Saved ✓".into()));
                        editing.set(None);
                    }
                    Err(e) => toast.set(Some(format!("Save failed: {e}"))),
                }
            });
        })
    };

    // Faction badge colour
    fn faction_color(f: &str) -> &'static str {
        match f { "friendly" => "#3a8a3a", "hostile" => "#8a2020", _ => "#5a5a7a" }
    }

    html! {
        <div class="section">
            <div class="section-header">
                <div>
                    <h1 class="section-title">{"🧙 NPCs"}</h1>
                    <p class="section-sub">{"Manage NPC definitions, AI prompts and dialogue."}</p>
                </div>
                <button class="btn btn-primary" onclick={on_new}>{"+ New NPC"}</button>
            </div>

            if let Some(msg) = (*toast).clone() {
                <div class="toast">{ msg }</div>
            }

            if let Some(n) = (*editing).clone() {
                <NpcForm npc={n} is_new={*is_new}
                    on_change={on_change} on_save={on_save} on_cancel={on_cancel} />
            }

            if *loading {
                <div class="loading-spinner">{"Loading…"}</div>
            } else {
                <div class="npc-table">
                    <div class="table-head">
                        <span>{"NPC"}</span>
                        <span>{"Pos"}</span>
                        <span>{"Faction"}</span>
                        <span>{"Quest"}</span>
                        <span>{"Model"}</span>
                        <span>{"Actions"}</span>
                    </div>
                    { for (*npcs).iter().map(|n| {
                        let n2      = n.clone();
                        let editing = editing.clone();
                        let is_new  = is_new.clone();
                        let npcs2   = npcs.clone();
                        let toast2  = toast.clone();
                        let fc      = faction_color(&n.faction);

                        let on_edit = Callback::from(move |_: MouseEvent| {
                            editing.set(Some(n2.clone())); is_new.set(false);
                        });
                        let nid = n.id.clone();
                        let on_del = Callback::from(move |_: MouseEvent| {
                            let id    = nid.clone();
                            let npcs  = npcs2.clone();
                            let toast = toast2.clone();
                            spawn_local(async move {
                                match api::delete_npc(&id).await {
                                    Ok(_) => { if let Ok(l) = api::get_npcs().await { npcs.set(l); } }
                                    Err(e) => toast.set(Some(format!("Delete failed: {e}"))),
                                }
                            });
                        });

                        html! {
                            <div class="table-row">
                                <span class="npc-name">
                                    <span class="npc-icon">{ &n.icon }</span>
                                    <span>
                                        <strong>{ &n.name }</strong>
                                        <small>{ &n.id }</small>
                                    </span>
                                </span>
                                <span class="mono">{ format!("{},{}", n.x, n.y) }</span>
                                <span>
                                    <span class="badge" style={format!("background:{fc}")}>
                                        { &n.faction }
                                    </span>
                                </span>
                                <span class="mono">
                                    { n.quest_id.as_deref().unwrap_or("—") }
                                </span>
                                <span class="mono small">{ n.model.trim_start_matches("ollama/") }</span>
                                <span class="row-actions">
                                    <button class="btn btn-sm btn-secondary" onclick={on_edit}>{"Edit"}</button>
                                    <button class="btn btn-sm btn-danger"    onclick={on_del}>{"Del"}</button>
                                </span>
                            </div>
                        }
                    }) }
                </div>
            }
        </div>
    }
}
