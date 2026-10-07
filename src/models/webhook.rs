//! Webhook subscription and delivery models.

use std::fmt;

use serde::{Deserialize, Serialize};

/// The account's oldest webhook endpoint, as returned by the subscription routes.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[non_exhaustive]
pub struct WebhookSubscription {
    /// Optional subscription identifier. The account-level singleton routes do
    /// not require an identifier.
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
    /// Identifier of the webhook endpoint that received the delivery; `None`
    /// once that endpoint is deleted.
    #[serde(default)]
    pub endpoint_id: Option<String>,
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

/// A URL that receives the account's webhook events. Accounts have one
/// endpoint, or up to three on paid plans; every active endpoint subscribed
/// to an event receives it.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[non_exhaustive]
pub struct WebhookEndpoint {
    /// Endpoint identifier.
    pub id: String,
    /// Label that tells endpoints apart.
    #[serde(default)]
    pub name: Option<String>,
    /// Destination URL.
    #[serde(default)]
    pub url: String,
    /// Contact email for delivery-failure notices.
    #[serde(default)]
    pub email: Option<String>,
    /// Event names delivered to this endpoint.
    #[serde(default)]
    pub events: Vec<String>,
    /// Whether events are delivered.
    #[serde(default)]
    pub is_active: bool,
    /// Whether deliveries carry a `webhook-signature` header.
    #[serde(default)]
    pub signing_enabled: bool,
    /// Creation timestamp.
    #[serde(default)]
    pub created_at: Option<serde_json::Value>,
    /// Last-modification timestamp.
    #[serde(default)]
    pub updated_at: Option<serde_json::Value>,
}

/// A webhook endpoint's Standard Webhooks signing secret (`whsec_…`).
///
/// `Debug` redacts the secret; read [`secret`](Self::secret) explicitly.
#[derive(Clone, Serialize, Deserialize)]
#[non_exhaustive]
pub struct WebhookEndpointSecret {
    /// `whsec_` followed by the base64-encoded signing key.
    pub secret: String,
}

impl fmt::Debug for WebhookEndpointSecret {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("WebhookEndpointSecret")
            .field("secret", &"**redacted**")
            .finish()
    }
}
