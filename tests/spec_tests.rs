use anyhow::Result;
use axum::{
    body::Body,
    extract::State,
    http::{header::CONTENT_TYPE, Request},
    response::Response,
    routing::any,
    Router,
};
use mockserver::spec::{self, HttpRequest, HttpResponse, SharedSpecStore, Spec};
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::RwLock;
use tower::ServiceBuilder;
use tower::ServiceExt;

#[derive(Clone)]
struct AppState {
    spec_store: SharedSpecStore,
}

async fn handle_request(State(state): State<AppState>, request: Request<Body>) -> Response<Body> {
    let method = request.method().to_string();
    let path = request.uri().path().to_string();

    let (parts, body) = request.into_parts();

    let mut headers = HashMap::new();
    for (name, value) in parts.headers.iter() {
        if let Ok(v) = value.to_str() {
            headers.insert(name.to_string(), v.to_string());
        }
    }

    let content_type = parts
        .headers
        .get(CONTENT_TYPE)
        .and_then(|v| v.to_str().ok());

    let body_bytes = axum::body::to_bytes(body, 65536).await.unwrap_or_default();
    let body_str = String::from_utf8_lossy(&body_bytes);
    let body_json: Option<serde_json::Value> = serde_json::from_str(&body_str).ok();

    spec::match_request(
        &state.spec_store,
        &method,
        &path,
        &headers,
        body_json.as_ref(),
        content_type,
    )
    .await
    .unwrap_or_else(|| {
        Response::builder()
            .status(404)
            .header(CONTENT_TYPE, "application/json")
            .body(Body::from(r#"{"error":"no matching spec"}"#))
            .unwrap()
    })
}

fn create_test_app(spec_store: SharedSpecStore) -> Router {
    let app_state = AppState { spec_store };

    Router::new()
        .route("/", any(handle_request))
        .route("/{*path}", any(handle_request))
        .layer(ServiceBuilder::new().into_inner())
        .with_state(app_state)
}

fn create_test_specs() -> Vec<Spec> {
    serde_json::from_str(
        r#"[
        {
            "httpRequest": {
                "method": "GET",
                "path": "/api/user/:id"
            },
            "httpResponse": {
                "statusCode": 200,
                "headers": {
                    "Content-Type": "application/json"
                },
                "body": "{\"id\":1,\"name\":\"张三\",\"email\":\"zhangsan@example.com\"}"
            }
        },
        {
            "httpRequest": {
                "method": "POST",
                "path": "/api/login",
                "body": {"username":"admin","password":"123456"}
            },
            "httpResponse": {
                "statusCode": 200,
                "body": "{\"token\":\"fake-jwt-token\",\"expires_in\":3600}"
            }
        },
        {
            "httpRequest": {
                "method": "POST",
                "path": "/api/login"
            },
            "httpResponse": {
                "statusCode": 401,
                "body": "{\"error\":\"用户名或密码错误\"}"
            }
        },
        {
            "httpRequest": {
                "method": "GET",
                "path": "/api/products/[0-9]+"
            },
            "httpResponse": {
                "statusCode": 200,
                "body": "{\"id\":1,\"name\":\"商品名称\",\"price\":99.00}",
                "delay": {
                    "timeUnit": "SECONDS",
                    "value": 2
                }
            }
        }
    ]"#,
    )
    .unwrap()
}

#[tokio::test]
async fn test_normal_matching() -> Result<()> {
    let specs = create_test_specs();
    let compiled = spec::compile_specs(specs)?;
    let store: SharedSpecStore = Arc::new(RwLock::new(compiled));
    let app = create_test_app(store);

    let response = app
        .clone()
        .oneshot(
            Request::builder()
                .method("GET")
                .uri("/api/user/1")
                .body(Body::empty())?,
        )
        .await?;

    assert_eq!(response.status(), 200);
    let body = axum::body::to_bytes(response.into_body(), 65536).await?;
    let body_str = String::from_utf8_lossy(&body);
    assert!(body_str.contains("zhangsan@example.com"));

    Ok(())
}

#[tokio::test]
async fn test_path_parameter_matching() -> Result<()> {
    let specs = create_test_specs();
    let compiled = spec::compile_specs(specs)?;
    let store: SharedSpecStore = Arc::new(RwLock::new(compiled));
    let app = create_test_app(store);

    let response = app
        .clone()
        .oneshot(
            Request::builder()
                .method("GET")
                .uri("/api/user/123")
                .body(Body::empty())?,
        )
        .await?;

    assert_eq!(response.status(), 200);

    Ok(())
}

#[tokio::test]
async fn test_header_body_matching() -> Result<()> {
    let specs = create_test_specs();
    let compiled = spec::compile_specs(specs)?;
    let store: SharedSpecStore = Arc::new(RwLock::new(compiled));
    let app = create_test_app(store);

    let response = app
        .clone()
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/api/login")
                .header(CONTENT_TYPE, "application/json")
                .body(Body::from(r#"{"username":"admin","password":"123456"}"#))?,
        )
        .await?;

    assert_eq!(response.status(), 200);
    let body = axum::body::to_bytes(response.into_body(), 65536).await?;
    let body_str = String::from_utf8_lossy(&body);
    assert!(body_str.contains("fake-jwt-token"));

    Ok(())
}

#[tokio::test]
async fn test_no_match_returns_404() -> Result<()> {
    let specs = create_test_specs();
    let compiled = spec::compile_specs(specs)?;
    let store: SharedSpecStore = Arc::new(RwLock::new(compiled));
    let app = create_test_app(store);

    let response = app
        .clone()
        .oneshot(
            Request::builder()
                .method("GET")
                .uri("/unknown/path")
                .body(Body::empty())?,
        )
        .await?;

    assert_eq!(response.status(), 404);
    let body = axum::body::to_bytes(response.into_body(), 65536).await?;
    let body_str = String::from_utf8_lossy(&body);
    assert!(body_str.contains("no matching spec"));

    Ok(())
}

#[tokio::test]
async fn test_method_mismatch() -> Result<()> {
    let specs = create_test_specs();
    let compiled = spec::compile_specs(specs)?;
    let store: SharedSpecStore = Arc::new(RwLock::new(compiled));
    let app = create_test_app(store);

    let response = app
        .clone()
        .oneshot(
            Request::builder()
                .method("DELETE")
                .uri("/api/user/1")
                .body(Body::empty())?,
        )
        .await?;

    assert_eq!(response.status(), 404);

    Ok(())
}

#[tokio::test]
async fn test_hot_reload() -> Result<()> {
    let specs = create_test_specs();
    let compiled = spec::compile_specs(specs)?;
    let store: SharedSpecStore = Arc::new(RwLock::new(compiled));

    let new_specs = vec![Spec {
        name: "reloaded".to_string(),
        http_request: HttpRequest {
            method: "GET".to_string(),
            path: "/reloaded".to_string(),
            headers: None,
            body: None,
        },
        http_response: HttpResponse {
            status_code: 200,
            headers: None,
            body: "reloaded".to_string(),
            delay: None,
        },
    }];

    spec::update_spec_store(&store, new_specs).await?;

    let locked = store.read().await;
    assert_eq!(locked.len(), 1);
    assert_eq!(locked[0].spec.http_request.path, "/reloaded");

    Ok(())
}

#[tokio::test]
async fn test_delay_response() -> Result<()> {
    let specs = create_test_specs();
    let compiled = spec::compile_specs(specs)?;
    let store: SharedSpecStore = Arc::new(RwLock::new(compiled));
    let app = create_test_app(store);

    let start = std::time::Instant::now();
    let response = app
        .clone()
        .oneshot(
            Request::builder()
                .method("GET")
                .uri("/api/products/123")
                .body(Body::empty())?,
        )
        .await?;
    let elapsed = start.elapsed();

    assert_eq!(response.status(), 200);
    assert!(elapsed.as_secs() >= 1);

    Ok(())
}
