//! MockServer 核心规格模块
//!
//! 提供 HTTP 请求匹配和响应模拟功能
//!
//! # 模块结构
//!
//! - [`models`] - 数据模型定义
//! - [`compiler`] - 路径模式编译
//! - [`matcher`] - 请求匹配逻辑
//! - [`store`] - 规格存储管理

use std::collections::HashMap;

use axum::{body::Body, response::Response};

pub mod compiler;
pub mod matcher;
pub mod models;
pub mod store;

pub use compiler::{compile_specs, SpecWithPattern};
pub use models::{Delay, HttpRequest, HttpResponse, Spec};
pub use store::{load_specs_from_file, update_spec_store, SharedSpecStore};

/// 匹配请求并返回响应
///
/// 按顺序匹配：HTTP 方法 -> 路径 -> 请求头 -> 请求体
///
/// # 参数
///
/// * `store` - 规格存储
/// * `method` - HTTP 方法
/// * `path` - 请求路径
/// * `headers` - 请求头
/// * `body` - 请求体（可选）
/// * `_content_type` - 内容类型（预留参数，当前未使用）
///
/// # 返回值
///
/// * `Some(Response)` - 找到匹配的规格，返回对应响应
/// * `None` - 没有找到匹配的规格
pub async fn match_request<'a>(
    store: &'a SharedSpecStore,
    method: &str,
    path: &str,
    headers: &HashMap<String, String>,
    body: Option<&serde_json::Value>,
    _content_type: Option<&str>,
) -> Option<Response<Body>> {
    let specs = store.read().await;
    matcher::match_spec(&specs, method, path, headers, body).await
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::Arc;
    use tokio::sync::RwLock;

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
                        "body": "{\"id\":1,\"name\":\"张三\"}"
                    }
                },
                {
                    "httpRequest": {
                        "method": "POST",
                        "path": "/api/login",
                        "headers": {"Content-Type": "application/json"},
                        "body": {"username":"admin","password":"123456"}
                    },
                    "httpResponse": {
                        "statusCode": 200,
                        "body": "{\"token\":\"fake-jwt-token\"}"
                    }
                }
            ]"#,
        )
        .unwrap()
    }

    #[test]
    fn test_load_specs_from_file() {
        let specs = create_test_specs();
        assert_eq!(specs.len(), 2);
        assert_eq!(specs[0].http_request.method, "GET");
        assert_eq!(specs[0].http_response.status_code, 200);
    }

    #[test]
    fn test_compile_specs() {
        let specs = create_test_specs();
        let compiled = compile_specs(specs).unwrap();
        assert_eq!(compiled.len(), 2);
        assert!(compiled[0].path_regex.is_match("/api/user/123"));
    }

    #[tokio::test]
    async fn test_no_match_returns_none() {
        let specs = create_test_specs();
        let compiled = compile_specs(specs).unwrap();
        let store: SharedSpecStore = Arc::new(RwLock::new(compiled));

        let headers = HashMap::new();
        let result = match_request(&store, "DELETE", "/unknown", &headers, None, None).await;
        assert!(result.is_none());
    }

    #[tokio::test]
    async fn test_method_mismatch() {
        let specs = create_test_specs();
        let compiled = compile_specs(specs).unwrap();
        let store: SharedSpecStore = Arc::new(RwLock::new(compiled));

        let headers = HashMap::new();
        let result = match_request(&store, "POST", "/api/user/1", &headers, None, None).await;
        assert!(result.is_none());
    }
}
