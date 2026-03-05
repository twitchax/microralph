//! PRD list view: table of all PRDs with status, dependencies, and task progress.
//!
//! Renders a filterable, sortable table of all PRDs parsed from `.mr/prds/`
//! with color-coded status badges, dependency links, task counts, and
//! completion percentages.

// Leptos component functions return `impl IntoView` which is consumed by the framework.
#![allow(clippy::must_use_candidate)]

#[allow(clippy::wildcard_imports)]
use leptos::prelude::*;
use thaw::{
    Badge, BadgeColor, Select, Table, TableBody, TableCell, TableHeader, TableHeaderCell, TableRow,
    Tag, TagSize,
};

use crate::types::{AppState, PrdSummary};

// ── Sort column ─────────────────────────────────────────────────────

/// Columns available for sorting in the PRD table.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum SortColumn {
    Id,
    Title,
    Status,
    Tasks,
    Completion,
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

/// Full PRD list page with status filter and sortable table.
#[component]
pub fn PrdList() -> impl IntoView {
    let app_state =
        use_context::<RwSignal<Option<AppState>>>().expect("AppState context not provided");

    let status_filter = RwSignal::new(String::from("all"));
    let sort_col = RwSignal::new(SortColumn::Id);
    let sort_dir = RwSignal::new(SortDir::Asc);

    let filtered_sorted = move || {
        app_state.with(|s| {
            let Some(state) = s.as_ref() else {
                return Vec::new();
            };
            filter_and_sort(
                &state.prds,
                &status_filter.get(),
                sort_col.get(),
                sort_dir.get(),
            )
        })
    };

    let has_entries = move || !filtered_sorted().is_empty();

    view! {
        <h1>"PRDs"</h1>
        <div class="mr-prd-list">
            <div class="mr-prd-list__toolbar">
                <PrdStatusFilter status_filter />
                <a href="/prds/new" class="mr-prd-list__new-link">
                    "+ New PRD"
                </a>
            </div>
            <Show
                when=has_entries
                fallback=move || view! {
                    <p class="mr-prd-list__empty">"No PRDs match the current filter."</p>
                }
            >
                <PrdTable filtered_sorted sort_col sort_dir />
            </Show>
        </div>
    }
}

// ── Filter and sort logic ───────────────────────────────────────────

/// Filters PRDs by status and sorts by the given column and direction.
fn filter_and_sort(
    prds: &[PrdSummary],
    filter_val: &str,
    col: SortColumn,
    dir: SortDir,
) -> Vec<PrdSummary> {
    let mut entries: Vec<PrdSummary> = prds
        .iter()
        .filter(|p| filter_val == "all" || p.status == filter_val)
        .cloned()
        .collect();

    entries.sort_by(|a, b| {
        let cmp = match col {
            SortColumn::Id => a.id.cmp(&b.id),
            SortColumn::Title => a.title.cmp(&b.title),
            SortColumn::Status => a.status.cmp(&b.status),
            SortColumn::Tasks => a.total_tasks.cmp(&b.total_tasks),
            SortColumn::Completion => {
                let a_pct = completion_pct(a);
                let b_pct = completion_pct(b);
                a_pct.total_cmp(&b_pct)
            }
        };

        match dir {
            SortDir::Asc => cmp,
            SortDir::Desc => cmp.reverse(),
        }
    });

    entries
}

/// Computes the completion percentage for a PRD (0.0–100.0).
fn completion_pct(prd: &PrdSummary) -> f64 {
    if prd.total_tasks == 0 {
        return 0.0;
    }

    #[allow(clippy::cast_precision_loss)]
    let pct = (prd.completed_tasks as f64 / prd.total_tasks as f64) * 100.0;

    pct
}

// ── Status filter toolbar ───────────────────────────────────────────

/// Toolbar with a status filter dropdown.
#[component]
fn PrdStatusFilter(status_filter: RwSignal<String>) -> impl IntoView {
    view! {
        <div class="mr-prd-list__filter">
            <label class="mr-prd-list__filter-label">"Filter by status:"</label>
            <Select value=status_filter>
                <option value="all">"All"</option>
                <option value="draft">"Draft"</option>
                <option value="active">"Active"</option>
                <option value="done">"Done"</option>
                <option value="parked">"Parked"</option>
            </Select>
        </div>
    }
}

// ── PRD table ───────────────────────────────────────────────────────

