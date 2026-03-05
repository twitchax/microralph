//! Dashboard home page components: overview cards, daemon health, and recent events.
//!
//! Renders a Sentry-inspired card grid with worktree status breakdown, daemon
//! health indicators, overlap warning badges, and a recent-events timeline.

// Leptos component functions return `impl IntoView` which is consumed by the framework.
#![allow(clippy::must_use_candidate)]

#[allow(clippy::wildcard_imports)]
use leptos::prelude::*;
use thaw::{Badge, BadgeColor, Card, CardHeader, Tag, TagSize};

use crate::types::{AppState, OverlapRisk, WorktreeEntry, WorktreeEvent, WorktreeStatus};

// ── Dashboard home ──────────────────────────────────────────────────

/// Full dashboard home page with overview cards and recent events.
#[component]
pub fn DashboardHome() -> impl IntoView {
    let app_state =
        use_context::<RwSignal<Option<AppState>>>().expect("AppState context not provided");

    view! {
        <h1>"Dashboard"</h1>
        <div class="mr-dashboard">
            <div class="mr-dashboard__cards">
                <StatusCards app_state />
                <DaemonHealthCard app_state />
                <OverlapWarningsCard app_state />
            </div>
            <RecentEventsTimeline app_state />
        </div>
    }
}

// ── Status cards ────────────────────────────────────────────────────

/// Grid of cards showing worktree counts by status.
#[component]
fn StatusCards(app_state: RwSignal<Option<AppState>>) -> impl IntoView {
    let count_by_status = move |target: WorktreeStatus| {
        app_state.with(|s| {
            s.as_ref().map_or(0, |state| {
                state
                    .worktree_state
                    .worktrees
                    .iter()
                    .filter(|wt| wt.status == target)
                    .count()
            })
        })
    };

    let active = move || count_by_status(WorktreeStatus::Active);
    let completed = move || count_by_status(WorktreeStatus::Completed);
    let merged = move || count_by_status(WorktreeStatus::Merged);
    let failed = move || {
        count_by_status(WorktreeStatus::MergeFailed) + count_by_status(WorktreeStatus::Conflicted)
    };

    let total = move || {
        app_state.with(|s| {
            s.as_ref()
                .map_or(0, |state| state.worktree_state.worktrees.len())
        })
    };

    view! {
        <Card class="mr-card mr-card--status">
            <CardHeader>"Worktrees"</CardHeader>
            <div class="mr-card__body">
                <div class="mr-status-grid">
                    <StatusItem label="Total" count=total color=BadgeColor::Informative />
                    <StatusItem label="Active" count=active color=BadgeColor::Success />
                    <StatusItem label="Completed" count=completed color=BadgeColor::Brand />
                    <StatusItem label="Merged" count=merged color=BadgeColor::Subtle />
                    <StatusItem label="Failed" count=failed color=BadgeColor::Danger />
                </div>
            </div>
        </Card>
    }
}

/// A single status count item with a colored badge.
#[component]
fn StatusItem<F>(
    /// Display label.
    label: &'static str,
    /// Reactive count function.
    count: F,
    /// Badge color for the count.
    color: BadgeColor,
) -> impl IntoView
where
    F: Fn() -> usize + Send + Sync + 'static,
{
    view! {
        <div class="mr-status-item">
            <span class="mr-status-item__label">{label}</span>
            <Badge color=Signal::derive(move || color.clone())>
                {move || count().to_string()}
            </Badge>
        </div>
    }
}

// ── Daemon health card ──────────────────────────────────────────────

