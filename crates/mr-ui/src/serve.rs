//! Production server entrypoint for `mr ui`.
//!
//! Provides [`serve_blocking`] which creates a tokio runtime and starts the
//! Axum/Leptos server. Called from the root `mr` binary when `mr ui` is invoked.

use std::net::SocketAddr;
use std::path::PathBuf;
use std::sync::Arc;

use axum::Extension;
use axum::Router;
use axum::routing::get;
#[allow(clippy::wildcard_imports)]
use leptos::prelude::*;
use leptos_axum::{LeptosRoutes, generate_route_list};
use tokio::sync::RwLock;

use crate::app::{App, shell};
use crate::state::StateService;
use crate::types::AppState;
use crate::ws::state_ws_handler;

/// Starts the Axum server with Leptos SSR at the given address.
///
/// This function blocks the current thread until the server exits.
/// It creates its own tokio runtime internally.
///
/// # Errors
///
/// Returns an error if the address cannot be parsed, the tokio runtime
/// cannot be created, or the TCP listener fails to bind.
pub fn serve_blocking(
    host: &str,
    port: u16,
) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    let addr: SocketAddr = format!("{host}:{port}").parse()?;

    let runtime = tokio::runtime::Builder::new_multi_thread()
        .enable_all()
        .build()?;

    runtime.block_on(serve_async(addr))
}

/// Async server implementation.
async fn serve_async(addr: SocketAddr) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    let leptos_options = LeptosOptions::builder()
        .output_name("mr-ui")
        .site_root("target/site")
        .site_pkg_dir("pkg")
        .site_addr(addr)
        .env(Env::PROD)
        .build();

    // Start the state service for filesystem polling.
    let root = find_project_root()?;
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
    let listener = tokio::net::TcpListener::bind(&addr).await?;
    axum::serve(listener, app.into_make_service()).await?;

    Ok(())
}

/// Finds the project root by looking for the `.mr/` directory.
///
/// Starts from the current working directory and walks up to find it.
/// Falls back to the current directory if `.mr/` is not found.
fn find_project_root() -> Result<PathBuf, Box<dyn std::error::Error + Send + Sync>> {
    let cwd = std::env::current_dir()?;
    let mut dir = cwd.as_path();

    loop {
        if dir.join(".mr").is_dir() {
            return Ok(dir.to_path_buf());
        }

        match dir.parent() {
            Some(parent) => dir = parent,
            None => return Ok(cwd),
        }
    }
}
