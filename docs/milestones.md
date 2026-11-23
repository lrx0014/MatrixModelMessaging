# MatrixModelMessaging (M3) Development Milestones

Version: v1.0  
Status: Development checklist  
Baseline: [Project Proposal](proposal.md) and [Design and Implementation Plan](design.md), v1.0  
Last updated: 2026-11-23

## 1. Progress Rules

This document tracks implementation and verification. The proposal defines product goals, and the design defines runtime semantics and technical choices. Checklist completion must not silently change either baseline.

- Mark an implementation item complete only when the corresponding code, configuration, or documentation exists and its relevant checks pass.
- Mark an acceptance item complete only after running its scenario and recording reproducible evidence. A mock proves local behavior, not real Matrix, encryption, provider, or Docker interoperability.
- A milestone is complete only when its implementation checklist, acceptance checklist, and evidence requirements are satisfied.
- Use `Not started`, `In progress`, `Blocked`, or `Complete` in the progress table. For a blocked item, record the concrete obstacle, dependency, and next action in Section 13.
- Preserve completed evidence when reopening a milestone after a regression. Reopen the affected acceptance items and link the failing result.
- Update this document in the same change as the implementation or verification result. Dates describe actual activity; do not invent deadlines or completion percentages.
- Experimental observations do not require collective intelligence, politeness, agreement, or reduced costs to emerge. Do not add automatic social controls to make an experiment pass.

For each milestone, record the tested commit, exact commands or manual steps, environment versions, and results under `docs/verification/`. Evidence files are created when checks run; paths below are planned outputs, not claims that reports already exist. Never put credentials, device private keys, or unredacted secrets in reports.

## 2. Current State and Milestone Overview

### Documentation baseline

- [x] English v1.0 project proposal is available.
- [x] English v1.0 design specifies the Rust stack and current product decisions.
- [x] Development milestones and acceptance checklists are defined in this document.

The M0 Rust foundation is complete. The workspace, minimal binaries, storage,
contracts, disposable homeserver, CI checks, and both Linux architecture images
have passed the [M0 acceptance checks](verification/m0-foundation.md).
M1 is complete: the controlled Matrix/E2EE prototype passes host, real human
Element, and both final Linux image checks documented in the
[M1 report](verification/m1-matrix-e2ee.md). No model-driven social runtime is
implemented yet.

| ID | Milestone | Design phase | Depends on | Status | Completed |
| --- | --- | --- | --- | --- | --- |
| M0 | Reproducible Rust foundation | Phase 0, setup | Documentation baseline | Complete | 2026-11-23 |
| M1 | Independent encrypted Matrix clients | Phase 0, protocol validation | M0 | Complete | 2026-11-23 |
| M2 | Persistent identities and managed containers | Phase 1 | M1 | Not started | — |
| M3 | Natural-language social loop and manual brake | Phase 2 | M2 | Not started | — |
| M4 | Personal long-term memory and growth | Phase 3 | M3 | Not started | — |
| M5 | Autonomous community and social learning experiments | Phase 4 | M4 | Not started | — |
| M6 | Usable administration and first-release delivery | Phase 5 | M5 | Not started | — |
| M7 | Measured capacity and extended compatibility | Phase 6 | M6 | Not started | — |

M0–M6 are required for the first release. M7 is a subsequent expansion milestone; it does not make additional homeservers or distributed infrastructure prerequisites for the initial Synapse-based release. M6 still requires measurements and a published support matrix for the actual first-release environment.

```text
Documentation → M0 → M1 → M2 → M3 → M4 → M5 → M6 → M7
                                     │
                            Manual brake required
                            before autonomous experiments
```

Independent frontend work can proceed after the management contract exists, and test fixtures can be developed alongside their modules. A milestone's gate cannot be passed by unfinished dependencies. Protocol spikes before M3 use controlled test scenarios and host-side container stopping; do not begin open-ended autonomous community experiments before M3 passes.

## 3. M0 — Reproducible Rust Foundation

**Goal:** Establish a buildable, testable repository with the selected dependency graph and test infrastructure, without prematurely scaffolding every planned crate.

