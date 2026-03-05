//! Log streaming viewer component for worktree run logs.
//!
//! Accessed via `/worktrees/:id/logs`. Connects to `/ws/logs/:id` via
//! WebSocket and renders streaming log output in a terminal-styled container
//! with auto-scroll, pause/resume, and error line highlighting.

// Leptos component functions return `impl IntoView` which is consumed by the framework.
#![allow(clippy::must_use_candidate)]

#[allow(clippy::wildcard_imports)]
use leptos::prelude::*;
use leptos_router::hooks::use_params_map;
use thaw::{Badge, BadgeColor, Button, ButtonSize};

use crate::types::AppState;

// ── Main component ──────────────────────────────────────────────────

/// Log streaming viewer page for a single worktree.
#[component]
pub fn LogViewer() -> impl IntoView {
    let app_state =
        use_context::<RwSignal<Option<AppState>>>().expect("AppState context not provided");

    let params = use_params_map();
    let wt_id = move || params.with(|p| p.get("id").unwrap_or_default().clone());

    let wt_entry = move || {
        app_state.with(|s| {
            let id = wt_id();
            s.as_ref().and_then(|state| {
                state
                    .worktree_state
                    .worktrees
                    .iter()
                    .find(|wt| wt.id == id)
                    .cloned()
            })
        })
    };

    // Log lines accumulator — updated by the WebSocket connection.
    let log_content = RwSignal::new(String::new());
    let is_paused = RwSignal::new(false);
    let is_connected = RwSignal::new(false);

    // Connect to the log WebSocket after hydration (client-only).
    #[cfg(feature = "hydrate")]
    {
        let wt_id_value = wt_id();
        connect_log_ws(wt_id_value, log_content, is_connected);
    }

    let back_href = move || format!("/worktrees/{}", wt_id());

    view! {
        <div class="mr-log-viewer">
            <div class="mr-log-viewer__header">
                <div class="mr-log-viewer__title-row">
                    <a href=back_href class="mr-log-viewer__back">"← Back"</a>
                    <h1 class="mr-log-viewer__title">
                        "Logs: " {move || wt_entry().map_or_else(wt_id, |wt| wt.prd.clone())}
                    </h1>
                    <ConnectionBadge connected=is_connected />
                </div>

                <div class="mr-log-viewer__controls">
                    <PauseButton paused=is_paused />
                    <ClearButton log_content />
                </div>
            </div>

            <LogTerminal content=log_content paused=is_paused />
        </div>
    }
}

// ── Connection badge ────────────────────────────────────────────────

/// Shows WebSocket connection status as a colored badge.
#[component]
fn ConnectionBadge(connected: RwSignal<bool>) -> impl IntoView {
    view! {
        {move || {
            if connected.get() {
                view! {
                    <Badge color=Signal::derive(|| BadgeColor::Success)>
                        "Connected"
                    </Badge>
                }.into_any()
            } else {
                view! {
                    <Badge color=Signal::derive(|| BadgeColor::Danger)>
                        "Disconnected"
                    </Badge>
                }.into_any()
            }
        }}
    }
}

// ── Pause / Clear buttons ───────────────────────────────────────────

/// Toggle button for pausing/resuming auto-scroll.
#[component]
fn PauseButton(paused: RwSignal<bool>) -> impl IntoView {
    let on_click = move |_| {
        paused.update(|p| *p = !*p);
    };

    view! {
        <Button size=Signal::derive(|| ButtonSize::Small) on_click>
            {move || if paused.get() { "▶ Resume" } else { "⏸ Pause" }}
        </Button>
    }
}

/// Button to clear the displayed log content.
#[component]
fn ClearButton(log_content: RwSignal<String>) -> impl IntoView {
    let on_click = move |_| {
        log_content.set(String::new());
    };

    view! {
        <Button size=Signal::derive(|| ButtonSize::Small) on_click>
            "🗑 Clear"
        </Button>
    }
}

// ── Terminal display ────────────────────────────────────────────────

/// Renders log content in a terminal-styled scrollable container.
///
/// When not paused, auto-scrolls to the bottom on each update.
#[component]
fn LogTerminal(content: RwSignal<String>, paused: RwSignal<bool>) -> impl IntoView {
    let container_ref = NodeRef::<leptos::html::Pre>::new();

    // Auto-scroll effect (client-only).
    #[cfg(feature = "hydrate")]
    Effect::new(move |_| {
        // Subscribe to content changes to re-trigger.
        let _ = content.get();

        if !paused.get()
            && let Some(el) = container_ref.get()
        {
            let el: &web_sys::Element = &el;
            el.set_scroll_top(el.scroll_height());
        }
    });

    // Suppress unused warnings on SSR where Effect is not compiled.
    #[cfg(not(feature = "hydrate"))]
    {
        let _ = paused;
        let _ = container_ref;
    }

    view! {
        <pre
            class="mr-log-viewer__terminal"
            node_ref=container_ref
        >
            <code inner_html=move || highlight_errors(&content.get()) />
        </pre>
    }
}

/// Highlights lines containing error patterns in red by wrapping them
/// in `<span>` elements with an error CSS class.
fn highlight_errors(text: &str) -> String {
    let mut result = String::with_capacity(text.len() + text.len() / 10);

    for line in text.split('\n') {
        let is_error = line_is_error(line);

        if is_error {
            result.push_str("<span class=\"mr-log-viewer__line--error\">");
            // Escape HTML in the line to prevent injection.
            push_html_escaped(&mut result, line);
            result.push_str("</span>\n");
        } else {
            push_html_escaped(&mut result, line);
            result.push('\n');
        }
    }

    result
}

/// Checks if a line should be highlighted as an error.
fn line_is_error(line: &str) -> bool {
    let lower = line.to_lowercase();
    lower.contains("error") || lower.contains("panic") || lower.contains("fatal")
}

/// Appends HTML-escaped text to the output string.
fn push_html_escaped(out: &mut String, text: &str) {
    for ch in text.chars() {
        match ch {
            '&' => out.push_str("&amp;"),
            '<' => out.push_str("&lt;"),
            '>' => out.push_str("&gt;"),
            '"' => out.push_str("&quot;"),
            _ => out.push(ch),
        }
    }
}

// ── Client-side WebSocket connection ────────────────────────────────

/// Opens a WebSocket to `/ws/logs/:id` and appends received text to the
/// provided signal. Only compiled for the `hydrate` (WASM) target.
#[cfg(feature = "hydrate")]
fn connect_log_ws(wt_id: String, content: RwSignal<String>, connected: RwSignal<bool>) {
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

        let url = format!("{protocol}://{location}/ws/logs/{wt_id}");

        let Ok(ws) = WebSocket::new(&url) else {
            return;
        };

        // Mark connected on open.
        let onopen = Closure::<dyn Fn()>::new(move || {
            connected.set(true);
        });
        ws.set_onopen(Some(onopen.as_ref().unchecked_ref()));
        onopen.forget();

        // Append received text to the log content signal.
        let onmessage = Closure::<dyn Fn(MessageEvent)>::new(move |event: MessageEvent| {
            if let Some(text) = event.data().as_string() {
                content.update(|c| c.push_str(&text));
            }
        });
        ws.set_onmessage(Some(onmessage.as_ref().unchecked_ref()));
        onmessage.forget();

        let onerror = Closure::<dyn Fn()>::new(move || {
            connected.set(false);
            web_sys::console::warn_1(&"Log WebSocket error".into());
        });
        ws.set_onerror(Some(onerror.as_ref().unchecked_ref()));
        onerror.forget();
    });
}
