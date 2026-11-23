//! Bounded, deterministic protocol scenarios. No models or autonomous activity.
use crate::matrix::PersistentClient;
use anyhow::{Context, ensure};
use matrix_sdk::ruma::{
    api::client::room::create_room, events::room::message::RoomMessageEventContent, serde::Raw,
};
use serde_json::json;
use std::path::Path;

async fn pump(a: &PersistentClient, b: &PersistentClient) -> anyhow::Result<()> {
    a.sync_once().await?;
    b.sync_once().await?;
    Ok(())
}
async fn verify(
    a: &PersistentClient,
    b: &PersistentClient,
    other_starts: bool,
) -> anyhow::Result<()> {
    let other = a
        .client
        .encryption()
        .get_device(b.client.user_id().unwrap(), b.client.device_id().unwrap())
        .await?
        .context("Peer device not discovered")?;
    let request = other.request_verification().await?;
    let remote = loop {
        pump(a, b).await?;
        if let Some(request) = b
            .client
            .encryption()
            .get_verification_request(a.client.user_id().unwrap(), request.flow_id())
            .await
        {
            break request;
        }
    };
    remote.accept().await?;
    while !request.is_ready() {
        ensure!(
            !request.is_done() && !request.is_cancelled(),
            "Verification request ended before negotiation"
        );
        pump(a, b).await?;
    }
    let (initiator, receiver, starter) = if other_starts {
        (b, a, &remote)
    } else {
        (a, b, &request)
    };
    let started = starter.start_sas().await?.context("SAS did not start")?;
    let received = loop {
        pump(a, b).await?;
        if let Some(sas) = receiver
            .client
            .encryption()
            .get_verification(initiator.client.user_id().unwrap(), request.flow_id())
            .await
            .and_then(|v| v.sas())
        {
            break sas;
        }
    };
    crate::matrix::accept_sas(&received).await?;
    let (sas, remote_sas) = if other_starts {
        (received, started)
    } else {
        (started, received)
    };
    while sas.decimals().is_none() || remote_sas.decimals().is_none() {
        ensure!(
            !sas.is_cancelled() && !remote_sas.is_cancelled(),
            "SAS cancelled: {:?} / {:?}",
            sas.cancel_info(),
            remote_sas.cancel_info()
        );
        pump(a, b).await?;
    }
    ensure!(sas.decimals() == remote_sas.decimals(), "SAS values differ");
    sas.confirm().await?;
    remote_sas.confirm().await?;
    while !sas.is_done() || !remote_sas.is_done() {
        pump(a, b).await?;
    }
    ensure!(
        a.client
            .encryption()
            .get_device(b.client.user_id().unwrap(), b.client.device_id().unwrap())
            .await?
            .context("Missing peer")?
            .is_verified(),
        "SAS did not verify peer"
    );
    println!("SAS (peer starts: {other_starts}): matching numbers and completed verification");
    Ok(())
}