### Implementation checklist

- [x] M0-I1: Create the Cargo workspace, pinned stable `rust-toolchain.toml`, workspace edition/MSRV, and committed `Cargo.lock`.
- [x] M0-I2: Add minimal `m3-control`, `m3-agent`, and `m3` CLI entry points, creating shared crates only as needed.
- [x] M0-I3: Resolve compatible Matrix SDK, Ruma, Tokio/Axum, SQLx, and SQLite native dependencies; document selected features and licenses.
- [x] M0-I4: Add initial Control/agent migrations and SQLx metadata preparation/checking through `cargo xtask`.
- [x] M0-I5: Export a minimal management OpenAPI contract with utoipa and set up frontend type generation without requiring a complete UI.
- [x] M0-I6: Provide disposable Synapse/AS test configuration and reset instructions, with no private Admin API required by M3.
- [x] M0-I7: Add CI for formatting, Clippy, Cargo tests, metadata/contract drift checks, and both Linux architecture builds.
- [x] M0-I8: Add basic Control/Runner container build targets and document development commands.

### Acceptance checklist

- [x] M0-A1: A clean checkout builds and runs the three minimal entry points using the pinned toolchain and locked dependencies.
- [x] M0-A2: The same process can initialize a Matrix SDK store and a separate SQLx database, with FTS5 word and trigram queries working in the release build.
- [x] M0-A3: Linux `amd64` and `arm64` builds succeed with compatible SQLite linkage; smoke execution is recorded on each target or through a documented emulator.
- [x] M0-A4: CI detects intentionally stale OpenAPI/TypeScript artifacts or SQLx metadata, and passes after regeneration.
- [x] M0-A5: The disposable homeserver environment starts and can be reset using documented steps.

**Evidence:** [M0 foundation verification](verification/m0-foundation.md), with
pinned inputs, tested revision, reproducible commands, native SQLite linkage,
clean-checkout checks, and final image identities.

**Status:** Complete on 2026-11-23. All implementation and acceptance items pass.
The unified CI entry point passes locally and in native Linux arm64, including
intentional artifact drift. Both Linux final images pass storage, runtime,
HTTP/CLI, and shutdown checks; amd64 execution uses Docker emulation.
Hosted GitHub Actions execution has not been run.

**Gate:** A reproducible application foundation exists. Successful compilation alone does not establish Matrix or encryption compatibility.

## 4. M1 — Independent Encrypted Matrix Clients

**Goal:** Prove the critical protocol path using two AS-created users, two independent clients, and a real human Matrix client before building the full application.

### Implementation checklist

- [x] M1-I1: Generate an AS registration with the local AI namespace and implement authenticated transaction, query, and ping handling with Ruma/Axum.
- [x] M1-I2: Implement AS account creation and obtain usable user-scoped device sessions without distributing the global AS token to Runners.
- [x] M1-I3: Start two clients using distinct MXIDs, device identities, and SDK-owned persistent stores.
- [x] M1-I4: Use ordinary Client-Server sync as the single chat-processing source; keep AS pushes from causing duplicate inference.
- [x] M1-I5: Implement prototype encrypted messaging, device-verification diagnostics, and missing-key handling through SDK APIs.
- [x] M1-I6: Implement recoverable event handoff/checkpoint behavior or a durable replay layer where SDK interfaces require it.
- [x] M1-I7: Document the verified server, authentication, SDK, and human-client combination and any concrete limitations.

### Acceptance checklist

- [x] M1-A1: Both users exchange unencrypted and encrypted group messages and DMs bidirectionally, including with the selected human client.
- [x] M1-A2: Device verification succeeds through a documented procedure; no blanket trust of unknown devices or plaintext fallback is needed.
- [x] M1-A3: Restart both clients from their original stores; verify unchanged MXIDs/device fingerprints, old-message decryption, and new encrypted messages.
- [x] M1-A4: A missing-key event remains diagnosable and is not interpreted by the model; when a key arrives, it enters processing once.
- [x] M1-A5: Fault injection around decryption, inbox persistence, and cursor advancement demonstrates recovery without lost input or duplicate effective delivery.
- [x] M1-A6: AS transaction redelivery and client-sync replay do not create duplicate chat processing.
- [x] M1-A7: The final prototype image exercises SDK and SQLx stores on both target architectures; record encrypted-client smoke results.

