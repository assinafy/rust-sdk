//! Document activity log model.

use serde::{Deserialize, Serialize};

/// Origin metadata captured with an activity entry.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[non_exhaustive]
pub struct ActivityOrigin {
    /// Originating IP.
    #[serde(default)]
    pub ip: Option<String>,
    /// User-Agent string from the originating request.
    #[serde(default, rename = "user-agent", alias = "user_agent")]
    pub user_agent: Option<String>,
}

/// One entry from `GET /documents/{id}/activities`.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[non_exhaustive]
pub struct Activity {
    /// Activity identifier.
    pub id: i64,
    /// Event code (e.g. `signature_requested`, `assignment_created`).
    pub event: String,
    /// Human-readable description.
    #[serde(default)]
    pub message: Option<String>,
    /// Event-specific payload. The exact schema varies by event.
    #[serde(default)]
    pub payload: serde_json::Value,
    /// Origin metadata.
    #[serde(default)]
    pub origin: Option<ActivityOrigin>,
    /// ISO-8601 timestamp.
    pub created_at: String,
}
