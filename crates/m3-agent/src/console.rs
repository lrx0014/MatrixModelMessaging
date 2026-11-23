//! Explicit operator-driven protocol console, never an autonomous chat loop.
use crate::matrix::PersistentClient;
use anyhow::{Context, bail};
use matrix_sdk::{
    encryption::verification::VerificationRequest,
    ruma::{
        OwnedDeviceId, OwnedRoomId, OwnedTransactionId, OwnedUserId,
        api::client::room::create_room, events::room::message::RoomMessageEventContent, serde::Raw,
    },
};
use serde_json::{Value, json};
use std::{collections::HashMap, path::Path, sync::Arc};
use tokio::io::{AsyncBufReadExt, BufReader};

pub async fn run(homeserver: &str, directory: &Path, session: &Path) -> anyhow::Result<()> {
    let runner = Arc::new(PersistentClient::open(homeserver, directory, session).await?);
    runner.client.add_event_handler(|event:matrix_sdk::ruma::events::key::verification::request::ToDeviceKeyVerificationRequestEvent| async move {
        println!("{}",json!({"verification_event":event.content.transaction_id,"user":event.sender}));
    });
    runner.client.add_event_handler(
        |event: matrix_sdk::ruma::events::room::message::OriginalSyncRoomMessageEvent| async move {
            if matches!(
                event.content.msgtype,
                matrix_sdk::ruma::events::room::message::MessageType::VerificationRequest(_)
            ) {
                println!(
                    "{}",
                    json!({"verification_event":event.event_id,"user":event.sender})
                );
            }
        },
    );
    runner.sync_once().await?;
    println!(
        "{}",
        json!({"mxid":runner.client.user_id(),"device":runner.client.device_id(),"ed25519":runner.fingerprint().await?})
    );
    let worker = runner.clone();
    let sync = tokio::spawn(async move {
        loop {
            if let Err(error) = worker.sync_once().await {
                eprintln!("Sync failed: {error}");
                tokio::time::sleep(std::time::Duration::from_secs(1)).await;
            }
        }
    });
    let mut lines = BufReader::new(tokio::io::stdin()).lines();
    let mut requests = HashMap::new();
    while let Some(line) = lines.next_line().await? {
        if line.trim().is_empty() {
            continue;
        }
        if line.trim() == "quit" {
            break;
        }
        match execute(&runner, &mut requests, &line).await {
            Ok(value) => println!("{value}"),
            Err(error) => eprintln!("Console command failed: {error}"),
        }
    }
    sync.abort();
    let _ = sync.await;
    Ok(())
}
fn text<'a>(value: &'a Value, key: &str) -> anyhow::Result<&'a str> {
    value[key]
        .as_str()
        .with_context(|| format!("Missing {key}"))
}
async fn execute(
    runner: &PersistentClient,
    requests: &mut HashMap<String, VerificationRequest>,
    line: &str,
) -> anyhow::Result<Value> {
    let command: Value = serde_json::from_str(line)?;
    let client = &runner.client;
    Ok(match text(&command, "op")? {
        "bootstrap" => {
            client.encryption().bootstrap_cross_signing(None).await?;
            json!({"cross_signing":"bootstrapped"})
        }
        "rooms" => json!(
            client
                .rooms()
                .iter()
                .map(|r| json!({"room_id":r.room_id(),"state":format!("{:?}",r.state())}))
                .collect::<Vec<_>>()
        ),
        "join" => {
            let room: OwnedRoomId = text(&command, "room")?.try_into()?;
            client.join_room_by_id(&room).await?;
            json!({"joined":room})
        }
        "create" => {
            let mut request = create_room::v3::Request::new();
            request.name = Some(
                command["name"]
                    .as_str()
                    .unwrap_or("M1 human-client test")
                    .to_owned(),
            );
            request.invite = command["users"]
                .as_array()
                .context("Missing users")?
                .iter()
                .map(|v| {
                    v.as_str()
                        .context("Invalid user")?
                        .try_into()
                        .map_err(Into::into)
                })
                .collect::<anyhow::Result<Vec<OwnedUserId>>>()?;
            request.is_direct = command["direct"] == true;
            if command["encrypted"] != false {
                request.initial_state.push(Raw::from_json_string(json!({"type":"m.room.encryption","state_key":"","content":{"algorithm":"m.megolm.v1.aes-sha2"}}).to_string())?);
            }
            let room = client.create_room(request).await?;
            json!({"created":room.room_id()})
        }
        "send" => {
            let room: OwnedRoomId = text(&command, "room")?.try_into()?;
            let txn = OwnedTransactionId::from(text(&command, "transaction")?);
            let event = client
                .get_room(&room)
                .context("Room unavailable")?
                .send(RoomMessageEventContent::text_plain(text(&command, "body")?))
                .with_transaction_id(txn)
                .await?;
            json!({"sent":event.response.event_id})
        }
        "event" => {
            let room: OwnedRoomId = text(&command, "room")?.try_into()?;
            let event: matrix_sdk::ruma::OwnedEventId = text(&command, "event")?.try_into()?;
            let event = client
                .get_room(&room)
                .context("Room unavailable")?
                .event(&event, None)
                .await?;
            json!({"unable_to_decrypt":event.kind.is_utd(),"event":serde_json::from_str::<Value>(event.raw().json().get())?})
        }
        "events" => {
            use sqlx::Row;
            let rows=sqlx::query("SELECT event_id,room_id,sender,state,body,diagnostic FROM matrix_events ORDER BY origin_server_ts,event_id").fetch_all(&runner.pool).await?;
            json!(rows.iter().map(|r|json!({"event":r.get::<String,_>("event_id"),"room":r.get::<String,_>("room_id"),"sender":r.get::<String,_>("sender"),"state":r.get::<String,_>("state"),"body":r.get::<Option<String>,_>("body"),"diagnostic":r.get::<Option<String>,_>("diagnostic")})).collect::<Vec<_>>())
        }
        "devices" => {
            let user: OwnedUserId = text(&command, "user")?.try_into()?;
            let devices = client.encryption().get_user_devices(&user).await?;
            json!(
                devices
                    .devices()
                    .map(|d| json!({"device":d.device_id(),"verified":d.is_verified()}))
                    .collect::<Vec<_>>()
            )
        }
        "verify-user" => {
            let user: OwnedUserId = text(&command, "user")?.try_into()?;
            let identity = client
                .encryption()
                .request_user_identity(&user)
                .await?
                .context("Peer has no cross-signing identity")?;
            let request = identity.request_verification().await?;
            let flow = request.flow_id().to_owned();
            requests.insert(flow.clone(), request);
            json!({"flow":flow})
        }
        "verification" => {
            let flow = text(&command, "flow")?;
            let request = requests.get(flow).context("Unknown local request")?;
            json!({"ready":request.is_ready(),"done":request.is_done(),"cancelled":request.is_cancelled()})
        }
        "verify" => {
            let user: OwnedUserId = text(&command, "user")?.try_into()?;
            let device: OwnedDeviceId = text(&command, "device")?.into();
            let device = client
                .encryption()
                .get_device(&user, &device)
                .await?
                .context("Unknown device; sync a shared room first")?;
            let request = device.request_verification().await?;
            let flow = request.flow_id().to_owned();
            requests.insert(flow.clone(), request);
            json!({"flow":flow})
        }
        "accept" | "start" | "sas" | "confirm" => {
            let user: OwnedUserId = text(&command, "user")?.try_into()?;
            let flow = text(&command, "flow")?;
            let op = text(&command, "op")?;
            if op == "accept" {
                let request = client
                    .encryption()
                    .get_verification_request(&user, flow)
                    .await
                    .context("Unknown verification request")?;
                request.accept().await?;
                requests.insert(flow.to_owned(), request);
                json!({"accepted":flow})
            } else if op == "start" {
                let request = requests.get(flow).context("Unknown local request")?;
                request.start_sas().await?.context("Not ready for SAS")?;
                json!({"started":flow})
            } else {
                let sas = client
                    .encryption()
                    .get_verification(&user, flow)
                    .await
                    .and_then(|v| v.sas())
                    .context("SAS not started")?;
                if op == "confirm" {
                    sas.confirm().await?;
                } else {
                    // SDK accept is a no-op outside Started. Request initiation
                    // does not determine which side started the SAS phase.
                    crate::matrix::accept_sas(&sas).await?;
                }
                json!({"decimals":sas.decimals(),"emoji":sas.emoji().map(|e|e.map(|v|v.symbol.to_owned())),"done":sas.is_done(),"cancelled":sas.is_cancelled(),"we_started":sas.we_started()})
            }
        }
        _ => bail!("Unknown operation"),
    })
}