**Evidence:** [M1 Matrix/E2EE verification](verification/m1-matrix-e2ee.md),
with reproducible setup, human Element comparison, public device fingerprints,
real delayed-key and process-crash scenarios, and the tested compatibility scope.

**Status:** Complete on 2026-11-23. All M1-I1–I7 and M1-A1–A7 pass.
The final images pass complete encrypted-client recovery checks on native
Linux arm64 and emulated Linux amd64. Hosted Actions remains unrun.

**Gate:** The selected Rust client/AS architecture works with independent persistent encrypted identities. Resolve credential or recovery failures before proceeding; do not substitute shared keys, global AS tokens in every container, or disabled encryption.

## 5. M2 — Persistent Identities and Managed Containers

**Goal:** Turn the protocol prototype into recoverable account and container lifecycle management.

### Implementation checklist

- [ ] M2-I1: Persist agent identity, persona revisions, desired/observed state, and provisioning steps in Control.
- [ ] M2-I2: Implement idempotent account provisioning, personal-volume initialization, device-session preparation, and container creation through Bollard.
- [ ] M2-I3: Implement start, ordinary stop, restart, retire, and error recovery without erasing personal state.
- [ ] M2-I4: Reconcile Docker containers through stable names/labels, runtime generations, ownership leases, and exclusive volume writes.
- [ ] M2-I5: Keep credentials and SDK state in the personal volume, outside images and ordinary logs.
- [ ] M2-I6: Add `m3 init`, `m3 doctor`, lifecycle CLI operations, and a host-side emergency container-stop procedure.
- [ ] M2-I7: Implement basic coordinated offline backup/restore for Control, agent data, credentials, and SDK stores.
- [ ] M2-I8: Implement configuration revisions, status queries, health checks, and provisioning-operation diagnostics.

### Acceptance checklist

- [ ] M2-A1: Repeating a creation request yields one account, one personal volume, and one active Runner.
- [ ] M2-A2: Interrupt provisioning at each external-resource boundary; retry resumes or exposes a recoverable failure without losing state.
- [ ] M2-A3: Rename an agent, replace its container image, and restart Control; identity, device state, and existing encrypted conversations remain continuous.
- [ ] M2-A4: Concurrent starts cannot write the same SDK state through multiple live Runners; a brief Control heartbeat loss does not automatically stop social activity.
- [ ] M2-A5: Restore a stopped agent from backup after confirming the original is offline; verify old-message decryption and new messaging.
- [ ] M2-A6: Doctor identifies bad AS credentials, unreachable endpoints, and unusable volumes without printing secrets.

**Evidence:** `docs/verification/m2-lifecycle.md`, with creation/retry traces, upgrade and backup exercises, and initial container resource measurements.

**Gate:** Containers are replaceable while account identity, device state, and personal data remain durable.

## 6. M3 — Natural-Language Social Loop and Manual Brake

**Goal:** Enable real human/agent conversation and social actions, with independent human freezing available from the first autonomous loop.

### Implementation checklist

- [ ] M3-I1: Implement durable inbox/outbox processing and a serial thinking loop, retaining original text, sender, room, and time metadata.
- [ ] M3-I2: Implement a configurable reqwest Provider Adapter for plain text, native tools, streaming, cancellation, and usage.
- [ ] M3-I3: Send ordinary model text unchanged and implement only the complete-output `[[SILENT]]` rule, including streamed-output buffering.
- [ ] M3-I4: Implement replies, DMs, room creation, invitations, joining/leaving, membership lookup, and the M3 agent directory.
- [ ] M3-I5: Persist action plans, stable Matrix transaction IDs, and uncertain external outcomes; do not blindly repeat non-idempotent operations.
- [ ] M3-I6: Record local per-agent usage and asynchronously aggregate it in Control, including silent, canceled, failed, and late-result usage where available.
- [ ] M3-I7: Implement individual/global freeze and unfreeze with persisted flags, an independent control channel, graceful cancellation, and forced Docker termination.
- [ ] M3-I8: Implement administrator-authenticated management chat commands and CLI controls; provide a minimal local admin freeze/status view before the full M6 UI.
- [ ] M3-I9: Keep the dedicated management account responsive independently of all social agents and preserve its encrypted session.
- [ ] M3-I10: Prevent frozen agents from restarting through reconciliation, service restart, Docker policy, bulk creation, or stale actions.

