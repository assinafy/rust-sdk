//! Error types returned by the SDK.

use std::fmt;

use serde::{Deserialize, Serialize};

/// Convenience alias for results returned by this crate.
pub type Result<T> = std::result::Result<T, Error>;

/// The error type for all SDK operations.
#[derive(Debug, thiserror::Error)]
#[non_exhaustive]
pub enum Error {
    /// Invalid configuration (e.g. missing credentials, malformed base URL).
    #[error("invalid configuration: {0}")]
    Config(String),

    /// The underlying HTTP transport produced an error.
    #[error("http transport error: {0}")]
    Http(#[from] reqwest::Error),

    /// Failed to serialize or deserialize JSON.
    #[error("serialization error: {0}")]
    Serde(#[from] serde_json::Error),

    /// Failed to build a multipart request from a local file.
    #[error("io error: {0}")]
    Io(#[from] std::io::Error),

    /// Failed to construct a URL.
    #[error("url error: {0}")]
    Url(#[from] url::ParseError),

    /// The server returned a non-2xx response.
    #[error("api error: {0}")]
    Api(#[from] ApiError),

    /// The server returned an unexpected payload that could not be decoded.
    #[error("unexpected response: {0}")]
    UnexpectedResponse(String),
}

impl Error {
    /// Returns the HTTP status of an API error, if available.
    pub fn status(&self) -> Option<u16> {
        match self {
            Error::Api(e) => Some(e.status),
            Error::Http(e) => e.status().map(|s| s.as_u16()),
            _ => None,
        }
    }

    /// Returns the [`ApiError`] payload if this is an [`Error::Api`].
    pub fn api(&self) -> Option<&ApiError> {
        match self {
            Error::Api(e) => Some(e),
            _ => None,
        }
    }

    /// Returns `true` if this is an API error with HTTP status `429 Too Many
    /// Requests`. The Assinafy API rate-limits requests (see the
    /// `X-Rate-Limit-*` response headers); use [`retry_after`](Error::retry_after)
    /// to learn how long to wait before retrying.
    pub fn is_rate_limited(&self) -> bool {
        self.status() == Some(429)
    }

    /// Returns the number of seconds to wait before retrying, when the server
    /// provided a `Retry-After` or `X-Rate-Limit-Reset` header on a failed
    /// response.
    pub fn retry_after(&self) -> Option<u64> {
        self.api().and_then(|e| e.retry_after)
    }
}

/// Structured payload returned by the Assinafy API for non-2xx responses.
///
/// The Assinafy API consistently uses the `{ status, message, data }`
/// envelope, even for error responses. This struct preserves the raw `data`
/// field as [`serde_json::Value`] so callers can inspect validation details
/// without losing fidelity.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ApiError {
    /// HTTP status code (also echoed inside the envelope).
    pub status: u16,
    /// Human-readable error message from the server.
    pub message: String,
    /// Optional structured details (validation errors, hints, etc.). For a
    /// "route not found" response (as opposed to a missing resource) this
    /// preserves the server's raw `{ name, code, ... }` body so callers can
    /// distinguish the two.
    #[serde(default)]
    pub data: serde_json::Value,
    /// Seconds to wait before retrying, parsed from the `Retry-After` or
    /// `X-Rate-Limit-Reset` response header when present.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub retry_after: Option<u64>,
}

impl fmt::Display for ApiError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{} (status {})", self.message, self.status)
    }
}

impl std::error::Error for ApiError {}
