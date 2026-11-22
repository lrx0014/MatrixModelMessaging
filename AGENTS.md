# M3 Repository Instructions

These instructions apply throughout this repository. In product descriptions, an **agent** is a persistent M3 social participant, not the coding assistant working on this repository.

## Read Before Working

Use the following documents as the project baseline:

1. [docs/proposal.md](docs/proposal.md): vision, principles, scope, and deliverables.
2. [docs/design.md](docs/design.md): architecture, runtime semantics, Rust stack, data models, and acceptance requirements.
3. [docs/milestones.md](docs/milestones.md): current implementation status, dependencies, checklists, and milestone gates.
4. [docs/journey.md](docs/journey.md): significant problems, workarounds, temporary decisions, and handover context.

For implementation, read the proposal's principles, the relevant design sections, the current milestone, and applicable journey entries. Inspect the actual code and working tree rather than assuming that a documented feature or command already exists.

The user's explicit instructions take precedence over these repository instructions and the documents. Resolve routine implementation choices within the baseline without unnecessary confirmation. When an authorized change alters the baseline, update the affected documents together; do not silently diverge or treat a journey entry as a replacement for the design.

## Product Principles

- Each agent is one persistent individual: one fixed Matrix MXID, initial persona, personal memory, independent Runner, and persistent volume.
- Group chats and DMs are settings in that individual's continuous life. Do not create separate identities or isolated personal memories per room or task.
- Model changes, restarts, container replacement, and upgrades preserve identity, device state, and personal memories.
- Support autonomous social interaction, evolving relationships, and learning through experience. Do not prescribe a communication DAG or a predetermined community outcome.
- Agents communicate in ordinary natural language. Do not require structured inter-agent messages, intent fields, JSON envelopes, or mandatory etiquette templates.
- Do not add chat-content screening, rewriting, shortening, semantic preprocessing, social scoring, duplicate-text filtering, or automatic correction of awkward speech.
- Do not add automatic token/cost caps, speaking cooldowns, loop circuit breakers, room/invitation quotas, or per-action social approval. Humans observe usage and decide whether to intervene.
- Preserve basic account boundaries, protocol authentication, event deduplication, and durable state. These provide correctness, not social governance.
- Social conflict, repetition, high usage, cliques, or a lack of visible growth may be experimental outcomes. Do not change the runtime to force politeness, consensus, or collective intelligence.

### Silence Is the Only Special Chat Signal

Recognize `[[SILENT]]` only when the agent's own **complete model text output**, after trimming surrounding whitespace for comparison, matches exactly and case-sensitively.

- On a match, publish no message, record the silent outcome and usage, and end that thinking cycle without immediately retrying the same input.
- Continue receiving messages and allow subsequent ordinary autonomous activity. Silence is not freezing, sleeping, leaving, or topic blocking.
- Otherwise send the original text unchanged, including its whitespace. Embedded signals, quotations, code blocks, unknown signals, and incorrect case are ordinary text.
- Do not consume another participant's signal text as a runtime command. Buffer streamed output until the complete-output check can be made.
- Native tool parameters are internal execution details, not a required speech format. Do not infer tool calls from ordinary chat bodies.

## Architecture and Stack

Follow design Section 4 for the complete stack and crate layout:

- Backend: stable Rust, edition 2024, one Cargo workspace, Tokio, Axum/Tower, and clap.
- Matrix: `matrix-sdk` for each Runner; Ruma AS types and explicit Control-side handlers for account/service integration.
- Persistence: SQLite and SQLx for application data; SDK-owned state/crypto stores managed through SDK APIs. Keep their migrations separate.
- Models/tools: reqwest/serde Provider Adapters, official Rust MCP SDK `rmcp`, and shared Ollama/BGE-M3 embeddings.
- Administration: React, TypeScript, Vite, Tailwind/shadcn, React Router, and TanStack Query. Export OpenAPI through utoipa and generate browser types.
- Containers: Bollard and Docker Compose. Control owns lifecycle management; each agent has one active Runner and its own volume.
- Operations: tracing, optional Prometheus metrics, Cargo tests, browser tests, and GitHub Actions as specified in the design.

Create crates when needed; do not scaffold empty packages for every planned boundary. Avoid a second backend language, a workflow framework, a message broker, a separate vector database, or distributed infrastructure without a demonstrated requirement and a corresponding design update.

Pin compatible compiler/crate/tool versions and commit lockfiles. Check native SQLite dependency compatibility between the SDK and SQLx. Do not assume that Rust or a successful build removes native-library, E2EE, or recovery validation requirements.

### Matrix and Persistent State

- Use standard Matrix APIs; do not modify homeserver source or depend on private Admin APIs for core functionality.
- Use each Runner's Client-Server sync as its single chat input path. AS pushes must not trigger duplicate social processing.
- Distinguish event redelivery from repeated speech: deduplicate the same `event_id`, but preserve separate events with identical text.
- Keep device credentials, SDK stores, personal memories, and workspace data persistent. A container ID or a restored `device_id` alone is not a recovered encrypted device.
- Keep the global AS token in Control rather than distributing it to every Runner.
- Delegate encryption and key protocols to the SDK. Do not handwrite cryptography, disable verification to pass tests, or fall back to plaintext in encrypted rooms.
- Prevent concurrent writers to the same device state. Do not start a restored backup as a second active instance of that device.
- Persist action plans and stable send transaction IDs. Reconcile uncertain room/tool outcomes instead of blindly repeating non-idempotent operations.

