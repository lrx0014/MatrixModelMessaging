//! Persistent protocol prototype, without inference or autonomous social work.
use anyhow::{Context, ensure};
use axum::{
    Router,
    body::{Body, to_bytes},
    extract::State,
    http::{Request, StatusCode},
    response::{IntoResponse, Response},
};
use futures_util::{FutureExt, Stream, StreamExt};
use matrix_sdk::{
    Client,
    authentication::matrix::MatrixSession,
    config::SyncSettings,
    ruma::{OwnedRoomId, events::room::encrypted::OriginalSyncRoomEncryptedEvent, serde::Raw},
};
use serde_json::Value;
use sqlx::{Row, SqlitePool};
use std::pin::Pin;
use std::{
    fs::{self, File, OpenOptions},
    path::Path,
    sync::Arc,
    time::Duration,
};

#[derive(Clone, Copy, Debug, PartialEq, Eq, clap::ValueEnum)]
pub enum CrashPoint {
    Journal,
    Sdk,
    Decryption,
    Inbox,
    Checkpoint,
}
fn crash_at(configured: Option<CrashPoint>, point: CrashPoint) {
    if configured == Some(point) {
        std::process::exit(86);
    }
}

/// Negotiate a peer's SAS start without making a trust decision. The SDK
/// operation is idempotent outside Started; the request initiator may differ
/// from the SAS-phase initiator. Confirmation still requires actual comparison.
pub async fn accept_sas(
    sas: &matrix_sdk::encryption::verification::SasVerification,
) -> anyhow::Result<()> {
    sas.accept().await?;
    Ok(())
}

struct GatewayTask(tokio::task::JoinHandle<()>);
impl Drop for GatewayTask {
    fn drop(&mut self) {
        self.0.abort();
    }
}

pub struct PersistentClient {
    pub client: Client,
    pub pool: SqlitePool,
    _gateway: GatewayTask,
    _lock: File,
    crash: Option<CrashPoint>,
    keys: tokio::sync::Mutex<Pin<Box<dyn Stream<Item = ()> + Send>>>,
}
#[derive(Clone)]
struct Gateway {
    upstream: Arc<str>,
    http: reqwest::Client,
    pool: SqlitePool,
    crash: Option<CrashPoint>,
}

