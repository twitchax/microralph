//! Server-side entrypoint for cargo-leptos development builds.
//!
//! This binary is compiled by `cargo leptos` when developing the UI.
//! The production integration uses `mr ui` from the root binary instead.

#![deny(unused)]
#![deny(clippy::unwrap_used)]
#![deny(clippy::correctness)]
#![deny(clippy::complexity)]
#![deny(clippy::pedantic)]

#[cfg(feature = "ssr")]
#[tokio::main]
async fn main() {
    use std::sync::Arc;

    use axum::Extension;
    use axum::Router;
    use axum::routing::get;
    #[allow(clippy::wildcard_imports)]
    use leptos::prelude::*;
    use leptos_axum::{LeptosRoutes, generate_route_list};
    use mr_ui::app::{App, shell};
    use mr_ui::state::StateService;
    use mr_ui::types::AppState;
    use mr_ui::ws::state_ws_handler;
    use tokio::sync::RwLock;

    let conf = get_configuration(None).expect("failed to load leptos configuration");
    let addr = conf.leptos_options.site_addr;
    let leptos_options = conf.leptos_options;

    // Start the state service for filesystem polling.
    let root = std::env::current_dir().expect("failed to get current directory");
    let service = StateService::new(root);
    let app_state: Arc<RwLock<AppState>> = service.shared();
    let state_tx = service.sender();
    tokio::spawn(service.run());

    let routes = generate_route_list(App);

    let app = Router::new()
        .route("/ws/state", get(state_ws_handler))
        .leptos_routes(&leptos_options, routes, {
            let leptos_options = leptos_options.clone();
            move || shell(leptos_options.clone())
        })
        .fallback(leptos_axum::file_and_error_handler(shell))
        .layer(Extension(app_state))
        .layer(Extension(state_tx))
        .with_state(leptos_options);

    tracing::info!("listening on http://{addr}");
    let listener = tokio::net::TcpListener::bind(&addr)
        .await
        .expect("failed to bind TCP listener");
    axum::serve(listener, app.into_make_service())
        .await
        .expect("server error");
}

#[cfg(not(feature = "ssr"))]
pub fn main() {
    // Client-side main is unused; see lib.rs for hydration.
}
