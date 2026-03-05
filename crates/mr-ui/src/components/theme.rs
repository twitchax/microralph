//! Dark/light theme provider and toggle switch.

// Leptos component functions return `impl IntoView` which is consumed by the framework.
#![allow(clippy::must_use_candidate)]

#[allow(clippy::wildcard_imports)]
use leptos::prelude::*;
use thaw::{ConfigProvider, Switch, Theme};

/// Wraps children in a Thaw `ConfigProvider` with dark theme by default.
///
/// Provides a reactive `RwSignal<Theme>` that can be toggled between dark and
/// light themes using the [`ThemeToggle`] component.
#[component]
pub fn ThemeProvider(children: Children) -> impl IntoView {
    let theme = RwSignal::new(Theme::dark());

    view! {
        <ConfigProvider theme=theme>
            {children()}
        </ConfigProvider>
    }
}

/// A switch toggle that flips between dark and light themes.
///
/// Must be placed inside a `ThemeProvider` (or Thaw `ConfigProvider`).
#[component]
pub fn ThemeToggle() -> impl IntoView {
    let theme = Theme::use_rw_theme();
    let is_dark = RwSignal::new(true);

    Effect::new(move |_| {
        let dark = is_dark.get();
        theme.set(if dark { Theme::dark() } else { Theme::light() });
    });

    view! {
        <div class="mr-theme-toggle">
            <span class="mr-theme-toggle__label">
                {move || if is_dark.get() { "🌙" } else { "☀️" }}
            </span>
            <Switch checked=is_dark />
        </div>
    }
}