impl PersistentClient {
    pub async fn open(
        homeserver: &str,
        directory: &Path,
        session_path: &Path,
    ) -> anyhow::Result<Self> {
        Self::open_with_crash(homeserver, directory, session_path, None).await
    }
    pub async fn open_with_crash(
        homeserver: &str,
        directory: &Path,
        session_path: &Path,
        crash: Option<CrashPoint>,
    ) -> anyhow::Result<Self> {
        fs::create_dir_all(directory)?;
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            fs::set_permissions(directory, fs::Permissions::from_mode(0o700))?;
        }
        let lock = OpenOptions::new()
            .create(true)
            .truncate(false)
            .read(true)
            .write(true)
            .open(directory.join("runner.lock"))?;
        lock.try_lock()
            .context("Another Runner owns this device directory")?;
        let session: MatrixSession = serde_json::from_slice(&fs::read(session_path)?)
            .context("Invalid device session file")?;
        let pool =
            m3_storage::open(&directory.join("agent.db"), m3_storage::DatabaseKind::Agent).await?;
        let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await?;
        let address = listener.local_addr()?;
        let state = Gateway {
            upstream: Arc::from(homeserver.trim_end_matches('/')),
            http: reqwest::Client::builder()
                .redirect(reqwest::redirect::Policy::none())
                .timeout(Duration::from_secs(40))
                .build()?,
            pool: pool.clone(),
            crash,
        };
        let gateway = GatewayTask(tokio::spawn(async move {
            let _ = axum::serve(listener, Router::new().fallback(forward).with_state(state)).await;
        }));
        // The loopback gateway only journals the ordinary Client-Server response;
        // the SDK owns all protocol, device-message, and crypto processing.
        let result = Client::builder()
            .homeserver_url(format!("http://{address}"))
            .sqlite_store(directory.join("matrix"), None)
            .build()
            .await;
        let client = match result {
            Ok(client) => client,
            Err(error) => {
                gateway.0.abort();
                return Err(error.into());
            }
        };
        if let Err(error) = client.restore_session(session).await {
            gateway.0.abort();
            return Err(error.into());
        }
        let keys = client
            .encryption()
            .room_keys_received_stream()
            .await
            .context("SDK room-key notifications unavailable")?
            .map(|_| ());
        // A restart may follow crypto commit but precede the inbox retry reset.
        sqlx::query(
            "UPDATE matrix_events SET decryption_attempts=0 WHERE state='pending_decryption'",
        )
        .execute(&pool)
        .await?;
        Ok(Self {
            keys: tokio::sync::Mutex::new(Box::pin(keys)),
            client,
            pool,
            _gateway: gateway,
            _lock: lock,
            crash,
        })
    }

    pub async fn sync_once(&self) -> anyhow::Result<()> {
        let token = m3_storage::queries::checkpoint(&self.pool, "matrix_sync").await?;
        let settings = SyncSettings::default().timeout(Duration::from_millis(100));
        // Never reuse the SDK cursor implicitly: it can be ahead of M3's inbox.
        let settings = if let Some(token) = token {
            settings.token(token)
        } else {
            settings.token(matrix_sdk::config::SyncToken::NoToken)
        };
        let response = self.client.sync_once(settings).await?;
        crash_at(self.crash, CrashPoint::Sdk);
        self.recover_events().await?;
        crash_at(self.crash, CrashPoint::Inbox);
        let mut transaction = self.pool.begin().await?;
        sqlx::query("INSERT INTO checkpoints(stream,position,committed_at) VALUES ('matrix_sync',?,unixepoch()) ON CONFLICT(stream) DO UPDATE SET position=excluded.position,committed_at=excluded.committed_at").bind(&response.next_batch).execute(&mut *transaction).await?;
        sqlx::query("DELETE FROM sync_batches")
            .execute(&mut *transaction)
            .await?;
        transaction.commit().await?;
        crash_at(self.crash, CrashPoint::Checkpoint);
        Ok(())
    }

    pub async fn recover_events(&self) -> anyhow::Result<()> {
        let mut keys = self.keys.lock().await;
        let mut changed = false;
        while keys.next().now_or_never().flatten().is_some() {
            changed = true;
        }
        drop(keys);
        if changed {
            sqlx::query(
                "UPDATE matrix_events SET decryption_attempts=0 WHERE state='pending_decryption'",
            )
            .execute(&self.pool)
            .await?;
        }
        let rows=sqlx::query("SELECT event_id,room_id,raw_json FROM matrix_events WHERE state IN ('received','pending_decryption') AND decryption_attempts < 3").fetch_all(&self.pool).await?;
        for row in rows {
            let event_id: String = row.try_get("event_id")?;
            let room_id: OwnedRoomId = row.try_get::<String, _>("room_id")?.try_into()?;
            let mut event: Value = serde_json::from_str(row.try_get::<&str, _>("raw_json")?)?;
            if event["type"] == "m.room.encrypted" {
                let Some(room) = self.client.get_room(&room_id) else {
                    continue;
                };
                let raw: Raw<OriginalSyncRoomEncryptedEvent> =
                    Raw::from_json_string(serde_json::to_string(&event)?)?;
                let decrypted = room.decrypt_event(&raw, None).await?;
                if let matrix_sdk::deserialized_responses::TimelineEventKind::UnableToDecrypt {
                    utd_info,
                    ..
                } = &decrypted.kind
                {
                    let diagnostic = format!("SDK unable to decrypt: {:?}", utd_info.reason);
                    sqlx::query("UPDATE matrix_events SET state='pending_decryption',decryption_attempts=decryption_attempts+1,diagnostic=? WHERE event_id=?").bind(diagnostic).bind(&event_id).execute(&self.pool).await?;
                    continue;
                }
                event = serde_json::from_str(decrypted.raw().json().get())?;
                crash_at(self.crash, CrashPoint::Decryption);
            }
            let body = if event["type"] == "m.room.message"
                && event["content"]["msgtype"] != "m.key.verification.request"
            {
                event["content"]["body"].as_str()
            } else {
                None
            };
            // Bodies are preserved verbatim. Event identity, not text, deduplicates.
            sqlx::query("UPDATE matrix_events SET state=?,body=?,diagnostic=NULL WHERE event_id=? AND state IN ('received','pending_decryption')")
                .bind(if body.is_some(){"ready"}else{"ignored"}).bind(body).bind(event_id).execute(&self.pool).await?;
        }
        Ok(())
    }
    pub async fn fingerprint(&self) -> anyhow::Result<String> {
        self.client
            .encryption()
            .ed25519_key()
            .await
            .context("SDK device key unavailable")
    }
}

