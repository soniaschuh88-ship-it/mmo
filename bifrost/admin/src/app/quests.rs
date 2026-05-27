//! Quests section — CRUD for quest definitions.

use wasm_bindgen_futures::spawn_local;
use web_sys::{HtmlInputElement, HtmlTextAreaElement};
use yew::prelude::*;

use crate::{api, types::Quest};

#[function_component(QuestsSection)]
pub fn quests_section() -> Html {
    let quests  = use_state(Vec::<Quest>::new);
    let loading = use_state(|| true);
    let editing = use_state(|| None::<Quest>);
    let is_new  = use_state(|| false);
    let toast   = use_state(|| None::<String>);

    {
        let quests  = quests.clone();
        let loading = loading.clone();
        let toast   = toast.clone();
        use_effect_with((), move |_| {
            spawn_local(async move {
                match api::get_quests().await {
                    Ok(list) => { quests.set(list); loading.set(false); }
                    Err(e)   => { toast.set(Some(format!("Load failed: {e}"))); loading.set(false); }
                }
            });
            || ()
        });
    }

    let on_new = {
        let editing = editing.clone();
        let is_new  = is_new.clone();
        Callback::from(move |_: MouseEvent| { editing.set(Some(Quest::default())); is_new.set(true); })
    };

    let on_cancel = {
        let editing = editing.clone();
        Callback::from(move |_: MouseEvent| editing.set(None))
    };

    let on_save = {
        let editing = editing.clone();
        let is_new  = is_new.clone();
        let quests  = quests.clone();
        let toast   = toast.clone();
        Callback::from(move |_: MouseEvent| {
            let q       = match (*editing).clone() { Some(q) => q, None => return };
            let editing = editing.clone();
            let is_new  = is_new.clone();
            let quests  = quests.clone();
            let toast   = toast.clone();
            spawn_local(async move {
                let result = if *is_new { api::create_quest(&q).await } else { api::update_quest(&q).await };
                match result {
                    Ok(_) => {
                        if let Ok(list) = api::get_quests().await { quests.set(list); }
                        toast.set(Some("Saved ✓".into()));
                        editing.set(None);
                    }
                    Err(e) => toast.set(Some(format!("Save failed: {e}"))),
                }
            });
        })
    };

    let form = if let Some(q) = (*editing).clone() {
        macro_rules! text_cb {
            ($field:ident) => {{
                let editing = editing.clone();
                Callback::from(move |e: InputEvent| {
                    let el: HtmlInputElement = e.target_unchecked_into();
                    if let Some(mut quest) = (*editing).clone() {
                        quest.$field = el.value(); editing.set(Some(quest));
                    }
                })
            }};
        }
        macro_rules! num_cb {
            ($field:ident, $t:ty) => {{
                let editing = editing.clone();
                Callback::from(move |e: InputEvent| {
                    let el: HtmlInputElement = e.target_unchecked_into();
                    if let Ok(v) = el.value().parse::<$t>() {
                        if let Some(mut quest) = (*editing).clone() {
                            quest.$field = v; editing.set(Some(quest));
                        }
                    }
                })
            }};
        }
        let on_desc = {
            let editing = editing.clone();
            Callback::from(move |e: InputEvent| {
                let el: HtmlTextAreaElement = e.target_unchecked_into();
                if let Some(mut quest) = (*editing).clone() {
                    quest.desc = el.value(); editing.set(Some(quest));
                }
            })
        };

        html! {
            <div class="form-panel">
                <h2 class="form-panel-title">
                    { q.icon.clone() }{" "}
                    { if *is_new { "New Quest" } else { "Edit Quest" } }
                </h2>
                <div class="form-grid">
                    <div class="field">
                        <label>{"ID"}</label>
                        <input type="text" class="input" value={q.id.clone()}
                            oninput={text_cb!(id)} readonly={!*is_new} />
                    </div>
                    <div class="field">
                        <label>{"Title"}</label>
                        <input type="text" class="input" value={q.title.clone()}
                            oninput={text_cb!(title)} />
                    </div>
                    <div class="field">
                        <label>{"Icon"}</label>
                        <input type="text" class="input input-sm" value={q.icon.clone()}
                            oninput={text_cb!(icon)} />
                    </div>
                    <div class="field">
                        <label>{"Target Monster ID"}</label>
                        <input type="text" class="input" value={q.target.clone()}
                            oninput={text_cb!(target)}
                            placeholder="wolf / goblin / rat / skeleton / …" />
                    </div>
                    <div class="field">
                        <label>{"Kill / Collect Count"}</label>
                        <input type="number" class="input input-sm" value={q.count.to_string()}
                            min="1" oninput={num_cb!(count, u32)} />
                    </div>
                    <div class="field">
                        <label>{"XP Reward"}</label>
                        <input type="number" class="input input-sm" value={q.xp.to_string()}
                            min="0" step="10" oninput={num_cb!(xp, u32)} />
                    </div>
                    <div class="field">
                        <label>{"Gold Reward"}</label>
                        <input type="number" class="input input-sm" value={q.gold.to_string()}
                            min="0" step="5" oninput={num_cb!(gold, u32)} />
                    </div>
                    <div class="field">
                        <label>{"Giver NPC Name"}</label>
                        <input type="text" class="input" value={q.giver_name.clone()}
                            oninput={text_cb!(giver_name)} />
                    </div>
                    <div class="field field-full">
                        <label>{"Description"}</label>
                        <textarea class="input textarea" rows="2"
                            value={q.desc.clone()} oninput={on_desc} />
                    </div>
                </div>
                <div class="form-actions">
                    <button class="btn btn-primary" onclick={on_save.clone()}>{"Save"}</button>
                    <button class="btn btn-ghost"   onclick={on_cancel.clone()}>{"Cancel"}</button>
                </div>
            </div>
        }
    } else {
        html! {}
    };

    html! {
        <div class="section">
            <div class="section-header">
                <div>
                    <h1 class="section-title">{"📜 Quests"}</h1>
                    <p class="section-sub">{"Define quest chains, objectives and rewards."}</p>
                </div>
                <button class="btn btn-primary" onclick={on_new}>{"+ New Quest"}</button>
            </div>

            if let Some(msg) = (*toast).clone() {
                <div class="toast">{ msg }</div>
            }

            { form }

            if *loading {
                <div class="loading-spinner">{"Loading…"}</div>
            } else {
                <div class="quest-grid">
                    { for (*quests).iter().map(|q| {
                        let q2      = q.clone();
                        let editing = editing.clone();
                        let is_new  = is_new.clone();
                        let quests2 = quests.clone();
                        let toast2  = toast.clone();

                        let on_edit = Callback::from(move |_: MouseEvent| {
                            editing.set(Some(q2.clone())); is_new.set(false);
                        });
                        let qid = q.id.clone();
                        let on_del = Callback::from(move |_: MouseEvent| {
                            let id    = qid.clone();
                            let qs    = quests2.clone();
                            let toast = toast2.clone();
                            spawn_local(async move {
                                match api::delete_quest(&id).await {
                                    Ok(_) => { if let Ok(l) = api::get_quests().await { qs.set(l); } }
                                    Err(e) => toast.set(Some(format!("Delete failed: {e}"))),
                                }
                            });
                        });

                        html! {
                            <div class="quest-card">
                                <div class="quest-icon">{ &q.icon }</div>
                                <div class="quest-body">
                                    <div class="quest-title">{ &q.title }</div>
                                    <div class="quest-meta">
                                        <span class="badge">{ format!("{}×{}", q.count, &q.target) }</span>
                                        <span class="badge gold">{ format!("{}💰", q.gold) }</span>
                                        <span class="badge xp">{ format!("{}XP", q.xp) }</span>
                                    </div>
                                    <div class="quest-desc">{ &q.desc }</div>
                                    <div class="quest-giver">{ format!("Giver: {}", &q.giver_name) }</div>
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
