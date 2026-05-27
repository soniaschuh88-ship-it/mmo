//! Biomes section — CRUD for world biome zones.

use wasm_bindgen_futures::spawn_local;
use web_sys::{HtmlInputElement, HtmlTextAreaElement};
use yew::prelude::*;

use crate::{api, types::Biome};

#[function_component(BiomesSection)]
pub fn biomes_section() -> Html {
    let biomes    = use_state(Vec::<Biome>::new);
    let loading   = use_state(|| true);
    let editing   = use_state(|| None::<Biome>);   // None = list view
    let is_new    = use_state(|| false);
    let toast     = use_state(|| None::<String>);

    // Load on mount
    {
        let biomes  = biomes.clone();
        let loading = loading.clone();
        let toast   = toast.clone();
        use_effect_with((), move |_| {
            spawn_local(async move {
                match api::get_biomes().await {
                    Ok(b)  => { biomes.set(b); loading.set(false); }
                    Err(e) => { toast.set(Some(format!("Load failed: {e}"))); loading.set(false); }
                }
            });
            || ()
        });
    }

    // Open "new biome" form
    let on_new = {
        let editing = editing.clone();
        let is_new  = is_new.clone();
        Callback::from(move |_: MouseEvent| {
            editing.set(Some(Biome::default()));
            is_new.set(true);
        })
    };

    let on_cancel = {
        let editing = editing.clone();
        Callback::from(move |_: MouseEvent| { editing.set(None); })
    };

    // ── Form field helpers ────────────────────────────────────────────────────

    macro_rules! text_cb {
        ($field:ident) => {{
            let editing = editing.clone();
            Callback::from(move |e: InputEvent| {
                let el: HtmlInputElement = e.target_unchecked_into();
                if let Some(mut b) = (*editing).clone() {
                    b.$field = el.value();
                    editing.set(Some(b));
                }
            })
        }};
    }

    let on_id    = text_cb!(id);
    let on_name  = text_cb!(name);
    let on_color = text_cb!(color);
    let on_tile  = text_cb!(tile_type);
    let on_zone  = text_cb!(zone);

    let on_desc = {
        let editing = editing.clone();
        Callback::from(move |e: InputEvent| {
            let el: HtmlTextAreaElement = e.target_unchecked_into();
            if let Some(mut b) = (*editing).clone() {
                b.description = el.value();
                editing.set(Some(b));
            }
        })
    };

    let on_rate = {
        let editing = editing.clone();
        Callback::from(move |e: InputEvent| {
            let el: HtmlInputElement = e.target_unchecked_into();
            if let Some(mut b) = (*editing).clone() {
                if let Ok(v) = el.value().parse::<f32>() {
                    b.encounter_rate = v;
                    editing.set(Some(b));
                }
            }
        })
    };

    // ── Save ──────────────────────────────────────────────────────────────────

    let on_save = {
        let editing = editing.clone();
        let is_new  = is_new.clone();
        let biomes  = biomes.clone();
        let toast   = toast.clone();
        Callback::from(move |_: MouseEvent| {
            let b       = match (*editing).clone() { Some(b) => b, None => return };
            let biomes  = biomes.clone();
            let editing = editing.clone();
            let is_new  = is_new.clone();
            let toast   = toast.clone();
            spawn_local(async move {
                let result = if *is_new { api::create_biome(&b).await } else { api::update_biome(&b).await };
                match result {
                    Ok(_) => {
                        match api::get_biomes().await {
                            Ok(list) => biomes.set(list),
                            Err(e)   => toast.set(Some(format!("Refresh failed: {e}"))),
                        }
                        toast.set(Some("Saved ✓".into()));
                        editing.set(None);
                    }
                    Err(e) => toast.set(Some(format!("Save failed: {e}"))),
                }
            });
        })
    };

    // ── Delete ────────────────────────────────────────────────────────────────

    let make_delete = |id: String, biomes: UseStateHandle<Vec<Biome>>, toast: UseStateHandle<Option<String>>| {
        Callback::from(move |_: MouseEvent| {
            let id     = id.clone();
            let biomes = biomes.clone();
            let toast  = toast.clone();
            spawn_local(async move {
                match api::delete_biome(&id).await {
                    Ok(_) => {
                        match api::get_biomes().await {
                            Ok(list) => biomes.set(list),
                            Err(e)   => toast.set(Some(format!("Refresh failed: {e}"))),
                        }
                        toast.set(Some("Deleted ✓".into()));
                    }
                    Err(e) => toast.set(Some(format!("Delete failed: {e}"))),
                }
            });
        })
    };

    // ── Render ────────────────────────────────────────────────────────────────

    html! {
        <div class="section">
            <div class="section-header">
                <div>
                    <h1 class="section-title">{"🌿 Biomes"}</h1>
                    <p class="section-sub">{"Define world biome zones and encounter settings."}</p>
                </div>
                <button class="btn btn-primary" onclick={on_new}>{"+ New Biome"}</button>
            </div>

            if let Some(msg) = (*toast).clone() {
                <div class="toast">{ msg }</div>
            }

            // ── Edit / Create form panel ──────────────────────────────────────
            if let Some(b) = (*editing).clone() {
                <div class="form-panel">
                    <h2 class="form-panel-title">
                        { if *is_new { "New Biome" } else { "Edit Biome" } }
                    </h2>
                    <div class="form-grid">
                        <div class="field">
                            <label>{"ID"}<span class="hint">{"(slug, e.g. forest)"}</span></label>
                            <input type="text" class="input" value={b.id.clone()} oninput={on_id}
                                readonly={!*is_new} />
                        </div>
                        <div class="field">
                            <label>{"Name"}</label>
                            <input type="text" class="input" value={b.name.clone()} oninput={on_name} />
                        </div>
                        <div class="field">
                            <label>{"Color"}<span class="hint">{"(hex)"}</span></label>
                            <div class="color-row">
                                <input type="color" class="color-swatch" value={b.color.clone()} oninput={on_color.clone()} />
                                <input type="text"  class="input input-sm" value={b.color.clone()} oninput={on_color} />
                            </div>
                        </div>
                        <div class="field">
                            <label>{"Tile Type"}</label>
                            <input type="text" class="input" value={b.tile_type.clone()} oninput={on_tile}
                                placeholder="forest / mountain / dungeon / village / water / swamp" />
                        </div>
                        <div class="field">
                            <label>{"Zone"}<span class="hint">{"(NW / NE / SE / SW / C)"}</span></label>
                            <input type="text" class="input input-sm" value={b.zone.clone()} oninput={on_zone} />
                        </div>
                        <div class="field">
                            <label>{"Encounter Rate"}<span class="hint">{"(0.0 – 1.0)"}</span></label>
                            <input type="number" class="input input-sm" value={b.encounter_rate.to_string()}
                                min="0" max="1" step="0.05" oninput={on_rate} />
                        </div>
                        <div class="field field-full">
                            <label>{"Description"}</label>
                            <textarea class="input textarea" rows="2"
                                value={b.description.clone()} oninput={on_desc} />
                        </div>
                    </div>
                    <div class="form-actions">
                        <button class="btn btn-primary" onclick={on_save}>{"Save"}</button>
                        <button class="btn btn-ghost"   onclick={on_cancel}>{"Cancel"}</button>
                    </div>
                </div>
            }

            // ── List ──────────────────────────────────────────────────────────
            if *loading {
                <div class="loading-spinner">{"Loading…"}</div>
            } else {
                <div class="card-grid">
                    { for (*biomes).iter().map(|b| {
                        let b2        = b.clone();
                        let editing2  = editing.clone();
                        let is_new2   = is_new.clone();
                        let on_edit   = Callback::from(move |_: MouseEvent| {
                            editing2.set(Some(b2.clone()));
                            is_new2.set(false);
                        });
                        let on_del = make_delete(b.id.clone(), biomes.clone(), toast.clone());
                        html! {
                            <div class="card" style={format!("border-left-color:{}", b.color)}>
                                <div class="card-header">
                                    <span class="card-icon"
                                        style={format!("background:{}", b.color)}>
                                    </span>
                                    <div>
                                        <div class="card-title">{ &b.name }</div>
                                        <div class="card-sub">{ &b.zone }{" · "}{ &b.tile_type }</div>
                                    </div>
                                </div>
                                <p class="card-desc">{ &b.description }</p>
                                <div class="card-meta">
                                    <span class="badge">{ format!("Encounters: {:.0}%", b.encounter_rate * 100.0) }</span>
                                </div>
                                <div class="card-actions">
                                    <button class="btn btn-sm btn-secondary" onclick={on_edit}>{"Edit"}</button>
                                    <button class="btn btn-sm btn-danger"    onclick={on_del}>{"Delete"}</button>
                                </div>
                            </div>
                        }
                    }) }
                </div>
            }
        </div>
    }
}
