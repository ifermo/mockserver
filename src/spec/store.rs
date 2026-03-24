//! 规格存储管理模块
//!
//! 负责规格的加载、更新和运行时管理
//! 支持热重载功能

use std::path::Path;

use anyhow::{Context, Result};
use std::sync::Arc;
use tokio::sync::RwLock;

use super::compiler::{SpecWithPattern, compile_specs};
use super::models::Spec;

/// 共享规格存储类型
pub type SharedSpecStore = Arc<RwLock<Vec<SpecWithPattern>>>;

/// 从文件加载规格列表
pub fn load_specs_from_file<P: AsRef<Path>>(path: P) -> Result<Vec<Spec>> {
    let content = std::fs::read_to_string(path.as_ref())
        .with_context(|| format!("Failed to read spec file: {}", path.as_ref().display()))?;
    let specs: Vec<Spec> = serde_json::from_str(&content).with_context(|| {
        format!(
            "Failed to parse spec JSON from: {}",
            path.as_ref().display()
        )
    })?;
    Ok(specs)
}

/// 更新规格存储
///
/// 将新的规格列表编译后替换存储中的内容
pub async fn update_spec_store(store: &SharedSpecStore, specs: Vec<Spec>) -> Result<()> {
    let compiled = compile_specs(specs)?;
    let mut locked = store.write().await;
    *locked = compiled;
    Ok(())
}

/// 从文件重新加载规格
pub async fn reload_from_file(store: &SharedSpecStore, spec_path: &Path) -> Result<()> {
    let specs = load_specs_from_file(spec_path)
        .with_context(|| format!("Failed to load spec file: {}", spec_path.display()))?;
    update_spec_store(store, specs).await
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::spec::{HttpRequest, HttpResponse};

    #[tokio::test]
    async fn test_update_spec_store() {
        let specs = vec![Spec {
            name: "initial".to_string(),
            http_request: HttpRequest {
                method: "GET".to_string(),
                path: "/initial".to_string(),
                headers: None,
                body: None,
            },
            http_response: HttpResponse {
                status_code: 200,
                headers: None,
                body: "initial".to_string(),
                delay: None,
            },
        }];

        let store: SharedSpecStore = Arc::new(RwLock::new(Vec::new()));
        update_spec_store(&store, specs).await.unwrap();

        let locked = store.read().await;
        assert_eq!(locked.len(), 1);
        assert_eq!(locked[0].spec.http_request.path, "/initial");
    }

    #[tokio::test]
    async fn test_hot_reload_update() {
        let specs = vec![Spec {
            name: "initial".to_string(),
            http_request: HttpRequest {
                method: "GET".to_string(),
                path: "/initial".to_string(),
                headers: None,
                body: None,
            },
            http_response: HttpResponse {
                status_code: 200,
                headers: None,
                body: "initial".to_string(),
                delay: None,
            },
        }];

        let store: SharedSpecStore = Arc::new(RwLock::new(Vec::new()));
        update_spec_store(&store, specs).await.unwrap();

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

        update_spec_store(&store, new_specs).await.unwrap();

        let locked = store.read().await;
        assert_eq!(locked.len(), 1);
        assert_eq!(locked[0].spec.http_request.path, "/reloaded");
    }
}
