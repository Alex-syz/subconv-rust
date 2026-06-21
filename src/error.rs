//! Unified error type for SubConv.

use axum::http::StatusCode;
use axum::response::{IntoResponse, Response};

/// All errors produced by SubConv operations.
#[derive(Debug, thiserror::Error)]
pub enum SubconvError {
    #[error("parse error: {0}")]
    Parse(String),

    #[error("config error: {0}")]
    Config(String),

    #[error("template not found: {0}")]
    TemplateNotFound(String),

    #[error("upstream fetch failed: {0}")]
    UpstreamFetch(String),

    #[error("invalid URL: {0}")]
    InvalidUrl(String),

    #[error("forbidden: {0}")]
    Forbidden(String),

    #[error("bad request: {0}")]
    BadRequest(String),

    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),

    #[error("YAML error: {0}")]
    Yaml(#[from] serde_yaml::Error),

    #[error("HTTP client error: {0}")]
    Http(#[from] reqwest::Error),

    #[error("UTF-8 decode error: {0}")]
    Utf8(#[from] std::string::FromUtf8Error),
}

impl SubconvError {
    /// Map this error to an HTTP status code.
    pub fn status_code(&self) -> StatusCode {
        match self {
            Self::BadRequest(_) | Self::InvalidUrl(_) => StatusCode::BAD_REQUEST,
            Self::Forbidden(_) => StatusCode::FORBIDDEN,
            Self::TemplateNotFound(_) => StatusCode::NOT_FOUND,
            Self::UpstreamFetch(_) => StatusCode::BAD_GATEWAY,
            Self::Parse(_) | Self::Config(_) => StatusCode::INTERNAL_SERVER_ERROR,
            Self::Io(_) | Self::Yaml(_) | Self::Http(_) | Self::Utf8(_) => {
                StatusCode::INTERNAL_SERVER_ERROR
            }
        }
    }
}

impl IntoResponse for SubconvError {
    fn into_response(self) -> Response {
        let status = self.status_code();
        match &self {
            // Client-meaningful errors: safe to expose the original message.
            Self::BadRequest(_)
            | Self::InvalidUrl(_)
            | Self::Forbidden(_)
            | Self::TemplateNotFound(_) => (status, self.to_string()).into_response(),

            // Internal errors: log full detail server-side, return generic message.
            Self::UpstreamFetch(_)
            | Self::Io(_)
            | Self::Yaml(_)
            | Self::Http(_)
            | Self::Utf8(_)
            | Self::Config(_)
            | Self::Parse(_) => {
                tracing::warn!(error = %self, "internal error returned to client");
                (
                    status,
                    status.canonical_reason().unwrap_or("Internal Server Error"),
                )
                    .into_response()
            }
        }
    }
}
