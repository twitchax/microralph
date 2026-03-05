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
    use axum::Router;
    #[allow(clippy::wildcard_imports)]
    use leptos::prelude::*;
    use leptos_axum::{LeptosRoutes, generate_route_list};
    use mr_ui::app::{App, shell};

    let conf = get_configuration(None).expect("failed to load leptos configuration");
    let addr = conf.leptos_options.site_addr;
    let leptos_options = conf.leptos_options;

    let routes = generate_route_list(App);

    let app = Router::new()
        .leptos_routes(&leptos_options, routes, {
            let leptos_options = leptos_options.clone();
            move || shell(leptos_options.clone())
        })
        .fallback(leptos_axum::file_and_error_handler(shell))
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
