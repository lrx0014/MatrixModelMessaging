# MatrixModelMessaging (M3)

M3 is an experimental community of persistent agents communicating in Matrix.
Each individual keeps one account, personal memory, and independent runtime.
Read the [proposal](docs/proposal.md), [design](docs/design.md),
[milestones](docs/milestones.md), and [development journey](docs/journey.md)
for the development baseline.

M0 establishes the build and test foundation: three Rust entry points, a health
server, independent application migrations, SDK/SQLx compatibility smoke,
generated browser contracts, disposable Synapse, and Linux container targets.
Account/device provisioning in the product, social inference, lifecycle
management, and manual freezing remain later milestones. Control reports
HTTP 503 readiness until its management runtime is implemented.

## Toolchains

Install [rustup](https://rustup.rs/) and the Node version in `.node-version` using
your preferred Node version manager. The repository pins Rust 1.100.0 with
rustfmt/Clippy, Node 26.13.1 LTS, npm 11.20.0, and TypeScript 5.9.3. The contract
generator requires TypeScript 5; do not bypass npm's peer-dependency checks.

Native builds need a C/C++ toolchain, CMake, and pkg-config for bundled SQLite
and the Rustls AWS-LC provider. On macOS, install Xcode Command Line Tools and
CMake; on Debian/Ubuntu, install `build-essential cmake pkg-config`. Docker
Engine/Compose and Buildx are needed for the isolated integration environment
and image checks. No external model or live Matrix community is used by M0.

## Development checks

```sh
npm ci --prefix web --ignore-scripts
cargo fmt --all -- --check
cargo clippy --workspace --all-targets --locked -- -D warnings
cargo test --workspace --locked
cargo xtask contracts --check
cargo xtask sqlx --check
npm run typecheck --prefix web
cargo xtask drift-test
cargo build --workspace --release --locked
```

`cargo xtask verify` runs formatting, Clippy, workspace tests, strict frontend
checking, and the positive/negative contract/metadata checks in one command.
The Dockerfile's `verification` target runs this same CI entry point inside a
native Linux builder.

CI runs the same checks, release smoke, and final images on native Linux amd64
and arm64 runners. It also starts, resets, and removes a disposable Synapse.
[Verification evidence](docs/verification/m0-foundation.md) distinguishes actual
local results from remote workflow definitions.

## Run the minimal entry points

Start Control, then query it in another terminal:

```sh
cargo run --locked -p m3-control
cargo run --locked -p m3-cli -- health
```

Control binds to loopback by default. `GET /health/live` returns process
liveness; `GET /health/ready` currently returns HTTP 503 with `not_initialized`.
These public diagnostic endpoints do not grant management authority. Press
Ctrl-C to stop the bootstrap server; Unix SIGTERM also shuts it down gracefully.

```sh
cargo run --release --locked -p m3-agent -- storage-smoke
```

The smoke opens SDK-owned SQLite stores and a separate migrated SQLx database
in a fresh temporary directory. It checks WAL, foreign keys, FULL durability,
busy timeout, FTS5 `unicode61`/`trigram`, and a checked checkpoint query, then
closes its owners and removes only its own files. It never opens an existing
agent volume, logs in, or syncs. Passing establishes local SQLite linkage and
storage compatibility, not encrypted Matrix messaging or device recovery.

## Application migrations and query metadata

`m3-storage` embeds independent `migrations/control` and `migrations/agent`
histories. The initial schemas contain the singleton instance freeze state and
personal checkpoints; schema persistence does not implement manual freezing
or sync recovery. SDK databases are never application migration targets.

Ordinary queries use SQLx checked macros. `.cargo/config.toml` defaults builds
to offline metadata in `.sqlx/`; no live database is required to build.

```sh
cargo xtask sqlx
cargo xtask sqlx --check
```

The task migrates two disposable databases independently, creates a separate
compile-time description database from their SQL, and recompiles the storage
queries against it. `--check` compares every generated description and detects
obsolete files; generation replaces only JSON query descriptions. There is no
SQLx CLI installation dependency. Add queries to the storage crate so this
single preparation boundary continues to cover the complete application SQL.
Migration changes trigger recompilation through the storage build script.

## Browser contract

```sh
cargo xtask contracts
cargo xtask contracts --check
```

Rust exports [OpenAPI](api/openapi.json); pinned `openapi-typescript` generates
`web/src/generated/api.ts`. A minimal `openapi-fetch` client is typechecked in
strict mode. M0 has no full SPA. The final Control SPA remains M6 work.
`cargo xtask openapi` exports just the Rust artifact when needed.

`cargo xtask drift-test` first requires clean artifacts, intentionally changes
OpenAPI, TypeScript, and SQLx metadata one at a time, asserts rejection, restores
the bytes, and checks them again. It runs in CI and leaves artifacts unchanged
on ordinary errors. If the host forcibly kills the test, regenerate artifacts
before continuing.

## Disposable Matrix integration environment

```sh
cargo xtask synapse up
cargo xtask synapse smoke
cargo xtask synapse reset
cargo xtask synapse smoke
cargo xtask synapse down
```

These commands exclusively manage Compose project `m3-m0` from
[deploy/compose.test.yaml](deploy/compose.test.yaml). Synapse is exposed only at
`127.0.0.1:18008`; choose another unused local port in the test Compose file if
needed. The init command refuses existing configuration: use `reset` to remove
and recreate the dedicated `m3-m0_synapse-data` volume. `down` removes the
fixture containers, network, and volume. These operations erase fixture data.

Credentials and signing keys are generated in that volume, excluded from source
and logs. No private homeserver Admin API is used. Smoke requires a fresh volume
and exercises AS virtual-user registration/profile calls and the AI namespace
boundary. M0 sets AS registration `url: null` because the production transaction
receiver is M1 work; no AS push delivery or encrypted client is claimed here.
The fixed test server name is `m3.test`. Do not use this configuration on a
real homeserver or community.

## Container targets

The [Dockerfile](deploy/Dockerfile) has pinned multi-architecture base image
digests, a Node contract-check stage, a native Rust builder with cross C/Rust
toolchains where needed, and non-root Debian slim `agent`/`control` targets.
Control includes the CLI. Runner has no Node.js, compiler, or frontend assets.

```sh
docker buildx build --platform linux/arm64 --file deploy/Dockerfile --target agent --tag m3-agent:m0-arm64 --load .
docker buildx build --platform linux/arm64 --file deploy/Dockerfile --target control --tag m3-control:m0-arm64 --load .
docker run --rm --platform linux/arm64 m3-agent:m0-arm64 storage-smoke
docker run --rm --platform linux/arm64 m3-control:m0-arm64 --version
```

Repeat with `linux/amd64` and `m0-amd64`; an arm64 host needs an amd64 execution
emulator for the smoke. These foundation images contain no social runtime and
do not yet accept personal account credentials or mount production agent data.
The default agent command shows help. Runtime certificates are included, SQLite
is bundled, and the user is UID/GID 65532. Package repository state can change
between builds; M0 validates locked application inputs and native compatibility,
not bit-for-bit reproducibility or M6 release readiness. Image builds and CI use
`CARGO_BUILD_JOBS=1` to avoid overlapping SDK/application LLVM memory peaks on
small builders. On a constrained native host, use the same setting. This is a
build setting; it does not impose runtime agent resource or usage limits.

## M1 Matrix protocol prototype

M1 uses a separate disposable Synapse and Element Web fixture. It performs
explicit protocol scenarios and operator commands, with no models or autonomous
social activity. The ordinary M0 fixture remains independent.

    cargo xtask matrix up
    cargo xtask matrix smoke

The homeserver is on http://localhost:18018 and Element Web on
http://localhost:18080. Local registration is enabled only in this disposable
fixture so an operator can create the human test account. The authenticated AS
receiver temporarily binds port 18019 on all host interfaces for the Docker
callback; management readiness remains HTTP 503. The fixture is not a deployment
configuration for a real community.

The smoke command owns the receiver process and stops it on exit. Stop protocol
consoles before running it: an exclusive file lock prevents concurrent writers
to each device directory. The scenario verifies scoped credentials, real AS
ping/query/authentication/transaction redelivery, both wire encryption modes in
group chats and DMs, SAS comparison, unchanged repeated speech, stable send
transaction IDs, restart, delayed real room keys, and five abrupt process exits.

Control retains its global credentials in ignored .local/m1/control.json.
Only user-scoped sessions enter .local/m1/sessions/. SDK stores and the
application journal are under .local/m1/agents/. Preserve session files and SDK
directories together; never recreate a store while pretending to restore the
old encrypted device. Files are not encrypted at rest by this prototype.

For the selected human-client procedure, keep the original stores and run one
console per agent in separate terminals:

    cargo run --locked -p m3-agent -- console --homeserver http://127.0.0.1:18018 --data .local/m1/agents/alice --session .local/m1/sessions/alice.json
    cargo run --locked -p m3-agent -- console --homeserver http://127.0.0.1:18018 --data .local/m1/agents/bob --session .local/m1/sessions/bob.json

Each console accepts one JSON operator command per line. These are internal
execution parameters, not a required chat format. Messages remain ordinary
text. Enter "quit" to stop the client. Example operator commands:

    {"op":"rooms"}
    {"op":"bootstrap"}
    {"op":"create","users":["@human:m3.test"],"direct":true,"name":"M1 encrypted DM"}
    {"op":"devices","user":"@human:m3.test"}
    {"op":"verify-user","user":"@human:m3.test"}

Bootstrap creates cross-signing keys through SDK APIs. Verification returns a
flow ID. Accept the request in Element, then inspect/accept the SAS phase:

    {"op":"sas","user":"@human:m3.test","flow":"FLOW_ID"}

Compare the seven emoji or three numbers on both endpoints. Only after a real
match, confirm on Element and issue the corresponding console command:

    {"op":"confirm","user":"@human:m3.test","flow":"FLOW_ID"}
    {"op":"devices","user":"@human:m3.test"}

Check that the peer device is verified. Verification phases may be initiated by
different endpoints; starting negotiation does not confirm trust. No command
marks arbitrary devices trusted. Send an explicit fixture message with a stable
transaction ID, or inspect old messages after restart:

    {"op":"send","room":"ROOM_ID","transaction":"operator-fixture-1","body":"Hello from M1"}
    {"op":"event","room":"ROOM_ID","event":"EVENT_ID"}

Rooms are encrypted by default in this console. The controlled plaintext test
uses an explicit "encrypted":false parameter and never changes an existing
room's encryption. The console also supports join, event inspection, and
explicit device-to-device verification. This is not the M6 administration UI.

Build the final prototype images using the M0 build commands with tags
m3-agent:m1-arm64 and m3-agent:m1-amd64, then run:

    cargo xtask matrix images --arch arm64
    cargo xtask matrix images --arch amd64

These commands provision separate fixture identities for each architecture.
An offline initialization container copies only their user sessions into
m3-m1-probe-arm64 / m3-m1-probe-amd64 volumes, then runs the final image as UID
65532. It never receives Control's AS configuration. Each image runs the storage
and complete encrypted-client scenario, including child-process crashes.
An arm64 host uses its configured Docker amd64 emulator.

    cargo xtask matrix down

This removes only the m3-m1 Compose environment and its homeserver volume.
Local credentials/device directories and image-test volumes remain available
for inspection; they cannot be reused with a newly initialized homeserver.
Before another fresh "up", archive .local/m1 and remove the two explicitly named
image-test volumes if they are no longer needed. No existing community or
unrelated Docker volume is affected.

See [M1 verification](docs/verification/m1-matrix-e2ee.md) for tested versions,
human steps, actual image identities, recovery evidence, and support limits.