async fn forward(State(state): State<Gateway>, request: Request<Body>) -> Response {
    match forward_inner(state, request).await {
        Ok(response) => response,
        Err(error) => {
            eprintln!("Matrix gateway failure: {error}");
            (StatusCode::BAD_GATEWAY, "Matrix transport/journal failed").into_response()
        }
    }
}
async fn forward_inner(state: Gateway, request: Request<Body>) -> anyhow::Result<Response> {
    let (parts, body) = request.into_parts();
    ensure!(
        parts.uri.path().starts_with("/_matrix/"),
        "Not a Matrix path"
    );
    let url = format!(
        "{}{}",
        state.upstream,
        parts.uri.path_and_query().context("Missing path")?
    );
    let body = to_bytes(body, 32 * 1024 * 1024).await?;
    let mut outgoing = state.http.request(parts.method, &url).body(body);
    for name in ["authorization", "content-type"] {
        if let Some(value) = parts.headers.get(name) {
            outgoing = outgoing.header(name, value);
        }
    }
    let result = outgoing.send().await?;
    let status = result.status();
    let body = result.bytes().await?;
    if parts.uri.path().ends_with("/sync") && status.is_success() {
        let mut value: Value = serde_json::from_slice(&body)?;
        // Backfill every limited room using public history pagination. The
        // original response returned to SDK is unchanged; only the journal gains
        // earlier events. Pagination size is a transport setting, not a chat cap.
        for kind in ["join", "leave"] {
            if let Some(rooms) = value["rooms"][kind].as_object_mut() {
                for (room_id, room) in rooms {
                    if room["timeline"]["limited"] == true {
                        let mut token = room["timeline"]["prev_batch"]
                            .as_str()
                            .context("Limited timeline has no pagination token")?
                            .to_owned();
                        loop {
                            let mut endpoint = reqwest::Url::parse(&state.upstream)?;
                            endpoint
                                .path_segments_mut()
                                .map_err(|_| anyhow::anyhow!("Invalid homeserver URL"))?
                                .extend(["_matrix", "client", "v3", "rooms", room_id, "messages"]);
                            endpoint.query_pairs_mut().extend_pairs([
                                ("dir", "b"),
                                ("from", token.as_str()),
                                ("limit", "100"),
                            ]);
                            let mut request = state.http.get(endpoint);
                            if let Some(auth) = parts.headers.get("authorization") {
                                request = request.header("authorization", auth);
                            }
                            let page = request.send().await?;
                            ensure!(
                                page.status().is_success(),
                                "History backfill failed: HTTP {}",
                                page.status()
                            );
                            let page: Value = page.json().await?;
                            let chunk = page["chunk"].as_array().context("Invalid history page")?;
                            room["timeline"]["events"]
                                .as_array_mut()
                                .context("Missing timeline")?
                                .extend(chunk.iter().cloned());
                            let Some(end) = page["end"].as_str() else {
                                break;
                            };
                            if chunk.is_empty() {
                                break;
                            }
                            ensure!(end != token, "History pagination made no progress");
                            token = end.to_owned();
                        }
                    }
                }
            }
        }
        capture(
            &state.pool,
            &value,
            reqwest::Url::parse(&url)?
                .query_pairs()
                .find(|(k, _)| k == "since")
                .map(|(_, v)| v.into_owned())
                .as_deref(),
        )
        .await?;
        crash_at(state.crash, CrashPoint::Journal);
    }
    Ok((status, [("content-type", "application/json")], body).into_response())
}

