//! App shell layout: sidebar, top bar, and content area.
//!
//! Provides a Sentry-inspired layout with a persistent left sidebar,
//! a top bar with daemon status indicator and theme toggle, and a
//! scrollable main content area.

// Leptos component functions return `impl IntoView` which is consumed by the framework.
#![allow(clippy::must_use_candidate)]

#[allow(clippy::wildcard_imports)]
use leptos::prelude::*;
use thaw::{Badge, BadgeColor, Layout, LayoutHeader, LayoutSider};

use super::sidebar::Sidebar;
use super::theme::ThemeToggle;
use crate::types::AppState;

/// Root app shell wrapping all page content.
///
/// Renders a fixed sidebar, a top header bar with daemon status and theme
/// toggle, and a scrollable content area for the active page.
#[component]
pub fn AppShell(children: Children) -> impl IntoView {
    let selected_nav = RwSignal::new(Some(String::from("/")));

    view! {
        <Layout has_sider=Signal::derive(|| true) class="mr-app-shell">
            <LayoutSider class="mr-sidebar">
                <div class="mr-sidebar__brand">
                    <span class="mr-sidebar__logo">"μr"</span>
                    <span class="mr-sidebar__title">"microralph"</span>
                </div>
                <Sidebar selected=selected_nav />
            </LayoutSider>
            <Layout class="mr-main-layout">
                <LayoutHeader class="mr-topbar">
                    <DaemonStatusIndicator />
                    <ThemeToggle />
                </LayoutHeader>
                <main class="mr-content">
                    {children()}
                </main>
            </Layout>
        </Layout>
    }
}

/// Displays the daemon's online/offline status as a colored badge.
#[component]
fn DaemonStatusIndicator() -> impl IntoView {
    let app_state =
        use_context::<RwSignal<Option<AppState>>>().expect("AppState context not provided");

    let daemon_online = move || {
        app_state.with(|s| {
            s.as_ref()
                .is_some_and(|state| state.worktree_state.daemon.is_some())
        })
    };

    let status_text = move || {
        if daemon_online() {
            "Daemon Online"
        } else {
            "Daemon Offline"
        }
    };

    let badge_color = move || {
        if daemon_online() {
            BadgeColor::Success
        } else {
            BadgeColor::Danger
        }
    };

    view! {
        <div class="mr-daemon-status">
            <Badge color=Signal::derive(badge_color)>
                {status_text}
            </Badge>
        </div>
    }
}
