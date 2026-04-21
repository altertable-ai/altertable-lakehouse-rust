use reqwest::StatusCode;
use std::error::Error as StdError;
use std::fmt;

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct ErrorContext {
    pub operation: &'static str,
    pub method: &'static str,
    pub path: String,
    pub status: Option<u16>,
    pub retriable: bool,
    pub request_id: Option<String>,
    pub correlation_id: Option<String>,
}

#[derive(Debug)]
pub enum AltertableError {
    AuthError {
        context: ErrorContext,
        message: String,
    },
    BadRequestError {
        context: ErrorContext,
        message: String,
    },
    NetworkError {
        context: ErrorContext,
        message: String,
        source: Option<Box<dyn StdError + Send + Sync>>,
    },
    TimeoutError {
        context: ErrorContext,
        message: String,
        source: Option<Box<dyn StdError + Send + Sync>>,
    },
    SerializationError {
        context: ErrorContext,
        message: String,
        source: Option<Box<dyn StdError + Send + Sync>>,
    },
    ParseError {
        context: ErrorContext,
        message: String,
        line: Option<usize>,
        source: Option<Box<dyn StdError + Send + Sync>>,
    },
    ApiError {
        context: ErrorContext,
        message: String,
        body: Option<String>,
    },
    ConfigurationError {
        message: String,
    },
}

impl AltertableError {
    pub fn context(&self) -> Option<&ErrorContext> {
        match self {
            Self::AuthError { context, .. }
            | Self::BadRequestError { context, .. }
            | Self::NetworkError { context, .. }
            | Self::TimeoutError { context, .. }
            | Self::SerializationError { context, .. }
            | Self::ParseError { context, .. }
            | Self::ApiError { context, .. } => Some(context),
            Self::ConfigurationError { .. } => None,
        }
    }

    pub fn from_reqwest(error: reqwest::Error, context: ErrorContext) -> Self {
        let message = error.to_string();
        if error.is_timeout() {
            return Self::TimeoutError {
                context,
                message,
                source: Some(Box::new(error)),
            };
        }

        Self::NetworkError {
            context,
            message,
            source: Some(Box::new(error)),
        }
    }

    pub fn from_status(context: ErrorContext, status: StatusCode, body: String) -> Self {
        let message = if body.is_empty() {
            format!("request failed with status {status}")
        } else {
            body.clone()
        };

        match status {
            StatusCode::UNAUTHORIZED | StatusCode::FORBIDDEN => {
                Self::AuthError { context, message }
            }
            StatusCode::BAD_REQUEST => Self::BadRequestError { context, message },
            _ => Self::ApiError {
                context,
                message,
                body: if body.is_empty() { None } else { Some(body) },
            },
        }
    }
}

impl fmt::Display for AltertableError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::AuthError { message, .. }
            | Self::BadRequestError { message, .. }
            | Self::NetworkError { message, .. }
            | Self::TimeoutError { message, .. }
            | Self::SerializationError { message, .. }
            | Self::ApiError { message, .. }
            | Self::ConfigurationError { message } => write!(f, "{message}"),
            Self::ParseError { message, line, .. } => {
                if let Some(line) = line {
                    write!(f, "{message} (line {line})")
                } else {
                    write!(f, "{message}")
                }
            }
        }
    }
}

impl StdError for AltertableError {
    fn source(&self) -> Option<&(dyn StdError + 'static)> {
        match self {
            Self::NetworkError { source, .. }
            | Self::TimeoutError { source, .. }
            | Self::SerializationError { source, .. }
            | Self::ParseError { source, .. } => source.as_deref().map(|e| e as _),
            _ => None,
        }
    }
}
