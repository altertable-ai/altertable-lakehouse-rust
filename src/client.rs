use crate::error::{AltertableError, ErrorContext};
use crate::models::{
    AppendRequest, AppendResponse, CancelQueryResponse, QueryColumn, QueryLogResponse,
    QueryMetadata, QueryRequest, QueryRow, TaskResponse, UploadFormat, UploadMode, ValidateRequest,
    ValidateResponse,
};
use base64::Engine;
use futures_util::{Stream, StreamExt};
use reqwest::header::{HeaderMap, HeaderValue, ACCEPT, AUTHORIZATION, CONTENT_TYPE, USER_AGENT};
use reqwest::{Client, Method, Response, StatusCode};
use serde::de::DeserializeOwned;
use serde_json::Value;
use std::collections::{HashMap, VecDeque};
use std::pin::Pin;
use std::task::{Context, Poll};
use std::time::Duration;

const DEFAULT_BASE_URL: &str = "https://api.altertable.ai";
const DEFAULT_CONNECT_TIMEOUT: Duration = Duration::from_secs(5);
const DEFAULT_REQUEST_TIMEOUT: Duration = Duration::from_mins(1);
const DEFAULT_USER_AGENT: &str = concat!(env!("CARGO_PKG_NAME"), "/", env!("CARGO_PKG_VERSION"));

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RetryConfig {
    pub max_retries: u32,
    pub retry_on_timeout: bool,
    pub retry_on_5xx: bool,
}

impl Default for RetryConfig {
    fn default() -> Self {
        Self {
            max_retries: 0,
            retry_on_timeout: true,
            retry_on_5xx: false,
        }
    }
}

#[derive(Debug, Clone)]
pub struct ClientConfig {
    pub base_url: String,
    pub auth: AuthConfig,
    pub connect_timeout: Duration,
    pub request_timeout: Duration,
    pub retry: RetryConfig,
    pub user_agent_suffix: Option<String>,
}

#[derive(Debug, Clone)]
pub enum AuthConfig {
    Basic { username: String, password: String },
    BasicToken(String),
}

#[derive(Debug, Clone, Default)]
pub struct ClientBuilder {
    base_url: Option<String>,
    username: Option<String>,
    password: Option<String>,
    basic_token: Option<String>,
    connect_timeout: Option<Duration>,
    request_timeout: Option<Duration>,
    retry: Option<RetryConfig>,
    user_agent_suffix: Option<String>,
}

#[derive(Debug, Clone)]
pub struct AltertableClient {
    http: Client,
    config: ClientConfig,
    auth_header: HeaderValue,
    user_agent: String,
}

#[derive(Debug, Clone, PartialEq)]
pub enum StreamChunk {
    Metadata(QueryMetadata),
    Columns(Vec<QueryColumn>),
    Row(QueryRow),
}

pub struct QueryRowStream {
    inner: Pin<Box<dyn Stream<Item = Result<QueryRow, AltertableError>> + Send>>,
}

pub struct QueryResult {
    pub metadata: QueryMetadata,
    pub columns: Vec<QueryColumn>,
    pub rows: QueryRowStream,
}

#[derive(Debug, Clone, PartialEq)]
pub struct QueryResultAll {
    pub metadata: QueryMetadata,
    pub columns: Vec<QueryColumn>,
    pub rows: Vec<QueryRow>,
}

impl ClientBuilder {
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    #[must_use]
    pub fn base_url(mut self, base_url: impl Into<String>) -> Self {
        self.base_url = Some(base_url.into());
        self
    }

    #[must_use]
    pub fn credentials(mut self, username: impl Into<String>, password: impl Into<String>) -> Self {
        self.username = Some(username.into());
        self.password = Some(password.into());
        self
    }

    #[must_use]
    pub fn basic_token(mut self, token: impl Into<String>) -> Self {
        self.basic_token = Some(token.into());
        self
    }

    #[must_use]
    pub fn connect_timeout(mut self, timeout: Duration) -> Self {
        self.connect_timeout = Some(timeout);
        self
    }

    #[must_use]
    pub fn request_timeout(mut self, timeout: Duration) -> Self {
        self.request_timeout = Some(timeout);
        self
    }

