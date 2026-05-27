//! Story section — manage story arcs, beats, and world mood.

use wasm_bindgen_futures::spawn_local;
use web_sys::{HtmlInputElement, HtmlSelectElement, HtmlTextAreaElement};
use yew::prelude::*;

use crate::{api, types::{StoryArc, StoryBeat, StoryData}};

// ─── Edit mode ────────────────────────────────────────────────────────────────

#[derive(Clone, PartialEq)]
enum EditTarget {
    Arc(StoryArc, bool),              // (arc, is_new)
    Beat(String, StoryBeat, bool),    // (arc_id, beat, is_new)
}

// ─── StorySection ─────────────────────────────────────────────────────────────

#[function_component(StorySection)]
pub fn story_section() -> Html {
    let story    = use_state(|| StoryData { world_mood: "calm".into(), arcs: vec![] });
    let loading  = use_state(|| true);
    let editing  = use_state(|| None::<EditTarget>);
    let expanded = use_state(Vec::<String>::new);  // arc ids that are expanded
    let toast    = use_state(|| None::<String>);

    // Load on mount
    {
        let story   = story.clone();
        let loading = loading.clone();
        let toast   = toast.clone();
        use_effect_with((), move |_| {
            spawn_local(async move {
                match api::get_story().await {
                    Ok(s)  => { story.set(s); loading.set(false); }
                    Err(e) => { toast.set(Some(format!("Load failed: {e}"))); loading.set(false); }
                }
            });
            || ()
        });
    }

    let reload = {
        let story = story.clone();
        let toast = toast.clone();
        move || {
            let story = story.clone();
            let toast = toast.clone();
            spawn_local(async move {
                match api::get_story().await {
                    Ok(s)  => story.set(s),
                    Err(e) => toast.set(Some(format!("Reload failed: {e}"))),
                }
            });
        }
    };

    let on_cancel = {
        let editing = editing.clone();
        Callback::from(move |_: MouseEvent| editing.set(None))
    };

    // ── World mood ────────────────────────────────────────────────────────────

    let on_mood = {
        let story = story.clone();
        let toast = toast.clone();
        Callback::from(move |e: Event| {
            let el: HtmlSelectElement = e.target_unchecked_into();
            let mood  = el.value();
            let story = story.clone();
            let toast = toast.clone();
            spawn_local(async move {
                match api::update_arc(&StoryArc {
                    id: "__mood__".into(), title: String::new(),
                    synopsis: String::new(), status: String::new(),
                    affected_zones: vec![], beats: vec![],
                }).await {
                    _ => {} // we call a dedicated mood endpoint below
                }
                // Patch via the mood route
                let client = gloo_net::http::Request::put("/admin-api/story/mood")
                    .header("Content-Type", "application/json")
                    .body(format!(r#"{{"worldMood":"{}"}}"#, mood))
                    .ok();
                if let Some(req) = client {
                    if let Ok(resp) = req.send().await {
                        if resp.ok() {
                            let mut s = (*story).clone();
                            s.world_mood = mood;
                            story.set(s);
                            toast.set(Some("World mood updated ✓".into()));
                        }
                    }
                }
            });
        })
    };

    // ── Arc CRUD ──────────────────────────────────────────────────────────────

    let on_new_arc = {
        let editing = editing.clone();
        Callback::from(move |_: MouseEvent| {
            editing.set(Some(EditTarget::Arc(StoryArc::default(), true)));
        })
    };

    let on_save = {
        let editing = editing.clone();
        let toast   = toast.clone();
        let reload  = reload.clone();
        Callback::from(move |_: MouseEvent| {
            let target  = match (*editing).clone() { Some(t) => t, None => return };
            let editing = editing.clone();
            let toast   = toast.clone();
            let reload  = reload.clone();
            spawn_local(async move {
                let result = match &target {
                    EditTarget::Arc(arc, true)  => api::create_arc(arc).await.map(|_| ()),
                    EditTarget::Arc(arc, false) => api::update_arc(arc).await.map(|_| ()),
                    EditTarget::Beat(arc_id, beat, true)  => api::create_beat(arc_id, beat).await.map(|_| ()),
                    EditTarget::Beat(arc_id, beat, false) => api::update_beat(arc_id, beat).await.map(|_| ()),
                    _ => Err("unknown target".into()),
                };
                match result {
                    Ok(_)  => { toast.set(Some("Saved ✓".into())); editing.set(None); reload(); }
                    Err(e) => toast.set(Some(format!("Save failed: {e}"))),
                }
            });
        })
    };

    // ── Render form ───────────────────────────────────────────────────────────

    let form_html = match (*editing).clone() {
        None => html! {},
        Some(EditTarget::Arc(arc, is_new)) => {
            let arc2 = arc.clone();

            macro_rules! arc_text {
                ($field:ident) => {{
                    let editing = editing.clone();
                    let a = arc2.clone();
                    Callback::from(move |e: InputEvent| {
                        let el: HtmlInputElement = e.target_unchecked_into();
                        let mut na = a.clone(); na.$field = el.value();
                        editing.set(Some(EditTarget::Arc(na, is_new)));
                    })
                }};
            }

            let on_synopsis = {
                let editing = editing.clone(); let a = arc2.clone();
                Callback::from(move |e: InputEvent| {
                    let el: HtmlTextAreaElement = e.target_unchecked_into();
                    let mut na = a.clone(); na.synopsis = el.value();
                    editing.set(Some(EditTarget::Arc(na, is_new)));
                })
            };
            let on_status = {
                let editing = editing.clone(); let a = arc2.clone();
                Callback::from(move |e: Event| {
                    let el: HtmlSelectElement = e.target_unchecked_into();
                    let mut na = a.clone(); na.status = el.value();
                    editing.set(Some(EditTarget::Arc(na, is_new)));
                })
            };
            let on_zones = {
                let editing = editing.clone(); let a = arc2.clone();
                Callback::from(move |e: InputEvent| {
                    let el: HtmlInputElement = e.target_unchecked_into();
                    let mut na = a.clone();
                    na.affected_zones = el.value().split(',')
                        .map(|s| s.trim().to_owned()).filter(|s| !s.is_empty()).collect();
                    editing.set(Some(EditTarget::Arc(na, is_new)));
                })
            };

            html! {
                <div class="form-panel">
                    <h2 class="form-panel-title">
                        { if is_new { "New Story Arc" } else { "Edit Story Arc" } }
                    </h2>
                    <div class="form-grid">
                        <div class="field">
                            <label>{"ID"}</label>
                            <input type="text" class="input" value={arc.id.clone()}
                                oninput={arc_text!(id)} readonly={!is_new} />
                        </div>
                        <div class="field">
                            <label>{"Title"}</label>
                            <input type="text" class="input" value={arc.title.clone()}
                                oninput={arc_text!(title)} />
                        </div>
                        <div class="field">
                            <label>{"Status"}</label>
                            <select class="input" onchange={on_status}>
                                { for ["active","completed","abandoned"].iter().map(|&s| html! {
                                    <option value={s} selected={arc.status == s}>{s}</option>
                                }) }
                            </select>
                        </div>
                        <div class="field">
                            <label>{"Affected Zones"}<span class="hint">{"(comma-separated)"}</span></label>
                            <input type="text" class="input"
                                value={arc.affected_zones.join(", ")} oninput={on_zones}
                                placeholder="village, mountain, dungeon" />
                        </div>
                        <div class="field field-full">
                            <label>{"Synopsis"}</label>
                            <textarea class="input textarea" rows="3"
                                value={arc.synopsis.clone()} oninput={on_synopsis} />
                        </div>
                    </div>
                    <div class="form-actions">
                        <button class="btn btn-primary" onclick={on_save.clone()}>{"Save Arc"}</button>
                        <button class="btn btn-ghost"   onclick={on_cancel.clone()}>{"Cancel"}</button>
                    </div>
                </div>
            }
        }
        Some(EditTarget::Beat(arc_id, beat, is_new)) => {
            let beat2  = beat.clone();
            let aid    = arc_id.clone();

            macro_rules! beat_text {
                ($field:ident) => {{
                    let editing = editing.clone(); let b = beat2.clone(); let aid2 = aid.clone();
                    Callback::from(move |e: InputEvent| {
                        let el: HtmlInputElement = e.target_unchecked_into();
                        let mut nb = b.clone(); nb.$field = el.value();
                        editing.set(Some(EditTarget::Beat(aid2.clone(), nb, is_new)));
                    })
                }};
            }

            let on_bdesc = {
                let editing = editing.clone(); let b = beat2.clone(); let aid2 = aid.clone();
                Callback::from(move |e: InputEvent| {
                    let el: HtmlTextAreaElement = e.target_unchecked_into();
                    let mut nb = b.clone(); nb.description = el.value();
                    editing.set(Some(EditTarget::Beat(aid2.clone(), nb, is_new)));
                })
            };
            let on_tick = {
                let editing = editing.clone(); let b = beat2.clone(); let aid2 = aid.clone();
                Callback::from(move |e: InputEvent| {
                    let el: HtmlInputElement = e.target_unchecked_into();
                    if let Ok(v) = el.value().parse::<u64>() {
                        let mut nb = b.clone(); nb.min_tick = v;
                        editing.set(Some(EditTarget::Beat(aid2.clone(), nb, is_new)));
                    }
                })
            };
            let on_req = {
                let editing = editing.clone(); let b = beat2.clone(); let aid2 = aid.clone();
                Callback::from(move |e: InputEvent| {
                    let el: HtmlInputElement = e.target_unchecked_into();
                    let mut nb = b.clone();
                    nb.required_quests = el.value().split(',')
                        .map(|s| s.trim().to_owned()).filter(|s| !s.is_empty()).collect();
                    editing.set(Some(EditTarget::Beat(aid2.clone(), nb, is_new)));
                })
            };
            let on_cons = {
                let editing = editing.clone(); let b = beat2.clone(); let aid2 = aid.clone();
                Callback::from(move |e: InputEvent| {
                    let el: HtmlTextAreaElement = e.target_unchecked_into();
                    let mut nb = b.clone();
                    nb.consequences = el.value().lines()
                        .map(|s| s.trim().to_owned()).filter(|s| !s.is_empty()).collect();
                    editing.set(Some(EditTarget::Beat(aid2.clone(), nb, is_new)));
                })
            };

            html! {
                <div class="form-panel">
                    <h2 class="form-panel-title">
                        { if is_new { "New Story Beat" } else { "Edit Story Beat" } }
                        <span class="hint">{format!(" (Arc: {})", arc_id)}</span>
                    </h2>
                    <div class="form-grid">
                        <div class="field">
                            <label>{"ID"}</label>
                            <input type="text" class="input" value={beat.id.clone()}
                                oninput={beat_text!(id)} readonly={!is_new} />
                        </div>
                        <div class="field">
                            <label>{"Title"}</label>
                            <input type="text" class="input" value={beat.title.clone()}
                                oninput={beat_text!(title)} />
                        </div>
                        <div class="field">
                            <label>{"Min Tick"}</label>
                            <input type="number" class="input input-sm" value={beat.min_tick.to_string()}
                                min="0" step="100" oninput={on_tick} />
                        </div>
                        <div class="field">
                            <label>{"Required Quests"}<span class="hint">{"(comma-separated IDs)"}</span></label>
                            <input type="text" class="input" value={beat.required_quests.join(", ")}
                                oninput={on_req} placeholder="wolves, goblins" />
                        </div>
                        <div class="field field-full">
                            <label>{"Description"}</label>
                            <textarea class="input textarea" rows="2"
                                value={beat.description.clone()} oninput={on_bdesc} />
                        </div>
                        <div class="field field-full">
                            <label>{"Consequences"}<span class="hint">{"(one per line, e.g. change_mood:tense)"}</span></label>
                            <textarea class="input textarea" rows="2"
                                value={beat.consequences.join("\n")} oninput={on_cons} />
                        </div>
                    </div>
                    <div class="form-actions">
                        <button class="btn btn-primary" onclick={on_save.clone()}>{"Save Beat"}</button>
                        <button class="btn btn-ghost"   onclick={on_cancel.clone()}>{"Cancel"}</button>
                    </div>
                </div>
            }
        }
    };

    // ── Arc list ──────────────────────────────────────────────────────────────

    let arc_list = if *loading {
        html! { <div class="loading-spinner">{"Loading…"}</div> }
    } else {
        let arcs = (*story).arcs.clone();
        arcs.into_iter().map(|arc| {
            let arc_id     = arc.id.clone();
            let is_open    = (*expanded).contains(&arc_id);

            // Toggle expand
            let toggle = {
                let expanded = expanded.clone(); let aid = arc_id.clone();
                Callback::from(move |_: MouseEvent| {
                    let mut v = (*expanded).clone();
                    if v.contains(&aid) { v.retain(|x| x != &aid); } else { v.push(aid.clone()); }
                    expanded.set(v);
                })
            };
            // Edit arc
            let on_edit_arc = {
                let editing = editing.clone(); let a = arc.clone();
                Callback::from(move |_: MouseEvent| editing.set(Some(EditTarget::Arc(a.clone(), false))))
            };
            // Delete arc
            let on_del_arc = {
                let toast  = toast.clone(); let reload = reload.clone(); let aid = arc_id.clone();
                Callback::from(move |_: MouseEvent| {
                    let id = aid.clone(); let toast = toast.clone(); let reload = reload.clone();
                    spawn_local(async move {
                        match api::delete_arc(&id).await {
                            Ok(_)  => { toast.set(Some("Arc deleted ✓".into())); reload(); }
                            Err(e) => toast.set(Some(format!("Delete failed: {e}"))),
                        }
                    });
                })
            };
            // New beat
            let on_new_beat = {
                let editing = editing.clone(); let aid = arc_id.clone();
                Callback::from(move |_: MouseEvent| {
                    editing.set(Some(EditTarget::Beat(aid.clone(), StoryBeat::default(), true)));
                })
            };

            let status_color = match arc.status.as_str() {
                "completed" => "#3a8a3a", "abandoned" => "#8a2020", _ => "#6060a0"
            };

            let beats_html = if is_open {
                let beats = arc.beats.clone();
                beats.into_iter().map(|beat| {
                    let on_edit_beat = {
                        let editing = editing.clone(); let aid = arc_id.clone(); let b = beat.clone();
                        Callback::from(move |_: MouseEvent| {
                            editing.set(Some(EditTarget::Beat(aid.clone(), b.clone(), false)));
                        })
                    };
                    let on_del_beat = {
                        let toast  = toast.clone(); let reload = reload.clone();
                        let aid    = arc_id.clone(); let bid = beat.id.clone();
                        Callback::from(move |_: MouseEvent| {
                            let aid = aid.clone(); let bid = bid.clone();
                            let toast = toast.clone(); let reload = reload.clone();
                            spawn_local(async move {
                                match api::delete_beat(&aid, &bid).await {
                                    Ok(_)  => { toast.set(Some("Beat deleted ✓".into())); reload(); }
                                    Err(e) => toast.set(Some(format!("Delete failed: {e}"))),
                                }
                            });
                        })
                    };
                    html! {
                        <div class="beat-row">
                            <div class="beat-info">
                                <span class="beat-title">{ &beat.title }</span>
                                <span class="beat-meta">
                                    { format!("tick≥{}", beat.min_tick) }
                                    { if !beat.required_quests.is_empty() {
                                        format!(" · needs: {}", beat.required_quests.join(", "))
                                    } else { String::new() } }
                                </span>
                            </div>
                            <div class="beat-actions">
                                <button class="btn btn-xs btn-secondary" onclick={on_edit_beat}>{"Edit"}</button>
                                <button class="btn btn-xs btn-danger"    onclick={on_del_beat}>{"Del"}</button>
                            </div>
                        </div>
                    }
                }).collect::<Html>()
            } else {
                html! {}
            };

            html! {
                <div class="arc-card">
                    <div class="arc-header" onclick={toggle}>
                        <div class="arc-title-row">
                            <span class="arc-chevron">{ if is_open { "▼" } else { "▶" } }</span>
                            <span class="arc-title">{ &arc.title }</span>
                            <span class="badge" style={format!("background:{status_color}")}>
                                { &arc.status }
                            </span>
                            { for arc.affected_zones.iter().map(|z| html! {
                                <span class="badge badge-zone">{ z }</span>
                            }) }
                        </div>
                        <div class="arc-synopsis">{ &arc.synopsis }</div>
                    </div>
                    <div class="arc-actions-row">
                        <button class="btn btn-sm btn-secondary" onclick={on_edit_arc}>{"Edit Arc"}</button>
                        <button class="btn btn-sm btn-accent"    onclick={on_new_beat}>{"+ Beat"}</button>
                        <button class="btn btn-sm btn-danger"    onclick={on_del_arc}>{"Delete Arc"}</button>
                        <span class="arc-beat-count">
                            { format!("{} beat{}", arc.beats.len(),
                              if arc.beats.len() == 1 { "" } else { "s" }) }
                        </span>
                    </div>
                    if is_open {
                        <div class="beats-list">{ beats_html }</div>
                    }
                </div>
            }
        }).collect::<Html>()
    };

    let mood = (*story).world_mood.clone();

    html! {
        <div class="section">
            <div class="section-header">
                <div>
                    <h1 class="section-title">{"📖 Story"}</h1>
                    <p class="section-sub">{"Manage story arcs, beats and world mood."}</p>
                </div>
                <button class="btn btn-primary" onclick={on_new_arc}>{"+ New Arc"}</button>
            </div>

            if let Some(msg) = (*toast).clone() {
                <div class="toast">{ msg }</div>
            }

            // World mood
            <div class="mood-bar">
                <label class="mood-label">{"🌍 World Mood"}</label>
                <select class="input input-sm" onchange={on_mood}>
                    { for ["calm","tense","war","festive","mysterious","grieving"].iter().map(|&m| html! {
                        <option value={m} selected={mood == m}>{ m }</option>
                    }) }
                </select>
            </div>

            { form_html }
            { arc_list }
        </div>
    }
}
