#![deny(unused)]
#![deny(clippy::unwrap_used)]
#![deny(clippy::correctness)]
#![deny(clippy::complexity)]
#![deny(clippy::pedantic)]

//! mr-ui: Local web dashboard for microralph worktree orchestration.
//!
//! Built with Leptos 0.8, Axum 0.8, and Thaw UI 0.5.

pub mod app;

#[cfg(feature = "hydrate")]
#[wasm_bindgen::prelude::wasm_bindgen]
pub fn hydrate() {
    console_error_panic_hook::set_once();
    leptos::mount::hydrate_body(app::App);
}
