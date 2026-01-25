# microralph — Suggest Generation Prompt

## Objective

Analyze the codebase, existing PRDs (especially completed ones), and external research to generate exactly 5 strategic PRD suggestions that balance quick wins with longer-term improvements.

## Context

You are analyzing a project to suggest new PRDs that would improve the codebase, fix technical debt, enhance features, or align with ecosystem best practices.

### Existing PRDs

Below are the existing PRDs in the project:

```
{{existing_prds}}
```

### Recent Completions

Pay special attention to recently completed PRDs—they may reveal patterns, gaps, or follow-up opportunities.

### Codebase Snapshot

Here is a snapshot of the repository structure and key files:

```
{{codebase_snapshot}}
```

## Analysis Strategy

Use a multi-faceted approach to identify opportunities:

1. **Pattern Analysis**: Look across completed PRDs for themes or missing pieces
2. **Technical Debt**: Scan for TODO comments, deprecated patterns, or outdated dependencies
3. **Quick Wins**: Identify low-effort, high-value improvements (e.g., missing flags, better error messages)
4. **Strategic Features**: Consider ecosystem trends and best practices (e.g., telemetry, observability)
5. **Test Coverage**: Identify gaps in testing or CI/CD workflows
6. **Documentation**: Find areas where docs are missing or outdated
7. **External Research**: Consider recent developments in relevant ecosystems (Rust, CLI tools, AI agents)

## Requirements

Generate **exactly 5 suggestions** in the following format:

```
1. [Title] — [One-line description]
   Category: [Quick Win | Strategic | Debt | Testing | Docs]
   Effort: [Low | Medium | High]
   Rationale: [2-3 sentences explaining the value and approach]

2. [Title] — [One-line description]
   ...
```

Each suggestion must:
- Be **actionable and scoped** (suitable for a PRD)
- Include a clear **category** (Quick Win, Strategic, Debt, Testing, or Docs)
- Estimate **effort** realistically
- Provide **rationale** that references specific gaps or opportunities

Balance the suggestions:
- Include at least 1-2 **Quick Wins** (low-hanging fruit)
- Include at least 1-2 **Strategic** features (longer-term value)
- Consider **Debt**, **Testing**, or **Docs** for remaining slots

## Output Format

Return the 5 suggestions in plain text using the format above. Do NOT use markdown headings or extra formatting. Just numbered entries.

## Constraints

- Suggest features that fit the project's scope and principles
- Do not suggest features that duplicate existing PRDs
- Prioritize improvements that align with completed work or stated goals
- Keep suggestions realistic and implementable

---

Generate exactly 5 PRD suggestions now.
