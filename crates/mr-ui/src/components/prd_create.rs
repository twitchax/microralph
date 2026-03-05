//! PRD creation form: gather context and invoke `mr new`.
//!
//! Provides a form with fields for slug, upfront context, runner selection,
//! and optional model override. On submission, invokes `mr new` server-side
//! via `tokio::process::Command` and displays the result.

// Leptos component functions return `impl IntoView` which is consumed by the framework.
#![allow(clippy::must_use_candidate)]

#[allow(clippy::wildcard_imports)]
use leptos::prelude::*;
use thaw::Select;

// ── Server function ─────────────────────────────────────────────────

/// Creates a new PRD by invoking `mr new` as a subprocess on the server.
///
/// The function spawns the `mr` binary with the provided arguments and
/// captures stdout/stderr. Stdin is set to `/dev/null` since the web UI
/// cannot provide interactive input.
#[server]
pub async fn create_prd(
    slug: String,
    context: String,
    runner: String,
    model: String,
) -> Result<String, ServerFnError> {
    use std::process::Stdio;

    let mr_bin = std::env::current_exe()
        .ok()
        .filter(|p| p.file_name().is_some_and(|n| n == "mr"))
        .unwrap_or_else(|| std::path::PathBuf::from("mr"));

    tracing::info!(slug = %slug, runner = %runner, "creating PRD via mr new");

    let mut cmd = tokio::process::Command::new(&mr_bin);
    cmd.arg("new").arg(&slug).arg("--runner").arg(&runner);

    if !context.is_empty() {
        cmd.arg("--context").arg(&context);
    }
    if !model.is_empty() {
        cmd.arg("--model").arg(&model);
    }

    cmd.stdin(Stdio::null())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped());

    let output: std::process::Output = cmd.output().await.map_err(|e| {
        let err: ServerFnError = ServerFnError::ServerError(format!("Failed to spawn mr new: {e}"));
        err
    })?;

    if output.status.success() {
        tracing::info!(slug = %slug, "PRD created successfully");
        Ok(format!("PRD '{slug}' created successfully."))
    } else {
        let stderr = String::from_utf8_lossy(&output.stderr);
        let stdout = String::from_utf8_lossy(&output.stdout);
        tracing::warn!(slug = %slug, exit = %output.status, "mr new failed");
        let err: ServerFnError = ServerFnError::ServerError(format!(
            "mr new exited with {}: {stderr}{stdout}",
            output.status
        ));
        Err(err)
    }
}

// ── Main component ──────────────────────────────────────────────────

/// PRD creation form with slug, context, runner, and model fields.
///
/// On submit, invokes the [`create_prd`] server function and shows a
/// spinner while the subprocess runs. Displays success/error messages
/// and links to navigate back to the PRD list.
#[component]
pub fn PrdCreate() -> impl IntoView {
    let slug = RwSignal::new(String::new());
    let context = RwSignal::new(String::new());
    let runner = RwSignal::new(String::from("copilot"));
    let model = RwSignal::new(String::new());

    let create_action = Action::new(move |(): &()| {
        let s = slug.get_untracked();
        let c = context.get_untracked();
        let r = runner.get_untracked();
        let m = model.get_untracked();
        async move { create_prd(s, c, r, m).await }
    });

    let is_pending = create_action.pending();
    let result_value = create_action.value();

    let slug_valid = move || !slug.get().trim().is_empty();
    let can_submit = move || slug_valid() && !is_pending.get();

    let on_submit = move |ev: leptos::ev::SubmitEvent| {
        ev.prevent_default();
        if can_submit() {
            create_action.dispatch(());
        }
    };

    view! {
        <h1>"Create New PRD"</h1>
        <div class="mr-prd-create">
            <form class="mr-prd-create__form" on:submit=on_submit>
                <div class="mr-prd-create__field">
                    <label class="mr-prd-create__label" for="prd-slug">"Slug"</label>
                    <input
                        class="mr-prd-create__input"
                        type="text"
                        id="prd-slug"
                        placeholder="e.g. add-user-auth"
                        prop:value=move || slug.get()
                        on:input=move |ev| slug.set(event_target_value(&ev))
                        disabled=move || is_pending.get()
                    />
                    <span class="mr-prd-create__hint">"Kebab-case identifier for the PRD file name"</span>
                </div>

                <div class="mr-prd-create__field">
                    <label class="mr-prd-create__label" for="prd-context">"Context"</label>
                    <textarea
                        class="mr-prd-create__textarea"
                        id="prd-context"
                        rows="4"
                        placeholder="Describe what this PRD should cover..."
                        prop:value=move || context.get()
                        on:input=move |ev| context.set(event_target_value(&ev))
                        disabled=move || is_pending.get()
                    ></textarea>
                    <span class="mr-prd-create__hint">"Upfront context to guide the AI agent"</span>
                </div>

                <div class="mr-prd-create__field">
                    <label class="mr-prd-create__label">"Runner"</label>
                    <Select value=runner>
                        <option value="copilot">"Copilot"</option>
                        <option value="claude">"Claude"</option>
                        <option value="codex">"Codex"</option>
                    </Select>
                </div>

                <div class="mr-prd-create__field">
                    <label class="mr-prd-create__label" for="prd-model">"Model (optional)"</label>
                    <input
                        class="mr-prd-create__input"
                        type="text"
                        id="prd-model"
                        placeholder="e.g. claude-sonnet-4.5"
                        prop:value=move || model.get()
                        on:input=move |ev| model.set(event_target_value(&ev))
                        disabled=move || is_pending.get()
                    />
                </div>

                <div class="mr-prd-create__actions">
                    <button
                        type="submit"
                        class="mr-prd-create__submit"
                        disabled=move || !can_submit()
                    >
                        <Show
                            when=move || is_pending.get()
                            fallback=|| view! { "Create PRD" }
                        >
                            <span class="mr-prd-create__spinner"></span>
                            " Creating..."
                        </Show>
                    </button>
                    <a href="/prds" class="mr-prd-create__cancel">"Cancel"</a>
                </div>
            </form>

            <ResultDisplay result_value />
        </div>
    }
}

// ── Result display ──────────────────────────────────────────────────

/// Displays the result of the PRD creation action.
#[component]
fn ResultDisplay(
    result_value: MappedSignal<Option<Result<String, ServerFnError>>>,
) -> impl IntoView {
    move || {
        result_value.get().map(|res| match res {
            Ok(msg) => view! {
                <div class="mr-prd-create__result mr-prd-create__result--success">
                    <p class="mr-prd-create__result-text">"✓ " {msg}</p>
                    <div class="mr-prd-create__result-actions">
                        <a href="/prds" class="mr-prd-create__result-link">"← Back to PRDs"</a>
                    </div>
                </div>
            }
            .into_any(),
            Err(ref e) => {
                let msg = e.to_string();
                view! {
                    <div class="mr-prd-create__result mr-prd-create__result--error">
                        <p class="mr-prd-create__result-text">"✗ " {msg}</p>
                    </div>
                }
                .into_any()
            }
        })
    }
}