/// The PRD data table with sortable headers and row entries.
#[component]
fn PrdTable<F>(
    filtered_sorted: F,
    sort_col: RwSignal<SortColumn>,
    sort_dir: RwSignal<SortDir>,
) -> impl IntoView
where
    F: Fn() -> Vec<PrdSummary> + Send + Sync + 'static,
{
    view! {
        <Table class="mr-prd-table">
            <TableHeader>
                <TableRow>
                    <SortableHeader label="ID" column=SortColumn::Id sort_col sort_dir />
                    <SortableHeader label="Title" column=SortColumn::Title sort_col sort_dir />
                    <SortableHeader label="Status" column=SortColumn::Status sort_col sort_dir />
                    <TableHeaderCell>"Dependencies"</TableHeaderCell>
                    <SortableHeader label="Tasks" column=SortColumn::Tasks sort_col sort_dir />
                    <SortableHeader label="Completion" column=SortColumn::Completion sort_col sort_dir />
                </TableRow>
            </TableHeader>
            <TableBody>
                <For
                    each=filtered_sorted
                    key=|p| p.id.clone()
                    let:prd
                >
                    <PrdRow entry=prd />
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
                Some(String::from("mr-prd-table__header--active"))
            } else {
                Some(String::from("mr-prd-table__header--sortable"))
            }
        })>
            <span
                class="mr-prd-table__sort-trigger"
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

/// A single PRD row with all columns.
#[component]
fn PrdRow(entry: PrdSummary) -> impl IntoView {
    let prd_id = entry.id.clone();
    let title = entry.title.clone();
    let status = entry.status.clone();
    let depends_on = entry.depends_on.clone();
    let task_count = format!("{}/{}", entry.completed_tasks, entry.total_tasks);
    let pct = completion_pct(&entry);
    let pct_display = format!("{pct:.0}%");
    let pct_width = format!("{pct:.0}%");

    view! {
        <TableRow class="mr-prd-table__row">
            <TableCell>
                <span class="mr-prd-table__id">{prd_id}</span>
            </TableCell>
            <TableCell>
                <span class="mr-prd-table__title">{title}</span>
            </TableCell>
            <TableCell>
                <PrdStatusBadge status />
            </TableCell>
            <TableCell>
                <DependencyTags deps=depends_on />
            </TableCell>
            <TableCell>
                <Tag size=Signal::derive(|| TagSize::Small) class="mr-prd-table__tasks-tag">
                    {task_count}
                </Tag>
            </TableCell>
            <TableCell>
                <div class="mr-prd-table__completion">
                    <div class="mr-prd-table__completion-bar">
                        <div
                            class="mr-prd-table__completion-fill"
                            style=format!("width: {pct_width}")
                        ></div>
                    </div>
                    <span class="mr-prd-table__completion-text">{pct_display}</span>
                </div>
            </TableCell>
        </TableRow>
    }
}

// ── Status badge ────────────────────────────────────────────────────

/// Renders a color-coded Thaw Badge for a PRD status.
#[component]
fn PrdStatusBadge(status: String) -> impl IntoView {
    let (color, label) = prd_status_color_label(&status);
    let label = label.to_string();

    view! {
        <Badge color=Signal::derive(move || color.clone())>
            {label.clone()}
        </Badge>
    }
}

/// Maps a PRD status string to a `BadgeColor` and display label.
fn prd_status_color_label(status: &str) -> (BadgeColor, &'static str) {
    match status {
        "draft" => (BadgeColor::Subtle, "Draft"),
        "active" => (BadgeColor::Success, "Active"),
        "done" => (BadgeColor::Brand, "Done"),
        "parked" => (BadgeColor::Warning, "Parked"),
        _ => (BadgeColor::Informative, "Unknown"),
    }
}

// ── Dependency tags ─────────────────────────────────────────────────

/// Renders a list of dependency PRD IDs as small tags.
#[component]
fn DependencyTags(deps: Vec<String>) -> impl IntoView {
    if deps.is_empty() {
        return view! {
            <span class="mr-prd-table__no-deps">"—"</span>
        }
        .into_any();
    }

    view! {
        <div class="mr-prd-table__deps">
            {deps.into_iter().map(|dep| {
                view! {
                    <Tag size=Signal::derive(|| TagSize::Small) class="mr-prd-table__dep-tag">
                        {dep}
                    </Tag>
                }
            }).collect::<Vec<_>>()}
        </div>
    }
    .into_any()
}
