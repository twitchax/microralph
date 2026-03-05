//! Worktree detail view: status header, event timeline, task progress,
//! modified files list, and merge information.
//!
//! Accessed via `/worktrees/:id` where `:id` is the worktree ID (e.g., `wt-001`).

// Leptos component functions return `impl IntoView` which is consumed by the framework.
#![allow(clippy::must_use_candidate)]

#[allow(clippy::wildcard_imports)]
use leptos::prelude::*;
use leptos_router::hooks::use_params_map;
use thaw::{Badge, BadgeColor, Card, CardHeader, Tag, TagSize};

use crate::components::wt_kickoff::{WtKickoffButton, WtKickoffDialog};
use crate::types::{
    AppState, EventType, TaskSummary, WorktreeEntry, WorktreeEvent, WorktreeStatus,
};

// ── Main component ──────────────────────────────────────────────────

/// Full worktree detail page with all sections.
#[component]
pub fn WorktreeDetail() -> impl IntoView {
    let app_state =
        use_context::<RwSignal<Option<AppState>>>().expect("AppState context not provided");

    let params = use_params_map();
    let wt_id = move || params.with(|p| p.get("id").unwrap_or_default().clone());

    let entry = move || {
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

    let prd_for_entry = move || {
        app_state.with(|s| {
            let id = wt_id();
            s.as_ref().and_then(|state| {
                let wt = state
                    .worktree_state
                    .worktrees
                    .iter()
                    .find(|wt| wt.id == id)?;
                state.prds.iter().find(|p| p.id == wt.prd).cloned()
            })
        })
    };

    // Kickoff dialog state for re-running the PRD.
    let kickoff_prd_id = RwSignal::new(String::new());
    let kickoff_visible = RwSignal::new(false);

    view! {
        {move || {
            if let Some(wt) = entry() {
                let prd = prd_for_entry();
                let prd_id_for_btn = wt.prd.clone();
                view! {
                    <div class="mr-wt-detail">
                        <StatusHeader
                            entry=wt.clone()
                            prd_title=prd.as_ref().map(|p| p.title.clone())
                            prd_id=prd_id_for_btn
                            kickoff_prd_id
                            kickoff_visible
                        />
                        <div class="mr-wt-detail__grid">
                            <EventTimeline events=wt.events.clone() />
                            <TaskProgress tasks=prd.map(|p| p.tasks).unwrap_or_default() />
                        </div>
                        <ModifiedFilesList files=wt.modified_files.clone() />
                        <MergeInfo entry=wt />
                    </div>
                }.into_any()
            } else {
                view! {
                    <div class="mr-wt-detail mr-wt-detail--not-found">
                        <h1>"Worktree Not Found"</h1>
                        <p>"No worktree with ID \""  {wt_id()} "\" exists."</p>
                        <a href="/worktrees" class="mr-wt-detail__back">"← Back to Worktrees"</a>
                    </div>
                }.into_any()
            }
        }}

        <WtKickoffDialog prd_id=kickoff_prd_id visible=kickoff_visible />
    }
}

// ── Status header ───────────────────────────────────────────────────

/// Header showing PRD title, branch, status badge, PID, and kickoff button.
#[component]
fn StatusHeader(
    entry: WorktreeEntry,
    prd_title: Option<String>,
    prd_id: String,
    kickoff_prd_id: RwSignal<String>,
    kickoff_visible: RwSignal<bool>,
) -> impl IntoView {
    let title = prd_title.unwrap_or_else(|| entry.prd.clone());
    let (badge_color, badge_label) = status_color_label(entry.status);
    let pid_text = entry
        .run_pid
        .map_or_else(|| String::from("—"), |pid| pid.to_string());
    let has_log = entry.log_file.is_some();
    let log_href = format!("/worktrees/{}/logs", entry.id);

    view! {
        <div class="mr-wt-detail__header">
            <div class="mr-wt-detail__title-row">
                <h1 class="mr-wt-detail__title">{title}</h1>
                <Badge color=Signal::derive(move || badge_color.clone())>
                    {badge_label}
                </Badge>
                <WtKickoffButton
                    prd_id=prd_id
                    target_prd_id=kickoff_prd_id
                    dialog_visible=kickoff_visible
                />
                {if has_log {
                    view! {
                        <a href=log_href class="mr-wt-detail__log-link">"📋 View Logs"</a>
                    }.into_any()
                } else {
                    view! { <span /> }.into_any()
                }}
            </div>
            <div class="mr-wt-detail__meta">
                <MetaItem label="PRD" value=entry.prd.clone() />
                <MetaItem label="Branch" value=entry.branch.clone() />
                <MetaItem label="PID" value=pid_text />
                <MetaItem label="Created" value=format_timestamp(&entry.created_at) />
            </div>
        </div>
    }
}

/// A small label-value pair in the metadata row.
#[component]
fn MetaItem(label: &'static str, value: String) -> impl IntoView {
    view! {
        <div class="mr-wt-detail__meta-item">
            <span class="mr-wt-detail__meta-label">{label}</span>
            <span class="mr-wt-detail__meta-value">{value}</span>
        </div>
    }
}

// ── Event timeline ──────────────────────────────────────────────────

/// Temporal-style event timeline for a single worktree.
#[component]
fn EventTimeline(events: Vec<WorktreeEvent>) -> impl IntoView {
    // Show events in reverse chronological order (newest first).
    let mut sorted = events;
    sorted.sort_by(|a, b| b.timestamp.cmp(&a.timestamp));

    let body = if sorted.is_empty() {
        view! { <p class="mr-timeline__empty">"No events recorded."</p> }.into_any()
    } else {
        let items = sorted
            .iter()
            .map(|evt| {
                let dot_class = event_dot_class(evt);
                let type_label = evt.event_type.to_string();
                let detail = evt.detail.clone().unwrap_or_default();
                let detail_check = detail.clone();
                let timestamp = format_timestamp(&evt.timestamp);

                view! {
                    <div class="mr-timeline__item">
                        <div class=format!("mr-timeline__dot {dot_class}")></div>
                        <div class="mr-timeline__content">
                            <div class="mr-timeline__header">
                                <span class="mr-timeline__type">{type_label}</span>
                                <span class="mr-timeline__time">{timestamp}</span>
                            </div>
                            <Show
                                when=move || !detail_check.is_empty()
                                fallback=|| ()
                            >
                                <span class="mr-timeline__detail">{detail.clone()}</span>
                            </Show>
                        </div>
                    </div>
                }
            })
            .collect_view();

        view! { <div class="mr-timeline">{items}</div> }.into_any()
    };

    view! {
        <Card class="mr-card mr-wt-detail__card">
            <CardHeader>"Event Timeline"</CardHeader>
            <div class="mr-card__body">
                {body}
            </div>
        </Card>
    }
}

// ── Task progress ───────────────────────────────────────────────────

/// Task list with status indicators derived from the matching PRD.
#[component]
fn TaskProgress(tasks: Vec<TaskSummary>) -> impl IntoView {
    let done_count = tasks.iter().filter(|t| t.status == "done").count();
    let total = tasks.len();
    let progress_text = format!("{done_count}/{total} completed");

    let fill_width = if total == 0 {
        0
    } else {
        done_count * 100 / total
    };

    let body = if tasks.is_empty() {
        view! { <p class="mr-wt-detail__empty">"No tasks found for this PRD."</p> }.into_any()
    } else {
        let task_items = tasks
            .iter()
            .map(|task| {
                let (icon, css) = task_status_indicator(&task.status);
                let label = format!("{}: {}", task.id, task.title);
                let status = task.status.clone();

                view! {
                    <div class=format!("mr-task-list__item {css}")>
                        <span class="mr-task-list__icon">{icon}</span>
                        <span class="mr-task-list__label">{label}</span>
                        <Tag size=Signal::derive(|| TagSize::Small)>
                            {status}
                        </Tag>
                    </div>
                }
            })
            .collect_view();

        view! {
            <div class="mr-task-progress">
                <div class="mr-task-progress__summary">
                    <span class="mr-task-progress__text">{progress_text}</span>
                    <div class="mr-task-progress__bar">
                        <div
                            class="mr-task-progress__fill"
                            style=format!("width: {fill_width}%")
                        ></div>
                    </div>
                </div>
                <div class="mr-task-list">
                    {task_items}
                </div>
            </div>
        }
        .into_any()
    };

    view! {
        <Card class="mr-card mr-wt-detail__card">
            <CardHeader>"Task Progress"</CardHeader>
            <div class="mr-card__body">
                {body}
            </div>
        </Card>
    }
}

// ── Modified files list ─────────────────────────────────────────────

/// List of files modified in this worktree.
#[component]
fn ModifiedFilesList(files: Vec<String>) -> impl IntoView {
    let count = files.len();

    let body = if files.is_empty() {
        view! { <p class="mr-wt-detail__empty">"No modified files detected."</p> }.into_any()
    } else {
        let file_items = files
            .iter()
            .map(|f| {
                view! {
                    <li class="mr-file-list__item">
                        <span class="mr-file-list__icon">"📄"</span>
                        <span class="mr-file-list__path">{f.clone()}</span>
                    </li>
                }
            })
            .collect_view();

        view! {
            <ul class="mr-file-list">
                {file_items}
            </ul>
        }
        .into_any()
    };

    view! {
        <Card class="mr-card mr-wt-detail__card">
            <CardHeader>{format!("Modified Files ({count})")}</CardHeader>
            <div class="mr-card__body">
                {body}
            </div>
        </Card>
    }
}

// ── Merge info ──────────────────────────────────────────────────────

/// Merge information derived from events and worktree metadata.
#[component]
fn MergeInfo(entry: WorktreeEntry) -> impl IntoView {
    let merge_target = entry.merge_target.clone();
    let merge_summary = derive_merge_summary(&entry);

    view! {
        <Card class="mr-card mr-wt-detail__card">
            <CardHeader>"Merge Info"</CardHeader>
            <div class="mr-card__body">
                <div class="mr-merge-info">
                    <div class="mr-merge-info__row">
                        <span class="mr-merge-info__label">"Target Branch"</span>
                        <span class="mr-merge-info__value">{merge_target}</span>
                    </div>
                    <div class="mr-merge-info__row">
                        <span class="mr-merge-info__label">"Status"</span>
                        <span class="mr-merge-info__value">{merge_summary}</span>
                    </div>
                </div>
            </div>
        </Card>
    }
}

// ── Helpers ─────────────────────────────────────────────────────────

/// Maps a `WorktreeStatus` to a `BadgeColor` and display label.
fn status_color_label(status: WorktreeStatus) -> (BadgeColor, &'static str) {
    match status {
        WorktreeStatus::Active => (BadgeColor::Success, "Active"),
        WorktreeStatus::Completed => (BadgeColor::Brand, "Completed"),
        WorktreeStatus::Merging => (BadgeColor::Warning, "Merging"),
        WorktreeStatus::Merged => (BadgeColor::Informative, "Merged"),
        WorktreeStatus::MergeFailed => (BadgeColor::Danger, "Failed"),
        WorktreeStatus::Conflicted => (BadgeColor::Danger, "Conflicted"),
        WorktreeStatus::Abandoned => (BadgeColor::Subtle, "Abandoned"),
    }
}

/// Returns an emoji icon and CSS class for a task status.
fn task_status_indicator(status: &str) -> (&'static str, &'static str) {
    match status {
        "done" => ("✅", "mr-task-list__item--done"),
        "in-progress" => ("🔄", "mr-task-list__item--active"),
        _ => ("⬜", "mr-task-list__item--todo"),
    }
}

/// Returns a CSS class for the timeline dot based on event type.
fn event_dot_class(event: &WorktreeEvent) -> &'static str {
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

/// Derives a human-readable merge summary from the worktree's event history.
fn derive_merge_summary(entry: &WorktreeEntry) -> String {
    let has_conflict_resolution = entry
        .events
        .iter()
        .any(|e| e.event_type == EventType::ConflictResolved);
    let has_merge_completed = entry
        .events
        .iter()
        .any(|e| e.event_type == EventType::MergeCompleted);
    let has_merge_failed = entry
        .events
        .iter()
        .any(|e| e.event_type == EventType::MergeFailed);
    let has_conflicted = entry
        .events
        .iter()
        .any(|e| e.event_type == EventType::Conflicted);

    if has_merge_completed && has_conflict_resolution {
        String::from("Merged (agent-resolved conflicts)")
    } else if has_merge_completed {
        String::from("Merged (fast-forward / clean)")
    } else if has_merge_failed {
        String::from("Merge failed")
    } else if has_conflicted {
        String::from("Conflicts detected — awaiting resolution")
    } else if entry.status == WorktreeStatus::Merging {
        String::from("Merge in progress…")
    } else {
        String::from("Not yet merged")
    }
}

/// Formats an ISO 8601 timestamp for display (trims to date + time).
fn format_timestamp(ts: &str) -> String {
    ts.replace('T', " ").trim_end_matches('Z').to_string()
}
