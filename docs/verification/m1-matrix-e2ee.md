# M1 Matrix and E2EE Verification

Date: 2026-11-23
Status: Complete; M1-I1–I7 and M1-A1–A7 pass

## Revision and environment

Tested implementation commit: `2a6b9750b70f124a69ce19eb490b2ce473e19d73`,
based on M0 evidence commit `438be96`. Host checks and final image builds use
the implementation sources in that commit; subsequent changes only record
acceptance evidence and milestone status.
No remote workflow execution or publication is claimed.

Host and compiler match the M0 environment: macOS 27.0.1 on Apple arm64,
Rust/Cargo 1.100.0, Node 26.13.1, npm 11.20.0, Docker Desktop 4.38.0,
Engine 27.5.1, and Compose 2.32.4. Application dependencies remain locked.
Matrix SDK 0.18.0, Ruma 0.16.0, SQLx 0.9.0, and bundled libsqlite3-sys 0.35.0
continue sharing SQLite 3.50.2. Ruma appservice server types 0.16.0 are now enabled.
SDK automatic-room-key-forwarding is enabled with its native trust policy;
no arbitrary-device trust override is used.

| Component | Tested configuration |
| --- | --- |
| Homeserver | Synapse 1.165.0, local m3.test, legacy authentication |
| AS | Standard registration and m.login.application_service; user/device-scoped tokens |
| Runner sync | Ordinary Client-Server sync through a local durable transport journal |
| Persistent stores | Separate per-user SDK SQLite state/crypto directories and SQLx agent database |
| Human client | Element Web 1.12.35 in Microsoft Edge, local disposable human account |
| Linux arm64 | Native execution inside the Docker VM |
| Linux amd64 | GNU cross build and Docker Desktop emulation; no native amd64 host claim |

Synapse uses the same pinned index as M0:
673b4223c81a7634fd07d832b939a58f97081cfdaae7cdd2d1664cdcf7611b62.
Element Web uses the official v1.12.35 index:
406b48b80153818284a235fa3600b2a5b5b6b0aa0e4ef4896b36d8d3461352f3.

## Reproduction and credentials

Follow the README M1 commands. In a fresh disposable fixture:

    cargo xtask matrix up
    cargo xtask matrix smoke

The fixture creates a separate m3-m1 Compose environment. Human registration is
enabled only in this local test configuration. Control alone reads the global
AS configuration. Provisioning calls standard registration with inhibit_login,
then standard AS login and user-token whoami. Session outputs are exclusive,
mode 0600 files under a private directory. Existing files are not overwritten;
a partial provisioning failure is retained for explicit recovery rather than
silently manufacturing a replacement device. M2 adds managed provisioning.

The real HTTP checks exercise homeserver-to-AS ping, authenticated known/unknown
user queries, rejected authentication, duplicate transactions, namespace
enforcement, and token scope. A user token cannot provision AS users or assert
another identity. The receiver records one durable acknowledgement per
transaction ID and no chat body or inference work. A real protocol run produces
new server-delivered acknowledgements.

The final image test commands are:

    cargo xtask matrix images --arch arm64
    cargo xtask matrix images --arch amd64

Each architecture has separate newly provisioned users and a persistent named
test volume. Only those user sessions enter the image; neither AS token enters
the container. Initialization runs offline, then the actual image executes as
UID 65532. The image harness rejects reuse of a session without its SDK volume.

## Automated protocol and recovery scenarios

The host unified Matrix smoke command passes these scenarios against real
Synapse, using distinct MXIDs, devices, and SDK stores:

1. Create a plaintext group and DM, then an encrypted group and DM. Both users
   send bidirectionally. The receiver compares exact bodies, including
   whitespace, an embedded SILENT string, Chinese text, and distinct identical
   messages. Raw event types confirm actual encryption rather than a plaintext
   fallback.
2. Retry sends with the same transaction ID and require the same event ID.
   Distinct transactions with identical text remain distinct durable events.
3. Request SAS verification from one client and start the SAS phase on its peer.
   The shared console helper accepts the negotiation, both endpoints compare
   actual numbers, then both confirm and report a verified device.
