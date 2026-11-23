# M0 Foundation Verification

Date: 2026-11-23
Status: Complete

## Revision and evidence scope

The implementation is based on `76d476048f7a3b0bddfa28b1e9e695519a3501d6`.
Clean-checkout validation first used a local-only candidate snapshot,
`258e49e1258eaf467773e2e0ee0a1d81eeb82fd0`, cloned under ignored `.local/`.
The committed implementation revision and its clean-checkout confirmation are
recorded below after the implementation commit. Nothing was pushed.
All 37 build/test input files match the delivered working tree; the
SHA-256 over sorted relative paths plus NUL-delimited contents is
`5020dc308ba9268e771232430793a7405fba0d2c6eac1e47e228180dfd627b43`. This covers manifests/lockfiles, toolchain/config,
SQLx metadata, Rust and browser sources, CI, deployment, and integration fixtures.
Documentation is updated after validation and is outside this build-input digest.
Image identities are recorded below.

[Initial-slice evidence](m0-initial-slice.md) preserves the earlier checks and
lockfile identity. It does not describe the current remaining work.

Current application lockfile SHA-256:
`e4d4cf314f21e1830bd81f2b5acef2fac0f3cb1a4000851e28fde4a42af690ed`.

Current npm lockfile SHA-256:
`615fe41e05c0c8792673ef4cb178faca276ffb2bd28633b85f60759fb8c88dfe`.

## Environment and pinned inputs

- Host: macOS 27.0.1 (26A434), Apple arm64; Apple clang 21.0.0,
  CMake 3.26.4, pkg-config 3.0.7.
- Rust 1.100.0 (`0ef1bc386`, 2026-11-09), Cargo 1.100.0 (`3acc79c0c`, 2026-11-05),
  edition 2024, declared/tested MSRV 1.100.
- Node 26.13.1 LTS, npm 11.20.0, TypeScript 5.9.3,
  openapi-typescript 7.13.0, openapi-fetch 0.17.0.
- The Node arm64 archive was checked against the official SHA-256 list:
  `aec2514ea1ceb2ba52455e9ec65986f6749a06dd759e5db2a72e6889668c9735`.
  Its local toolchain is under ignored `.local/toolchains/`; no production
  Node runtime is included in the images.
- Docker Desktop 4.38.0 (181591), Engine 27.5.1, Compose 2.32.4-desktop.1,
  BuildKit 0.26.2. The Linux builder exposes 12 CPUs and about 8 GiB memory.
- Linux arm64 executes natively in the Docker VM; amd64 execution uses Docker
  Desktop's configured emulation. No native amd64 host result is claimed.
  The emulator's separate interpreter version was not exposed by this check.

Pinned multi-architecture image indexes:

| Input | SHA-256 digest |
| --- | --- |
| rust:1.100.0-bookworm | ade39a710ef85f3506cffcfec3e8a24ba090badef1dff2fb798f71d878833fea |
| node:26.13.1-bookworm-slim | 5df79989ed685e09f0a98019db4cebfb22fb9df1a53c0161664a011f7ff6b0f1 |
| debian:bookworm-slim | 734a72b3d6142a71993b4bc580ca01d8ca687f77a2c422d96a961d3376b179bc |
| ghcr.io/element-hq/synapse:v1.165.0 | 673b4223c81a7634fd07d832b939a58f97081cfdaae7cdd2d1664cdcf7611b62 |

Base images and application dependencies are pinned. Debian build/runtime
packages still come from the configured package repositories; bit-for-bit
image reproducibility and release dependency notices are M6 work.

## SQLite compatibility and application storage

Cargo rejected SDK 0.19.1 plus SQLx 0.9.0: the SDK's rusqlite requires
`libsqlite3-sys ^0.38.1`, outside SQLx's `>=0.30.1, <0.38.0` range. The selected
SDK 0.18.0 plus SQLx 0.9.0 graph shares exactly one bundled native library:

```text
libsqlite3-sys 0.35.0
├── rusqlite 0.37.0 → matrix-sdk-sqlite 0.18.0 → matrix-sdk 0.18.0
└── sqlx-sqlite 0.9.0 → sqlx 0.9.0
```

Verified using `cargo tree --locked --offline -i libsqlite3-sys` and
`cargo tree --locked --offline -e features -i libsqlite3-sys`. SDK defaults
are disabled; enabled features are `e2e-encryption` and `bundled-sqlite`.
The SDK selects reqwest/Rustls. SQLx enables `runtime-tokio`, `sqlite-bundled`,
`macros`, and `migrate`; no PostgreSQL/MySQL runtime is used.

`m3-storage` embeds separate Control and agent migrators. The initial Control
schema stores the singleton freeze flag; the agent schema stores checkpoints.
Tests verify migration replay/reopen preserves both state and checkpoint
updates, the schemas remain separate, and invalid freeze values fail. This
schema is not a manual brake or a sync handoff implementation.