    #[must_use]
    pub fn retry(mut self, retry: RetryConfig) -> Self {
        self.retry = Some(retry);
        self
    }

    #[must_use]
    pub fn user_agent_suffix(mut self, suffix: impl Into<String>) -> Self {
        self.user_agent_suffix = Some(suffix.into());
        self
    }

    #[must_use = "call `build` to construct an `AltertableClient`"]
    pub fn build(self) -> Result<AltertableClient, AltertableError> {
        let auth = resolve_auth(self.username, self.password, self.basic_token)?;
        let config = ClientConfig {
            base_url: self
                .base_url
                .unwrap_or_else(|| DEFAULT_BASE_URL.to_string()),
            auth,
            connect_timeout: self.connect_timeout.unwrap_or(DEFAULT_CONNECT_TIMEOUT),
            request_timeout: self.request_timeout.unwrap_or(DEFAULT_REQUEST_TIMEOUT),
            retry: self.retry.unwrap_or_default(),
            user_agent_suffix: self.user_agent_suffix,
        };

        AltertableClient::from_config(config)
    }
}

impl AltertableClient {
    #[must_use]
    pub fn builder() -> ClientBuilder {
        ClientBuilder::new()
    }

    pub fn from_env() -> Result<Self, AltertableError> {
        ClientBuilder::new().build()
    }

    pub fn from_config(config: ClientConfig) -> Result<Self, AltertableError> {
        let http = Client::builder()
            .connect_timeout(config.connect_timeout)
            .timeout(config.request_timeout)
            .pool_max_idle_per_host(8)
            .tcp_keepalive(Duration::from_mins(1))
            .build()
            .map_err(|error| AltertableError::ConfigurationError {
                message: format!("failed to build HTTP client: {error}"),
            })?;

        let auth_header = build_auth_header(&config.auth)?;
        let user_agent = if let Some(suffix) = &config.user_agent_suffix {
            format!("{DEFAULT_USER_AGENT} {suffix}")
        } else {
            DEFAULT_USER_AGENT.to_string()
        };

        Ok(Self {
            http,
            config,
            auth_header,
            user_agent,
        })
    }

    pub fn config(&self) -> &ClientConfig {
        &self.config
    }

    pub async fn append(
        &self,
        catalog: &str,
        schema: &str,
        table: &str,
        body: &AppendRequest,
    ) -> Result<AppendResponse, AltertableError> {
        ensure_non_empty("catalog", catalog)?;
        ensure_non_empty("schema", schema)?;
        ensure_non_empty("table", table)?;

        self.send_json(
            "append",
            Method::POST,
            "/append",
            vec![("catalog", catalog), ("schema", schema), ("table", table)],
            Some(body),
        )
        .await
    }

    pub async fn validate(
        &self,
        request: &ValidateRequest,
    ) -> Result<ValidateResponse, AltertableError> {
        ensure_non_empty("statement", &request.statement)?;
        self.send_json("validate", Method::POST, "/validate", vec![], Some(request))
            .await
    }

    pub async fn get_query(&self, query_id: &str) -> Result<QueryLogResponse, AltertableError> {
        ensure_non_empty("query_id", query_id)?;
        let path = format!("/query/{query_id}");
        self.send_json::<(), QueryLogResponse>("get_query", Method::GET, &path, vec![], None)
            .await
    }

    pub async fn cancel_query(
        &self,
        query_id: &str,
        session_id: &str,
    ) -> Result<CancelQueryResponse, AltertableError> {
        ensure_non_empty("query_id", query_id)?;
        ensure_non_empty("session_id", session_id)?;
        let path = format!("/query/{query_id}");
        self.send_json::<(), CancelQueryResponse>(
            "cancel_query",
            Method::DELETE,
            &path,
            vec![("session_id", session_id)],
            None,
        )
        .await
    }

