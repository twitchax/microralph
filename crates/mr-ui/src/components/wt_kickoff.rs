//! Worktree kickoff: confirmation dialog to trigger `mr wt run` from the UI.
//!
//! Provides a dialog with runner/model selection and a server function that
//! invokes `mr wt run <prd-id>` as a subprocess. Used from both the PRD list
//! page and the worktree detail page.

// Leptos component functions return `impl IntoView` which is consumed by the framework.
#![allow(clippy::must_use_candidate)]

#[allow(clippy::wildcard_imports)]
use leptos::prelude::*;
use thaw::Select;

// ── Server function ─────────────────────────────────────────────────

/// Starts a worktree run by invoking `mr wt run <prd-id>` as a subprocess.
///
/// Stdin is set to `/dev/null` since the web UI cannot provide interactive input.
/// The command is expected to return quickly — it spawns a detached `mr run` process
/// in the new worktree and exits.
#[server]
pub async fn run_worktree(
    prd_id: String,
    runner: String,
    model: String,
) -> Result<String, ServerFnError> {
    use std::process::Stdio;

    let mr_bin = std::env::current_exe()
        .ok()
        .filter(|p| p.file_name().is_some_and(|n| n == "mr"))
        .unwrap_or_else(|| std::path::PathBuf::from("mr"));

    tracing::info!(prd_id = %prd_id, runner = %runner, "starting worktree run via mr wt run");

    let mut cmd = tokio::process::Command::new(&mr_bin);
    cmd.arg("wt")
        .arg("run")
        .arg(&prd_id)
        .arg("--runner")
        .arg(&runner);

    if !model.is_empty() {
        cmd.arg("--model").arg(&model);
    }

    cmd.stdin(Stdio::null())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped());

    let output: std::process::Output = cmd.output().await.map_err(|e| {
        let err: ServerFnError =
            ServerFnError::ServerError(format!("Failed to spawn mr wt run: {e}"));
        err
    })?;

    if output.status.success() {
        tracing::info!(prd_id = %prd_id, "worktree run started successfully");
        Ok(format!("Worktree run started for {prd_id}."))
    } else {
        let stderr = String::from_utf8_lossy(&output.stderr);
        let stdout = String::from_utf8_lossy(&output.stdout);
        tracing::warn!(prd_id = %prd_id, exit = %output.status, "mr wt run failed");
        Err(ServerFnError::ServerError(format!(
            "mr wt run exited with {}: {stderr}{stdout}",
            output.status
        )))
    }
}

// ── Kickoff dialog component ────────────────────────────────────────

