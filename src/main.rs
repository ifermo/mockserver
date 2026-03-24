//! MockServer 主程序入口
//!
//! 一个基于 axum 框架的 HTTP 模拟服务器
//! 根据配置文件匹配请求并返回预设的响应

mod app;
mod config;

use std::path::Path;

use anyhow::{Context, Result};
use tracing_subscriber::{fmt, EnvFilter};

use crate::app::{create_app, handle_sighup, initialize_spec_store, shutdown_signal};

#[tokio::main]
async fn main() -> Result<()> {
    // 初始化日志系统
    let filter = EnvFilter::try_from_default_env().unwrap_or_else(|_| EnvFilter::new("info"));
    fmt().with_env_filter(filter).with_target(false).init();

    // 加载服务器配置
    let config_path = Path::new("config.toml");
    let config = config::load_config(config_path)?;

    // 加载规格文件
    let spec_path = Path::new(&config.spec_path);
    tracing::info!("Mock server starting on {}", config.server_addr);

    // 创建 TCP 监听器
    let listener = tokio::net::TcpListener::bind(&config.server_addr)
        .await
        .context("Failed to bind to port")?;

    // 初始化规格存储
    let spec_store = initialize_spec_store(spec_path).await?;

    // 启动热重载任务（SIGHUP 信号处理）
    tokio::spawn(handle_sighup(spec_store.clone(), spec_path.to_path_buf()));

    // 创建并启动服务器
    let app = create_app(spec_store, &config);
    axum::serve(listener, app)
        .with_graceful_shutdown(shutdown_signal())
        .await
        .context("Server error")?;

    Ok(())
}
