//! Root application component, router, and HTML shell for SSR.

// Leptos component functions return `impl IntoView` which is consumed by the framework,
// not by callers directly — `#[must_use]` is not applicable.
#![allow(clippy::must_use_candidate)]

#[allow(clippy::wildcard_imports)]
use leptos::prelude::*;
use leptos_meta::{MetaTags, Stylesheet, Title, provide_meta_context};
use leptos_router::{
    ParamSegment, StaticSegment,
    components::{Route, Router, Routes},
};

use crate::components::dashboard::DashboardHome;
use crate::components::layout::AppShell;
use crate::components::log_viewer::LogViewer;
use crate::components::overlap_matrix::OverlapMatrix;
use crate::components::prd_create::PrdCreate;
use crate::components::prd_list::PrdList;
use crate::components::theme::ThemeProvider;
use crate::components::worktree_detail::WorktreeDetail;
use crate::components::worktrees::WorktreeList;
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

        <ThemeProvider>
            <Router>
                <AppShell>
                    <Routes fallback=|| "Page not found.".into_view()>
                        <Route path=StaticSegment("") view=HomePage/>
                        <Route path=StaticSegment("worktrees") view=WorktreesPage/>
                        <Route path=(StaticSegment("worktrees"), ParamSegment("id")) view=WorktreeDetailPage/>
                        <Route path=(StaticSegment("worktrees"), ParamSegment("id"), StaticSegment("logs")) view=LogViewerPage/>
                        <Route path=StaticSegment("overlap") view=OverlapPage/>
                        <Route path=(StaticSegment("prds"), StaticSegment("new")) view=PrdCreatePage/>
                        <Route path=StaticSegment("prds") view=PrdsPage/>
                    </Routes>
                </AppShell>
            </Router>
        </ThemeProvider>
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
            if let Some(text) = event.data().as_string()
                && let Ok(app_state) = serde_json::from_str::<AppState>(&text)
            {
                state.set(Some(app_state));
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

/// Renders the dashboard home page with overview cards and recent events.
#[component]
fn HomePage() -> impl IntoView {
    view! {
        <DashboardHome />
    }
}

/// Worktree list page with real-time status table (T-008).
#[component]
fn WorktreesPage() -> impl IntoView {
    view! {
        <WorktreeList />
    }
}

/// PRD list page with status, dependencies, and task progress (T-010).
#[component]
fn PrdsPage() -> impl IntoView {
    view! {
        <PrdList />
    }
}

/// PRD creation form page (T-011).
#[component]
fn PrdCreatePage() -> impl IntoView {
    view! {
        <PrdCreate />
    }
}

/// Worktree detail page with event timeline, task progress, and files (T-009).
#[component]
fn WorktreeDetailPage() -> impl IntoView {
    view! {
        <WorktreeDetail />
    }
}

/// Overlap risk visualization page (T-014).
#[component]
fn OverlapPage() -> impl IntoView {
    view! {
        <OverlapMatrix />
    }
}

/// Log streaming viewer page for a worktree (T-013).
#[component]
fn LogViewerPage() -> impl IntoView {
    view! {
        <LogViewer />
    }
}
