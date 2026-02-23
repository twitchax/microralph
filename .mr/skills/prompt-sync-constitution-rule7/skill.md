# prompt-sync-constitution-rule7

**Summary**: Prompt templates must be defined as constants in `src/commands/init.rs` AND materialized to `.mr/prompts/` — both must stay in sync.

## When to Use

- Adding or modifying any prompt template
- Adding new placeholder variables to prompts
- Updating the run_task, refactor, suggest, or any other prompt

## The Rule

Constitution Rule 7 requires that every prompt exists in two synchronized locations:

1. **Embedded constant** in `src/commands/init.rs` (e.g., `PROMPT_RUN_TASK`, `PROMPT_REFACTOR`)
2. **File on disk** at `.mr/prompts/<name>.md` (materialized during `mr init` / `mr restore`)

### Step-by-Step

1. Edit the constant in `src/commands/init.rs` (the source of truth for defaults)
2. Update the corresponding `.mr/prompts/<name>.md` file to match
3. Verify the constant name is listed in `PROMPT_FILES` array (maps filename → constant)
4. Run `cargo make ci` to ensure tests pass

### The PROMPT_FILES Array

```rust
const PROMPT_FILES: &[(&str, &str)] = &[
    ("run_task.md", PROMPT_RUN_TASK),
    ("refactor.md", PROMPT_REFACTOR),
    // ... each prompt file mapped to its constant
];
```

### Common Mistake

Editing only the `.mr/prompts/` file without updating the constant means `mr restore` will overwrite your changes with the stale constant. Always update both.
