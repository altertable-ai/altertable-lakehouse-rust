# altertable-lakehouse-rust

Official Rust SDK for the Altertable Lakehouse API.

## Features

- Typed client for `append`, `get_task`, `query`, `query_all`, `upload`, `get_query`, `cancel_query`, `validate`, and `autocomplete`
- Basic auth via direct credentials, pre-encoded token, or environment discovery
- Streamed NDJSON query support with accumulated `query_all`
- `reqwest` + `rustls` transport with keep-alive and sensible timeout defaults
- Mock-backed integration coverage via Testcontainers for query, query_all, get_query, cancel_query, validate, autocomplete, append, and upload
- Request-level coverage for serialization, auth, request validation, and query parsing

## Installation

```toml
[dependencies]
altertable-lakehouse = "0.1.0"
```

## Authentication

The client supports:

- direct credentials
- pre-encoded Basic token
- environment variables:
  - `ALTERTABLE_USERNAME`
  - `ALTERTABLE_PASSWORD`
  - `ALTERTABLE_BASIC_AUTH_TOKEN`

## Usage

```rust
use altertable_lakehouse::{AltertableClient, QueryRequest};

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let client = AltertableClient::builder()
        .credentials("testuser", "testpass")
        .base_url("http://localhost:15000")
        .build()?;

    let result = client
        .query_all(&QueryRequest {
            statement: "select 1 as value".into(),
            ..Default::default()
        })
        .await?;

    println!("rows: {}", result.rows.len());
    Ok(())
}
```

### append

```rust
use altertable_lakehouse::{AltertableClient, AppendRequest};
use serde_json::json;
use std::collections::HashMap;

# #[tokio::main]
# async fn main() -> Result<(), Box<dyn std::error::Error>> {
let client = AltertableClient::builder()
    .credentials("testuser", "testpass")
    .base_url("http://localhost:15000")
    .build()?;

let payload = HashMap::from([
    ("id".to_string(), json!(1)),
    ("name".to_string(), json!("Ada")),
]);

client
    .append(
        "demo",
        "public",
        "users",
        Some(false),
        &AppendRequest::Single(payload),
    )
    .await?;
# Ok(()) }
```

### get_task

```rust
# use altertable_lakehouse::{AltertableClient, AppendRequest};
# use serde_json::json;
# use std::collections::HashMap;
# #[tokio::main]
# async fn main() -> Result<(), Box<dyn std::error::Error>> {
# let client = AltertableClient::builder().credentials("testuser", "testpass").base_url("http://localhost:15000").build()?;
let payload = HashMap::from([("id".to_string(), json!(1))]);
let append = client
    .append("demo", "public", "users", Some(false), &AppendRequest::Single(payload))
    .await?;

if let Some(task_id) = append.task_id {
    let task = client.get_task(&task_id.to_string()).await?;
    println!("task status = {:?}", task.status);
}
# Ok(()) }
```

### query

```rust
use altertable_lakehouse::{AltertableClient, QueryRequest};
use futures_util::StreamExt;

# #[tokio::main]
# async fn main() -> Result<(), Box<dyn std::error::Error>> {
let client = AltertableClient::builder()
    .credentials("testuser", "testpass")
    .base_url("http://localhost:15000")
    .build()?;

let mut result = client
    .query(&QueryRequest {
        statement: "select 1 as value".into(),
        ..Default::default()
    })
    .await?;

while let Some(row) = result.rows.next().await {
    println!("row = {:?}", row?);
}
# Ok(()) }
```

### query_all

```rust
# use altertable_lakehouse::{AltertableClient, QueryRequest};
# #[tokio::main]
# async fn main() -> Result<(), Box<dyn std::error::Error>> {
# let client = AltertableClient::builder().credentials("testuser", "testpass").base_url("http://localhost:15000").build()?;
let result = client
    .query_all(&QueryRequest {
        statement: "select 1 as value".into(),
        ..Default::default()
    })
    .await?;

assert!(!result.rows.is_empty());
# Ok(()) }
```

### get_query

```rust
# use altertable_lakehouse::AltertableClient;
# #[tokio::main]
# async fn main() -> Result<(), Box<dyn std::error::Error>> {
# let client = AltertableClient::builder().credentials("testuser", "testpass").base_url("http://localhost:15000").build()?;
let log = client
    .get_query("123e4567-e89b-12d3-a456-426614174000")
    .await?;
println!("query {}", log.log.uuid);
# Ok(()) }
```

### cancel_query

```rust
# use altertable_lakehouse::AltertableClient;
# #[tokio::main]
# async fn main() -> Result<(), Box<dyn std::error::Error>> {
# let client = AltertableClient::builder().credentials("testuser", "testpass").base_url("http://localhost:15000").build()?;
client
    .cancel_query(
        "123e4567-e89b-12d3-a456-426614174000",
        "session-123",
    )
    .await?;
# Ok(()) }
```

### upload

```rust
# use altertable_lakehouse::{AltertableClient, UploadMode};
# #[tokio::main]
# async fn main() -> Result<(), Box<dyn std::error::Error>> {
# let client = AltertableClient::builder().credentials("testuser", "testpass").base_url("http://localhost:15000").build()?;
client
    .upload(
        "demo",
        "public",
        "users",
        Some(UploadMode::Append),
        None,
        Some("text/csv"),
        b"id,name\n1,Ada\n".to_vec(),
    )
    .await?;
# Ok(()) }
```

### validate

```rust
# use altertable_lakehouse::{AltertableClient, ValidateRequest};
# #[tokio::main]
# async fn main() -> Result<(), Box<dyn std::error::Error>> {
# let client = AltertableClient::builder().credentials("testuser", "testpass").base_url("http://localhost:15000").build()?;
let response = client
    .validate(&ValidateRequest {
        statement: "select 1".into(),
        ..Default::default()
    })
    .await?;

assert!(response.valid);
# Ok(()) }
```

### autocomplete

```rust
# use altertable_lakehouse::{AltertableClient, AutocompleteRequest};
# #[tokio::main]
# async fn main() -> Result<(), Box<dyn std::error::Error>> {
# let client = AltertableClient::builder().credentials("testuser", "testpass").base_url("http://localhost:15000").build()?;
let response = client
    .autocomplete(&AutocompleteRequest {
        statement: "SEL".into(),
        max_suggestions: Some(5),
        ..Default::default()
    })
    .await?;

println!("suggestions = {}", response.suggestions.len());
# Ok(()) }
```

## Validation

Use `/usr/bin` before the Swift toolchain shims when building locally on this host:

```bash
PATH=/usr/bin:/bin:$PATH cargo fmt --all
PATH=/usr/bin:/bin:$PATH cargo clippy --all-targets --all-features -- -D warnings
PATH=/usr/bin:/bin:$PATH cargo test --all-features
```

The integration suite starts `ghcr.io/altertable-ai/altertable-mock:latest` automatically with Testcontainers and runs against the mapped `localhost` port.
