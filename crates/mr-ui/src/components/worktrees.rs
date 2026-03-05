//! Worktree list view: table with real-time status from WebSocket state.
//!
//! Renders a filterable, sortable table of all registered worktrees with
//! color-coded status badges, current task, modified file count, last event,
//! and computed age.

// Leptos component functions return `impl IntoView` which is consumed by the framework.
#![allow(clippy::must_use_candidate)]

#[allow(clippy::wildcard_imports)]
use leptos::prelude::*;
use thaw::{
    Badge, BadgeColor, Select, Table, TableBody, TableCell, TableHeader, TableHeaderCell, TableRow,
    Tag, TagSize,
};

use crate::types::{AppState, EventType, WorktreeEntry, WorktreeStatus};

// ── Sort column ─────────────────────────────────────────────────────

/// Columns available for sorting in the worktree table.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum SortColumn {
    Prd,
    Branch,
    Status,
    ModifiedFiles,
    LastEvent,
    Age,
}

/// Sort direction.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum SortDir {
    Asc,
    Desc,
}

impl SortDir {
    fn toggle(self) -> Self {
        match self {
            Self::Asc => Self::Desc,
            Self::Desc => Self::Asc,
        }
    }

    fn arrow(self) -> &'static str {
        match self {
            Self::Asc => " ↑",
            Self::Desc => " ↓",
        }
    }
}

// ── Main component ──────────────────────────────────────────────────

/// Full worktree list page with status filter and sortable table.
#[component]
pub fn WorktreeList() -> impl IntoView {
    let app_state =
        use_context::<RwSignal<Option<AppState>>>().expect("AppState context not provided");

    let status_filter = RwSignal::new(String::from("all"));
    let sort_col = RwSignal::new(SortColumn::Prd);
    let sort_dir = RwSignal::new(SortDir::Asc);

    let filtered_sorted = move || {
        app_state.with(|s| {
            let Some(state) = s.as_ref() else {
                return Vec::new();
            };
            filter_and_sort(
                &state.worktree_state.worktrees,
                &status_filter.get(),
                sort_col.get(),
                sort_dir.get(),
            )
        })
    };

    let has_entries = move || !filtered_sorted().is_empty();

    view! {
        <h1>"Worktrees"</h1>
        <div class="mr-worktree-list">
            <StatusFilter status_filter />
            <Show
                when=has_entries
                fallback=move || view! {
                    <p class="mr-worktree-list__empty">"No worktrees match the current filter."</p>
                }
            >
                <WorktreeTable filtered_sorted sort_col sort_dir />
            </Show>
        </div>
    }
}

// ── Filter and sort logic ───────────────────────────────────────────

/// Filters worktrees by status and sorts by the given column and direction.
fn filter_and_sort(
    worktrees: &[WorktreeEntry],
    filter_val: &str,
    col: SortColumn,
    dir: SortDir,
) -> Vec<WorktreeEntry> {
    let mut entries: Vec<WorktreeEntry> = worktrees
        .iter()
        .filter(|wt| filter_val == "all" || wt.status.to_string() == filter_val)
        .cloned()
        .collect();

    entries.sort_by(|a, b| {
        let cmp = match col {
            SortColumn::Prd => a.prd.cmp(&b.prd),
            SortColumn::Branch => a.branch.cmp(&b.branch),
            SortColumn::Status => a.status.to_string().cmp(&b.status.to_string()),
            SortColumn::ModifiedFiles => a.modified_files.len().cmp(&b.modified_files.len()),
            SortColumn::LastEvent => {
                let a_ts = a.events.last().map_or("", |e| e.timestamp.as_str());
                let b_ts = b.events.last().map_or("", |e| e.timestamp.as_str());
                a_ts.cmp(b_ts)
            }
            SortColumn::Age => a.created_at.cmp(&b.created_at),
        };

        match dir {
            SortDir::Asc => cmp,
            SortDir::Desc => cmp.reverse(),
        }
    });

    entries
}

// ── Status filter toolbar ───────────────────────────────────────────

/// Toolbar with a status filter dropdown.
#[component]
fn StatusFilter(status_filter: RwSignal<String>) -> impl IntoView {
    view! {
        <div class="mr-worktree-list__toolbar">
            <label class="mr-worktree-list__filter-label">"Filter by status:"</label>
            <Select value=status_filter>
                <option value="all">"All"</option>
                <option value="active">"Active"</option>
                <option value="completed">"Completed"</option>
                <option value="merging">"Merging"</option>
                <option value="merged">"Merged"</option>
                <option value="merge_failed">"Merge Failed"</option>
                <option value="conflicted">"Conflicted"</option>
                <option value="abandoned">"Abandoned"</option>
            </Select>
        </div>
    }
}

// ── Worktree table ──────────────────────────────────────────────────

