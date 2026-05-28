//! Webhook subscription and delivery models.

use serde::{Deserialize, Serialize};

/// Workspace webhook subscription.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[non_exhaustive]
pub struct WebhookSubscription {
    /// Subscription identifier, when returned by the API.
    #[serde(default)]
    pub id: Option<String>,
    /// Destination URL.
    #[serde(default)]
    pub url: Option<String>,
    /// Contact email for delivery failures and ownership.
    #[serde(default)]
    pub email: Option<String>,
    /// Event names that trigger delivery.
    #[serde(default)]
    pub events: Vec<String>,
    /// Whether delivery is active.
    #[serde(default)]
    pub is_active: bool,
    /// Creation timestamp.
    #[serde(default)]
    pub created_at: Option<serde_json::Value>,
    /// Last-modification timestamp.
    #[serde(default)]
    pub updated_at: Option<serde_json::Value>,
}

/// Supported webhook event type.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[non_exhaustive]
pub struct WebhookEventTypeInfo {
    /// Event identifier.
    pub id: String,
    /// Human-readable event description.
    #[serde(default)]
    pub description: String,
}

/// Webhook delivery history row.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[non_exhaustive]
pub struct WebhookDispatch {
    /// Resource discriminator, when returned by single-resource endpoints.
    #[serde(default)]
    pub resource: Option<String>,
    /// Dispatch identifier.
    pub id: String,
    /// Event name.
    pub event: String,
    /// Source activity identifier.
    #[serde(default)]
    pub activity_id: Option<i64>,
    /// Delivery endpoint URL.
    #[serde(default)]
    pub endpoint: Option<String>,
    /// Payload sent to the endpoint.
    #[serde(default)]
    pub payload: Option<serde_json::Value>,
    /// Whether delivery succeeded.
    #[serde(default)]
    pub delivered: bool,
    /// HTTP status returned by the endpoint, when available.
    #[serde(default)]
    pub http_status: Option<u16>,
    /// Response body returned by the endpoint, when available.
    #[serde(default)]
    pub response_body: Option<String>,
    /// Delivery error, when available.
    #[serde(default)]
    pub error: Option<String>,
    /// Creation timestamp.
    #[serde(default)]
    pub created_at: Option<serde_json::Value>,
    /// Last-modification timestamp.
    #[serde(default)]
    pub updated_at: Option<serde_json::Value>,
}