4. Reopen the original stores and sessions. Require unchanged MXIDs, device IDs,
   and Ed25519 fingerprints; re-decrypt this run's acknowledged old ciphertext
   through the SDK and exchange a new encrypted message.
5. Delay real encrypted to-device packets in a test-only transport. The room
   event remains pending_decryption with no body, with three bounded attempts.
   Restart the recipient, release the real packets with a fresh server batch,
   and require exactly one ready row with the original body.
6. Send an encrypted fixture event and launch a recipient child that exits with
   code 86 at each boundary below. Reopen its original stores and require exact
   recovery and one ready event row after explicit replay.

| Abrupt exit boundary | Durable expectation checked |
| --- | --- |
| Journal commit, before SDK receives sync | Ciphertext exists; application cursor unchanged |
| SDK processing, before application decryption | Ciphertext exists; application cursor unchanged |
| Decryption, before inbox update | Ciphertext exists; application cursor unchanged |
| Inbox persistence, before checkpoint | One ready row; application cursor unchanged |
| Checkpoint commit | One ready row; application cursor advanced |

The process exits bypass destructors and cooperative cleanup. SDK crypto/state
remain owned by SDK APIs. These are actual process-crash tests, not merely
synthetic database fixtures; they do not simulate arbitrary power loss or make
three databases one transaction. Deduplication here establishes one effective
durable handoff. No model/inference consumer exists until M3.

The raw journal is written before SDK processing because the pinned SDK can
suppress an already-seen next_batch. Limited room timelines are paginated before
handoff. Missing-key retries use SDK key notifications and retain diagnostics;
native key-verification messages never become social input. The SDK creates and
deduplicates its own key requests. No chat-body screening or rewriting occurs.

Deterministic tests additionally verify authenticated AS rejection and query
semantics, transaction idempotency, atomic rollback of malformed batches,
event-ID deduplication while preserving identical speech, migration reopening,
and SDK/SQLx/FTS5 coexistence. The unified verification command also runs
formatting, Clippy, strict TypeScript, and positive/negative contract/SQLx drift
checks. Expected negative drift errors are not final failures.

## Host checks

The following commands completed successfully against the candidate sources:

    PATH="$PWD/.local/toolchains/node-v26.13.1-darwin-arm64/bin:$PATH" cargo xtask verify
    cargo build --workspace --release --locked
    cargo xtask matrix smoke

The verification command passed all six Rust tests, formatting, Clippy with
warnings denied, TypeScript checks, and contract/SQLx drift rejection and
restoration. The locked release build passed. A clean, detached local clone of the tested
commit also passed `cargo build --workspace --release --locked --offline`
and the resulting release `m3-agent storage-smoke` (SQLite 3.50.2), using
the existing dependency/build cache. The Matrix smoke completed real
AS checks and every protocol/recovery scenario listed above. Documentation
relative links and workflow YAML parsing also passed. Hosted workflow jobs were
not executed.

## Human Element Web procedure and observations

The human operator registered and logged into the local Element instance using
their own temporary password. No password was collected by the coding assistant
or written into this report. The tested identity was @human:m3.test, device
QDQHCUEBZQ.

The two persistent prototype clients used:

| MXID | Device | Public Ed25519 fingerprint |
| --- | --- | --- |
| @_m3_alice:m3.test | KDTOHEKNER | euPssmpjvHcDpX8fJFaCbMS4dayoqY65BVHDf2ML97U |
| @_m3_bob:m3.test | XFCCDQNSRQ | 44EXw0d7HKjELNpMmY6JWHdQQCwv7nWCF4oEistSxM8 |

The operator joined the controlled encrypted three-member group, read both
prototype messages, and sent the agreed fixture reply. Both clients received
that exact body from encrypted wire events. The operator then joined a
plaintext group and separate encrypted DMs with both clients, read messages,
and replied. The personal databases showed the agreed bodies as ready, with
plaintext group and encrypted DM wire types.