/// The worktree data table with sortable headers and row entries.
#[component]
fn WorktreeTable<F>(
    filtered_sorted: F,
    sort_col: RwSignal<SortColumn>,
    sort_dir: RwSignal<SortDir>,
) -> impl IntoView
where
    F: Fn() -> Vec<WorktreeEntry> + Send + Sync + 'static,
{
    view! {
        <Table class="mr-wt-table">
            <TableHeader>
                <TableRow>
                    <SortableHeader label="PRD" column=SortColumn::Prd sort_col sort_dir />
                    <SortableHeader label="Branch" column=SortColumn::Branch sort_col sort_dir />
                    <SortableHeader label="Status" column=SortColumn::Status sort_col sort_dir />
                    <TableHeaderCell>"Current Task"</TableHeaderCell>
                    <SortableHeader label="Files" column=SortColumn::ModifiedFiles sort_col sort_dir />
                    <SortableHeader label="Last Event" column=SortColumn::LastEvent sort_col sort_dir />
                    <SortableHeader label="Age" column=SortColumn::Age sort_col sort_dir />
                </TableRow>
            </TableHeader>
            <TableBody>
                <For
                    each=filtered_sorted
                    key=|wt| wt.id.clone()
                    let:wt
                >
                    <WorktreeRow entry=wt />
                </For>
            </TableBody>
        </Table>
    }
}

// ── Sortable header cell ────────────────────────────────────────────

/// A table header cell that toggles sort when clicked.
#[component]
fn SortableHeader(
    label: &'static str,
    column: SortColumn,
    sort_col: RwSignal<SortColumn>,
    sort_dir: RwSignal<SortDir>,
) -> impl IntoView {
    let on_click = move |_| {
        if sort_col.get_untracked() == column {
            sort_dir.update(|d| *d = d.toggle());
        } else {
            sort_col.set(column);
            sort_dir.set(SortDir::Asc);
        }
    };

    let header_text = move || {
        if sort_col.get() == column {
            format!("{label}{}", sort_dir.get().arrow())
        } else {
            label.to_string()
        }
    };

    let is_active = move || sort_col.get() == column;

    view! {
        <TableHeaderCell class=Signal::derive(move || {
            if is_active() {
                Some(String::from("mr-wt-table__header--active"))
            } else {
                Some(String::from("mr-wt-table__header--sortable"))
            }
        })>
            <span
                class="mr-wt-table__sort-trigger"
                on:click=on_click
                role="button"
                tabindex=0
            >
                {header_text}
            </span>
        </TableHeaderCell>
    }
}

// ── Table row ───────────────────────────────────────────────────────

/// A single worktree row with all columns.
#[component]
fn WorktreeRow(entry: WorktreeEntry) -> impl IntoView {
    let status = entry.status;
    let prd_id = entry.prd.clone();
    let branch = entry.branch.clone();
    let current_task = derive_current_task(&entry);
    let file_count = entry.modified_files.len();
    let last_event = entry
        .events
        .last()
        .map(|e| e.event_type.to_string())
        .unwrap_or_default();
    let age = compute_age(&entry.created_at);

    view! {
        <TableRow class="mr-wt-table__row">
            <TableCell>
                <span class="mr-wt-table__prd">{prd_id}</span>
            </TableCell>
            <TableCell>
                <span class="mr-wt-table__branch">{branch}</span>
            </TableCell>
            <TableCell>
                <StatusBadge status />
            </TableCell>
            <TableCell>
                <span class="mr-wt-table__task">{current_task}</span>
            </TableCell>
            <TableCell>
                <Tag size=Signal::derive(|| TagSize::Small) class="mr-wt-table__files-tag">
                    {file_count.to_string()}
                </Tag>
            </TableCell>
            <TableCell>
                <span class="mr-wt-table__event">{last_event}</span>
            </TableCell>
            <TableCell>
                <span class="mr-wt-table__age">{age}</span>
            </TableCell>
        </TableRow>
    }
}

// ── Status badge ────────────────────────────────────────────────────

/// Renders a color-coded Thaw Badge for a worktree status.
#[component]
fn StatusBadge(status: WorktreeStatus) -> impl IntoView {
    let (color, label) = status_color_label(status);

    view! {
        <Badge color=Signal::derive(move || color.clone())>
            {label}
        </Badge>
    }
}

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

// ── Helpers ─────────────────────────────────────────────────────────

/// Derives the current task from the most recent `TaskStarted` event that
/// has not been followed by a `TaskCompleted` event with the same detail.
fn derive_current_task(entry: &WorktreeEntry) -> String {
    let mut started: Option<&str> = None;

    for evt in &entry.events {
        match evt.event_type {
            EventType::TaskStarted => {
                started = evt.detail.as_deref();
            }
            EventType::TaskCompleted => {
                if evt.detail.as_deref() == started {
                    started = None;
                }
            }
            _ => {}
        }
    }

    started.unwrap_or("—").to_string()
}

/// Computes a human-readable age string from an ISO 8601 timestamp.
///
/// This is a simple heuristic: it compares the date portion of the timestamp
/// with a hardcoded "now" approach. Since we run in WASM without `std::time`,
/// we produce a relative label from the timestamp structure.
fn compute_age(created_at: &str) -> String {
    // Parse "YYYY-MM-DDTHH:MM:SSZ" → extract date for a rough age.
    // In a WASM context we don't have reliable system time without js_sys,
    // so we just display the date portion as a compact timestamp.
    if let Some(date_part) = created_at.split('T').next() {
        date_part.to_string()
    } else {
        created_at.to_string()
    }
}