    #[allow(clippy::too_many_arguments)]
    pub async fn upload(
        &self,
        catalog: &str,
        schema: &str,
        table: &str,
        format: UploadFormat,
        mode: UploadMode,
        primary_key: Option<&str>,
        bytes: Vec<u8>,
    ) -> Result<TaskResponse, AltertableError> {
        ensure_non_empty("catalog", catalog)?;
        ensure_non_empty("schema", schema)?;
        ensure_non_empty("table", table)?;
        if matches!(mode, UploadMode::Upsert) && primary_key.is_none() {
            return Err(AltertableError::ConfigurationError {
                message: "primary_key is required when mode=upsert".to_string(),
            });
        }

        let mut query = vec![
            ("catalog", catalog.to_string()),
            ("schema", schema.to_string()),
            ("table", table.to_string()),
            (
                "format",
                serde_plain::to_string(&format).unwrap_or_else(|_| "json".to_string()),
            ),
            (
                "mode",
                serde_plain::to_string(&mode).unwrap_or_else(|_| "append".to_string()),
            ),
        ];
        if let Some(primary_key) = primary_key {
            query.push(("primary_key", primary_key.to_string()));
        }

        let response = self
            .request("upload", Method::POST, "/upload")
            .query(&query)
            .header(CONTENT_TYPE, "application/octet-stream")
            .body(bytes)
            .send()
            .await
            .map_err(|error| {
                AltertableError::from_reqwest(
                    error,
                    error_context("upload", "POST", "/upload", None, false, None),
                )
            })?;

        self.decode_response("upload", Method::POST, "/upload", response)
            .await
    }

    pub async fn query(&self, request: &QueryRequest) -> Result<QueryResult, AltertableError> {
        ensure_non_empty("statement", &request.statement)?;

        let response = self
            .request("query", Method::POST, "/query")
            .header(ACCEPT, "application/x-ndjson")
            .json(request)
            .send()
            .await
            .map_err(|error| {
                AltertableError::from_reqwest(
                    error,
                    error_context("query", "POST", "/query", None, false, None),
                )
            })?;

        self.decode_query_response(response).await
    }

    pub async fn query_all(
        &self,
        request: &QueryRequest,
    ) -> Result<QueryResultAll, AltertableError> {
        let QueryResult {
            metadata,
            columns,
            mut rows,
        } = self.query(request).await?;
        let mut all_rows = Vec::new();
        while let Some(row) = rows.next().await {
            all_rows.push(row?);
        }
        Ok(QueryResultAll {
            metadata,
            columns,
            rows: all_rows,
        })
    }

    fn request(
        &self,
        _operation: &'static str,
        method: Method,
        path: &str,
    ) -> reqwest::RequestBuilder {
        let url = format!("{}{}", self.config.base_url.trim_end_matches('/'), path);
        self.http
            .request(method, url)
            .header(AUTHORIZATION, self.auth_header.clone())
            .header(USER_AGENT, self.user_agent.clone())
    }

    async fn send_json<T, R>(
        &self,
        operation: &'static str,
        method: Method,
        path: &str,
        query: Vec<(&str, &str)>,
        body: Option<&T>,
    ) -> Result<R, AltertableError>
    where
        T: serde::Serialize + ?Sized,
        R: DeserializeOwned,
    {
        let mut request = self.request(operation, method.clone(), path).query(&query);
        if let Some(body) = body {
            request = request.json(body);
        }

        let response = request.send().await.map_err(|error| {
            AltertableError::from_reqwest(
                error,
                error_context(operation, method.as_str(), path, None, false, None),
            )
        })?;

        self.decode_response(operation, method, path, response)
            .await
    }

    async fn decode_response<R>(
        &self,
        operation: &'static str,
        method: Method,
        path: &str,
        response: Response,
    ) -> Result<R, AltertableError>
    where
        R: DeserializeOwned,
    {
        let status = response.status();
        let headers = response.headers().clone();
        let body = response.text().await.map_err(|error| {
            AltertableError::from_reqwest(
                error,
                error_context(
                    operation,
                    method.as_str(),
                    path,
                    Some(status),
                    status.is_server_error(),
                    Some(&headers),
                ),
            )
        })?;

        if !status.is_success() {
            return Err(AltertableError::from_status(
                error_context(
                    operation,
                    method.as_str(),
                    path,
                    Some(status),
                    status.is_server_error(),
                    Some(&headers),
                ),
                status,
                body,
            ));
        }

        serde_json::from_str(&body).map_err(|error| AltertableError::SerializationError {
            context: error_context(
                operation,
                method.as_str(),
                path,
                Some(status),
                false,
                Some(&headers),
            ),
            message: format!("failed to decode JSON response for {operation}"),
            source: Some(Box::new(error)),
        })
    }

