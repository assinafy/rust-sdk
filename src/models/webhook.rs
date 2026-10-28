//! Webhook subscription and delivery models.

use std::fmt;

use serde::{Deserialize, Serialize};

use super::ActivityOrigin;

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
    /// Whether delivery is paused after repeated failures.
    #[serde(default)]
    pub is_held: bool,
    /// Unix timestamp when the delivery pause began.
    #[serde(default)]
    pub held_since: Option<i64>,
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

/// Kind of webhook delivery attempt.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum WebhookDispatchKind {
    /// Delivery made when the event occurred.
    Scheduled,
    /// Probe sent to test a paused endpoint.
    Probe,
    /// Delivery requested through the retry endpoint.
    Manual,
    /// A server value not yet modeled by the SDK.
    #[serde(untagged)]
    Other(String),
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
    /// Stable `webhook-id` shared by every attempt of this event to this
    /// endpoint.
    #[serde(default)]
    pub webhook_id: Option<String>,
    /// 1-based attempt number.
    #[serde(default)]
    pub attempt: Option<u32>,
    /// Whether this was a scheduled delivery, probe, or manual retry.
    #[serde(default)]
    pub kind: Option<WebhookDispatchKind>,
    /// Whether another automatic attempt follows this one.
    #[serde(default)]
    pub will_retry: bool,
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
#[derive(Clone, Serialize, Deserialize)]
#[non_exhaustive]
pub struct WebhookEndpoint {
    /// Resource discriminator (`"activity_event_subscription"`) when present.
    #[serde(default)]
    pub resource: Option<String>,
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
    /// Whether delivery is paused after repeated failures.
    #[serde(default)]
    pub is_held: bool,
    /// Unix timestamp when the delivery pause began.
    #[serde(default)]
    pub held_since: Option<i64>,
    /// Whether deliveries carry a `webhook-signature` header.
    #[serde(default)]
    pub signing_enabled: bool,
    /// Signing secret returned only when creating a signed endpoint. Store it
    /// securely: OAuth applications cannot retrieve or rotate it later.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub secret: Option<String>,
    /// Creation timestamp.
    #[serde(default)]
    pub created_at: Option<serde_json::Value>,
    /// Last-modification timestamp.
    #[serde(default)]
    pub updated_at: Option<serde_json::Value>,
}

impl fmt::Debug for WebhookEndpoint {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("WebhookEndpoint")
            .field("resource", &self.resource)
            .field("id", &self.id)
            .field("name", &self.name)
            .field("url", &self.url)
            .field("email", &self.email)
            .field("events", &self.events)
            .field("is_active", &self.is_active)
            .field("is_held", &self.is_held)
            .field("held_since", &self.held_since)
            .field("signing_enabled", &self.signing_enabled)
            .field("secret", &self.secret.as_ref().map(|_| "**redacted**"))
            .field("created_at", &self.created_at)
            .field("updated_at", &self.updated_at)
            .finish()
    }
}

/// Body delivered to a webhook endpoint.
///
/// Verify the Standard Webhooks signature over the raw body before parsing it
/// into this type. Unlike REST responses, timestamps in webhook payloads are
/// Unix seconds.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[non_exhaustive]
pub struct WebhookEvent {
    /// Activity identifier that produced the event.
    pub id: i64,
    /// Event type.
    pub event: String,
    /// Reserved human-readable message.
    #[serde(default)]
    pub message: Option<String>,
    /// Event-specific parameters.
    #[serde(default)]
    pub payload: Option<serde_json::Value>,
    /// Request origin, when the event was triggered by a request.
    #[serde(default)]
    pub origin: Option<ActivityOrigin>,
    /// Time the event was recorded, as Unix seconds.
    pub created_at: i64,
    /// Actor that performed the action, including its `type` discriminator.
    pub subject: serde_json::Value,
    /// Resource acted on, including its `type` discriminator.
    pub object: serde_json::Value,
    /// Workspace that owns the event.
    pub account_id: String,
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
