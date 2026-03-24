//! MockServer 数据模型定义
//!
//! 定义 HTTP 请求/响应的数据结构，用于配置模拟服务的行为

use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// 延迟配置结构体
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct Delay {
    #[serde(rename = "timeUnit")]
    pub time_unit: String,
    pub value: u64,
}

/// HTTP 请求规格结构体
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct HttpRequest {
    pub method: String,
    pub path: String,
    #[serde(default)]
    pub headers: Option<HashMap<String, String>>,
    #[serde(default)]
    pub body: Option<serde_json::Value>,
}

/// HTTP 响应规格结构体
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct HttpResponse {
    #[serde(rename = "statusCode")]
    pub status_code: u16,
    #[serde(default)]
    pub headers: Option<HashMap<String, String>>,
    pub body: String,
    #[serde(default)]
    pub delay: Option<Delay>,
}

/// Mock 规格定义，包含请求匹配规则和对应的响应
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct Spec {
    pub name: String,
    #[serde(rename = "httpRequest")]
    pub http_request: HttpRequest,
    #[serde(rename = "httpResponse")]
    pub http_response: HttpResponse,
}