pub async fn run(homeserver: &str, sessions: &Path, state: &Path) -> anyhow::Result<()> {
    let transport = crate::probe_transport::DelayedTransport::start(homeserver).await?;
    let a = PersistentClient::open(
        homeserver,
        &state.join("alice"),
        &sessions.join("alice.json"),
    )
    .await?;
    let b = PersistentClient::open(
        &transport.url,
        &state.join("bob"),
        &sessions.join("bob.json"),
    )
    .await?;
    ensure!(
        a.client.user_id() != b.client.user_id(),
        "Shared user identity"
    );
    ensure!(
        a.client.device_id() != b.client.device_id(),
        "Shared device identity"
    );
    pump(&a, &b).await?;
    let a_identity = (
        a.client.user_id().unwrap().to_owned(),
        a.client.device_id().unwrap().to_owned(),
        a.fingerprint().await?,
    );
    let b_identity = (
        b.client.user_id().unwrap().to_owned(),
        b.client.device_id().unwrap().to_owned(),
        b.fingerprint().await?,
    );
    let mut rooms = Vec::new();
    let mut encrypted_events = Vec::new();
    for (encrypted, direct, label) in [
        (false, false, "plain-group"),
        (false, true, "plain-dm"),
        (true, false, "encrypted-group"),
        (true, true, "encrypted-dm"),
    ] {
        let mut request = create_room::v3::Request::new();
        request.invite = vec![b.client.user_id().unwrap().to_owned()];
        request.is_direct = direct;
        request.name = Some(format!("M1 {label}"));
        if encrypted {
            request.initial_state.push(Raw::from_json_string(json!({"type":"m.room.encryption","state_key":"","content":{"algorithm":"m.megolm.v1.aes-sha2"}}).to_string())?);
        }
        println!("Creating {label}");
        let room = a.client.create_room(request).await?;
        println!("Created {label}; syncing invitation");
        pump(&a, &b).await?;
        println!("Joining {label}");
        b.client.join_room_by_id(room.room_id()).await?;
        println!("Joined {label}");
        pump(&a, &b).await?;
        if label == "encrypted-group" {
            verify(&a, &b, true).await?;
        }
        let peer = b
            .client
            .get_room(room.room_id())
            .context("Joined room missing")?;
        for (sender, recipient, body, txn) in [
            (
                &room,
                &b,
                "  M1 text\n[[SILENT]] inside ordinary chat  ",
                format!("{label}-a"),
            ),
            (&peer, &a, "相同文字", format!("{label}-b1")),
            (&peer, &a, "相同文字", format!("{label}-b2")),
        ] {
            let txn = matrix_sdk::ruma::OwnedTransactionId::from(txn);
            let sent = sender
                .send(RoomMessageEventContent::text_plain(body))
                .with_transaction_id(txn.clone())
                .await?;
            // A repeated transaction is the same event, not a second experience.
            let retry = sender
                .send(RoomMessageEventContent::text_plain(body))
                .with_transaction_id(txn.clone())
                .await?;
            ensure!(
                sent.response.event_id == retry.response.event_id,
                "Send transaction was not idempotent"
            );
            loop {
                pump(&a, &b).await?;
                let text: Option<String> = sqlx::query_scalar(
                    "SELECT body FROM matrix_events WHERE event_id=? AND state='ready'",
                )
                .bind(sent.response.event_id.as_str())
                .fetch_optional(&recipient.pool)
                .await?;
                if let Some(text) = text {
                    ensure!(text == body, "Message body changed");
                    break;
                }
            }
            let raw: serde_json::Value = serde_json::from_str(
                &sqlx::query_scalar::<_, String>(
                    "SELECT raw_json FROM matrix_events WHERE event_id=?",
                )
                .bind(sent.response.event_id.as_str())
                .fetch_one(&recipient.pool)
                .await?,
            )?;
            ensure!(
                (raw["type"] == "m.room.encrypted") == encrypted,
                "Wrong wire encryption"
            );
            if encrypted {
                encrypted_events.push(sent.response.event_id.clone());
            }
        }
        println!(
            "{label}: bidirectional exact text, repeated speech, and stable transaction retry passed"
        );
        rooms.push(room.room_id().to_owned());
    }
    let final_room = rooms.pop().unwrap();
    // Drop all SDK room handles before reopening the same exclusive directories.
    drop(a);
    drop(b);
    let a = PersistentClient::open(
        homeserver,
        &state.join("alice"),
        &sessions.join("alice.json"),
    )
    .await?;
    let b = PersistentClient::open(
        &transport.url,
        &state.join("bob"),
        &sessions.join("bob.json"),
    )
    .await?;
    pump(&a, &b).await?;
    ensure!(
        (
            a.client.user_id().unwrap().to_owned(),
            a.client.device_id().unwrap().to_owned(),
            a.fingerprint().await?
        ) == a_identity,
        "Alice device changed"
    );
    ensure!(
        (
            b.client.user_id().unwrap().to_owned(),
            b.client.device_id().unwrap().to_owned(),
            b.fingerprint().await?
        ) == b_identity,
        "Bob device changed"
    );
    // Re-decrypt this run's acknowledged ciphertext through SDK. Earlier
    // interrupted fault fixtures may have permanently withheld keys; retain
    // those diagnosable rows instead of deleting history to make a test pass.
    for client in [&a, &b] {
        for event in &encrypted_events {
            sqlx::query("UPDATE matrix_events SET state='received',body=NULL,decryption_attempts=0 WHERE event_id=?").bind(event.as_str()).execute(&client.pool).await?;
        }
        client.recover_events().await?;
        for event in &encrypted_events {
            let state: String =
                sqlx::query_scalar("SELECT state FROM matrix_events WHERE event_id=?")
                    .bind(event.as_str())
                    .fetch_one(&client.pool)
                    .await?;
            ensure!(
                state == "ready",
                "Acknowledged ciphertext could not be decrypted after restart"
            );
        }
    }
    let room = a
        .client
        .get_room(&final_room)
        .context("Restored room missing")?;
    let event = room
        .send(RoomMessageEventContent::text_plain("after restart"))
        .await?;
    loop {
        pump(&a, &b).await?;
        if sqlx::query_scalar::<_, i64>(
            "SELECT count(*) FROM matrix_events WHERE event_id=? AND state='ready'",
        )
        .bind(event.response.event_id.as_str())
        .fetch_one(&b.pool)
        .await?
            == 1
        {
            break;
        }
    }
    println!(
        "Restart: MXIDs, device IDs/fingerprints, old decryption, and new encrypted messages passed"
    );
    drop(room);
    let mut request = create_room::v3::Request::new();
    request.invite = vec![b.client.user_id().unwrap().to_owned()];
    request.initial_state.push(Raw::from_json_string(json!({"type":"m.room.encryption","state_key":"","content":{"algorithm":"m.megolm.v1.aes-sha2"}}).to_string())?);
    let room = a.client.create_room(request).await?;
    pump(&a, &b).await?;
    b.client.join_room_by_id(room.room_id()).await?;
    pump(&a, &b).await?;
    transport.hold(true).await;
    let delayed = room
        .send(RoomMessageEventContent::text_plain("delayed real room key"))
        .await?
        .response
        .event_id;
    for _ in 0..4 {
        pump(&a, &b).await?;
    }
    let row: (String, Option<String>, i64) =
        sqlx::query_as("SELECT state,body,decryption_attempts FROM matrix_events WHERE event_id=?")
            .bind(delayed.as_str())
            .fetch_one(&b.pool)
            .await?;
    ensure!(
        row.0 == "pending_decryption" && row.1.is_none() && row.2 == 3,
        "Missing-key event was not durably deferred with bounded attempts"
    );
    drop(b);
    let b = PersistentClient::open(
        &transport.url,
        &state.join("bob"),
        &sessions.join("bob.json"),
    )
    .await?;
    transport.hold(false).await;
    // A real to-device delivery advances the server cursor. Ensure the delayed
    // fixture also delivers against a fresh server batch; injecting packets into
    // an unchanged next_batch would correctly be ignored by SDK replay guards.
    room.send(RoomMessageEventContent::text_plain("release delayed key"))
        .await?;
    loop {
        pump(&a, &b).await?;
        if sqlx::query_scalar::<_,i64>("SELECT count(*) FROM matrix_events WHERE event_id=? AND state='ready' AND body='delayed real room key'").bind(delayed.as_str()).fetch_one(&b.pool).await?==1{break;}
    }
    println!(
        "Delayed key: persisted pending state survives restart; real SDK key delivery decrypts into one ready row"
    );
    let _b = crash_windows(&a, b, &room, &transport.url, sessions, state).await?;
    println!(
        "Alice {} {} ed25519:{}",
        a_identity.0, a_identity.1, a_identity.2
    );
    println!(
        "Bob {} {} ed25519:{}",
        b_identity.0, b_identity.1, b_identity.2
    );
    Ok(())
}

