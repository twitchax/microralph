---
id: PRD-NNNN
title: "{{title}}"
status: draft                 # draft | active | done | parked
owner: "{{owner}}"
created: {{date}}
updated: {{date}}

principles:
- Principle 1 (guiding constraint or design decision)
- Principle 2

references:
- name: Reference Name
  url: https://example.com/reference

acceptance_tests:
- id: uat-001
  name: Description of what the test verifies
  command: cargo make uat # or specific test command

tasks:
- id: T-001
  title: First task title
  priority: 1
  status: todo
  notes: Additional context, dependencies, or implementation hints.

---

# Summary

{{summary}}

---

# Problem

{{problem}}

---

# Goals

1. Goal 1
2. Goal 2
3. Goal 3

---

# Non-Goals (MVP)

- Non-goal 1
- Non-goal 2

---

# History

(Entries appended by `mr run` will go below this line.)

---