/// Card displaying daemon health: PID, start time, last heartbeat, status.
#[component]
fn DaemonHealthCard(app_state: RwSignal<Option<AppState>>) -> impl IntoView {
    let daemon_info = move || {
        app_state.with(|s| {
            s.as_ref()
                .and_then(|state| state.worktree_state.daemon.clone())
        })
    };

    let is_online = move || daemon_info().is_some();

    view! {
        <Card class="mr-card mr-card--daemon">
            <CardHeader>"Daemon Health"</CardHeader>
            <div class="mr-card__body">
                {move || {
                    if let Some(daemon) = daemon_info() {
                        view! {
                            <div class="mr-daemon-detail">
                                <div class="mr-daemon-detail__row">
                                    <span class="mr-daemon-detail__label">"Status"</span>
                                    <Badge color=Signal::derive(move || {
                                        if is_online() { BadgeColor::Success } else { BadgeColor::Danger }
                                    })>
                                        {move || if is_online() { "Online" } else { "Offline" }}
                                    </Badge>
                                </div>
                                <div class="mr-daemon-detail__row">
                                    <span class="mr-daemon-detail__label">"PID"</span>
                                    <span class="mr-daemon-detail__value">{daemon.pid.to_string()}</span>
                                </div>
                                <div class="mr-daemon-detail__row">
                                    <span class="mr-daemon-detail__label">"Started"</span>
                                    <span class="mr-daemon-detail__value">{format_timestamp(&daemon.started_at)}</span>
                                </div>
                                <div class="mr-daemon-detail__row">
                                    <span class="mr-daemon-detail__label">"Heartbeat"</span>
                                    <span class="mr-daemon-detail__value">{format_timestamp(&daemon.last_heartbeat)}</span>
                                </div>
                            </div>
                        }.into_any()
                    } else {
                        view! {
                            <div class="mr-daemon-detail mr-daemon-detail--offline">
                                <Badge color=Signal::derive(|| BadgeColor::Danger)>
                                    "Offline"
                                </Badge>
                                <p class="mr-daemon-detail__hint">"No daemon is running."</p>
                            </div>
                        }.into_any()
                    }
                }}
            </div>
        </Card>
    }
}

// ── Overlap warnings card ───────────────────────────────────────────

/// Card showing overlap warning count with risk-colored tags.
#[component]
fn OverlapWarningsCard(app_state: RwSignal<Option<AppState>>) -> impl IntoView {
    let warnings = move || {
        app_state.with(|s| {
            s.as_ref().map_or_else(Vec::new, |state| {
                state.worktree_state.overlap_warnings.clone()
            })
        })
    };

    let warning_count = move || warnings().len();

    let high_count = move || {
        warnings()
            .iter()
            .filter(|w| w.risk == OverlapRisk::High)
            .count()
    };
    let medium_count = move || {
        warnings()
            .iter()
            .filter(|w| w.risk == OverlapRisk::Medium)
            .count()
    };
    let low_count = move || {
        warnings()
            .iter()
            .filter(|w| w.risk == OverlapRisk::Low)
            .count()
    };

    let has_high = move || high_count() != 0;
    let has_medium = move || medium_count() != 0;
    let has_low = move || low_count() != 0;

    view! {
        <Card class="mr-card mr-card--overlap">
            <CardHeader>"Overlap Warnings"</CardHeader>
            <div class="mr-card__body">
                <div class="mr-overlap-summary">
                    <span class="mr-overlap-summary__total">
                        {move || warning_count().to_string()}
                        " warning(s)"
                    </span>
                    <div class="mr-overlap-summary__badges">
                        <Show when=has_high fallback=|| ()>
                            <Tag size=Signal::derive(|| TagSize::Small) class="mr-tag--danger">
                                {move || format!("{} high", high_count())}
                            </Tag>
                        </Show>
                        <Show when=has_medium fallback=|| ()>
                            <Tag size=Signal::derive(|| TagSize::Small) class="mr-tag--warning">
                                {move || format!("{} medium", medium_count())}
                            </Tag>
                        </Show>
                        <Show when=has_low fallback=|| ()>
                            <Tag size=Signal::derive(|| TagSize::Small) class="mr-tag--info">
                                {move || format!("{} low", low_count())}
                            </Tag>
                        </Show>
                    </div>
                </div>
            </div>
        </Card>
    }
}

