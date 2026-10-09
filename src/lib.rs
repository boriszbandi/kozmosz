// Leptos view types nest deeply (the project cards with their readouts exceed the default 128
// in the hydrate build).
#![recursion_limit = "256"]

pub mod app;
pub mod calendar;
mod content;
pub mod drive;
mod gallery;
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
