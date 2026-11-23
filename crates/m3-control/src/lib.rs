use axum::{Json, Router, http::StatusCode, routing::get};
use serde::Serialize;
use utoipa::{OpenApi, ToSchema};

#[derive(Serialize, ToSchema)]
pub struct Health {
    pub service: &'static str,
    pub status: &'static str,
}

#[utoipa::path(get, path = "/health/live", responses((status = 200, body = Health)))]
async fn live() -> Json<Health> {
    Json(Health {
        service: "m3-control",
        status: "alive",
    })
}

#[utoipa::path(get, path = "/health/ready", responses((status = 503, description = "Management runtime is not implemented yet", body = Health)))]
async fn ready() -> (StatusCode, Json<Health>) {
    (
        StatusCode::SERVICE_UNAVAILABLE,
        Json(Health {
            service: "m3-control",
            status: "not_initialized",
        }),
    )
}

#[derive(OpenApi)]
#[openapi(paths(live, ready), components(schemas(Health)))]
pub struct ApiDoc;

pub fn router() -> Router {
    Router::new()
        .route("/health/live", get(live))
        .route("/health/ready", get(ready))
}

#[cfg(test)]
mod tests {
    use axum::{
        body::{Body, to_bytes},
        http::Request,
    };
    use tower::ServiceExt;

    use super::*;

    #[tokio::test]
    async fn liveness_does_not_claim_runtime_readiness() {
        for (path, status, value) in [
            ("/health/live", StatusCode::OK, "alive"),
            (
                "/health/ready",
                StatusCode::SERVICE_UNAVAILABLE,
                "not_initialized",
            ),
        ] {
            let response = router()
                .oneshot(Request::builder().uri(path).body(Body::empty()).unwrap())
                .await
                .unwrap();
            assert_eq!(response.status(), status);
            let body = to_bytes(response.into_body(), 1024).await.unwrap();
            let body: serde_json::Value = serde_json::from_slice(&body).unwrap();
            assert_eq!(body["service"], "m3-control");
            assert_eq!(body["status"], value);
        }
    }

    #[tokio::test]
    async fn management_actions_are_not_exposed_by_the_bootstrap_server() {
        let response = router()
            .oneshot(
                Request::builder()
                    .method("POST")
                    .uri("/api/v1/freeze-all")
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(response.status(), StatusCode::NOT_FOUND);
    }
}