`cargo xtask sqlx` migrates separate disposable databases, then compiles the
storage crate's three checked queries against a third build-only description
database. Ordinary builds use offline `.sqlx/` metadata. Migration changes and
preparation environment changes trigger recompilation. FTS virtual-table SQL
uses bound runtime queries. SDK databases receive no application migrations.

Selected declared licenses, read from the downloaded package manifests:

| Package/version | License |
| --- | --- |
| matrix-sdk / matrix-sdk-sqlite 0.18.0 | Apache-2.0 |
| ruma 0.16.0 | MIT |
| sqlx 0.9.0 | MIT OR Apache-2.0 |
| rusqlite 0.37.0 / libsqlite3-sys 0.35.0 | MIT |
| axum 0.8.9 / tower 0.5.3 / tokio 1.53.1 | MIT |
| clap 4.6.7 / anyhow 1.0.104 | MIT OR Apache-2.0 |
| reqwest 0.13.5 / tempfile 3.27.0 | MIT OR Apache-2.0 |
| serde 1.0.229 / serde_json 1.0.151 / utoipa 5.4.0 | MIT OR Apache-2.0 |
| tracing 0.1.44 / tracing-subscriber 0.3.23 | MIT |
| aws-lc-rs 1.19.0 | ISC AND (Apache-2.0 OR ISC) |

AWS-LC's native `aws-lc-sys 0.47.0` additionally declares Apache-2.0, MIT,
BSD-3-Clause, and MIT-0 combinations. This selection record is not the full
redistribution notice inventory required for release in M6. Preserve all
applicable transitive/native notices in release artifacts.

