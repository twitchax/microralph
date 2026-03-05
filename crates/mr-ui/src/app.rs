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

/// Renders the dashboard home page.
#[component]
fn HomePage() -> impl IntoView {
    view! {
        <h1>"microralph Dashboard"</h1>
        <p>"Worktree orchestration control UI — coming soon."</p>
    }
}
