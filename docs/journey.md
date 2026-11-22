# M3 Development Journey

## Purpose

This is a selective record of important development events for project handover: significant problems, their solutions, and temporary decisions that a future contributor needs to understand.

Update it only when an event adds durable context. Routine progress, successful commands, minor fixes, and session-by-session activity do not need entries. There is no required update frequency, and ending a work session does not require an update.

## When to Add an Entry

- A difficult or recurring problem is understood or resolved, and its cause or solution is not obvious from the code.
- An assumption about Matrix, encryption, persistence, dependencies, or deployment proves incorrect and changes the implementation path.
- A workaround or temporary decision introduces a limitation, follow-up obligation, or condition for reconsideration.
- An incident, recovery exercise, or consequential experiment reveals something important for future development or operation.
- A handover would otherwise lose essential context about an unresolved issue or a major tradeoff.

Describe what happened, why it matters, what was done, and what remains. Keep entries concise and link to commits, issues, tests, or verification reports instead of copying large logs. Distinguish observed facts from hypotheses and proposed fixes from verified solutions. Do not include credentials, private keys, or unnecessary private conversation content.

## Relationship to Other Documents

- [proposal.md](proposal.md) defines the project's goals and principles.
- [design.md](design.md) defines the current technical baseline. Update it when an important decision changes the implementation contract; a journey entry alone does not change that baseline.
- [milestones.md](milestones.md) tracks work and acceptance status. Do not duplicate routine checklist updates here.
- Verification reports hold detailed procedures and results. Link them when relevant rather than using this journal as a test log.

Use this file as the main narrative handover record. Do not create a separate decision document for every small choice. If a substantial decision needs its own document, link it from the relevant entry.

## Entry Format

Add entries in chronological order with stable IDs such as `J001`. Use actual event dates. Omit fields that do not apply; the template is a guide, not a form that must be filled for every event.

```markdown
### J001 — YYYY-MM-DD — Short event title

Status: Open / Resolved / Temporary / Superseded
Related milestone: Mx, if applicable

Context and impact:
What happened, the affected behavior, and why future contributors need to know.

Findings:
Observed cause or evidence; identify anything still uncertain.

Resolution or decision:
What was implemented or temporarily chosen, and the relevant tradeoff.

Verification:
How the result was checked, with links to supporting evidence.

Remaining work / revisit condition:
What remains, when a workaround should be reconsidered, and an owner if assigned.

References:
Relevant commits, issues, files, or reports.
```

For temporary decisions, include a concrete revisit condition or follow-up item. When an important event is resolved or superseded, update its status and link to the resolution; retain the original rationale so a handover can explain the sequence. Add a new entry only when the follow-up is itself significant.

## Events

No development events recorded yet.