Upstream references: [SDK 0.18 SQLite manifest](https://github.com/matrix-org/matrix-rust-sdk/blob/matrix-sdk-0.18.0/crates/matrix-sdk-sqlite/Cargo.toml),
[SDK 0.19.1 SQLite manifest](https://github.com/matrix-org/matrix-rust-sdk/blob/matrix-sdk-0.19.1/crates/matrix-sdk-sqlite/Cargo.toml),
[SQLx 0.9 SQLite manifest](https://github.com/launchbadge/sqlx/blob/v0.9.0/sqlx-sqlite/Cargo.toml).


## Executed acceptance checks

Local macOS checks passed:

```sh
npm ci --prefix web --ignore-scripts
cargo xtask verify
cargo build --workspace --release --locked
```

The unified verification entry point runs formatting, Clippy with warnings
denied, workspace tests, strict browser typechecking, and positive/negative
OpenAPI, TypeScript, and SQLx checks. Four tests pass with no ignored tests.
`drift-test` intentionally changes each generated artifact, confirms rejection,
restores it, and checks it again. Its expected negative-test error output does
not indicate a failed final verification.

The generated TypeScript uses a real openapi-fetch client with paths checked
against the Rust OpenAPI artifact. There is no full admin SPA in M0.

Release smoke passed on macOS with SQLite 3.50.2. It opens SDK state/crypto
stores and a separate migrated SQLx database in the same process; checks WAL,
foreign keys, FULL synchronous durability, a 5-second busy timeout, word and
trigram FTS5, and a checked checkpoint write/read. All stores are disposable.
A fixed offline server-version fixture prevents network discovery; no login,
client session, or encryption interoperability is claimed.

The release HTTP harness ran Control on a temporary loopback port, checked
liveness 200 and readiness 503, compared CLI output with the live response,
sent SIGTERM, observed exit code 0 within 5 seconds, and confirmed CLI failure
after shutdown. The earlier initial-slice SIGINT check also passed.

### Clean candidate checkout (M0-A1)

A host-side harness exports tracked/untracked non-ignored source files into a
new local Git repository, creates a candidate commit there, clones it, and
requires `git status --porcelain` to be empty before and after validation.
It runs npm clean installation/typechecking and a locked offline release Cargo
workspace build, then executes all three version entry points and Runner
storage smoke. Dependency/build caches are reused; no generated project files
or local databases are supplied by the original working directory.

### Linux images and CI entry point (M0-A3/A4)

| Architecture / check | Result |
| --- | --- |
| arm64 Runner final image | Passed SDK/SQLx/FTS5 smoke, SQLite 3.50.2 |
| arm64 Control and CLI final image | Both version entry points passed |
| arm64 runtime properties | UID 65532, certificates present, no Node/Rust compiler; `ldd` resolves libc/libm/libgcc_s only |
| amd64 Runner / emulated execution | Passed SDK/SQLx/FTS5 smoke, SQLite 3.50.2 |
| amd64 Control and CLI / emulated execution | Both version entry points passed |
| amd64 runtime properties | UID 65532, certificates present, no Node/Rust compiler; `ldd` resolves libc/libm/libgcc_s only |
| both architectures: HTTP, CLI, stopping | Liveness 200, readiness 503, CLI health succeeds, SIGTERM exits 0 within 5 seconds |
| native Linux CI verification entry point | Passed: formatting, Clippy, 4 tests, strict TypeScript, OpenAPI/TypeScript/SQLx drift rejection and restoration |

Validated local image identities:

- `m3-agent:m0-arm64`: `sha256:8ef29093775a204ad777f67b1cf9150affd990586c0e1824f5e6b46898efc53d`.
- `m3-control:m0-arm64`: `sha256:b6da1fc36b480fc5ff1b2eaaec8e225cad42fc06206aac89cca592b0f468ba18`.
- `m3-agent:m0-amd64`: `sha256:210549f38d296db4b068151eab6dd66f68da4471c25231ba10d94cafb8513abe`.
- `m3-control:m0-amd64`: `sha256:ea9a924988f042bb6519d06685a50180a971046cd540c01e6016f79f76e7007f`.

For each architecture, run the Runner storage smoke and all three `--version`
entry points, inspect the runtime UID/certificates/tools/linkage, then start
Control with `--listen 0.0.0.0:8080` and an ephemeral loopback-published port.
Request `/health/live` and `/health/ready`, execute `m3 health` inside the
container, and run `docker stop --time 5`; inspect exit code 0 and remove the
test container. Both final images passed these steps.

Reproduce image builds with the README's `docker buildx build` commands for
both `linux/arm64` and `linux/amd64`, targeting `agent` and `control` separately.
The native builder installs the target Rust standard library and, for a
cross-target build, the corresponding GNU C/C++ toolchain. Final images run
as UID/GID 65532, include certificates, and contain no build toolchain.

The Dockerfile's native `verification` target executes `cargo xtask verify`,
the same entry point wired into GitHub Actions. Remote GitHub workflow
execution is not claimed; the CI commands are exercised locally and inside
Linux. The workflow also defines native amd64/arm64 runner jobs and the
Synapse reset scenario.

An initial default-parallel build failed with `ResourceExhausted: cannot
allocate memory` on the 12-CPU/8-GiB Docker builder. No application/runtime
quota was added. Docker and CI now set `CARGO_BUILD_JOBS=1` to serialize
compiler processes. Interrupted/canceled attempts are not successful evidence.

### Disposable Synapse (M0-A5)

Executed commands (the smoke/reset/down calls also ran directly through the
built `target/debug/xtask` entry point):

```sh
cargo xtask synapse up
cargo xtask synapse smoke
cargo xtask synapse reset
cargo xtask synapse smoke
cargo xtask synapse down
```

Both fresh-server runs passed against real Synapse 1.165.0:

- Before provisioning, the probe profile returns HTTP 404, proving no reused
  probe account from the previous run.
- Standard AS registration creates `@m3_probe:m3.test`; standard Client-Server
  profile update/read returns the exact fixture display name.
- Registration outside the local AI namespace fails with HTTP 400.
- Reset removes and recreates only `m3-m0_synapse-data`; the same fresh-server
  checks pass again, then the dedicated containers/network/volume are removed.

Credentials and keys were generated only in the test volume and not logged.
No private Admin API or real community was used. M0 registration has `url: null`;
M1 implements and validates the authenticated transaction receiver and
independent encrypted sessions. This is not an AS push-delivery or E2EE test.

## Acceptance outcome

| Item | Result and evidence |
| --- | --- |
| M0-A1 | Passed clean-checkout locked build and all three entry points |
| M0-A2 | Passed same-process SDK/SQLx, durability settings, word/trigram FTS5 in release builds |
| M0-A3 | Passed both Linux architecture builds and runtime smoke; amd64 uses documented emulation |
| M0-A4 | Passed the actual CI verification entry point locally and on native Linux arm64, including intentional artifact drift |
| M0-A5 | Passed real disposable Synapse startup, provisioning, reset, rerun, and cleanup |

All M0 implementation and acceptance items are satisfied. Hosted GitHub Actions
execution, native amd64-host execution, and M1 protocol validation are not claimed.

## Remaining product limits

M0 provides a reproducible development foundation. Product account/device
provisioning, authenticated management, encrypted messaging, sync recovery,
autonomous inference, persistent personal memory, and manual freezing are
later milestones. Control correctly remains not ready (HTTP 503).

The SDK's aquamarine dependency brings proc-macro-error2 2.0.1, which emits a
future-Rust warning about private proc_macro re-export (Rust issue 127909).
Current pinned-toolchain checks pass; revalidate before upgrading it.

No models were called, no autonomous community ran, no release was published,
and no production infrastructure was modified.