// ── Recent events timeline ──────────────────────────────────────────

/// Displays the last 10 events across all worktrees in a timeline layout.
#[component]
fn RecentEventsTimeline(app_state: RwSignal<Option<AppState>>) -> impl IntoView {
    let recent_events = move || {
        app_state.with(|s| {
            let Some(state) = s.as_ref() else {
                return Vec::new();
            };

            let mut events: Vec<(String, String, WorktreeEvent)> = state
                .worktree_state
                .worktrees
                .iter()
                .flat_map(|wt: &WorktreeEntry| {
                    wt.events
                        .iter()
                        .map(move |evt| (wt.id.clone(), wt.prd.clone(), evt.clone()))
                })
                .collect();

            // ISO 8601 timestamps sort correctly as strings (descending = newest first).
            events.sort_by(|a, b| b.2.timestamp.cmp(&a.2.timestamp));
            events.truncate(10);
            events
        })
    };

    view! {
        <div class="mr-timeline-section">
            <h2>"Recent Events"</h2>
            <div class="mr-timeline">
                <Show
                    when=move || !recent_events().is_empty()
                    fallback=|| view! { <p class="mr-timeline__empty">"No events recorded yet."</p> }
                >
                    <For
                        each=recent_events
                        key=|item| format!("{}-{}", item.0, item.2.timestamp)
                        let:item
                    >
                        <TimelineItem wt_id=item.0 prd_id=item.1 event=item.2 />
                    </For>
                </Show>
            </div>
        </div>
    }
}

/// A single event in the timeline.
#[component]
fn TimelineItem(wt_id: String, prd_id: String, event: WorktreeEvent) -> impl IntoView {
    let dot_class = event_dot_class(&event);
    let type_label = event.event_type.to_string();
    let detail_for_check = event.detail.clone().unwrap_or_default();
    let detail_for_render = detail_for_check.clone();
    let timestamp = format_timestamp(&event.timestamp);

    view! {
        <div class="mr-timeline__item">
            <div class=format!("mr-timeline__dot {dot_class}")></div>
            <div class="mr-timeline__content">
                <div class="mr-timeline__header">
                    <span class="mr-timeline__type">{type_label}</span>
                    <span class="mr-timeline__meta">{prd_id} " · " {wt_id}</span>
                </div>
                <Show
                    when=move || !detail_for_check.is_empty()
                    fallback=|| ()
                >
                    <span class="mr-timeline__detail">{detail_for_render.clone()}</span>
                </Show>
                <span class="mr-timeline__time">{timestamp.clone()}</span>
            </div>
        </div>
    }
}

// ── Helpers ─────────────────────────────────────────────────────────

/// Returns a CSS class for the timeline dot based on event type.
fn event_dot_class(event: &WorktreeEvent) -> &'static str {
    use crate::types::EventType;

    match event.event_type {
        EventType::RunCompleted | EventType::MergeCompleted | EventType::ConflictResolved => {
            "mr-timeline__dot--success"
        }
        EventType::RunFailed | EventType::MergeFailed | EventType::Conflicted => {
            "mr-timeline__dot--danger"
        }
        EventType::RunStarted | EventType::MergeStarted | EventType::ConflictResolutionStarted => {
            "mr-timeline__dot--active"
        }
        EventType::Created
        | EventType::TaskStarted
        | EventType::TaskCompleted
        | EventType::StateCommitted
        | EventType::Removed
        | EventType::RecoveryPerformed => "mr-timeline__dot--neutral",
    }
}

/// Formats an ISO 8601 timestamp for display (trims to date + time).
fn format_timestamp(ts: &str) -> String {
    // Convert "2026-03-04T22:00:00Z" → "2026-03-04 22:00:00"
    ts.replace('T', " ").trim_end_matches('Z').to_string()
}