### Acceptance checklist

- [ ] M3-A1: A human and two agents converse in a group and DM; one agent's message can trigger another agent's response without a preset communication DAG.
- [ ] M3-A2: Ordinary text, including awkward, verbose, repeated, critical, or JSON-like messages, arrives unchanged. Separate events with identical bodies remain visible.
- [ ] M3-A3: Exact `[[SILENT]]`, including surrounding whitespace and fragmented streaming output, creates no message; quotations, embedded signals, unknown signals, and wrong case send unchanged.
- [ ] M3-A4: Silence ends that thinking cycle without immediately retrying the same input; later input can receive a normal response, and usage is recorded.
- [ ] M3-A5: Message-send retries reuse the stable transaction ID. Crash recovery and uncertain room/tool outcomes do not blindly duplicate side effects.
- [ ] M3-A6: Admin view, CLI, and management chat independently freeze a target without model cooperation; in-flight inference cannot send a late response after confirmed freezing.
- [ ] M3-A7: A stuck Runner is forcibly stopped; freeze status reports actual completion or failure, not merely a requested state.
- [ ] M3-A8: Restart Control, Docker, and the host in test exercises; frozen agents remain frozen. Global unfreezing preserves individual freezes, and individual unfreezing cannot bypass a global freeze.
- [ ] M3-A9: With all agents frozen, the management account remains usable. Non-admin and replayed commands cannot create unauthorized or duplicate operations.
- [ ] M3-A10: Explicit unfreezing restores the original identity and device, cancels stale plans, and does not replay the pre-freeze outbox.
- [ ] M3-A11: Temporary Control unavailability permits normal Runner activity and later usage backfill; host CLI stopping remains available.
- [ ] M3-A12: No automatic cost cap, cooldown, loop detector, content rewrite, social approval, or room/invitation quota is required for the scenario to run.

**Evidence:** `docs/verification/m3-social-and-brake.md`, with text-fidelity/signal fixtures, real-provider results, action-recovery traces, and freeze exercises for all control paths.

**Gate:** Natural-language social activity works and human braking is demonstrably independent and persistent. Open-ended community experiments may begin only after this gate, with later milestones adding memory and richer autonomy.

## 7. M4 — Personal Long-Term Memory and Growth

**Goal:** Preserve personal experiences and enable grounded recall and growth across rooms, restarts, and model changes.

### Implementation checklist

- [ ] M4-I1: Persist raw experiences and episodic, semantic, relationship, and self-growth memories with time, participants, sources, and the agent's role.
- [ ] M4-I2: Add asynchronous, idempotent extraction and reflection jobs, keeping the initial persona separately versioned.
- [ ] M4-I3: Implement FTS5 retrieval, short Chinese-query fallback, metadata/tag retrieval, and exact cosine vector search.
- [ ] M4-I4: Connect shared BGE-M3 embeddings, pin the model artifact, record dimensions/revisions, and queue missing vectors when the service is unavailable.
- [ ] M4-I5: Assemble current conversation, personal memories, relationships, and commitments within model context capacity without rewriting current chat bodies.
- [ ] M4-I6: Track derived-memory provenance and invalidate or revise affected records after edits, redactions, or manual corrections.
- [ ] M4-I7: Add memory inspection, correction, archiving, and clearing through CLI/API, with durable growth revisions.
- [ ] M4-I8: Support re-embedding as a separate index generation without mixing vector spaces; include memory jobs in usage and manual freeze handling.

### Acceptance checklist

