//! Root application component, router, and HTML shell for SSR.

// Leptos component functions return `impl IntoView` which is consumed by the framework,
// not by callers directly — `#[must_use]` is not applicable.
#![allow(clippy::must_use_candidate)]

#[allow(clippy::wildcard_imports)]
use leptos::prelude::*;
use leptos_meta::{MetaTags, Stylesheet, Title, provide_meta_context};
use leptos_router::{
    StaticSegment,
    components::{Route, Router, Routes},
};

use crate::types::AppState;

/// The HTML shell rendered on the server for SSR with hydration scripts.
pub fn shell(options: LeptosOptions) -> impl IntoView {
    view! {
        <!DOCTYPE html>
        <html lang="en">
            <head>
                <meta charset="utf-8"/>
                <meta name="viewport" content="width=device-width, initial-scale=1"/>
                <AutoReload options=options.clone() />
                <HydrationScripts options/>
                <MetaTags/>
            </head>
            <body>
                <App/>
            </body>
        </html>
    }
}

/// Root application component with router.
#[component]
pub fn App() -> impl IntoView {
    provide_meta_context();

    // Create a reactive signal for real-time state; starts empty on SSR,
    // populated by the WebSocket connection after hydration.
    let app_state = RwSignal::new(Option::<AppState>::None);
    provide_context(app_state);

    // Connect to the WebSocket after hydration (client-only).
    #[cfg(feature = "hydrate")]
    connect_state_ws(app_state);

    view! {
        <Stylesheet id="leptos" href="/pkg/mr-ui.css"/>
        <Title text="microralph — Dashboard"/>

        <Router>
            <main>
                <Routes fallback=|| "Page not found.".into_view()>
                    <Route path=StaticSegment("") view=HomePage/>
                </Routes>
            </main>
        </Router>
    }
}

/// Opens a WebSocket to `/ws/state` and updates the provided signal on each
/// message. Only compiled for the `hydrate` (client-side WASM) target.
#[cfg(feature = "hydrate")]
fn connect_state_ws(state: RwSignal<Option<AppState>>) {
    use wasm_bindgen::JsCast;
    use wasm_bindgen::prelude::*;
    use web_sys::{MessageEvent, WebSocket};

    Effect::new(move |_| {
        let Some(window) = web_sys::window() else {
            return;
        };
        let Ok(location) = window.location().host() else {
            return;
        };
        let protocol = window.location().protocol().map_or_else(
            |_| String::from("ws"),
            |p| {
                if p == "https:" {
                    String::from("wss")
                } else {
                    String::from("ws")
                }
            },
        );

        let url = format!("{protocol}://{location}/ws/state");

        let Ok(ws) = WebSocket::new(&url) else {
            return;
        };

        let onmessage = Closure::<dyn Fn(MessageEvent)>::new(move |event: MessageEvent| {
            if let Some(text) = event.data().as_string() {
                if let Ok(app_state) = serde_json::from_str::<AppState>(&text) {
                    state.set(Some(app_state));
                }
            }
        });
        ws.set_onmessage(Some(onmessage.as_ref().unchecked_ref()));
        onmessage.forget();

        let onerror = Closure::<dyn Fn()>::new(|| {
            web_sys::console::warn_1(&"WebSocket connection error".into());
        });
        ws.set_onerror(Some(onerror.as_ref().unchecked_ref()));
        onerror.forget();
    });
}

/// Renders the dashboard home page with real-time state from the WebSocket.
#[component]
fn HomePage() -> impl IntoView {
    let app_state =
        use_context::<RwSignal<Option<AppState>>>().expect("AppState context not provided");

    let worktree_count = move || {
        app_state.with(|s| {
            s.as_ref()
                .map_or(0, |state| state.worktree_state.worktrees.len())
        })
    };

    let prd_count = move || app_state.with(|s| s.as_ref().map_or(0, |state| state.prds.len()));

    let daemon_status = move || {
        app_state.with(|s| {
            s.as_ref().map_or_else(
                || String::from("unknown"),
                |state| {
                    state
                        .worktree_state
                        .daemon
                        .as_ref()
                        .map_or_else(|| String::from("offline"), |_| String::from("online"))
                },
            )
        })
    };

    view! {
        <h1>"microralph Dashboard"</h1>
        <div class="dashboard-summary">
            <p>"Daemon: " {daemon_status}</p>
            <p>"Worktrees: " {worktree_count}</p>
            <p>"PRDs: " {prd_count}</p>
        </div>
    }
}
