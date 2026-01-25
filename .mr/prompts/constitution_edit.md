# microralph — Constitution Edit Prompt

## Objective

Intelligently update the project constitution based on a natural language request.

## Context

The constitution is a governance file at `.mr/constitution.md` that defines project-specific rules and constraints. The user wants to modify it based on the following request.

## User Request

{{user_request}}

## Current Constitution Content

```markdown
{{constitution_content}}
```

## Q/A History (if any)

{{#each qa_history}}
**Q**: {{question}}
**A**: {{answer}}

{{/each}}

## Required Actions

1. **Understand the request**: Parse the user's natural language request to identify what rule(s) should be added, modified, or removed.
2. **Analyze the constitution**: Review the current constitution structure and content.
3. **Apply changes intelligently**: 
   - Add new rules with appropriate numbering
   - Modify existing rules while preserving intent
   - Remove rules if requested
   - Maintain clear, actionable language
4. **Preserve structure**: Keep the constitution format consistent (Purpose section, Rules section, numbered list).
5. **Be precise**: Rules should be unambiguous and enforceable by an LLM.

## Constraints

- Maintain the basic structure: `# Constitution`, `## Purpose`, `## Rules`
- Rules must be numbered (e.g., `1. **Rule title**: Description`)
- Keep rules concise but complete
- Ensure rules are actionable and verifiable
- Do not add vague or unenforceable rules
- If removing a rule, renumber remaining rules appropriately

## Output Format

If you need more information, respond with a numbered list of questions (1-3 max):
```
1. Question one?
2. Question two?
```

If you have enough information, **directly edit the `.mr/constitution.md` file** using your file editing tools to apply the changes. After making the edits, respond with exactly `EDIT_COMPLETE` on its own line to signal that the changes have been applied.
