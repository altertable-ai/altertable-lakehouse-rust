use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::collections::HashMap;
use uuid::Uuid;

pub type AppendPayload = HashMap<String, Value>;
pub type QueryRow = Value;

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(untagged)]
pub enum AppendRequest {
    Single(AppendPayload),
    Batch(Vec<AppendPayload>),
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum AppendErrorCode {
    InvalidData,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct AppendResponse {
    pub ok: bool,
    #[serde(default)]
    pub error_code: Option<AppendErrorCode>,
    #[serde(default)]
    pub error_message: Option<String>,
    #[serde(default)]
    pub task_id: Option<Uuid>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum ComputeSize {
    S,
    M,
    L,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum UploadFormat {
    Csv,
    Json,
    Parquet,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum UploadMode {
    Create,
    Append,
    Upsert,
    Overwrite,
}

#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
pub struct QueryRequest {
    pub statement: String,
    #[serde(default)]
    pub cache: Option<bool>,
    #[serde(default)]
    pub catalog: Option<String>,
    #[serde(default)]
    pub compute_size: Option<ComputeSize>,
    #[serde(default)]
    pub ephemeral: Option<bool>,
    #[serde(default)]
    pub limit: Option<u64>,
    #[serde(default)]
    pub offset: Option<u64>,
    #[serde(default)]
    pub query_id: Option<String>,
    #[serde(default)]
    pub requested_by: Option<String>,
    #[serde(default)]
    pub sanitize: Option<bool>,
    #[serde(default)]
    pub schema: Option<String>,
    #[serde(default)]
    pub session_id: Option<String>,
    #[serde(default)]
    pub timezone: Option<String>,
    #[serde(default)]
    pub visible: Option<bool>,
}

#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
pub struct ValidateRequest {
    pub statement: String,
    #[serde(default)]
    pub catalog: Option<String>,
    #[serde(default)]
    pub schema: Option<String>,
    #[serde(default)]
    pub session_id: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ValidateResponse {
    pub valid: bool,
    pub statement: String,
    pub connections_errors: HashMap<String, String>,
    #[serde(default)]
    pub error: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct CancelQueryResponse {
    pub cancelled: bool,
    pub message: String,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct TaskResponse {
    pub task_id: Uuid,
    pub status: TaskStatus,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum TaskStatus {
    Pending,
    Completed,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum SessionKind {
    ArrowFlightSQL,
    HttpQuery,
    HttpCancel,
    HttpValidate,
    HttpExplain,
    HttpAutocomplete,
    Postgres,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct QueryLog {
    pub uuid: Uuid,
    pub start_time: String,
    #[serde(default)]
    pub end_time: Option<String>,
    #[serde(default)]
    pub duration_ms: Option<u64>,
    pub query: String,
    pub client_interface: SessionKind,
    pub visible: bool,
    pub session_id: String,
    #[serde(default)]
    pub error: Option<String>,
    #[serde(default)]
    pub requested_by: Option<String>,
    #[serde(default)]
    pub user_agent: Option<String>,
    #[serde(default)]
    pub stats: Option<QueryStats>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Progress {
    pub percentage: f64,
    pub rows_processed: u64,
    pub total_rows: u64,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct QueryLogResponse {
    #[serde(flatten)]
    pub log: QueryLog,
    #[serde(default)]
    pub progress: Option<Progress>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct QueryStats {
    #[serde(default)]
    pub caching: Option<CachingStats>,
    #[serde(default)]
    pub memory: Option<MemoryStats>,
    #[serde(default)]
    pub scan: Option<ScanStats>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct CachingStats {
    pub data_hits: u64,
    pub data_misses: u64,
    pub data_bytes_hits: u64,
    pub data_bytes_misses: u64,
    pub filehandle_hits: u64,
    pub filehandle_misses: u64,
    pub metadata_hits: u64,
    pub metadata_misses: u64,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct MemoryStats {
    pub total_usage_bytes: u64,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ScanStats {
    pub estimated_result_rows: u64,
    pub estimated_scanned_rows: u64,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct QueryColumn {
    pub name: String,
    #[serde(default)]
    pub data_type: Option<String>,
    #[serde(default)]
    pub nullable: Option<bool>,
    #[serde(flatten)]
    pub extra: HashMap<String, Value>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct QueryMetadata {
    #[serde(flatten)]
    pub values: HashMap<String, Value>,
}
