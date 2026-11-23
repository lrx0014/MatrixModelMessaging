//! Authenticated AS protocol boundary. Social input belongs only to Runner sync.
use std::sync::Arc;

use axum::{
    Router,
    body::{Body, to_bytes},
    extract::{Path, State},
    http::{Request, StatusCode},
    response::{IntoResponse, Response},
    routing::{get, post, put},
};
use ruma::{
    UserId,
    api::{
        IncomingRequest,
        appservice::{
            event::push_events,
            ping::send_ping,
            query::{query_room_alias, query_user_id},
        },
    },
};
use sqlx::SqlitePool;

#[derive(Clone)]
pub struct Appservice {
    pub hs_token: Arc<str>,
    pub server_name: Arc<str>,
    pub pool: SqlitePool,
}

impl Appservice {
    pub fn owns(&self, user: &UserId) -> bool {
        user.server_name().as_str() == self.server_name.as_ref()
            && user.localpart().strip_prefix("_m3_").is_some_and(|id| {
                !id.is_empty()
                    && id
                        .bytes()
                        .all(|c| c.is_ascii_lowercase() || c.is_ascii_digit() || c == b'_')
            })
    }
}

pub fn router(state: Appservice) -> Router {
    Router::new()
        .route("/_matrix/app/v1/transactions/{txn}", put(transaction))
        .route("/_matrix/app/v1/users/{user}", get(user))
        .route("/_matrix/app/v1/rooms/{alias}", get(alias))
        .route("/_matrix/app/v1/ping", post(ping))
        .with_state(state)
}

type Failure = (StatusCode, axum::Json<serde_json::Value>);
fn failure(status: StatusCode, code: &str) -> Failure {
    (
        status,
        axum::Json(serde_json::json!({"errcode":code,"error":code})),
    )
}
async fn parse<T: IncomingRequest>(
    state: &Appservice,
    request: Request<Body>,
    args: &[String],
) -> Result<T, Failure> {
    // Authenticate before parsing or retaining any homeserver content. Tokens in
    // query strings are intentionally unsupported to keep them out of URLs/logs.
    let expected = format!("Bearer {}", state.hs_token);
    if request
        .headers()
        .get("authorization")
        .and_then(|v| v.to_str().ok())
        != Some(expected.as_str())
    {
        return Err(failure(StatusCode::UNAUTHORIZED, "M_UNKNOWN_TOKEN"));
    }
    let (parts, body) = request.into_parts();
    let body = to_bytes(body, 8 * 1024 * 1024)
        .await
        .map_err(|_| failure(StatusCode::PAYLOAD_TOO_LARGE, "M_TOO_LARGE"))?;
    T::try_from_http_request(Request::from_parts(parts, body), args)
        .map_err(|_| failure(StatusCode::BAD_REQUEST, "M_BAD_JSON"))
}
async fn transaction(
    State(state): State<Appservice>,
    Path(txn): Path<String>,
    request: Request<Body>,
) -> Result<axum::Json<serde_json::Value>, Failure> {
    let request: push_events::v1::Request = parse(&state, request, &[txn]).await?;
    // Persist acknowledgement before returning 200. No message content or social
    // work is created by AS delivery, including redelivery of the same batch.
    sqlx::query("INSERT INTO as_transactions(transaction_id,event_count) VALUES (?,?) ON CONFLICT DO NOTHING")
        .bind(request.txn_id.as_str()).bind(request.events.len() as i64).execute(&state.pool).await
        .map_err(|_| failure(StatusCode::INTERNAL_SERVER_ERROR,"M_UNKNOWN"))?;
    Ok(axum::Json(serde_json::json!({})))
}
async fn ping(
    State(state): State<Appservice>,
    request: Request<Body>,
) -> Result<axum::Json<serde_json::Value>, Failure> {
    let _: send_ping::v1::Request = parse(&state, request, &[]).await?;
    Ok(axum::Json(serde_json::json!({})))
}
async fn user(
    State(state): State<Appservice>,
    Path(user): Path<String>,
    request: Request<Body>,
) -> Response {
    let result = async {
        let request: query_user_id::v1::Request = parse(&state, request, &[user]).await?;
        let exists: i64 = sqlx::query_scalar("SELECT count(*) FROM as_users WHERE user_id = ?")
            .bind(request.user_id.as_str())
            .fetch_one(&state.pool)
            .await
            .map_err(|_| failure(StatusCode::INTERNAL_SERVER_ERROR, "M_UNKNOWN"))?;
        if !state.owns(&request.user_id) || exists != 1 {
            return Err(failure(StatusCode::NOT_FOUND, "M_NOT_FOUND"));
        }
        Ok(axum::Json(serde_json::json!({})))
    }
    .await;
    result.into_response()
}
async fn alias(
    State(state): State<Appservice>,
    Path(alias): Path<String>,
    request: Request<Body>,
) -> Result<axum::Json<serde_json::Value>, Failure> {
    let _: query_room_alias::v1::Request = parse(&state, request, &[alias]).await?;
    Err(failure(StatusCode::NOT_FOUND, "M_NOT_FOUND"))
}

#[cfg(test)]
mod tests {
    use super::*;
    use tower::ServiceExt;
    #[tokio::test]
    async fn authentication_redelivery_and_queries_are_durable() {
        let directory = tempfile::tempdir().unwrap();
        let pool = m3_storage::open(
            &directory.path().join("control.db"),
            m3_storage::DatabaseKind::Control,
        )
        .await
        .unwrap();
        sqlx::query("INSERT INTO as_users VALUES ('@_m3_alice:m3.test')")
            .execute(&pool)
            .await
            .unwrap();
        let app = router(Appservice {
            hs_token: Arc::from("fixture-hs-token"),
            server_name: Arc::from("m3.test"),
            pool: pool.clone(),
        });
        for (method, path, body, token, status) in [
            (
                "PUT",
                "/_matrix/app/v1/transactions/one",
                r#"{"events":[]}"#,
                false,
                401,
            ),
            (
                "PUT",
                "/_matrix/app/v1/transactions/one",
                r#"{"events":[]}"#,
                true,
                200,
            ),
            (
                "PUT",
                "/_matrix/app/v1/transactions/one",
                r#"{"events":[]}"#,
                true,
                200,
            ),
            ("PUT", "/_matrix/app/v1/transactions/bad", "{}", true, 400),
            ("POST", "/_matrix/app/v1/ping", "{}", true, 200),
            (
                "GET",
                "/_matrix/app/v1/users/@_m3_alice:m3.test",
                "",
                true,
                200,
            ),
            (
                "GET",
                "/_matrix/app/v1/users/@_m3_unknown:m3.test",
                "",
                true,
                404,
            ),
            (
                "GET",
                "/_matrix/app/v1/users/@_m3_alice:elsewhere.test",
                "",
                true,
                404,
            ),
            (
                "GET",
                "/_matrix/app/v1/rooms/%23unknown:m3.test",
                "",
                true,
                404,
            ),
        ] {
            let mut request = Request::builder()
                .method(method)
                .uri(path)
                .header("content-type", "application/json");
            if token {
                request = request.header("authorization", "Bearer fixture-hs-token");
            }
            let response = app
                .clone()
                .oneshot(request.body(Body::from(body)).unwrap())
                .await
                .unwrap();
            assert_eq!(response.status().as_u16(), status, "{path}");
        }
        assert_eq!(
            sqlx::query_scalar::<_, i64>("SELECT count(*) FROM as_transactions")
                .fetch_one(&pool)
                .await
                .unwrap(),
            1
        );
        pool.close().await;
    }
}