/// Commit ciphertext and event metadata before returning a sync response to SDK.
/// If the process dies before crypto commit, the application cursor still points
/// to the previous batch. If SDK has advanced, retained ciphertext can be decrypted
/// from its persisted crypto state even when SDK suppresses that batch's replay.
pub async fn capture(pool: &SqlitePool, batch: &Value, since: Option<&str>) -> anyhow::Result<()> {
    let next = batch["next_batch"]
        .as_str()
        .context("Missing sync cursor")?;
    let mut transaction = pool.begin().await?;
    for kind in ["join", "leave"] {
        if let Some(rooms) = batch["rooms"][kind].as_object() {
            for (room_id, room) in rooms {
                if let Some(events) = room["timeline"]["events"].as_array() {
                    for event in events {
                        let id = event["event_id"]
                            .as_str()
                            .context("Timeline event has no ID")?;
                        let sender = event["sender"]
                            .as_str()
                            .context("Timeline event has no sender")?;
                        let time = event["origin_server_ts"]
                            .as_i64()
                            .context("Timeline event has no timestamp")?;
                        sqlx::query("INSERT INTO matrix_events(event_id,room_id,raw_json,state,sender,origin_server_ts) VALUES (?,?,?,'received',?,?) ON CONFLICT DO NOTHING")
                            .bind(id).bind(room_id).bind(serde_json::to_string(event)?).bind(sender).bind(time).execute(&mut *transaction).await?;
                    }
                }
            }
        }
    }
    sqlx::query(
        "INSERT INTO sync_batches(next_batch,since_token) VALUES (?,?) ON CONFLICT DO NOTHING",
    )
    .bind(next)
    .bind(since)
    .execute(&mut *transaction)
    .await?;
    transaction.commit().await?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;
    #[tokio::test]
    async fn capture_is_atomic_and_event_identity_preserves_repeated_speech() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("agent.db");
        let pool = m3_storage::open(&path, m3_storage::DatabaseKind::Agent)
            .await
            .unwrap();
        let event = |id: &str| json!({"event_id":id,"sender":"@a:m3.test","origin_server_ts":1,"type":"m.room.message","content":{"msgtype":"m.text","body":" identical \n"}});
        let batch = json!({"next_batch":"first","rooms":{"join":{"!room:m3.test":{"timeline":{"events":[event("$a"),event("$b")]}}}}});
        capture(&pool, &batch, None).await.unwrap();
        capture(&pool, &batch, None).await.unwrap();
        assert_eq!(
            sqlx::query_scalar::<_, i64>("SELECT count(*) FROM matrix_events")
                .fetch_one(&pool)
                .await
                .unwrap(),
            2
        );
        assert_eq!(
            m3_storage::queries::checkpoint(&pool, "matrix_sync")
                .await
                .unwrap(),
            None
        );
        let malformed = json!({"next_batch":"bad","rooms":{"join":{"!room:m3.test":{"timeline":{"events":[event("$c"),{"type":"m.room.message"}]}}}}});
        assert!(capture(&pool, &malformed, Some("first")).await.is_err());
        pool.close().await;
        let pool = m3_storage::open(&path, m3_storage::DatabaseKind::Agent)
            .await
            .unwrap();
        assert_eq!(
            sqlx::query_scalar::<_, i64>("SELECT count(*) FROM matrix_events")
                .fetch_one(&pool)
                .await
                .unwrap(),
            2
        );
        assert_eq!(
            sqlx::query_scalar::<_, i64>("SELECT count(*) FROM sync_batches")
                .fetch_one(&pool)
                .await
                .unwrap(),
            1
        );
        pool.close().await;
    }
}
