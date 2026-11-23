//! Test-only delayed delivery of real to-device packets. No cryptography is mocked.
use axum::{
    Router,
    body::{Body, to_bytes},
    extract::State,
    http::{Request, StatusCode},
    response::{IntoResponse, Response},
};
use serde_json::Value;
use std::sync::Arc;
use tokio::sync::Mutex;
#[derive(Clone)]
struct Transport {
    upstream: String,
    http: reqwest::Client,
    packets: Arc<Mutex<(bool, Vec<Value>)>>,
}
pub struct DelayedTransport {
    pub url: String,
    state: Transport,
    task: tokio::task::JoinHandle<()>,
}
impl Drop for DelayedTransport {
    fn drop(&mut self) {
        self.task.abort();
    }
}
impl DelayedTransport {
    pub async fn start(upstream: &str) -> anyhow::Result<Self> {
        let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await?;
        let url = format!("http://{}", listener.local_addr()?);
        let state = Transport {
            upstream: upstream.trim_end_matches('/').to_owned(),
            http: reqwest::Client::new(),
            packets: Arc::new(Mutex::new((false, Vec::new()))),
        };
        let server = state.clone();
        let task = tokio::spawn(async move {
            let _ = axum::serve(listener, Router::new().fallback(forward).with_state(server)).await;
        });
        Ok(Self { url, state, task })
    }
    pub async fn hold(&self, value: bool) {
        self.state.packets.lock().await.0 = value;
    }
}
async fn forward(State(state): State<Transport>, request: Request<Body>) -> Response {
    match inner(state, request).await {
        Ok(response) => response,
        Err(_) => (StatusCode::BAD_GATEWAY, "Fault fixture transport failed").into_response(),
    }
}
async fn inner(state: Transport, request: Request<Body>) -> anyhow::Result<Response> {
    let (parts, body) = request.into_parts();
    let sync = parts.uri.path().ends_with("/sync");
    let mut request = state
        .http
        .request(
            parts.method,
            format!("{}{}", state.upstream, parts.uri.path_and_query().unwrap()),
        )
        .body(to_bytes(body, 32 * 1024 * 1024).await?);
    for key in ["authorization", "content-type"] {
        if let Some(value) = parts.headers.get(key) {
            request = request.header(key, value);
        }
    }
    let response = request.send().await?;
    let status = response.status();
    let mut body = response.bytes().await?.to_vec();
    if sync && status.is_success() {
        let mut batch: Value = serde_json::from_slice(&body)?;
        let mut packets = state.packets.lock().await;
        if batch.get("to_device").is_none() {
            batch["to_device"] = serde_json::json!({"events":[]});
        }
        let events = batch["to_device"]["events"]
            .as_array_mut()
            .ok_or_else(|| anyhow::anyhow!("Missing to-device list"))?;
        if packets.0 {
            packets.1.append(events);
        } else {
            events.append(&mut packets.1);
        }
        body = serde_json::to_vec(&batch)?;
    }
    Ok((status, [("content-type", "application/json")], body).into_response())
}