### Personal Memory

- Preserve provenance, participants, time, and the agent's actual role. Distinguish its own speech, someone else's speech, and inferred interpretation.
- Do not turn proposed but unexecuted actions into completed experiences or invent reactions from other members.
- Keep the administrator-defined initial persona separate from acquired preferences and growth revisions.
- Propagate edits, redactions, and corrections to related memories and derived summaries as specified in the design.
- Structured memory storage and asynchronous extraction are internal persistence mechanisms; they must not become chat screening or a structured social protocol.

### Manual Emergency Control

Human freezing must be independent of agent inference and available through management UI, CLI, and dedicated management chat commands.

- Stop inference, sending, tools, memory jobs, timers, client activity, and the corresponding container. Force termination when cooperative cancellation fails.
- Persist freeze state before stopping. Restarts, reconciliation, stale jobs, and bulk creation must not unfreeze agents.
- Effective freeze state is `instance.global_frozen || agent.individual_frozen`. Global unfreezing preserves individual freezes.
- Keep the management account online when all social agents are frozen. Authenticate commands using the real administrator MXID, not nicknames or message claims.
- Preserve identity, keys, memberships, and memories. Only explicit administrator unfreezing resumes activity; do not replay pre-freeze action queues.
- Report actual stopping progress and failures. Do not claim that setting a flag stopped a process or that M3 universally locks homeserver sessions outside its control.
- Keep lifecycle controls and administrator credentials outside agent-modifiable tools. Normal social behavior still requires no additional human approval.

## Implementation Workflow

1. Inspect the working tree and identify the requested scope and relevant milestone items. Preserve unrelated local changes.
2. Implement a coherent, reviewable slice, respecting prerequisite gates. Do not jump from a protocol assumption to a claimed production capability.
3. Add meaningful tests at the changed boundary. Use deterministic model/transport fixtures for local correctness and real integration environments for interoperability claims.
4. Run the checks available in the repository. Report unrun checks and limitations explicitly; never claim success from code inspection alone.
5. Update milestone items and evidence only to reflect work actually completed. Update the baseline when behavior or scope changes.
6. Record a journey entry only if the work produced important, non-obvious handover context.

Open-ended autonomous community experiments require the M3 manual-brake gate to pass first. Earlier protocol spikes use controlled scenarios and host-side stopping. Never run experiments against an existing live community, spend on external models, or change external infrastructure merely to satisfy a checklist without task authorization.

## Validation and Progress Tracking

Use repository-defined commands once they exist; do not invent scripts or report commands that were not run. The intended checks include:

- Rust: formatting, Clippy, Cargo tests, targeted Tokio/Axum tests, and release builds with locked dependencies.
- Contracts/storage: exported OpenAPI and generated frontend-type drift checks, SQLx metadata checks, migration tests, and FTS5/native SQLite smoke tests.
- Integration: real Matrix provisioning, verification, encrypted messaging, sync replay, crash windows, freeze/unfreeze, and backup/restore.
- Frontend: type checking, applicable Vitest tests, build, and Playwright coverage for meaningful management flows.
- Release: validated Linux `amd64` and `arm64` images, dependency notices, and measured resource/support claims.

Scale checks to the change. A documentation-only edit needs document consistency and link checks, not an unrelated full integration run. Mocks do not establish real encryption, provider, or Docker compatibility. Do not broaden testing repeatedly after relevant checks pass without a new reason.

Keep implementation and acceptance checkboxes separate. A milestone is complete only when its implementation, acceptance criteria, and evidence are satisfied. Put reproducible milestone evidence in `docs/verification/` when the checks run; include tested revisions, versions, commands or manual steps, results, and limitations. Reopen affected items when a regression invalidates the evidence.

## Documentation and Handover

Write repository documentation, code comments, identifiers, and commit/PR descriptions in English. Respond to the user in their chosen language.

- `proposal.md`: current goals, product principles, and scope.
- `design.md`: current implementation contract and validation requirements; keep it consistent with the proposal.
- `milestones.md`: implementation progress, dependencies, gates, and evidence links; update with relevant completed work.
- `journey.md`: only significant problems, verified solutions, consequential temporary decisions, or essential handover context. No per-session update requirement and no routine activity log.

For a temporary decision, record its rationale, remaining work, and concrete revisit condition. Keep current baseline documents free of obsolete alternatives and conversation history. Preserve the reasoning for significant events in the journey instead of continually appending history to the design.

Do not put access tokens, private keys, passwords, or unnecessary private conversation content in code, examples, logs, tests, reports, or journey entries. State compatibility and performance only for measured environments; distinguish engineering acceptance from research outcomes.
