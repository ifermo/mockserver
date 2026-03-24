# MockServer

基于 Rust axum 框架的 HTTP Mock 服务器，从 JSON 文件读取预定义请求-响应对并匹配返回。

## 特性

- 从 `spec.json` 读取预定义请求-响应对
- 支持路径参数（如 `/users/:id`）
- 支持 Header 和 Body 匹配
- 支持响应延迟配置
- SIGHUP 信号热重载
- 基于 tower 的请求-响应日志记录

## 快速开始

### 构建

```bash
cargo build --release
```

### 启动

```bash
./target/release/mockserver
```

服务器将监听 `0.0.0.0:3000`。

### 配置

编辑 `spec.json` 文件定义请求-响应对：

```json
[
    {
        "name": "get_user",
        "httpRequest": {
            "method": "GET",
            "path": "/api/user/:id"
        },
        "httpResponse": {
            "statusCode": 200,
            "headers": {
                "Content-Type": "application/json"
            },
            "body": "{\"id\":1,\"name\":\"张三\"}"
        }
    }
]
```

### 热重载

修改 `spec.json` 后，发送 SIGHUP 信号：

```bash
kill -HUP <pid>
```

服务器将自动重新加载配置。

## 测试

### 运行单元测试

```bash
cargo test
```

### 运行集成测试

先启动服务器：

```bash
./target/release/mockserver
```

然后运行集成测试：

```bash
chmod +x integration.sh
./integration.sh
```

### 性能压测

```bash
wrk -t4 -c100 -d10s http://localhost:3000/api/user/1
```

## API

### 请求匹配规则

1. 先比较 HTTP 方法和路径
2. 若 spec 定义了 headers，则逐键值比对
3. 若 spec 定义了 body，则按 Content-Type 进行结构化或文本比对
4. 所有条件满足时返回对应 response
5. 无匹配返回 404：`{"error":"no matching spec"}`

### 路径参数

使用 `:` 前缀定义路径参数：

```json
"path": "/api/users/:id"
```

匹配 `/api/users/1`、`/api/users/abc` 等。

### 正则路径

使用反斜杠转义：

```json
"path": "/api/products/\\d+"
```

### 响应延迟

```json
"httpResponse": {
    "delay": {
        "timeUnit": "SECONDS",
        "value": 2
    }
}
```

支持的单位：`MILLISECONDS`、`SECONDS`、`MINUTES`。

## 日志格式

```
method path status duration_ms
```

示例：

```
GET /api/user/1 200 5
```

## 项目结构

```
.
├── src/
│   ├── main.rs      # 主程序入口
│   ├── spec.rs      # Spec 加载和匹配逻辑
│   └── lib.rs       # 库入口
├── tests/
│   └── spec_tests.rs # 单元测试
├── spec.json        # 示例配置
├── integration.sh   # 集成测试脚本
├── Cargo.toml
└── README.md
```

## 依赖

- axum 0.7+
- tokio
- serde_json
- anyhow
- tower
- tower-http
- tracing
- regex
- mime