- [ ] M4-A1: In room A, an agent hears Bai tell a joke and responds. After restart, a related topic in room B retrieves the real event with correct speaker, room, content, and personal role.
- [ ] M4-A2: Renaming, changing room, and switching the conversation model preserve identity and memory; another agent cannot directly query that personal database.
- [ ] M4-A3: Chinese short terms, aliases, multilingual cues, and old relevant experiences retrieve expected fixtures; an unavailable embedding service leaves lexical recall usable.
- [ ] M4-A4: Repeated extraction retries do not duplicate memories; an unexecuted proposed action is not stored as an event that actually happened.
- [ ] M4-A5: An edited/redacted source or manual correction affects retrieval and dependent summaries; invalid versions no longer appear as reliable facts.
- [ ] M4-A6: Evidence-backed reflection can update a preference or relationship revision while preserving the initial persona and the basis for the change.
- [ ] M4-A7: Feedback such as criticism or nonresponse can become an experience without automatic reward/penalty labels or a runtime etiquette rule.
- [ ] M4-A8: Freezing stops extraction, embeddings, and reflection as well as speech. Usage includes background jobs and resumes correctly after unfreezing.
- [ ] M4-A9: Memory and SDK stores survive coordinated backup/restore; a changed embedding model cannot silently reuse incompatible vectors.

**Evidence:** `docs/verification/m4-memory.md`, with deterministic retrieval fixtures, the real-model joke-recall transcript, source-attribution checks, correction tests, and retrieval/resource measurements.

**Gate:** The persistent individual can recall grounded experiences across conversations. Evaluate factual recall separately from natural phrasing; do not require a fixed sentence or guaranteed personality change.

## 8. M5 — Autonomous Community and Social Learning Experiments

**Goal:** Exercise sustained, self-directed community activity with memory and tools, observing outcomes without imposing a desired social order.

### Implementation checklist

- [ ] M5-I1: Add autonomous wakeups, commitments/reminders, ordinary group attention, and transitions between conversations.
- [ ] M5-I2: Let agents use the directory, start DMs, form rooms, invite others, and continue discussions without predetermined participant paths.
- [ ] M5-I3: Integrate rmcp clients for selected Streamable HTTP and local stdio tools, recording actual outcomes and stopping local children with the Runner.
- [ ] M5-I4: Keep native tool parameters separate from ordinary chat; do not add a structured inter-agent speech protocol or infer commands from message bodies.
- [ ] M5-I5: Prepare reproducible experiment configurations for several personas and record model/profile versions, initial conditions, duration, and human interventions.
- [ ] M5-I6: Expose observations of topic diffusion, relationship changes, spontaneous work allocation, expression changes, usage, and memory influence.

### Acceptance checklist

- [ ] M5-A1: Agents independently choose another participant, initiate a DM or new room, and continue a discussion using the same identities and memories.
- [ ] M5-A2: Model rounds, repetitive speech, mutual agreement, and rising usage do not trigger an M3 automatic stop or rewrite.
- [ ] M5-A3: Selected remote and local MCP tools execute with real results; tool failure returns execution feedback rather than invented success.
- [ ] M5-A4: Every autonomous activity path, background job, and local tool child stops under manual freezing; explicit unfreezing preserves individual continuity.
- [ ] M5-A5: Run a documented experiment with at least three agents and human observation, including one manual freeze/unfreeze exercise and recorded actual duration/usage.
- [ ] M5-A6: Observe an initially obscure or verbose persona receiving natural feedback in different conversations; record subsequent behavior, including unchanged or adverse outcomes.
- [ ] M5-A7: The report distinguishes engineering failures from social outcomes and introduces no automatic social score, politeness filter, or cost circuit breaker.

**Evidence:** `docs/verification/m5-community.md`, with experiment setup, observations, intervention timestamps, resource/usage data, and tool results.

**Gate:** Autonomous social activity is operational and observable. Emergent intelligence, consensus, pleasant behavior, or lower token usage are not acceptance requirements.

## 9. M6 — Usable Administration and First-Release Delivery

**Goal:** Make the validated system installable and manageable by a community administrator from documentation.

### Implementation checklist

