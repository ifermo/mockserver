//! MockServer 应用层模块
//!
//! 负责 HTTP 请求处理、路由配置和生命周期管理

use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::sync::Arc;
use std::time::Instant;

use anyhow::{Context, Result};
use axum::{
    Router,
    body::Body,
    extract::State,
    http::{Request, header::CONTENT_TYPE},
    response::Response,
    routing::any,
};
use tokio::signal;
use tokio::sync::RwLock;
use tower::ServiceBuilder;
use tower_http::trace::TraceLayer;

use crate::config::ServerConfig;
use mockserver::spec::{SharedSpecStore, match_request, update_spec_store};

/// 应用状态，包含规格存储和服务器配置
#[derive(Clone)]
pub struct AppState {
    pub spec_store: SharedSpecStore,
    pub config: ServerConfig,
}

/// 创建 Axum Router 实例
pub fn create_app(spec_store: SharedSpecStore, config: &ServerConfig) -> Router {
    let app_state = AppState {
        spec_store,
        config: config.clone(),
    };

    Router::new()
        .route("/", any(handle_request))
        .route("/{*path}", any(handle_request))
        .layer(
            ServiceBuilder::new()
                .layer(TraceLayer::new_for_http())
                .into_inner(),
        )
        .with_state(app_state)
}

/// 初始化规格存储
///
/// 1. 创建空的规格存储
/// 2. 从文件加载初始规格
/// 3. 编译并存储规格
pub async fn initialize_spec_store(spec_path: &Path) -> Result<SharedSpecStore> {
    let spec_store: SharedSpecStore = Arc::new(RwLock::new(Vec::new()));
    let initial_specs = mockserver::spec::load_specs_from_file(spec_path)
        .with_context(|| format!("Failed to load spec file: {}", spec_path.display()))?;
    update_spec_store(&spec_store, initial_specs)
        .await
        .context("Failed to compile initial specs")?;
    Ok(spec_store)
}

/// 处理 SIGTERM 信号，实现优雅关闭
pub async fn shutdown_signal() {
    if let Ok(mut signal) = signal::unix::signal(signal::unix::SignalKind::terminate()) {
        signal.recv().await;
        tracing::info!("Received SIGTERM, shutting down gracefully...");
    }
}

/// 处理 SIGHUP 信号，实现热重载
pub async fn handle_sighup(spec_store: SharedSpecStore, spec_path: PathBuf) {
    while let Ok(mut signal) = signal::unix::signal(signal::unix::SignalKind::hangup()) {
        signal.recv().await;
        tracing::info!("Received SIGHUP, reloading spec file...");
        reload_specs(&spec_store, &spec_path).await;
    }
}

/// 重新加载规格文件
async fn reload_specs(spec_store: &SharedSpecStore, spec_path: &Path) {
    match mockserver::spec::load_specs_from_file(spec_path) {
        Ok(new_specs) => {
            if let Err(e) = update_spec_store(spec_store, new_specs).await {
                tracing::error!(error = %e, "Failed to update spec store");
            } else {
                tracing::info!("Spec file reloaded successfully");
            }
        }
        Err(e) => {
            tracing::error!(error = %e, "Failed to reload spec file, continuing with existing specs");
        }
    }
}

/// 处理所有 HTTP 请求的核心处理函数
///
/// 流程：
/// 1. 提取请求信息（方法、路径、请求头、请求体）
/// 2. 与规格存储中的规则进行匹配
/// 3. 返回匹配的响应或 404
async fn handle_request(State(state): State<AppState>, request: Request<Body>) -> Response<Body> {
    let start = Instant::now();

    // 提取请求基本信息
    let method = request.method().to_string();
    let path = request.uri().path().to_string();

    // 解析请求体
    let (parts, body) = request.into_parts();
    let headers = extract_headers(&parts.headers);
    let content_type = parts
        .headers
        .get(CONTENT_TYPE)
        .and_then(|v| v.to_str().ok());
    let body_bytes = axum::body::to_bytes(body, state.config.body_limit)
        .await
        .unwrap_or_default();
    let body_text = String::from_utf8_lossy(&body_bytes);
    let body_json: Option<serde_json::Value> = serde_json::from_str(&body_text).ok();

    // 记录请求日志
    tracing::debug!(headers = ?headers, body = %body_text, "incoming request");

    // 匹配请求并获取响应
    let response = match_request(
        &state.spec_store,
        &method,
        &path,
        &headers,
        body_json.as_ref(),
        content_type,
    )
    .await;

    // 记录响应日志
    let duration_ms = start.elapsed().as_millis() as u64;
    log_request(&method, &path, response.as_ref(), duration_ms);

    response.unwrap_or_else(create_not_found_response)
}

/// 从 HTTP HeaderMap 提取键值对到 HashMap
fn extract_headers(headers: &axum::http::HeaderMap) -> HashMap<String, String> {
    headers
        .iter()
        .filter_map(|(name, value)| {
            value
                .to_str()
                .ok()
                .map(|v| (name.to_string(), v.to_string()))
        })
        .collect()
}

/// 记录请求日志
fn log_request(method: &str, path: &str, response: Option<&Response<Body>>, duration_ms: u64) {
    match response {
        Some(resp) => tracing::info!(
            method = %method,
            path = %path,
            status = %resp.status().as_u16(),
            duration_ms = %duration_ms,
            "request completed"
        ),
        None => tracing::info!(
            method = %method,
            path = %path,
            status = 404,
            duration_ms = %duration_ms,
            "no matching spec"
        ),
    }
}

/// 创建 404 Not Found 响应
fn create_not_found_response() -> Response<Body> {
    Response::builder()
        .status(404)
        .header(CONTENT_TYPE, "application/json")
        .body(Body::from(r#"{"error":"no matching spec"}"#))
        .unwrap_or_else(|_| Response::builder().status(404).body(Body::empty()).unwrap())
}
