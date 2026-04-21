use altertable_lakehouse::{
    AltertableClient, AppendRequest, QueryRequest, UploadFormat, UploadMode, ValidateRequest,
};
use futures_util::StreamExt;
use serde_json::json;
use std::collections::HashMap;
use testcontainers::{
    core::{ImageExt, IntoContainerPort},
    runners::AsyncRunner,
    ContainerAsync, GenericImage,
};

const IMAGE_NAME: &str = "ghcr.io/altertable-ai/altertable-mock";
const IMAGE_TAG: &str = "latest";
const INTERNAL_PORT: u16 = 15000;

async fn spawn_mock() -> (ContainerAsync<GenericImage>, String) {
    let image = GenericImage::new(IMAGE_NAME, IMAGE_TAG)
        .with_exposed_port(INTERNAL_PORT.tcp())
        .with_env_var("ALTERTABLE_MOCK_USERS", "testuser:testpass");

    let container: ContainerAsync<GenericImage> =
        image.start().await.expect("failed to start mock container");
    let port = container
        .get_host_port_ipv4(INTERNAL_PORT)
        .await
        .expect("failed to read mapped port");
    let base_url = format!("http://127.0.0.1:{port}");
    (container, base_url)
}

fn client(base_url: String) -> AltertableClient {
    AltertableClient::builder()
        .credentials("testuser", "testpass")
        .base_url(base_url)
        .build()
        .expect("failed to create client")
}

#[tokio::test]
async fn validate_and_query_endpoints_work_against_mock() {
    let (_container, base_url) = spawn_mock().await;
    let client = client(base_url);

    let validate = client
        .validate(&ValidateRequest {
            statement: "SELECT 1".into(),
            ..Default::default()
        })
        .await
        .expect("validate should succeed");
    assert!(validate.valid);
    assert_eq!(validate.statement, "SELECT 1");

    let mut query = client
        .query(&QueryRequest {
            statement: "SELECT 1".into(),
            ..Default::default()
        })
        .await
        .expect("query should succeed");

    assert_eq!(query.metadata.values["statement"], json!("SELECT 1"));

    let mut rows = Vec::new();
    while let Some(row) = query.rows.next().await {
        rows.push(row.expect("row should parse"));
    }
    assert_eq!(rows, vec![json!(["1"]), json!([1])]);

    let query_all = client
        .query_all(&QueryRequest {
            statement: "SELECT 1".into(),
            ..Default::default()
        })
        .await
        .expect("query_all should succeed");
    assert_eq!(query_all.rows, vec![json!(["1"]), json!([1])]);
}

#[tokio::test]
async fn query_log_and_cancel_endpoints_work_against_mock() {
    let (_container, base_url) = spawn_mock().await;
    let client = client(base_url);

    let query = client
        .query_all(&QueryRequest {
            statement: "SELECT 1".into(),
            ..Default::default()
        })
        .await
        .expect("query_all should succeed");

    let query_id = query.metadata.values["query_id"]
        .as_str()
        .expect("query_id should be present")
        .to_string();
    let session_id = query.metadata.values["session_id"]
        .as_str()
        .expect("session_id should be present")
        .to_string();

    let log = client
        .get_query(&query_id)
        .await
        .expect("get_query should succeed");
    assert_eq!(log.log.query, "SELECT 1");
    assert_eq!(log.log.session_id, session_id);

    let cancel = client
        .cancel_query(&query_id, &session_id)
        .await
        .expect("cancel_query should succeed");
    assert!(!cancel.message.trim().is_empty());
}

#[tokio::test]
async fn append_and_upload_return_mock_responses() {
    let (_container, base_url) = spawn_mock().await;
    let client = client(base_url);

    let payload: HashMap<String, serde_json::Value> = [(String::from("event"), json!("signup"))]
        .into_iter()
        .collect();
    let append = client
        .append("demo", "public", "events", &AppendRequest::Single(payload))
        .await
        .expect("append should respond");
    assert!(!append.ok);
    assert!(append.error_code.is_some());

    let upload_error = client
        .upload(
            "demo",
            "public",
            "events",
            UploadFormat::Csv,
            UploadMode::Append,
            None,
            b"id,name\n1,Ada\n".to_vec(),
        )
        .await
        .expect_err("upload should fail against missing catalog");
    assert!(upload_error
        .to_string()
        .contains("Catalog \"demo\" does not exist"));
}