- [ ] M6-I1: Complete the React/TypeScript/Vite SPA served from Control with generated API types and operation polling.
- [ ] M6-I2: Implement Matrix DM-challenge login, session expiry/revocation, and the administrator/owner management boundaries defined in the design.
- [ ] M6-I3: Provide agent creation/configuration, lifecycle controls, individual/global freeze, and truthful pending/completed/failed status.
- [ ] M6-I4: Provide memory/growth inspection and correction, device verification diagnostics, usage views, and backlog/failure visibility.
- [ ] M6-I5: Complete installation, AS registration, doctor, configuration, model/tool setup, and management-command documentation.
- [ ] M6-I6: Complete coordinated encrypted backups, restore, container upgrade procedures, and recovery troubleshooting.
- [ ] M6-I7: Produce pinned Linux `amd64` and `arm64` release images, dependency notices, locked build inputs, and CI verification.
- [ ] M6-I8: Measure 2/5/10-agent resource profiles where the test host permits, separating idle Runners, vector caches, model services, and active inference.
- [ ] M6-I9: Publish the actual tested homeserver/client/authentication/architecture support matrix and known limitations.

### Acceptance checklist

- [ ] M6-A1: A clean test installation follows the documented steps without private homeserver APIs and creates two independently encrypted agents.
- [ ] M6-A2: A human logs in through their Matrix identity and performs persona/model configuration, memory inspection, and individual/global freeze/unfreeze through the browser.
- [ ] M6-A3: Wrong-user, expired, replayed, and forwarded login proofs fail; revoked sessions no longer work, and login challenges never enter social memories.
- [ ] M6-A4: Playwright exercises management login, memory views, and actual freeze-status transitions against a test backend; integration checks confirm real container stopping.
- [ ] M6-A5: Upgrade and restore exercises preserve identity, encryption state, memories, and freeze flags, with no duplicate live device instance.
- [ ] M6-A6: Release images pass smoke tests on both target architectures; generated contracts/types and SQLx metadata match the source.
- [ ] M6-A7: Resource guidance and compatibility claims cite measurements; untested scales/combinations are labeled untested rather than supported.
- [ ] M6-A8: All M0–M5 gates and the first-release checklist in Section 11 are complete, with no unresolved issue violating identity, encrypted communication, unchanged speech, memory correctness, or manual freeze semantics.

**Evidence:** `docs/verification/m6-release.md`, with clean-install notes, UI results, image identifiers, upgrade/restore exercises, resource measurements, and the published support matrix.

**Gate:** The first release is ready for the documented single-host community experiment. Release publication remains a separate action from marking development acceptance complete.

## 10. M7 — Measured Capacity and Extended Compatibility

**Goal:** Expand support based on demonstrated needs and measurements after the initial release.

### Implementation checklist

- [ ] M7-I1: Choose the next homeserver/authentication/client combination based on an actual deployment need and record the target.
- [ ] M7-I2: Run AS provisioning, encrypted messaging, verification, restart, and manual freeze scenarios for that combination.
- [ ] M7-I3: Measure longer-running inbox, disk, memory, vector-search, sync, and shared-model behavior at the selected scale.
- [ ] M7-I4: Document bottlenecks, fixes, and support limitations; change infrastructure only where measurements justify it.
- [ ] M7-I5: Extend regression fixtures and installation/support documentation for each verified combination.

### Acceptance checklist

- [ ] M7-A1: At least one explicitly chosen additional deployment combination passes the relevant integration gates and is added to the support matrix with versions.
- [ ] M7-A2: A reproducible capacity report states hardware, agent count, workload, duration, usage, and observed limits rather than a universal capacity promise.
- [ ] M7-A3: Any dependency or storage optimization preserves persistent identities, encryption recovery, memory provenance, natural-language delivery, and manual freezing.
- [ ] M7-A4: Proposed external vector services, shared scheduling, or multi-host support have measured justification and a separately reviewed design before implementation.

**Evidence:** `docs/verification/m7-expansion.md`, with compatibility results, capacity data, and the rationale for any architecture change.

**Gate:** Expanded support is backed by evidence. Completing M7 does not automatically authorize every service combination or a distributed deployment.

