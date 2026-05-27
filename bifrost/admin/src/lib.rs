mod api;
mod types;
pub mod app;

use wasm_bindgen::prelude::*;

/// Entry point called by admin.html:
///   `import init, { run } from '/admin/pkg/bifrost_admin.js';`
///   `await init(); run();`
#[wasm_bindgen]
pub fn run() {
    yew::Renderer::<app::App>::new().render();
}
