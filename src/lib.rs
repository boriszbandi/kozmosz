pub mod app;
mod content;
#[allow(dead_code)] // generated icon palette: not every icon is in use
mod icons;
mod images;
mod layout;
mod media;
mod pages;
mod site;

#[cfg(feature = "ssr")]
pub mod server;

/// WASM entry point. In islands mode only `#[island]` components are hydrated;
/// everything else stays plain server-rendered HTML.
#[cfg(feature = "hydrate")]
#[wasm_bindgen::prelude::wasm_bindgen]
pub fn hydrate() {
    console_error_panic_hook::set_once();
    leptos::mount::hydrate_islands();
}