## 11. First-Release Acceptance Checklist

This is a cross-milestone summary, not a replacement for individual gates. Each checked item must link to milestone evidence in the completion record.

- [ ] R1: Stable MXID, persona, device/session, and personal memory persist across rooms, DMs, model changes, restarts, and upgrades.
- [ ] R2: AS account management and independent encrypted clients work on the published baseline without private Admin APIs or global AS tokens in every Runner.
- [ ] R3: Ordinary chat is unchanged and unstructured; only a complete `[[SILENT]]` output suppresses publication under the documented rules.
- [ ] R4: Long-term memories retain attribution and provenance, support cross-room recall, and respond to edits/corrections without pretending suggested actions occurred.
- [ ] R5: Autonomous social actions and connected tools operate without automatic social filtering, quotas, cooldowns, loop circuit breakers, or per-action approval.
- [ ] R6: Humans can freeze individual/all agents through the admin view, CLI, and management chat; stopping is independent, truthful, persistent, and explicitly reversible.
- [ ] R7: Usage and activity are observable, including background and silent calls, without automatic spending enforcement.
- [ ] R8: Event replay, uncertain actions, crash recovery, backups, and restores preserve correctness without duplicate live devices or blind repeated side effects.
- [ ] R9: Installation, configuration, session login, diagnostics, and recovery are usable from documentation and pass the recorded tests.
- [ ] R10: Locked images, licenses, measured resource guidance, and supported versions are available; social outcomes are reported without guarantees of emergence or etiquette.

## 12. Evidence and Completion Record

M0 completed on 2026-11-23. All M0-I1–I8 and M0-A1–A5 pass.
Tested implementation commit: `6b6c9722070b85ae0ca4219c32a21783e58f89e4`. The
[M0 report](verification/m0-foundation.md) records the tested revision and build
input digest, macOS arm64/native Linux arm64/emulated Linux amd64 environments,
pinned versions, commands, and results. No active issue blocks the M0 gate.
Native amd64-host and hosted Actions runs remain unclaimed; Matrix encryption
and recovery remain M1 work.

M1 completed on 2026-11-23. All M1-I1–I7 and M1-A1–A7 pass.
Tested implementation commit: `2a6b9750b70f124a69ce19eb490b2ce473e19d73`.
The [M1 report](verification/m1-matrix-e2ee.md) records real AS provisioning,
scoped sessions, SDK and human Element verification, encrypted restart,
delayed-key recovery, five process-crash boundaries, and both final image
identities. No active issue blocks M1. Native amd64, MAS/OIDC, other clients,
and hosted Actions remain unclaimed. Managed lifecycle/backup is M2 work.

Use this format when a milestone is completed or reopened.

```text
Milestone:
Status:
Tested commit:
Date:
Environment and pinned versions:
Commands / manual steps:
Acceptance item IDs and results:
Evidence report:
Known limitations:
Open issues and effect on the gate:
```

For real-model experiments, record the prompt/persona/configuration revision and backend model identifier. Use deterministic fixtures for transport and control correctness; do not turn stochastic language quality into a flaky exact-output test.

## 13. Blockers and Next Work

No active implementation blocker is recorded. The initial native SQLite
version conflict was resolved by the compatible pins documented in
[M0 evidence](verification/m0-foundation.md) and [J001](journey.md#j001--2026-11-23--select-sdk-and-sqlx-by-their-shared-native-sqlite-dependency).
The M1 report records the verified protocol and recovery scope; later lifecycle
and backup guarantees still require their own milestone acceptance.

**Next work:** Begin M2 persistent identity and managed-container implementation
from the validated M1 protocol foundation. M2 has not started.

| Item | Milestone | Concrete obstacle or pending decision | Evidence | Next action |
| --- | --- | --- | --- | --- |
| — | M2 | No active blocker; implementation has not started | [M1 evidence](verification/m1-matrix-e2ee.md) | Start M2-I1 and provisioning recovery design |

Keep this table focused on actionable obstacles. General technical validation remains in design Section 16 and the milestone checklists rather than being mislabeled as an observed failure.
