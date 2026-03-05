//! Overlap risk visualization: table of file-overlap warnings between worktrees.
//!
//! Displays each `OverlapWarning` from `state.yaml` with risk-colored badges,
//! linked worktree IDs, and shared file paths. Accessible at `/overlap`.

// Leptos component functions return `impl IntoView` which is consumed by the framework.
#![allow(clippy::must_use_candidate)]

#[allow(clippy::wildcard_imports)]
use leptos::prelude::*;
use thaw::{
    Badge, BadgeColor, Card, CardHeader, Table, TableBody, TableCell, TableHeader, TableHeaderCell,
    TableRow, Tag, TagSize,
};

use crate::types::{AppState, OverlapRisk, OverlapWarning};

// ── Main component ──────────────────────────────────────────────────

/// Full overlap risk visualization page.
#[component]
pub fn OverlapMatrix() -> impl IntoView {
    let app_state =
        use_context::<RwSignal<Option<AppState>>>().expect("AppState context not provided");

    let warnings = move || {
        app_state.with(|s| {
            s.as_ref().map_or_else(Vec::new, |state| {
                state.worktree_state.overlap_warnings.clone()
            })
        })
    };

    let has_warnings = move || !warnings().is_empty();

    view! {
        <h1>"Overlap Risk"</h1>
        <div class="mr-overlap">
            <SummaryCards app_state />
            <Show
                when=has_warnings
                fallback=|| view! {
                    <Card class="mr-card mr-overlap__empty-card">
                        <div class="mr-card__body">
                            <p class="mr-overlap__empty">"No overlap warnings detected. All worktrees are modifying independent files."</p>
                        </div>
                    </Card>
                }
            >
                <WarningsTable warnings />
            </Show>
        </div>
    }
}

// ── Summary cards ───────────────────────────────────────────────────

/// Summary row showing total warnings and breakdown by risk level.
#[component]
fn SummaryCards(app_state: RwSignal<Option<AppState>>) -> impl IntoView {
    let warnings = move || {
        app_state.with(|s| {
            s.as_ref().map_or_else(Vec::new, |state| {
                state.worktree_state.overlap_warnings.clone()
            })
        })
    };

    let total = move || warnings().len();
    let high = move || {
        warnings()
            .iter()
            .filter(|w| w.risk == OverlapRisk::High)
            .count()
    };
    let medium = move || {
        warnings()
            .iter()
            .filter(|w| w.risk == OverlapRisk::Medium)
            .count()
    };
    let low = move || {
        warnings()
            .iter()
            .filter(|w| w.risk == OverlapRisk::Low)
            .count()
    };

    let has_high = move || high() != 0;
    let has_medium = move || medium() != 0;
    let has_low = move || low() != 0;

    view! {
        <div class="mr-overlap__summary">
            <Card class="mr-card mr-overlap__summary-card">
                <CardHeader>"Warning Summary"</CardHeader>
                <div class="mr-card__body">
                    <div class="mr-overlap__summary-grid">
                        <div class="mr-overlap__summary-item">
                            <span class="mr-overlap__summary-label">"Total"</span>
                            <Badge color=Signal::derive(|| BadgeColor::Informative)>
                                {move || total().to_string()}
                            </Badge>
                        </div>
                        <Show when=has_high fallback=|| ()>
                            <div class="mr-overlap__summary-item">
                                <span class="mr-overlap__summary-label">"High"</span>
                                <Badge color=Signal::derive(|| BadgeColor::Danger)>
                                    {move || high().to_string()}
                                </Badge>
                            </div>
                        </Show>
                        <Show when=has_medium fallback=|| ()>
                            <div class="mr-overlap__summary-item">
                                <span class="mr-overlap__summary-label">"Medium"</span>
                                <Badge color=Signal::derive(|| BadgeColor::Warning)>
                                    {move || medium().to_string()}
                                </Badge>
                            </div>
                        </Show>
                        <Show when=has_low fallback=|| ()>
                            <div class="mr-overlap__summary-item">
                                <span class="mr-overlap__summary-label">"Low"</span>
                                <Badge color=Signal::derive(|| BadgeColor::Informative)>
                                    {move || low().to_string()}
                                </Badge>
                            </div>
                        </Show>
                    </div>
                </div>
            </Card>
        </div>
    }
}