async fn crash_windows(
    a: &PersistentClient,
    mut b: PersistentClient,
    room: &matrix_sdk::Room,
    upstream: &str,
    sessions: &Path,
    state: &Path,
) -> anyhow::Result<PersistentClient> {
    for point in ["journal", "sdk", "decryption", "inbox", "checkpoint"] {
        let body = format!("crash window {point}");
        let event = room
            .send(RoomMessageEventContent::text_plain(&body))
            .await?
            .response
            .event_id;
        let before = m3_storage::queries::checkpoint(&b.pool, "matrix_sync").await?;
        drop(b);
        let status = tokio::process::Command::new(std::env::current_exe()?)
            .args(["crash-probe", "--homeserver", upstream, "--data"])
            .arg(state.join("bob"))
            .arg("--session")
            .arg(sessions.join("bob.json"))
            .args(["--point", point])
            .stdin(std::process::Stdio::null())
            .kill_on_drop(true)
            .status()
            .await?;
        ensure!(
            status.code() == Some(86),
            "Child did not terminate at {point}: {status}"
        );
        b = PersistentClient::open(upstream, &state.join("bob"), &sessions.join("bob.json"))
            .await?;
        let after = m3_storage::queries::checkpoint(&b.pool, "matrix_sync").await?;
        ensure!(
            (before == after) == (point != "checkpoint"),
            "Unexpected application checkpoint at {point}"
        );
        let captured: i64 =
            sqlx::query_scalar("SELECT count(*) FROM matrix_events WHERE event_id=?")
                .bind(event.as_str())
                .fetch_one(&b.pool)
                .await?;
        ensure!(captured == 1, "Ciphertext was not durable at {point}");
        loop {
            pump(a, &b).await?;
            let stored: Option<String> = sqlx::query_scalar(
                "SELECT body FROM matrix_events WHERE event_id=? AND state='ready'",
            )
            .bind(event.as_str())
            .fetch_optional(&b.pool)
            .await?;
            if let Some(stored) = stored {
                ensure!(stored == body, "Crash recovery changed text");
                break;
            }
        }
        // Explicit Client-Server replay cannot create a second delivery row.
        let raw: String = sqlx::query_scalar("SELECT raw_json FROM matrix_events WHERE event_id=?")
            .bind(event.as_str())
            .fetch_one(&b.pool)
            .await?;
        crate::matrix::capture(&b.pool,&json!({"next_batch":"fixture-replay","rooms":{"join":{room.room_id().as_str():{"timeline":{"events":[serde_json::from_str::<serde_json::Value>(&raw)?]}}}}}),None).await?;
        b.recover_events().await?;
        ensure!(
            sqlx::query_scalar::<_, i64>(
                "SELECT count(*) FROM matrix_events WHERE event_id=? AND state='ready'"
            )
            .bind(event.as_str())
            .fetch_one(&b.pool)
            .await?
                == 1,
            "Replay duplicated inbox input"
        );
        println!(
            "Crash {point}: child exit 86, durable ciphertext, checkpoint ordering, exact recovery and replay deduplication passed"
        );
    }
    Ok(b)
}
