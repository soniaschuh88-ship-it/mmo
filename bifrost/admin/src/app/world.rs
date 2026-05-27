//! World settings section — edit the global world configuration.

use wasm_bindgen_futures::spawn_local;
use web_sys::{HtmlInputElement, HtmlTextAreaElement};
use yew::prelude::*;

use crate::{api, types::WorldSettings};

#[function_component(WorldSection)]
pub fn world_section() -> Html {
    let world   = use_state(WorldSettings::default);
    let loading = use_state(|| true);
    let saving  = use_state(|| false);
    let toast   = use_state(|| None::<String>);

    // Load on mount
    {
        let world   = world.clone();
        let loading = loading.clone();
        let toast   = toast.clone();
        use_effect_with((), move |_| {
            spawn_local(async move {
                match api::get_world().await {
                    Ok(w)  => { world.set(w); loading.set(false); }
                    Err(e) => { toast.set(Some(format!("Load failed: {e}"))); loading.set(false); }
                }
            });
            || ()
        });
    }

    // Input callbacks
    let on_name = {
        let world = world.clone();
        Callback::from(move |e: InputEvent| {
            let el: HtmlInputElement = e.target_unchecked_into();
            let mut w = (*world).clone(); w.name = el.value(); world.set(w);
        })
    };
    let on_size = {
        let world = world.clone();
        Callback::from(move |e: InputEvent| {
            let el: HtmlInputElement = e.target_unchecked_into();
            if let Ok(v) = el.value().parse::<u32>() {
                let mut w = (*world).clone(); w.size = v; world.set(w);
            }
        })
    };
    let on_desc = {
        let world = world.clone();
        Callback::from(move |e: InputEvent| {
            let el: HtmlTextAreaElement = e.target_unchecked_into();
            let mut w = (*world).clone(); w.description = el.value(); world.set(w);
        })
    };

    let on_save = {
        let world  = world.clone();
        let saving = saving.clone();
        let toast  = toast.clone();
        Callback::from(move |_: MouseEvent| {
            let w      = (*world).clone();
            let saving = saving.clone();
            let toast  = toast.clone();
            spawn_local(async move {
                saving.set(true);
                match api::save_world(&w).await {
                    Ok(_)  => toast.set(Some("World settings saved ✓".into())),
                    Err(e) => toast.set(Some(format!("Save failed: {e}"))),
                }
                saving.set(false);
            });
        })
    };

    let w = (*world).clone();

    html! {
        <div class="section">
            <div class="section-header">
                <h1 class="section-title">{"⚙ World Settings"}</h1>
                <p class="section-sub">{"Global configuration for the NOVA world."}</p>
            </div>

            if let Some(msg) = (*toast).clone() {
                <div class="toast">{ msg }</div>
            }

            if *loading {
                <div class="loading-spinner">{"Loading…"}</div>
            } else {
                <div class="form-card">
                    <div class="field">
                        <label>{"World Name"}</label>
                        <input
                            type="text"
                            class="input"
                            value={w.name.clone()}
                            oninput={on_name}
                        />
                    </div>
                    <div class="field">
                        <label>{"Map Size"}<span class="hint">{"(tiles, N×N)"}</span></label>
                        <input
                            type="number"
                            class="input input-sm"
                            value={w.size.to_string()}
                            min="16" max="256"
                            oninput={on_size}
                        />
                    </div>
                    <div class="field">
                        <label>{"Description"}</label>
                        <textarea
                            class="input textarea"
                            rows="3"
                            value={w.description.clone()}
                            oninput={on_desc}
                        />
                    </div>
                    <div class="form-actions">
                        <button
                            class="btn btn-primary"
                            disabled={*saving}
                            onclick={on_save}
                        >
                            { if *saving { "Saving…" } else { "Save World" } }
                        </button>
                    </div>
                </div>
            }
        </div>
    }
}
