use yew::prelude::*;

// Section modules — filled in subsequent TODOs
pub mod world;
pub mod biomes;
pub mod story;
pub mod npcs;
pub mod quests;
pub mod loot;
pub mod wac;

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum Section {
    World,
    Biomes,
    Story,
    Npcs,
    Quests,
    Loot,
    Wac,
}

impl Section {
    fn label(self) -> &'static str {
        match self {
            Section::World  => "⚙  World",
            Section::Biomes => "🌿 Biomes",
            Section::Story  => "📖 Story",
            Section::Npcs   => "🧙 NPCs",
            Section::Quests => "📜 Quests",
            Section::Loot   => "💎 Loot",
            Section::Wac    => "⚡ WAC",
        }
    }
}

#[function_component(App)]
pub fn app() -> Html {
    let section = use_state(|| Section::World);

    let nav = {
        let section = section.clone();
        [
            Section::World, Section::Biomes, Section::Story,
            Section::Npcs,  Section::Quests, Section::Loot,
            Section::Wac,
        ]
        .iter()
        .map(|&s| {
            let section = section.clone();
            let active  = *section == s;
            html! {
                <button
                    class={classes!("nav-btn", active.then_some("active"))}
                    onclick={Callback::from(move |_| section.set(s))}
                >
                    { s.label() }
                </button>
            }
        })
        .collect::<Html>()
    };

    let content = match *section {
        Section::World  => html! { <world::WorldSection /> },
        Section::Biomes => html! { <biomes::BiomesSection /> },
        Section::Story  => html! { <story::StorySection /> },
        Section::Npcs   => html! { <npcs::NpcsSection /> },
        Section::Quests => html! { <quests::QuestsSection /> },
        Section::Loot   => html! { <loot::LootSection /> },
        Section::Wac    => html! { <wac::WacSection /> },
    };

    html! {
        <div id="admin-root">
            <aside id="sidebar">
                <div class="sidebar-logo">
                    <span class="logo-icon">{"⚡"}</span>
                    <span class="logo-text">{"NOVA Admin"}</span>
                </div>
                <nav id="nav">{ nav }</nav>
                <div class="sidebar-footer">
                    <a href="/" class="back-link">{"← Back to game"}</a>
                </div>
            </aside>
            <main id="content">
                { content }
            </main>
        </div>
    }
}
