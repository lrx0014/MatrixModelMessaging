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

### J001 — 2026-11-23 — Select SDK and SQLx by their shared native SQLite dependency

Status: Resolved for the initial foundation; upgrades require revalidation
Related milestone: M0

Cargo rejected Matrix SDK 0.19.1 with SQLx 0.9.0 because their required
`libsqlite3-sys` ranges do not overlap. Both crates link SQLite into the Runner,
and Cargo correctly permits only one dependency with `links = "sqlite3"`.
Selecting each library's newest release independently is therefore insufficient.

The foundation pins SDK 0.18.0 and SQLx 0.9.0, resolved to one bundled
`libsqlite3-sys 0.35.0`. No upstream fork or native-library workaround is used.
See the [M0 verification report](verification/m0-foundation.md) for the graph,
features, checks, and limitations. Revisit the SDK pin when newer SDK/SQLx
releases have overlapping SQLite requirements; rerun store/FTS checks and all
applicable M1 interoperability/recovery gates before claiming compatibility.


### J002 — 2026-11-23 — Prepare checked SQL across independent application databases

Status: Resolved for the initial schemas
Related milestone: M0

The storage crate contains checked Control and agent queries, while SQLx's macro
compilation uses one `DATABASE_URL`. Application databases must retain separate
migration histories and must never receive SDK migrations.

`cargo xtask sqlx` first exercises the real Control/agent migrators against two
separate temporary databases. It then applies their embedded SQL to a third,
build-only description database and regenerates all checked-query metadata.
That database is never used at runtime; application opens select one migrator.
Generation compares all metadata files, including obsolete descriptions, and the
CI negative test proves drift rejection. The initial table names are distinct.

If future Control and agent schemas need conflicting table definitions, revisit
the preparation boundary (for example, separate checked-query compilation
features) before merging those definitions into one description database. Do not
solve a tooling collision by combining runtime databases or migration histories.
See [M0 evidence](verification/m0-foundation.md) and the README for commands.


### J003 — 2026-11-23 — Serialize compilation on the small Linux builder

Status: Resolved
Related milestone: M0

The Docker builder exposed 12 CPUs but only about 8 GiB memory. A default
parallel release build reached overlapping SDK/application compilation and
failed with `ResourceExhausted: cannot allocate memory`; a concurrent build
request was canceled. Compilation concurrency was unsuitable for this builder.

Docker build stages and CI now set `CARGO_BUILD_JOBS=1`. Both architecture image builds, their runtime smoke checks (amd64 under
emulation), and the complete native Linux arm64 CI verification entry point
pass with that setting. The shared Cargo target cache also uses a locked BuildKit mount so
independent build requests cannot compile into it concurrently.

This is a development/build setting, not a runtime resource or social usage
limit. Keep failed/canceled attempts separate from passing architecture evidence.
Revisit the job count only after measuring the compiler's memory peaks on a
larger builder. See [M0 evidence](verification/m0-foundation.md) for results.

### J004 — 2026-11-23 — Journal Client-Server input before SDK cursor advancement

Status: Resolved for the pinned M1 protocol prototype
Related milestone: M1

SDK 0.18.0 processes crypto/state and saves its sync cursor before returning to
the caller. It also suppresses a response with an already-seen next_batch.
An application callback alone therefore cannot make a separate SQLx inbox
atomic with SDK state: a restart can skip events the application never saved.

The Runner's local transport gateway commits raw timeline events and a batch
ledger before returning sync to the SDK. It backfills limited timelines through
public history pagination without changing the response delivered to the SDK.
The application explicitly supplies its own last committed cursor. Retained
ciphertext is decrypted through SDK room APIs and deduplicated by event ID;
pending-key rows also remain durable before the application checkpoint advances.

The M1 process-exit suite covers journal, SDK, decryption, inbox, and checkpoint
boundaries. The delayed-key fixture retains actual to-device packets and releases
them with a fresh real server batch: inserting packets into an unchanged
next_batch would correctly trip the SDK replay guard and would be an invalid
fault fixture. SDK room-key notifications, rather than the processed sync
to-device list alone, trigger deferred-decryption retries.

Revisit the gateway when the SDK provides a public durable handoff interface
that can replace it. Rerun the full encryption/recovery suite before changing
the boundary. This does not authorize an SDK fork or handwritten crypto.
See [M1 evidence](verification/m1-matrix-e2ee.md).

### J005 — 2026-11-23 — Distinguish request initiation from SAS negotiation

Status: Resolved
Related milestone: M1

The first operator console gated SAS acceptance on the request initiator.
Element could start the SAS phase after M3 requested verification; the console
then left Element waiting because those two initiation roles are different.

The console now invokes the SDK's idempotent SAS acceptance operation, which
negotiates only an appropriate started phase. It never confirms a comparison
or marks a device trusted automatically. Confirmation still follows a real
emoji/number comparison on both endpoints. The shared helper is exercised with
the peer starting SAS after the other client initiated the request.

The human Element test completed the corrected flow, and the verified peer
device status survived reopening the original SDK store. See
[M1 evidence](verification/m1-matrix-e2ee.md) for the exact tested client scope.