    async fn decode_query_response(
        &self,
        response: Response,
    ) -> Result<QueryResult, AltertableError> {
        let status = response.status();
        let headers = response.headers().clone();
        if !status.is_success() {
            let body = response.text().await.unwrap_or_default();
            return Err(AltertableError::from_status(
                error_context(
                    "query",
                    "POST",
                    "/query",
                    Some(status),
                    status.is_server_error(),
                    Some(&headers),
                ),
                status,
                body,
            ));
        }

        let bytes = response.bytes().await.map_err(|error| {
            AltertableError::from_reqwest(
                error,
                error_context(
                    "query",
                    "POST",
                    "/query",
                    Some(status),
                    false,
                    Some(&headers),
                ),
            )
        })?;
        let text =
            String::from_utf8(bytes.to_vec()).map_err(|error| AltertableError::ParseError {
                context: error_context(
                    "query",
                    "POST",
                    "/query",
                    Some(status),
                    false,
                    Some(&headers),
                ),
                message: "query response was not valid UTF-8".to_string(),
                line: None,
                source: Some(Box::new(error)),
            })?;

        let mut metadata = None;
        let mut columns = None;
        let mut rows = VecDeque::new();

        for (index, raw_line) in text.lines().enumerate() {
            let line_no = index + 1;
            let trimmed = raw_line.trim();
            if trimmed.is_empty() {
                continue;
            }

            let value: Value =
                serde_json::from_str(trimmed).map_err(|error| AltertableError::ParseError {
                    context: error_context(
                        "query",
                        "POST",
                        "/query",
                        Some(status),
                        false,
                        Some(&headers),
                    ),
                    message: format!("failed to parse NDJSON line: {trimmed}"),
                    line: Some(line_no),
                    source: Some(Box::new(error)),
                })?;

            if metadata.is_none() {
                metadata = Some(parse_metadata(&value));
                continue;
            }

            if columns.is_none() && is_columns_line(&value) {
                columns = Some(parse_columns(&value)?);
                continue;
            }

            rows.push_back(value);
        }

        let metadata = metadata.unwrap_or(QueryMetadata {
            values: HashMap::default(),
        });
        let columns = columns.unwrap_or_default();
        let stream = futures_util::stream::iter(rows.into_iter().map(Ok));

        Ok(QueryResult {
            metadata,
            columns,
            rows: QueryRowStream {
                inner: Box::pin(stream),
            },
        })
    }
}

impl Stream for QueryRowStream {
    type Item = Result<QueryRow, AltertableError>;

