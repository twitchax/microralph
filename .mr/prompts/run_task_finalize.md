# microralph — PRD Finalization Prompt

## Objective

Execute the complete finalization workflow for a PRD: verify acceptance tests, clean up temporary artifacts, update documentation, and append finalization history entry.

## Context

You are finalizing `{{prd_id}}`: **{{prd_title}}**.

All tasks in this PRD have been completed. This is the final wrap-up step before marking the PRD as done.

## PRD Summary

{{prd_summary}}

## Completed Tasks

{{completed_tasks}}

{{#if constitution}}
## Project Constitution

The following governance rules and constraints apply to this project:

{{constitution}}

**Note**: Your finalization work (documentation updates, cleanup decisions) should respect these constitutional rules.
{{/if}}

---

## Required Actions

Execute the following steps in order:

### 1. Verify All Acceptance Tests Pass

Run the full test suite to ensure nothing is broken:

```bash
cargo make uat
```

**Criteria**:
- All tests must pass
- No warnings that indicate broken functionality
- If tests fail, stop and report the failure — do not proceed with finalization

### 2. Clean Up Temporary Files and Excessive Comments

Search for and remove:

**Temporary files**:
- Debug scripts or test files created during development
- Temporary data files (`.tmp`, `.bak`, scratch files)
- Generated files that shouldn't be committed

**Excessive comments**:
- TODO comments that are now resolved
- Debug logging statements (e.g., `println!`, `console.log`, `dbg!`)
- Commented-out code that is no longer needed
- Development notes that don't belong in final code

**Do NOT remove**:
- Legitimate TODOs for future work
- Documentation comments
- Necessary inline explanations

### 3. Append Finalization History Entry

Add a final history entry to the PRD file documenting the finalization:

**Format**:
```markdown
## YYYY-MM-DD — PRD Finalized
- **Status**: ✅ Finalized
- **Tasks Completed**: N tasks (T-001 through T-NNN)
- **Outcome**: All tasks completed, acceptance tests passed (XXX/XXX tests)
- **Cleanup**: [Brief note on any cleanup performed]
- **Summary**:
  - [Key accomplishment 1]
  - [Key accomplishment 2]
  - [Key accomplishment 3]
```

### 4. Print Summary to Console

After appending the history entry, print a summary to stdout for the user.

**This is important** - the user should see a clear finalization summary in their terminal.

**Format**:
```
━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━
🎉 Finalization Complete: {{prd_id}} — {{prd_title}}
━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━

✅ UATs: XXX/XXX tests passed
✅ Tasks: N tasks completed (T-001 through T-NNN)
✅ Cleanup: [Summary of cleanup or "None required"]
{{#if commit}}
✅ Commit: [commit_hash] — prd({{prd_id}})finalize: [description]
{{else}}
⏸️ Commit: Skipped (--no-commit flag active) — changes left for manual review
{{/if}}

━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━
```

{{#if commit}}
### 5. Commit All Changes

After all finalization steps are complete, commit the changes.

**CRITICAL**: You MUST commit these files (they've all been updated):
1. `.mr/prds/{{prd_id}}-*.md` — The PRD file with your appended history entry
2. `.mr/PRDS.md` — Auto-regenerated with updated PRD status (already done, just commit it)
3. Any other files you modified during cleanup

**Git commands**:
```bash
git add .mr/prds/{{prd_id}}*.md .mr/PRDS.md
git commit -m "prd({{prd_id}})finalize: [brief description]"
```

If you modified other files during cleanup, add them too:
```bash
git add -A
git commit -m "prd({{prd_id}})finalize: [brief description]"
```

**Commit message format**: `prd({{prd_id}})finalize: [brief description]`

Example: `prd(PRD-0001)finalize: Complete MVP build with finalization workflow`
{{else}}
### 5. Do NOT Commit Changes

**CRITICAL**: Do NOT commit any changes. Leave all modifications staged or unstaged for manual review.

The following files have been updated and should be reviewed before committing:
1. `.mr/prds/{{prd_id}}-*.md` — The PRD file with your appended history entry
2. `.mr/PRDS.md` — Auto-regenerated with updated PRD status
3. Any other files you modified during cleanup

The user will review and commit these changes manually.
{{/if}}

---

## Final Documentation Check

Ensure these documents are up-to-date:

- [ ] **README.md** — Reflects any new features or usage changes
- [ ] **AGENTS.md** — Updated with new conventions or patterns discovered
- [ ] **Inline documentation** — Code comments and docstrings are accurate

---

## Constraints

- **No new features**: This is finalization only — polish and documentation
- **No breaking changes**: The codebase should be in a releasable state
- **Concise entries**: History entries should be brief but complete

---

## Output

After completing all steps, print a structured summary to stdout:

**Format**:
```
━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━
🎉 Finalization Complete: {{prd_id}} — {{prd_title}}
━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━

✅ UATs: XXX/XXX tests passed
✅ Tasks: N tasks completed (T-001 through T-NNN)
✅ Cleanup: [Summary of cleanup or "None required"]
{{#if commit}}
✅ Commit: [commit_hash] — prd({{prd_id}})finalize: [description]
{{else}}
⏸️ Commit: Skipped (--no-commit flag active) — changes left for manual review
{{/if}}

━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━
```