/// Inline confirmation dialog for starting a worktree run.
///
/// When `visible` is `true`, renders a modal-like overlay with runner/model
/// selection. On submit, calls [`run_worktree`] and shows the result.
#[component]
pub fn WtKickoffDialog(
    /// PRD ID to start the worktree for.
    prd_id: RwSignal<String>,
    /// Controls visibility of the dialog.
    visible: RwSignal<bool>,
) -> impl IntoView {
    let runner = RwSignal::new(String::from("copilot"));
    let model = RwSignal::new(String::new());

    let kickoff_action = Action::new(move |(): &()| {
        let id = prd_id.get_untracked();
        let r = runner.get_untracked();
        let m = model.get_untracked();
        async move { run_worktree(id, r, m).await }
    });

    let is_pending = kickoff_action.pending();
    let result_value = kickoff_action.value();

    let on_submit = move |ev: leptos::ev::SubmitEvent| {
        ev.prevent_default();
        if !is_pending.get_untracked() {
            kickoff_action.dispatch(());
        }
    };

    let on_cancel = move |_| {
        visible.set(false);
    };

    let on_overlay_click = move |_| {
        if !is_pending.get_untracked() {
            visible.set(false);
        }
    };

    view! {
        <Show when=move || visible.get() fallback=|| ()>
            <div class="mr-kickoff-overlay" on:click=on_overlay_click>
                <div class="mr-kickoff-dialog" on:click=move |ev| ev.stop_propagation()>
                    <h2 class="mr-kickoff-dialog__title">
                        "Start Worktree Run"
                    </h2>
                    <p class="mr-kickoff-dialog__prd">
                        "PRD: "
                        <strong>{move || prd_id.get()}</strong>
                    </p>

                    <form class="mr-kickoff-dialog__form" on:submit=on_submit>
                        <div class="mr-kickoff-dialog__field">
                            <label class="mr-kickoff-dialog__label">"Runner"</label>
                            <Select value=runner>
                                <option value="copilot">"Copilot"</option>
                                <option value="claude">"Claude"</option>
                                <option value="codex">"Codex"</option>
                            </Select>
                        </div>

                        <div class="mr-kickoff-dialog__field">
                            <label class="mr-kickoff-dialog__label" for="kickoff-model">
                                "Model (optional)"
                            </label>
                            <input
                                class="mr-kickoff-dialog__input"
                                type="text"
                                id="kickoff-model"
                                placeholder="e.g. claude-sonnet-4.5"
                                prop:value=move || model.get()
                                on:input=move |ev| model.set(event_target_value(&ev))
                                disabled=move || is_pending.get()
                            />
                        </div>

                        <div class="mr-kickoff-dialog__actions">
                            <button
                                type="submit"
                                class="mr-kickoff-dialog__submit"
                                disabled=move || is_pending.get()
                            >
                                <Show
                                    when=move || is_pending.get()
                                    fallback=|| view! { "🚀 Start Run" }
                                >
                                    <span class="mr-prd-create__spinner"></span>
                                    " Starting..."
                                </Show>
                            </button>
                            <button
                                type="button"
                                class="mr-kickoff-dialog__cancel"
                                on:click=on_cancel
                                disabled=move || is_pending.get()
                            >
                                "Cancel"
                            </button>
                        </div>
                    </form>

                    <KickoffResult result_value visible />
                </div>
            </div>
        </Show>
    }
}

// ── Result display ──────────────────────────────────────────────────

/// Displays the result of the worktree kickoff action.
#[component]
fn KickoffResult(
    result_value: MappedSignal<Option<Result<String, ServerFnError>>>,
    visible: RwSignal<bool>,
) -> impl IntoView {
    move || {
        result_value.get().map(|res| match res {
            Ok(msg) => {
                // Auto-close after success: the worktree list will auto-refresh via WebSocket.
                let close_visible = visible;
                view! {
                    <div class="mr-kickoff-dialog__result mr-kickoff-dialog__result--success">
                        <p class="mr-kickoff-dialog__result-text">"✓ " {msg}</p>
                        <div class="mr-kickoff-dialog__result-actions">
                            <a href="/worktrees" class="mr-kickoff-dialog__result-link">
                                "→ View Worktrees"
                            </a>
                            <button
                                type="button"
                                class="mr-kickoff-dialog__result-close"
                                on:click=move |_| close_visible.set(false)
                            >
                                "Close"
                            </button>
                        </div>
                    </div>
                }
                .into_any()
            }
            Err(ref e) => {
                let msg = e.to_string();
                view! {
                    <div class="mr-kickoff-dialog__result mr-kickoff-dialog__result--error">
                        <p class="mr-kickoff-dialog__result-text">"✗ " {msg}</p>
                    </div>
                }
                .into_any()
            }
        })
    }
}

// ── Inline kickoff button ───────────────────────────────────────────

/// A small action button that opens the kickoff dialog for a given PRD.
///
/// Intended for use in tables and detail headers. Sets the shared `prd_id`
/// signal and toggles the `visible` signal to open the dialog.
#[component]
pub fn WtKickoffButton(
    /// The PRD ID to start a worktree for.
    prd_id: String,
    /// Shared signal for the target PRD ID (updated on click).
    target_prd_id: RwSignal<String>,
    /// Shared visibility signal for the kickoff dialog.
    dialog_visible: RwSignal<bool>,
) -> impl IntoView {
    let title_text = format!("Start worktree for {prd_id}");

    let on_click = move |ev: leptos::ev::MouseEvent| {
        ev.stop_propagation();
        target_prd_id.set(prd_id.clone());
        dialog_visible.set(true);
    };

    view! {
        <button
            class="mr-kickoff-btn"
            title=title_text
            on:click=on_click
        >
            "▶ Run"
        </button>
    }
}