    fn poll_next(mut self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Option<Self::Item>> {
        self.inner.as_mut().poll_next(cx)
    }
}

fn ensure_non_empty(field: &str, value: &str) -> Result<(), AltertableError> {
    if value.trim().is_empty() {
        return Err(AltertableError::ConfigurationError {
            message: format!("{field} must not be empty"),
        });
    }
    Ok(())
}

fn error_context(
    operation: &'static str,
    method: &str,
    path: &str,
    status: Option<StatusCode>,
    retriable: bool,
    headers: Option<&HeaderMap>,
) -> ErrorContext {
    ErrorContext {
        operation,
        method: method_to_static(method),
        path: path.to_string(),
        status: status.map(|s| s.as_u16()),
        retriable,
        request_id: headers.and_then(|h| header_value(h, "x-request-id")),
        correlation_id: headers.and_then(|h| header_value(h, "x-correlation-id")),
    }
}

fn resolve_auth(
    username: Option<String>,
    password: Option<String>,
    basic_token: Option<String>,
) -> Result<AuthConfig, AltertableError> {
    if let Some(token) = basic_token.or_else(|| std::env::var("ALTERTABLE_BASIC_AUTH_TOKEN").ok()) {
        let token = token.trim().to_string();
        if token.is_empty() {
            return Err(AltertableError::ConfigurationError {
                message: "ALTERTABLE_BASIC_AUTH_TOKEN must not be empty".to_string(),
            });
        }
        return Ok(AuthConfig::BasicToken(token));
    }

    let username = username.or_else(|| std::env::var("ALTERTABLE_USERNAME").ok());
    let password = password.or_else(|| std::env::var("ALTERTABLE_PASSWORD").ok());

    match (username, password) {
        (Some(username), Some(password)) if !username.is_empty() && !password.is_empty() => {
            Ok(AuthConfig::Basic { username, password })
        }
        _ => Err(AltertableError::ConfigurationError {
            message: "no Altertable credentials configured; set username/password or ALTERTABLE_BASIC_AUTH_TOKEN".to_string(),
        }),
    }
}

fn build_auth_header(auth: &AuthConfig) -> Result<HeaderValue, AltertableError> {
    let raw = match auth {
        AuthConfig::Basic { username, password } => {
            let encoded =
                base64::engine::general_purpose::STANDARD.encode(format!("{username}:{password}"));
            format!("Basic {encoded}")
        }
        AuthConfig::BasicToken(token) => {
            if token.starts_with("Basic ") {
                token.clone()
            } else {
                format!("Basic {token}")
            }
        }
    };

    let mut value =
        HeaderValue::from_str(&raw).map_err(|error| AltertableError::ConfigurationError {
            message: format!("invalid authorization header value: {error}"),
        })?;
    value.set_sensitive(true);
    Ok(value)
}

fn parse_metadata(value: &Value) -> QueryMetadata {
    match value {
        Value::Object(map) => QueryMetadata {
            values: map.iter().map(|(k, v)| (k.clone(), v.clone())).collect(),
        },
        other => QueryMetadata {
            values: [("value".to_string(), other.clone())].into_iter().collect(),
        },
    }
}

fn is_columns_line(value: &Value) -> bool {
    match value {
        Value::Array(items) => items.iter().all(Value::is_object),
        Value::Object(map) => map.get("columns").is_some_and(Value::is_array),
        _ => false,
    }
}

fn parse_columns(value: &Value) -> Result<Vec<QueryColumn>, AltertableError> {
    let columns_value = match value {
        Value::Array(_) => value.clone(),
        Value::Object(map) => map.get("columns").cloned().unwrap_or(Value::Null),
        _ => Value::Null,
    };

    serde_json::from_value(columns_value).map_err(|error| AltertableError::ParseError {
        context: ErrorContext {
            operation: "query",
            method: "POST",
            path: "/query".to_string(),
            status: None,
            retriable: false,
            request_id: None,
            correlation_id: None,
        },
        message: "failed to parse query columns".to_string(),
        line: None,
        source: Some(Box::new(error)),
    })
}

fn method_to_static(method: &str) -> &'static str {
    match method {
        "GET" => "GET",
        "POST" => "POST",
        "DELETE" => "DELETE",
        _ => "UNKNOWN",
    }
}

fn header_value(headers: &HeaderMap, name: &str) -> Option<String> {
    headers
        .get(name)
        .and_then(|v| v.to_str().ok())
        .map(ToOwned::to_owned)
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn resolves_direct_credentials() {
        let auth = resolve_auth(Some("user".into()), Some("pass".into()), None).unwrap();
        match auth {
            AuthConfig::Basic { username, password } => {
                assert_eq!(username, "user");
                assert_eq!(password, "pass");
            }
            AuthConfig::BasicToken(_) => panic!("expected basic auth"),
        }
    }

    #[test]
    fn resolves_basic_token() {
        let auth = resolve_auth(None, None, Some("abc123".into())).unwrap();
        let header = build_auth_header(&auth).unwrap();
        assert_eq!(header.to_str().unwrap(), "Basic abc123");
    }

    #[test]
    fn append_request_serializes_as_one_of() {
        let payload = AppendRequest::Single([(String::from("id"), json!(1))].into_iter().collect());
        let body = serde_json::to_value(payload).unwrap();
        assert_eq!(body["id"], json!(1));

        let batch =
            AppendRequest::Batch(vec![[(String::from("id"), json!(2))].into_iter().collect()]);
        let body = serde_json::to_value(batch).unwrap();
        assert!(body.is_array());
    }

    #[test]
    fn parses_query_lines() {
        let metadata = parse_metadata(&json!({"query_id":"123"}));
        assert_eq!(metadata.values["query_id"], json!("123"));

        let columns = parse_columns(&json!([
            {"name":"value","data_type":"Int32","nullable":false}
        ]))
        .unwrap();
        assert_eq!(columns[0].name, "value");
    }
}
