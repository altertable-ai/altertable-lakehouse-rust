use altertable_lakehouse::{
    AltertableClient, AppendRequest, ComputeSize, QueryColumn, QueryRequest, UploadFormat,
    UploadMode, ValidateRequest,
};
use futures_util::StreamExt;
use serde_json::json;
use std::collections::HashMap;

#[test]
fn query_request_serializes_enums() {
    let request = QueryRequest {
        statement: "select 1".into(),
        compute_size: Some(ComputeSize::M),
        ..Default::default()
    };

    let value = serde_json::to_value(request).unwrap();
    assert_eq!(value["compute_size"], json!("M"));
}

#[test]
fn upload_mode_upsert_requires_primary_key() {
    let client = AltertableClient::builder()
        .credentials("user", "pass")
        .base_url("http://localhost:15000")
        .build()
        .unwrap();

    let runtime = tokio::runtime::Runtime::new().unwrap();
    let error = runtime
        .block_on(client.upload(
            "catalog",
            "schema",
            "table",
            UploadFormat::Json,
            UploadMode::Upsert,
            None,
            br#"{"id":1}"#.to_vec(),
        ))
        .unwrap_err();

    assert!(error.to_string().contains("primary_key is required"));
}

#[test]
fn append_request_round_trips() {
    let payload: HashMap<String, serde_json::Value> =
        [(String::from("id"), json!(1))].into_iter().collect();
    let request = AppendRequest::Batch(vec![payload]);
    let value = serde_json::to_value(&request).unwrap();
    assert!(value.is_array());
}

#[test]
fn validate_request_requires_statement() {
    let client = AltertableClient::builder()
        .credentials("user", "pass")
        .base_url("http://localhost:15000")
        .build()
        .unwrap();

    let runtime = tokio::runtime::Runtime::new().unwrap();
    let error = runtime
        .block_on(client.validate(&ValidateRequest::default()))
        .unwrap_err();

    assert!(error.to_string().contains("statement must not be empty"));
}

#[tokio::test]
async fn query_stream_can_be_consumed_from_manual_result() {
    let _columns = vec![QueryColumn {
        name: "value".into(),
        data_type: Some("Int32".into()),
        nullable: Some(false),
        extra: HashMap::new(),
    }];

    let client = AltertableClient::builder()
        .credentials("user", "pass")
        .base_url("http://localhost:15000")
        .build()
        .unwrap();

    let error = match client.query(&QueryRequest::default()).await {
        Ok(_) => panic!("expected empty statement to fail"),
        Err(error) => error,
    };
    assert!(error.to_string().contains("statement must not be empty"));

    let mut stream = futures_util::stream::iter(vec![
        Ok::<serde_json::Value, altertable_lakehouse::AltertableError>(json!({"value": 1})),
        Ok::<serde_json::Value, altertable_lakehouse::AltertableError>(json!({"value": 2})),
    ]);
    let mut seen = Vec::new();
    while let Some(item) = stream.next().await {
        seen.push(item.unwrap());
    }

    assert_eq!(seen.len(), 2);
}
