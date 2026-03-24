//! 请求匹配模块
//!
//! 实现 HTTP 请求与规格的匹配逻辑
//! 匹配顺序：HTTP 方法 -> 路径 -> 请求头 -> 请求体

use std::collections::HashMap;

use axum::{body::Body, response::Response};

use super::compiler::SpecWithPattern;
use super::models::Spec;

/// 检查请求头是否匹配
fn headers_match(
    required: Option<&HashMap<String, String>>,
    actual: &HashMap<String, String>,
) -> bool {
    match required {
        Some(req_headers) => req_headers
            .iter()
            .all(|(key, value)| actual.get(key) == Some(value)),
        None => true,
    }
}

/// 检查请求体是否匹配
///
/// 匹配规则：
/// - required 为 None：始终匹配
/// - required 为 "*" 或空字符串：匹配任意值
/// - required 与 actual 完全相等
fn body_match(required: Option<&serde_json::Value>, actual: Option<&serde_json::Value>) -> bool {
    match (required, actual) {
        (Some(req_val), Some(act_val)) => values_equal(req_val, act_val),
        (Some(req_val), None) => is_wildcard_or_empty(req_val),
        (None, _) => true,
    }
}

/// 判断 JSON 值是否为通配符（"*" 或空字符串）
fn is_wildcard_or_empty(value: &serde_json::Value) -> bool {
    matches!(value, serde_json::Value::String(s) if s == "*" || s.is_empty())
}

/// 比较两个 JSON 值是否相等
fn values_equal(required: &serde_json::Value, actual: &serde_json::Value) -> bool {
    match (required, actual) {
        (serde_json::Value::String(req_s), serde_json::Value::String(act_s)) => {
            req_s == "*" || req_s.is_empty() || req_s == act_s
        }
        _ => required == actual,
    }
}

/// 在规格列表中查找匹配的规格
///
/// 按顺序检查：HTTP 方法、路径、请求头、请求体
fn find_matching_spec<'a>(
    mut specs: std::slice::Iter<'a, SpecWithPattern>,
    method: &str,
    path: &str,
    headers: &HashMap<String, String>,
    body: Option<&serde_json::Value>,
) -> Option<&'a Spec> {
    specs
        .find(|spec_with_pattern| {
            let spec = &spec_with_pattern.spec;
            let http_req = &spec.http_request;

            http_req.method == method
                && spec_with_pattern.path_regex.is_match(path)
                && headers_match(http_req.headers.as_ref(), headers)
                && body_match(http_req.body.as_ref(), body)
        })
        .map(|s| &s.spec)
}

/// 构建 HTTP 响应
async fn build_response(spec: &Spec) -> Response<Body> {
    tracing::debug!(name = spec.name, "matched spec.");

    let mut builder = Response::builder().status(spec.http_response.status_code);

    if let Some(ref headers) = spec.http_response.headers {
        for (key, value) in headers {
            builder = builder.header(key.as_str(), value.as_str());
        }
    }

    if let Some(ref delay) = spec.http_response.delay {
        let delay_ms = parse_delay_ms(delay);
        tokio::time::sleep(tokio::time::Duration::from_millis(delay_ms)).await;
    }

    builder
        .body(Body::from(spec.http_response.body.clone()))
        .unwrap_or_else(|_| Response::builder().status(500).body(Body::empty()).unwrap())
}

/// 解析延迟时间配置
fn parse_delay_ms(delay: &super::models::Delay) -> u64 {
    match delay.time_unit.to_uppercase().as_str() {
        "MILLISECONDS" | "MILLISECOND" => delay.value,
        "SECONDS" | "SECOND" => delay.value.saturating_mul(1000),
        "MINUTES" | "MINUTE" => delay.value.saturating_mul(60 * 1000),
        _ => delay.value,
    }
}

/// 执行请求匹配并返回响应
///
/// 返回值：
/// - Some(Response): 找到匹配的规格，返回对应的响应
/// - None: 没有找到匹配的规格
pub async fn match_spec<'a>(
    specs: &[SpecWithPattern],
    method: &str,
    path: &str,
    headers: &HashMap<String, String>,
    body: Option<&serde_json::Value>,
) -> Option<Response<Body>> {
    let spec = find_matching_spec(specs.iter(), method, path, headers, body)?;
    Some(build_response(spec).await)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::spec::compiler::compile_specs;

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

    #[tokio::test]
    async fn test_no_match_returns_none() {
        let specs = create_test_specs();
        let compiled = compile_specs(specs).unwrap();
        let headers = HashMap::new();

        let result = match_spec(&compiled, "DELETE", "/unknown", &headers, None).await;
        assert!(result.is_none());
    }

    #[tokio::test]
    async fn test_method_mismatch() {
        let specs = create_test_specs();
        let compiled = compile_specs(specs).unwrap();
        let headers = HashMap::new();

        let result = match_spec(&compiled, "POST", "/api/user/1", &headers, None).await;
        assert!(result.is_none());
    }
}
