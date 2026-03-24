# AGENTS.md - MockServer Development Guide

This document provides guidance for AI agents working with the MockServer codebase.

## Project Overview

MockServer is a Rust-based HTTP mock server built with axum that matches incoming requests against predefined specs and returns configured responses.

## Build, Lint, and Test Commands

### Build
```bash
# Debug build
cargo build

# Release build
cargo build --release

# Build with all features
cargo build --all-features
```

### Linting and Formatting
```bash
# Format code
cargo fmt

# Check formatting (without modifying)
cargo fmt -- --check

# Run clippy lints
cargo clippy --all-targets

# Fix clippy warnings automatically
cargo clippy --fix --allow-dirty
```

### Running Tests

#### Run All Tests
```bash
# All tests (unit + integration)
cargo test

# With output captured
cargo test -- --nocapture
```

#### Run a Single Test
```bash
# Run a specific test by name (partial match works)
cargo test test_load_specs_from_file
cargo test test_normal_matching

# Run a specific test with full path
cargo test spec_tests::test_normal_matching -- --nocapture

# Run only unit tests (tests within src/)
cargo test --lib

# Run only integration tests (tests in tests/)
cargo test --test '*'
```

#### Run Tests with Logging
```bash
# Show tracing output during tests
RUST_LOG=debug cargo test
```

### Running the Server
```bash
# Development
cargo run

# Release
./target/release/mockserver
```

## Code Style Guidelines

### Formatting
- Use `cargo fmt` for automatic formatting
- 4-space indentation (Rust standard)
- Line length: let the formatter decide

### Imports
- Group imports by external crates first, then standard library, then local
- Use `use` statements for frequently used items
- Example:
```rust
use anyhow::{Context, Result};
use axum::{body::Body, extract::State, Router};
use std::collections::HashMap;
use std::sync::Arc;

use crate::config::ServerConfig;
use mockserver::spec::{self, SharedSpecStore};
```

### Naming Conventions
- **Types/Structs/Enums**: `PascalCase` (e.g., `HttpRequest`, `SpecWithPattern`)
- **Functions/Methods**: `snake_case` (e.g., `load_specs_from_file`, `match_request`)
- **Variables/Parameters**: `snake_case` (e.g., `spec_store`, `body_limit`)
- **Constants**: `SCREAMING_SNAKE_CASE` (e.g., `DEFAULT_SERVER_ADDR`)
- **Modules**: `snake_case` (e.g., `spec`, `config`)

### Type Conventions
- Use `Arc<RwLock<T>>` for shared mutable state
- Prefer `&str` over `&String` in function signatures
- Use `Option<T>` for nullable values, not `null`
- Use `Result<T, E>` for fallible operations

### Error Handling
- Use `anyhow::Result<T>` for application-level errors
- Use `.context()` to add context to errors
- Use `.with_context(|| ...)` for lazy context evaluation
- Example:
```rust
pub fn load_specs_from_file<P: AsRef<Path>>(path: P) -> Result<Vec<Spec>> {
    let content = std::fs::read_to_string(path.as_ref())
        .with_context(|| format!("Failed to read spec file: {}", path.as_ref().display()))?;
    // ...
}
```

### Async/Await
- Mark async test functions with `#[tokio::test]`
- Use `tokio::time::sleep` for delays in async code
- Use `.await` to execute futures

### Struct and Enum Definitions
- Derive `Debug`, `Clone` for most structs
- Use `#[serde(rename = "camelCase")]` for JSON field mapping
- Use `#[serde(default)]` for optional fields
- Example:
```rust
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct HttpRequest {
    pub method: String,
    pub path: String,
    #[serde(default)]
    pub headers: Option<HashMap<String, String>>,
    #[serde(default)]
    pub body: Option<serde_json::Value>,
}
```

### Test Conventions
- Unit tests go in `#[cfg(test)] mod tests` within source files
- Integration tests go in `tests/` directory
- Use `create_test_specs()` helper for test data
- Use `assert!`, `assert_eq!` for assertions
- Mark async tests with `#[tokio::test]`

### Documentation
- Document public functions with doc comments
- Document struct fields with comments
- Use Chinese comments in this codebase (as per existing style)

### Key Dependencies
- **axum**: HTTP framework
- **tokio**: Async runtime
- **serde**: Serialization/deserialization
- **anyhow**: Error handling
- **tower/tower-http**: Middleware layers
- **tracing**: Logging

### Module Structure
```
src/
├── main.rs      # Binary entry point
├── lib.rs       # Library entry (exports spec module)
├── app.rs       # Application setup, request handling
├── config.rs    # Configuration loading
└── spec.rs      # Spec parsing and matching logic
```

### Common Patterns

#### Shared State Pattern
```rust
pub type SharedSpecStore = Arc<RwLock<Vec<SpecWithPattern>>>;

// Reading
let specs = store.read().await;

// Writing
let mut locked = store.write().await;
*locked = new_specs;
```

#### Response Building
```rust
fn create_not_found_response() -> Response<Body> {
    Response::builder()
        .status(404)
        .header(CONTENT_TYPE, "application/json")
        .body(Body::from(r#"{"error":"no matching spec"}"#))
        .unwrap_or_else(|_| Response::builder().status(404).body(Body::empty()).unwrap())
}
```

#### Match Request Pattern
```rust
pub async fn match_request(
    store: &SharedSpecStore,
    method: &str,
    path: &str,
    headers: &HashMap<String, String>,
    body: Option<&serde_json::Value>,
    content_type: Option<&str>,
) -> Option<Response<Body>> {
    // ...
}
```

### Configuration File
- TOML format for config.toml
- JSON format for spec.json
- Use `#[serde(default = "function")]` for default values
