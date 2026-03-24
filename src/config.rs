//! MockServer 配置模块
//!
//! 负责从 config.toml 文件加载服务器配置

use anyhow::{Context, Result};
use serde::Deserialize;
use std::path::Path;

/// 默认服务器监听地址
const DEFAULT_SERVER_ADDR: &str = "0.0.0.0:3000";
/// 默认请求体大小限制（字节）
const DEFAULT_BODY_LIMIT: usize = 65536;
/// 默认规格文件路径
const DEFAULT_SPEC_PATH: &str = "spec.json";

/// 服务器配置结构体
///
/// 支持从 TOML 文件加载，所有字段都有默认值
#[derive(Debug, Clone, Deserialize)]
pub struct ServerConfig {
    /// 服务器监听地址，格式为 "host:port"
    #[serde(default = "default_server_addr")]
    pub server_addr: String,
    /// 请求体大小限制（字节），超过此大小的请求会被截断
    #[serde(default = "default_body_limit")]
    pub body_limit: usize,
    /// Mock 规格配置文件路径
    #[serde(default = "default_spec_path")]
    pub spec_path: String,
}

/// 提供默认配置值的函数
impl Default for ServerConfig {
    fn default() -> Self {
        Self {
            server_addr: default_server_addr(),
            body_limit: default_body_limit(),
            spec_path: default_spec_path(),
        }
    }
}

fn default_server_addr() -> String {
    DEFAULT_SERVER_ADDR.to_string()
}

fn default_body_limit() -> usize {
    DEFAULT_BODY_LIMIT
}

fn default_spec_path() -> String {
    DEFAULT_SPEC_PATH.to_string()
}

/// 从配置文件加载服务器配置
///
/// 如果配置文件不存在或解析失败，返回默认配置并记录警告日志
///
/// # 参数
///
/// * `config_path` - 配置文件路径
///
/// # 返回值
///
/// 成功返回配置，失败返回默认配置
pub fn load_config(config_path: &Path) -> Result<ServerConfig> {
    match std::fs::read_to_string(config_path) {
        Ok(content) => {
            let config: ServerConfig = toml::from_str(&content).with_context(|| {
                format!("Failed to parse config file: {}", config_path.display())
            })?;
            tracing::info!(
                "Loaded config from {}: server_addr={}, body_limit={}",
                config_path.display(),
                config.server_addr,
                config.body_limit
            );
            Ok(config)
        }
        Err(e) => {
            tracing::warn!(
                "Failed to read config file {}: {}, using default config",
                config_path.display(),
                e
            );
            Ok(ServerConfig::default())
        }
    }
}
