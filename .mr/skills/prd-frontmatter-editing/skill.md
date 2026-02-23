# prd-frontmatter-editing

**Summary**: Correctly edit PRD YAML frontmatter — quote strings with colons/hashes, preserve structure, and follow status conventions.

## When to Use

- Updating task status in a PRD during `mr run`
- Changing PRD status (draft → active → done)
- Adding new tasks or acceptance tests
- Appending History entries

## YAML Quoting Rules

Strings containing `:` or `#` MUST be quoted:
```yaml
# ✅ Correct
title: "Feature: Add new command"
notes: "See issue #42 for details"

# ❌ Wrong — YAML parser will break
title: Feature: Add new command
notes: See issue #42 for details
```

## Status Values

| Field | Valid Values |
|-------|-------------|
| PRD `status` | `draft`, `active`, `done`, `parked` |
| Task `status` | `todo`, `in-progress`, `done` |
| UAT `uat_status` | `unverified`, `verified`, `skipped` |

## Task Addition

When adding tasks dynamically during execution:
- Use next available `T-XXX` ID
- Set `status: todo` and appropriate `priority`
- Document in History entry why the task was added

## History Entry Format

```markdown
## YYYY-MM-DD — T-XXX Completed
- **Task**: [Task title]
- **Status**: ✅ Done (or ❌ Failed)
- **Changes**:
  - Bullet points describing changes
  - Include file names and key details
  - Note UAT pass/fail with evidence
```

## Regenerating Index

After PRD changes, regenerate the index: `cargo run -- list`
