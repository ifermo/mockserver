//! 路径模式编译模块
//!
//! 将用户定义的路径模式（如 `/api/user/:id`）转换为正则表达式
//! 支持的参数形式：
//! - `:param` - 路径参数，匹配任意非斜杠字符
//! - `[regex]` - 正则表达式模式

use anyhow::{Context, Result};
use regex::Regex;

use super::models::Spec;

/// 带编译后正则表达式的规格结构
#[derive(Debug, Clone)]
pub struct SpecWithPattern {
    pub spec: Spec,
    pub path_regex: Regex,
}

/// 判断路径是否为正则表达式模式
fn is_regex_path(path: &str) -> bool {
    path.contains('[') || path.contains('(') || path.contains('\\')
}

/// 将普通路径模式转换为正则表达式
///
/// 转换规则：
/// - `:param` 形式转换为 `[^/]+`（匹配任意非斜杠字符）
/// - 普通路径段进行转义
fn convert_path_to_regex(path: &str) -> String {
    let segments: Vec<&str> = path.split('/').collect();
    let regex_parts: String = segments
        .iter()
        .filter(|s| !s.is_empty())
        .map(|segment| {
            if segment.starts_with(':') {
                r"/[^/]+".to_string()
            } else {
                format!("/{}", regex::escape(segment))
            }
        })
        .collect();

    format!("^{}$", regex_parts)
}

/// 构建路径正则表达式
fn build_path_regex(path: &str) -> Result<Regex> {
    let pattern = if is_regex_path(path) {
        path.to_string()
    } else {
        convert_path_to_regex(path)
    };

    Regex::new(&pattern).with_context(|| format!("Failed to build regex from path: {}", path))
}

/// 将规格列表编译为带正则表达式的规格列表
pub fn compile_specs(specs: Vec<Spec>) -> Result<Vec<SpecWithPattern>> {
    specs
        .into_iter()
        .map(|spec| {
            let regex_pattern = build_path_regex(&spec.http_request.path)?;
            Ok(SpecWithPattern {
                spec,
                path_regex: regex_pattern,
            })
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::spec::{HttpRequest, HttpResponse};

    #[test]
    fn test_is_regex_path() {
        assert!(is_regex_path("/api/products/[0-9]+"));
        assert!(is_regex_path("/api/test(.*)"));
        assert!(!is_regex_path("/api/user/:id"));
        assert!(!is_regex_path("/api/users"));
    }

    #[test]
    fn test_convert_to_regex() {
        assert_eq!(convert_path_to_regex("/api/user/:id"), "^/api/user/[^/]+$");
        assert_eq!(convert_path_to_regex("/api/users"), "^/api/users$");
    }

    #[test]
    fn test_path_parameter_matching() {
        let specs = vec![Spec {
            name: "path_parameter".to_string(),
            http_request: HttpRequest {
                method: "GET".to_string(),
                path: "/api/user/:id".to_string(),
                headers: None,
                body: None,
            },
            http_response: HttpResponse {
                status_code: 200,
                headers: None,
                body: "test".to_string(),
                delay: None,
            },
        }];
        let compiled = compile_specs(specs).unwrap();
        assert!(compiled[0].path_regex.is_match("/api/user/1"));
        assert!(compiled[0].path_regex.is_match("/api/user/abc"));
        assert!(!compiled[0].path_regex.is_match("/api/user/"));
    }
}