For verification, the clients bootstrapped their own cross-signing through SDK
APIs. Alice requested the human identity's verification in their existing
encrypted DM. Element initiated the SAS phase. The operator compared all seven
emoji / three numbers and explicitly reported a match and clicked the match
button. Alice confirmed on its SDK endpoint, which subsequently reported the
human device verified. After reopening Alice's original stores, that verified
status remained true. Bob's device is not claimed to have been independently
verified by the human; the SDK pair verifies devices in the automated scenario.

Both clients reopened the original stores with unchanged fingerprints. SDK
event fetch/decryption successfully recovered the previously received human DM
fixture messages, without relying on cached plaintext. Both then sent new
encrypted DM messages after restart; the operator confirmed they were readable.
The operator also joined two explicit plaintext test DMs and replied; both
received the exact agreed body with m.room.message wire types. Further replies
in the encrypted DMs arrived as encrypted events.

The initial console incorrectly gated SAS acceptance on request initiation and
left Element waiting. That attempt is not passing evidence. The corrected
idempotent SDK acceptance flow above completed with actual comparison and
persistent verified status. See J005 in the journey.

## Image results

Both final non-root prototype images pass SDK state/crypto initialization,
SQLx/FTS5 coexistence, all four messaging modes, peer-started SAS verification,
persistent device restart, delayed real keys, and all five abrupt-exit recovery
boundaries. Each architecture uses its own pair of identities and SDK volume.

| Architecture | Execution | Result | Local image identity |
| --- | --- | --- | --- |
| arm64 | Native Docker Linux VM | Passed | `sha256:7e2d7de40b9ef6154fde53e4a02e59633cc67338d1dd81388d216634b51e5e60` |
| amd64 | Docker Desktop emulation | Passed | `sha256:fc1a3b2b16728f81e48b1c0bf3a29abba29e51f636b187cb18af550d05559f92` |

Actual commands executed (sequentially, with build logs under ignored .local/m1):

    docker buildx build --platform linux/arm64 --file deploy/Dockerfile --target agent --tag m3-agent:m1-arm64 --load .
    docker buildx build --platform linux/amd64 --file deploy/Dockerfile --target agent --tag m3-agent:m1-amd64 --load .
    python3 tests/integration/matrix_images.py --arch arm64
    python3 tests/integration/matrix_images.py --arch amd64

The documented `cargo xtask matrix images --arch ARCH` entry point delegates to
that same harness. Both executions returned success and wrote passing result
records; SDK fingerprints remained stable across their internal restarts.
The local fixture remains available for inspection. Protocol clients and crash
children have exited; only the disposable Synapse and Element services remain.

| Acceptance | Result and evidence |
| --- | --- |
| M1-A1 | SDK pair and real Element group/DM replies, both wire encryption modes |
| M1-A2 | SDK SAS and human Element comparison; persisted verified status |
| M1-A3 | Original devices/stores, old ciphertext decryption and new encrypted sends |
| M1-A4 | Real delayed keys, persistent diagnostic state, one ready event |
| M1-A5 | Five actual process exits and exact recovery, host and both images |
| M1-A6 | Authenticated AS redelivery plus sync replay/event-ID deduplication |
| M1-A7 | Both final images pass the complete SDK/SQLx encrypted-client scenario |


## Limits and remaining product work

This validates the listed Synapse legacy-auth/SDK/Element combination only.
MAS/OIDC, federated servers, other human clients, native amd64 hosts, and a
production support matrix are not claimed. Remote GitHub Actions has not run.

No models were called and no autonomous community ran. There is no M2 lifecycle
manager, managed provisioning recovery, coordinated backup, or M3 manual brake.
Control readiness correctly remains HTTP 503. The operator console and local
protocol fixture are not a production administration surface.

Personal databases contain decrypted fixture text and are not encrypted at rest.
The retained SDK warning about proc-macro-error2 and future Rust compatibility
is unchanged from M0. Revalidate dependency and native-library compatibility on
upgrade. Earlier interrupted delayed-packet fixtures can retain permanently
missing-key rows; they remain diagnosable rather than being deleted to pass
checks. Restart/decryption acceptance targets the acknowledged events of the
current scenario and preserves all unrelated history.
