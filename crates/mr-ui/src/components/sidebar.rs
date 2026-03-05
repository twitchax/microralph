//! Collapsible sidebar navigation using Thaw `NavDrawer`.

// Leptos component functions return `impl IntoView` which is consumed by the framework.
#![allow(clippy::must_use_candidate)]

#[allow(clippy::wildcard_imports)]
use leptos::prelude::*;
use thaw::NavDrawer;
use thaw::NavItem;

/// Sidebar navigation for the dashboard.
///
/// Uses Thaw's `NavDrawer` with `NavItem` entries for each page.
/// Navigation values correspond to client-side routes.
#[component]
pub fn Sidebar(
    /// The currently selected navigation value (route).
    selected: RwSignal<Option<String>>,
) -> impl IntoView {
    view! {
        <NavDrawer selected_value=selected>
            <NavItem
                icon=icondata_ai::AiDashboardOutlined
                value=Signal::derive(|| String::from("/"))
                href=Signal::derive(|| String::from("/"))
            >
                "Dashboard"
            </NavItem>
            <NavItem
                icon=icondata_ai::AiForkOutlined
                value=Signal::derive(|| String::from("/worktrees"))
                href=Signal::derive(|| String::from("/worktrees"))
            >
                "Worktrees"
            </NavItem>
            <NavItem
                icon=icondata_ai::AiFileOutlined
                value=Signal::derive(|| String::from("/prds"))
                href=Signal::derive(|| String::from("/prds"))
            >
                "PRDs"
            </NavItem>
            <NavItem
                icon=icondata_ai::AiWarningOutlined
                value=Signal::derive(|| String::from("/overlap"))
                href=Signal::derive(|| String::from("/overlap"))
            >
                "Overlap Risk"
            </NavItem>
        </NavDrawer>
    }
}
