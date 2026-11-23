> Historical initial-slice evidence. See [current M0 acceptance](m0-foundation.md).

# M0 Foundation Verification

Date: 2026-11-23
Status: First implementation slice; M0 gate remains open
Tested revision: Working tree based on `76d476048f7a3b0bddfa28b1e9e695519a3501d6`,
with the foundation files delivered alongside this report. No implementation
commit or remote CI run is claimed. The tested `Cargo.lock` SHA-256 is
`5deedce904edf8ae0e8ac3e09a7adb719d3343eb65e6ac55310f258bc28cd325`.

## Scope and environment

This slice supplies a Cargo workspace, the `m3-control`, `m3-agent`, and `m3`
entry points, a health-only Axum server, a CLI liveness request, a disposable
SDK/SQLx smoke command, and Rust OpenAPI export/drift checking. A native Linux
CI matrix is defined but has not been run in this session.

Local environment:

- macOS 27.0.1 (26A434), Apple arm64 (`aarch64-apple-darwin`).
- Rust 1.100.0 (`0ef1bc386`, 2026-11-09), Cargo 1.100.0 (`3acc79c0c`, 2026-11-05).
- Edition 2024; declared MSRV equals the tested compiler, Rust 1.100.
- Apple clang 21.0.0 (`clang-2100.4.1.1`), CMake 3.26.4, pkg-config 3.0.7.
- The previous local compiler was Rust 1.70.0. Rustup installed the pinned
  toolchain; the repository override selects it without changing the default
  compiler for other projects.

## Dependency compatibility and features

Cargo initially rejected Matrix SDK 0.19.1 with SQLx 0.9.0: the SDK's
`rusqlite 0.40.2` requires `libsqlite3-sys ^0.38.1`, while SQLx 0.9.0 permits
`>=0.30.1, <0.38.0`. They cannot share one `links = "sqlite3"` native library.
No SDK fork, registry patch, or second native SQLite copy was introduced.

The selected graph, verified with `cargo tree --locked --offline -i libsqlite3-sys`, is:

```text
libsqlite3-sys 0.35.0
├── rusqlite 0.37.0 → matrix-sdk-sqlite 0.18.0 → matrix-sdk 0.18.0 → m3-agent
└── sqlx-sqlite 0.9.0 → sqlx 0.9.0 → m3-agent
```

`cargo tree --locked --offline -e features -i libsqlite3-sys` confirms bundled
SQLite is enabled by both paths. SDK defaults are disabled; explicitly enabled
features are `e2e-encryption` and `bundled-sqlite`, which select SDK-owned state,
event-cache/media, and crypto stores. SDK 0.18 uses reqwest's Rustls backend;
CLI reqwest also selects Rustls. SQLx enables only `runtime-tokio` and
`sqlite-bundled`. Checked-query macros and migration features will be added
with actual application storage in M0-I4. The smoke uses literal parameterized
runtime SQL for FTS virtual-table operations.

Direct version pins live in the workspace manifest; `Cargo.lock` locks the
transitive graph, including Ruma 0.16.0. Direct AS server API integration is
still M1 work. The SDK's ordinary Client-Server sync is not exercised here.

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

## Executed checks

All results below apply to the working tree and local macOS arm64 environment
specified above. Dependency downloads/builds used Cargo's local registry cache;
`--offline` checks required no model or homeserver access.

| Command / scenario | Result |
| --- | --- |
| `cargo fmt --all -- --check` | Passed |
| `cargo clippy --workspace --all-targets --locked --offline -- -D warnings` | Passed; upstream future-incompatibility warning noted below |
| `cargo test --workspace --locked --offline` | Passed: 3 tests, 0 failures; no ignored tests |
| `cargo build --workspace --release --locked --offline` | Passed for local arm64; all three entry points and xtask built |
| `target/release/m3-agent storage-smoke` | Passed; SQLite 3.50.2, SDK stores + SQLx, word/trigram queries |
| `target/release/m3-agent --version` | `m3-agent 0.1.0` |
| `target/release/m3-control --version` | `m3-control 0.1.0` |
| `target/release/m3 --version` | `m3 0.1.0` |
| `cargo xtask openapi` / `cargo xtask openapi --check` | Export and check passed |
| Deliberately append a newline to `api/openapi.json`, run `target/debug/xtask openapi --check`, restore bytes, check again | Stale artifact rejected with nonzero exit; restored artifact accepted |
| Release Control + CLI HTTP scenario below | Passed |
| Local Markdown link check across README and docs, plus `git diff --check` | Passed |

The three tests cover SDK/SQLx coexistence (including WAL, foreign keys, FULL
synchronous durability, 5-second busy timeout, and both FTS5 tokenizers),
liveness versus readiness response bodies/statuses, and absence of a fake
management action endpoint. SQLx connection-option checks validate configuration;
they do not establish application crash recovery.

Release HTTP scenario, executed by a temporary host-side Python harness:

1. Reserve an ephemeral loopback port with a socket, release it, and launch
   `target/release/m3-control --listen 127.0.0.1:<port>` with `RUST_LOG=info`.
2. Poll `GET /health/live` directly (without an HTTP proxy) until HTTP 200;
   assert `{"service":"m3-control","status":"alive"}`.
3. Read `GET /health/ready`; assert HTTP 503 and status `not_initialized`.
4. Run `target/release/m3 health --control-url http://127.0.0.1:<port>`;
   assert success and the same liveness JSON.
5. Send SIGINT to Control; assert a clean exit within 5 seconds.
6. Repeat the CLI health call to the now-closed port; assert nonzero exit.
7. Kill the temporary process in cleanup if any assertion fails.

An initial harness waited for an INFO startup log while inheriting
`RUST_LOG=warn`, so that harness timed out. The final harness above polls HTTP
and sets the log level explicitly; it passed. No application change was needed.
No test process was left running.

M0-A2 is satisfied locally by the release smoke. M0-A1 remains open because a
clean checkout of an implementation commit has not been exercised; this report
covers the current working tree. M0-A3/A4/A5 remain open. In particular, the
local OpenAPI negative test does not prove the full CI/TypeScript/SQLx drift
gate.

## Limitations and remaining M0 work

- Local execution covers only macOS arm64. Linux amd64/arm64 execution and CI
  results remain unverified, so M0-A3 is open.
- Application migrations, checked queries, offline metadata, and metadata
  preparation/checking remain M0-I4 work.
- The exported health contract has no frontend TypeScript generator yet.
  Full OpenAPI/TypeScript/SQLx drift acceptance M0-A4 remains open.
- Disposable Synapse/AS configuration, reset instructions, and container build
  targets remain M0-I6/M0-I8 work. No Docker integration was run.
- There is no account/device session, verification, encryption exchange,
  sync replay, restore, or manual-brake validation. The smoke supplies a fixed
  offline Matrix version to the builder to avoid capability discovery; it
  does not claim support for that server version.
- Control reports readiness as HTTP 503 (`not_initialized`). The health CLI
  reads liveness only. Neither exposes lifecycle or freeze operations.
- The SDK's `aquamarine` dependency brings `proc-macro-error2 2.0.1`, which
  emits a future-incompatibility warning about re-exporting the private
  `proc_macro` extern crate. `cargo report future-incompatibilities --id 1`
  identifies upstream Rust issue 127909. Tests and Clippy pass on the pinned
  compiler; reevaluate this transitive dependency before changing the compiler.
- No external models or live community were used.
