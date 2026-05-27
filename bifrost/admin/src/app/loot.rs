//! Loot section — Monsters (with drops) and Items.

use wasm_bindgen_futures::spawn_local;
use web_sys::{HtmlInputElement, HtmlSelectElement, HtmlTextAreaElement};
use yew::prelude::*;

use crate::{api, types::{LootItem, Monster}};

#[derive(Clone, Copy, PartialEq)]
enum LootTab { Monsters, Items }

#[function_component(LootSection)]
pub fn loot_section() -> Html {
    let tab      = use_state(|| LootTab::Monsters);
    let monsters = use_state(Vec::<Monster>::new);
    let items    = use_state(Vec::<LootItem>::new);
    let loading  = use_state(|| true);
    let edit_m   = use_state(|| None::<Monster>);
    let edit_i   = use_state(|| None::<LootItem>);
    let is_new   = use_state(|| false);
    let toast    = use_state(|| None::<String>);

    // Load on mount
    {
        let monsters = monsters.clone();
        let items    = items.clone();
        let loading  = loading.clone();
        let toast    = toast.clone();
        use_effect_with((), move |_| {
            spawn_local(async move {
                let (r1, r2) = futures_join(api::get_monsters(), api::get_items()).await;
                match r1 { Ok(m) => monsters.set(m), Err(e) => toast.set(Some(format!("Monsters load failed: {e}"))) }
                match r2 { Ok(i) => items.set(i),    Err(e) => toast.set(Some(format!("Items load failed: {e}"))) }
                loading.set(false);
            });
            || ()
        });
    }

    // Tab switcher
    let on_tab_m = { let tab = tab.clone(); Callback::from(move |_: MouseEvent| tab.set(LootTab::Monsters)) };
    let on_tab_i = { let tab = tab.clone(); Callback::from(move |_: MouseEvent| tab.set(LootTab::Items)) };

    // Cancel form
    let on_cancel = {
        let edit_m = edit_m.clone(); let edit_i = edit_i.clone();
        Callback::from(move |_: MouseEvent| { edit_m.set(None); edit_i.set(None); })
    };

    // ─── Monsters ─────────────────────────────────────────────────────────────

    let on_new_monster = {
        let edit_m = edit_m.clone(); let is_new = is_new.clone();
        Callback::from(move |_: MouseEvent| { edit_m.set(Some(Monster::default())); is_new.set(true); })
    };

    let on_save_monster = {
        let edit_m   = edit_m.clone();
        let is_new   = is_new.clone();
        let monsters = monsters.clone();
        let toast    = toast.clone();
        Callback::from(move |_: MouseEvent| {
            let m        = match (*edit_m).clone() { Some(m) => m, None => return };
            let edit_m   = edit_m.clone();
            let is_new   = is_new.clone();
            let monsters = monsters.clone();
            let toast    = toast.clone();
            spawn_local(async move {
                let r = if *is_new { api::create_monster(&m).await } else { api::update_monster(&m).await };
                match r {
                    Ok(_) => {
                        if let Ok(list) = api::get_monsters().await { monsters.set(list); }
                        toast.set(Some("Saved ✓".into())); edit_m.set(None);
                    }
                    Err(e) => toast.set(Some(format!("Save failed: {e}"))),
                }
            });
        })
    };

    let monster_form = if let Some(m) = (*edit_m).clone() {
        macro_rules! m_text {
            ($field:ident) => {{
                let em = edit_m.clone();
                Callback::from(move |e: InputEvent| {
                    let el: HtmlInputElement = e.target_unchecked_into();
                    if let Some(mut mo) = (*em).clone() { mo.$field = el.value(); em.set(Some(mo)); }
                })
            }};
        }
        macro_rules! m_num {
            ($field:ident, $t:ty) => {{
                let em = edit_m.clone();
                Callback::from(move |e: InputEvent| {
                    let el: HtmlInputElement = e.target_unchecked_into();
                    if let Ok(v) = el.value().parse::<$t>() {
                        if let Some(mut mo) = (*em).clone() { mo.$field = v; em.set(Some(mo)); }
                    }
                })
            }};
        }
        html! {
            <div class="form-panel">
                <h2 class="form-panel-title">
                    { m.icon.clone() }{" "}{ if *is_new { "New Monster" } else { "Edit Monster" } }
                </h2>
                <div class="form-grid">
                    <div class="field">
                        <label>{"ID"}</label>
                        <input type="text" class="input" value={m.id.clone()}
                            oninput={m_text!(id)} readonly={!*is_new} />
                    </div>
                    <div class="field">
                        <label>{"Name"}</label>
                        <input type="text" class="input" value={m.name.clone()} oninput={m_text!(name)} />
                    </div>
                    <div class="field">
                        <label>{"Icon"}</label>
                        <input type="text" class="input input-sm" value={m.icon.clone()} oninput={m_text!(icon)} />
                    </div>
                    <div class="field">
                        <label>{"Color"}</label>
                        <div class="color-row">
                            <input type="color" class="color-swatch" value={m.color.clone()} oninput={m_text!(color).clone()} />
                            <input type="text"  class="input input-sm" value={m.color.clone()} oninput={m_text!(color)} />
                        </div>
                    </div>
                    <div class="field">
                        <label>{"Zone"}</label>
                        <input type="text" class="input" value={m.zone.clone()} oninput={m_text!(zone)}
                            placeholder="forest / mountain / dungeon" />
                    </div>
                    <div class="field">
                        <label>{"HP"}</label>
                        <input type="number" class="input input-sm" value={m.hp.to_string()}
                            min="1" oninput={m_num!(hp, u32)} />
                    </div>
                    <div class="field">
                        <label>{"ATK"}</label>
                        <input type="number" class="input input-sm" value={m.atk.to_string()}
                            min="0" oninput={m_num!(atk, u32)} />
                    </div>
                    <div class="field">
                        <label>{"DEF"}</label>
                        <input type="number" class="input input-sm" value={m.def.to_string()}
                            min="0" oninput={m_num!(def, u32)} />
                    </div>
                    <div class="field">
                        <label>{"XP"}</label>
                        <input type="number" class="input input-sm" value={m.xp.to_string()}
                            min="0" step="5" oninput={m_num!(xp, u32)} />
                    </div>
                    <div class="field">
                        <label>{"Gold"}</label>
                        <input type="number" class="input input-sm" value={m.gold.to_string()}
                            min="0" oninput={m_num!(gold, u32)} />
                    </div>
                </div>
                <div class="form-actions">
                    <button class="btn btn-primary" onclick={on_save_monster}>{"Save"}</button>
                    <button class="btn btn-ghost"   onclick={on_cancel.clone()}>{"Cancel"}</button>
                </div>
            </div>
        }
    } else { html! {} };

    // ─── Items ────────────────────────────────────────────────────────────────

    let on_new_item = {
        let edit_i = edit_i.clone(); let is_new = is_new.clone();
        Callback::from(move |_: MouseEvent| { edit_i.set(Some(LootItem::default())); is_new.set(true); })
    };

    let on_save_item = {
        let edit_i = edit_i.clone();
        let is_new = is_new.clone();
        let items  = items.clone();
        let toast  = toast.clone();
        Callback::from(move |_: MouseEvent| {
            let i      = match (*edit_i).clone() { Some(i) => i, None => return };
            let edit_i = edit_i.clone();
            let is_new = is_new.clone();
            let items  = items.clone();
            let toast  = toast.clone();
            spawn_local(async move {
                let r = if *is_new { api::create_item(&i).await } else { api::update_item(&i).await };
                match r {
                    Ok(_) => {
                        if let Ok(list) = api::get_items().await { items.set(list); }
                        toast.set(Some("Saved ✓".into())); edit_i.set(None);
                    }
                    Err(e) => toast.set(Some(format!("Save failed: {e}"))),
                }
            });
        })
    };

    let item_form = if let Some(i) = (*edit_i).clone() {
        macro_rules! i_text {
            ($field:ident) => {{
                let ei = edit_i.clone();
                Callback::from(move |e: InputEvent| {
                    let el: HtmlInputElement = e.target_unchecked_into();
                    if let Some(mut it) = (*ei).clone() { it.$field = el.value(); ei.set(Some(it)); }
                })
            }};
        }
        let on_val = {
            let ei = edit_i.clone();
            Callback::from(move |e: InputEvent| {
                let el: HtmlInputElement = e.target_unchecked_into();
                if let Ok(v) = el.value().parse::<u32>() {
                    if let Some(mut it) = (*ei).clone() { it.value = v; ei.set(Some(it)); }
                }
            })
        };
        let on_itype = {
            let ei = edit_i.clone();
            Callback::from(move |e: Event| {
                let el: HtmlSelectElement = e.target_unchecked_into();
                if let Some(mut it) = (*ei).clone() { it.item_type = el.value(); ei.set(Some(it)); }
            })
        };
        let on_idesc = {
            let ei = edit_i.clone();
            Callback::from(move |e: InputEvent| {
                let el: HtmlTextAreaElement = e.target_unchecked_into();
                if let Some(mut it) = (*ei).clone() { it.description = el.value(); ei.set(Some(it)); }
            })
        };
        html! {
            <div class="form-panel">
                <h2 class="form-panel-title">
                    { i.icon.clone() }{" "}{ if *is_new { "New Item" } else { "Edit Item" } }
                </h2>
                <div class="form-grid">
                    <div class="field">
                        <label>{"ID"}</label>
                        <input type="text" class="input" value={i.id.clone()}
                            oninput={i_text!(id)} readonly={!*is_new} />
                    </div>
                    <div class="field">
                        <label>{"Name"}</label>
                        <input type="text" class="input" value={i.name.clone()} oninput={i_text!(name)} />
                    </div>
                    <div class="field">
                        <label>{"Icon"}</label>
                        <input type="text" class="input input-sm" value={i.icon.clone()} oninput={i_text!(icon)} />
                    </div>
                    <div class="field">
                        <label>{"Value (gold)"}</label>
                        <input type="number" class="input input-sm" value={i.value.to_string()}
                            min="0" oninput={on_val} />
                    </div>
                    <div class="field">
                        <label>{"Type"}</label>
                        <select class="input" onchange={on_itype}>
                            { for ["material","currency","equipment","consumable"].iter().map(|&t| html! {
                                <option value={t} selected={i.item_type == t}>{t}</option>
                            }) }
                        </select>
                    </div>
                    <div class="field field-full">
                        <label>{"Description"}</label>
                        <textarea class="input textarea" rows="2"
                            value={i.description.clone()} oninput={on_idesc} />
                    </div>
                </div>
                <div class="form-actions">
                    <button class="btn btn-primary" onclick={on_save_item}>{"Save"}</button>
                    <button class="btn btn-ghost"   onclick={on_cancel.clone()}>{"Cancel"}</button>
                </div>
            </div>
        }
    } else { html! {} };

    html! {
        <div class="section">
            <div class="section-header">
                <div>
                    <h1 class="section-title">{"💎 Loot"}</h1>
                    <p class="section-sub">{"Manage monsters and item drops."}</p>
                </div>
                <div class="tab-bar">
                    <button class={classes!("btn", (*tab == LootTab::Monsters).then_some("btn-primary").unwrap_or("btn-ghost"))}
                        onclick={on_tab_m}>{"Monsters"}</button>
                    <button class={classes!("btn", (*tab == LootTab::Items).then_some("btn-primary").unwrap_or("btn-ghost"))}
                        onclick={on_tab_i}>{"Items"}</button>
                </div>
            </div>

            if let Some(msg) = (*toast).clone() {
                <div class="toast">{ msg }</div>
            }

            if *tab == LootTab::Monsters {
                <>
                    <div class="sub-header">
                        <button class="btn btn-primary" onclick={on_new_monster}>{"+ New Monster"}</button>
                    </div>
                    { monster_form }
                    if *loading {
                        <div class="loading-spinner">{"Loading…"}</div>
                    } else {
                        <div class="monster-grid">
                            { for (*monsters).iter().map(|m| {
                                let m2       = m.clone();
                                let edit_m   = edit_m.clone();
                                let is_new   = is_new.clone();
                                let monsters = monsters.clone();
                                let toast    = toast.clone();
                                let on_edit  = Callback::from(move |_: MouseEvent| {
                                    edit_m.set(Some(m2.clone())); is_new.set(false);
                                });
                                let mid = m.id.clone();
                                let on_del = Callback::from(move |_: MouseEvent| {
                                    let id = mid.clone(); let ms = monsters.clone(); let toast = toast.clone();
                                    spawn_local(async move {
                                        match api::delete_monster(&id).await {
                                            Ok(_) => { if let Ok(l) = api::get_monsters().await { ms.set(l); } }
                                            Err(e) => toast.set(Some(format!("Delete failed: {e}"))),
                                        }
                                    });
                                });
                                html! {
                                    <div class="monster-card" style={format!("border-color:{}", m.color)}>
                                        <div class="monster-icon">{ &m.icon }</div>
                                        <div class="monster-name">{ &m.name }</div>
                                        <div class="monster-stats">
                                            <span title="HP">{ format!("❤{}", m.hp) }</span>
                                            <span title="ATK">{ format!("⚔{}", m.atk) }</span>
                                            <span title="DEF">{ format!("🛡{}", m.def) }</span>
                                        </div>
                                        <div class="monster-rewards">
                                            <span>{ format!("{}XP", m.xp) }</span>
                                            <span>{ format!("{}💰", m.gold) }</span>
                                        </div>
                                        <div class="badge badge-zone">{ &m.zone }</div>
                                        <div class="card-actions">
                                            <button class="btn btn-sm btn-secondary" onclick={on_edit}>{"Edit"}</button>
                                            <button class="btn btn-sm btn-danger"    onclick={on_del}>{"Del"}</button>
                                        </div>
                                    </div>
                                }
                            }) }
                        </div>
                    }
                </>
            }

            if *tab == LootTab::Items {
                <>
                    <div class="sub-header">
                        <button class="btn btn-primary" onclick={on_new_item}>{"+ New Item"}</button>
                    </div>
                    { item_form }
                    if *loading {
                        <div class="loading-spinner">{"Loading…"}</div>
                    } else {
                        <div class="item-table">
                            <div class="table-head">
                                <span>{"Item"}</span>
                                <span>{"Type"}</span>
                                <span>{"Value"}</span>
                                <span>{"Description"}</span>
                                <span>{"Actions"}</span>
                            </div>
                            { for (*items).iter().map(|i| {
                                let i2       = i.clone();
                                let edit_i   = edit_i.clone();
                                let is_new   = is_new.clone();
                                let items    = items.clone();
                                let toast    = toast.clone();
                                let on_edit  = Callback::from(move |_: MouseEvent| {
                                    edit_i.set(Some(i2.clone())); is_new.set(false);
                                });
                                let iid = i.id.clone();
                                let on_del = Callback::from(move |_: MouseEvent| {
                                    let id = iid.clone(); let is = items.clone(); let toast = toast.clone();
                                    spawn_local(async move {
                                        match api::delete_item(&id).await {
                                            Ok(_) => { if let Ok(l) = api::get_items().await { is.set(l); } }
                                            Err(e) => toast.set(Some(format!("Delete failed: {e}"))),
                                        }
                                    });
                                });
                                html! {
                                    <div class="table-row">
                                        <span class="item-name">
                                            { &i.icon }{" "}{ &i.name }
                                        </span>
                                        <span><span class="badge">{ &i.item_type }</span></span>
                                        <span class="mono">{ format!("{}💰", i.value) }</span>
                                        <span class="small">{ &i.description }</span>
                                        <span class="row-actions">
                                            <button class="btn btn-sm btn-secondary" onclick={on_edit}>{"Edit"}</button>
                                            <button class="btn btn-sm btn-danger"    onclick={on_del}>{"Del"}</button>
                                        </span>
                                    </div>
                                }
                            }) }
                        </div>
                    }
                </>
            }
        </div>
    }
}

// ── Minimal futures join (no futures crate needed) ────────────────────────────

async fn futures_join<A, B, E1, E2>(
    f1: impl std::future::Future<Output = Result<A, E1>>,
    f2: impl std::future::Future<Output = Result<B, E2>>,
) -> (Result<A, E1>, Result<B, E2>) {
    // Sequential — simple and sufficient for a low-concurrency admin UI.
    let r1 = f1.await;
    let r2 = f2.await;
    (r1, r2)
}