// ── Warnings table ──────────────────────────────────────────────────

/// Table listing each overlap warning with risk, worktrees, and shared files.
#[component]
fn WarningsTable<F>(warnings: F) -> impl IntoView
where
    F: Fn() -> Vec<OverlapWarning> + Send + Sync + 'static,
{
    // Sort warnings by risk: High first, then Medium, then Low.
    let sorted = move || {
        let mut w = warnings();
        w.sort_by_key(|warning| match warning.risk {
            OverlapRisk::High => 0,
            OverlapRisk::Medium => 1,
            OverlapRisk::Low => 2,
        });
        w
    };

    view! {
        <Card class="mr-card mr-overlap__table-card">
            <CardHeader>"Overlap Warnings"</CardHeader>
            <div class="mr-card__body">
                <Table class="mr-overlap-table">
                    <TableHeader>
                        <TableRow>
                            <TableHeaderCell>"Risk"</TableHeaderCell>
                            <TableHeaderCell>"Worktrees"</TableHeaderCell>
                            <TableHeaderCell>"Shared Files"</TableHeaderCell>
                        </TableRow>
                    </TableHeader>
                    <TableBody>
                        <For
                            each=sorted
                            key=|w| format!("{:?}-{:?}", w.worktrees, w.files)
                            let:warning
                        >
                            <WarningRow warning />
                        </For>
                    </TableBody>
                </Table>
            </div>
        </Card>
    }
}

// ── Warning row ─────────────────────────────────────────────────────

/// A single overlap warning row with risk badge, worktree links, and file list.
#[component]
fn WarningRow(warning: OverlapWarning) -> impl IntoView {
    let risk = warning.risk;
    let worktrees = warning.worktrees;
    let files = warning.files;

    let row_class = match risk {
        OverlapRisk::High => "mr-overlap-table__row mr-overlap-table__row--high",
        OverlapRisk::Medium => "mr-overlap-table__row mr-overlap-table__row--medium",
        OverlapRisk::Low => "mr-overlap-table__row mr-overlap-table__row--low",
    };

    view! {
        <TableRow class=row_class>
            <TableCell>
                <RiskBadge risk />
            </TableCell>
            <TableCell>
                <WorktreeLinks worktrees />
            </TableCell>
            <TableCell>
                <FileList files />
            </TableCell>
        </TableRow>
    }
}

// ── Risk badge ──────────────────────────────────────────────────────

/// Renders a color-coded badge for the overlap risk level.
#[component]
fn RiskBadge(risk: OverlapRisk) -> impl IntoView {
    let (color, label) = match risk {
        OverlapRisk::High => (BadgeColor::Danger, "High"),
        OverlapRisk::Medium => (BadgeColor::Warning, "Medium"),
        OverlapRisk::Low => (BadgeColor::Informative, "Low"),
    };

    view! {
        <Badge color=Signal::derive(move || color.clone())>
            {label}
        </Badge>
    }
}

// ── Worktree links ──────────────────────────────────────────────────

/// Renders worktree IDs as clickable links to their detail pages.
#[component]
fn WorktreeLinks(worktrees: Vec<String>) -> impl IntoView {
    let links = worktrees
        .iter()
        .map(|wt_id| {
            let href = format!("/worktrees/{wt_id}");
            let label = wt_id.clone();
            view! {
                <a href=href class="mr-overlap-table__wt-link">
                    <Tag size=Signal::derive(|| TagSize::Small)>
                        {label}
                    </Tag>
                </a>
            }
        })
        .collect_view();

    view! {
        <div class="mr-overlap-table__wt-links">
            {links}
        </div>
    }
}

// ── File list ───────────────────────────────────────────────────────

/// Renders the shared file paths as a compact list.
#[component]
fn FileList(files: Vec<String>) -> impl IntoView {
    let items = files
        .iter()
        .map(|f| {
            view! {
                <li class="mr-overlap-table__file">{f.clone()}</li>
            }
        })
        .collect_view();

    view! {
        <ul class="mr-overlap-table__files">
            {items}
        </ul>
    }
}
