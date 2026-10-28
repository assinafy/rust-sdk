//! Webhook endpoints, subscription, dispatch history and signature verification.

use std::time::{Duration, SystemTime, UNIX_EPOCH};

use base64::Engine as _;
use base64::engine::general_purpose::STANDARD as BASE64;
use hmac::{Hmac, Mac};
use reqwest::Method;
use serde::{Deserialize, Serialize};
use sha2::Sha256;

use crate::error::{Error, Result};
use crate::http::HttpClient;
use crate::models::{
    WebhookDispatch, WebhookEndpoint, WebhookEndpointSecret, WebhookEventTypeInfo,
    WebhookSubscription,
};
use crate::pagination::Page;

const DEFAULT_EVENTS: [&str; 5] = [
    "document_ready",
    "document_prepared",
    "signer_signed_document",
    "signer_rejected_document",
    "document_processing_failed",
];

fn default_events() -> Vec<String> {
    DEFAULT_EVENTS
        .iter()
        .map(|event| (*event).to_string())
        .collect()
}

/// Body for `PUT /accounts/{account_id}/webhooks/subscriptions`.
///
/// # Request payload
///
/// ```json
/// {
///   "url": "https://hooks.example.invalid/assinafy",
///   "email": "user@example.invalid",
///   "events": ["document_ready", "signer_signed_document"],
///   "is_active": true
/// }
/// ```
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RegisterWebhookBody {
    /// Destination URL.
    pub url: String,
    /// Contact email.
    pub email: String,
    /// Event names to deliver.
    #[serde(default)]
    pub events: Vec<String>,
    /// Whether the subscription is active.
    pub is_active: bool,
}

impl RegisterWebhookBody {
    /// Build a webhook registration body with the default event set.
    pub fn new<U, E>(url: U, email: E) -> Self
    where
        U: Into<String>,
        E: Into<String>,
    {
        Self {
            url: url.into(),
            email: email.into(),
            events: default_events(),
            is_active: true,
        }
    }

    /// Replace the event set.
    pub fn events<I, S>(mut self, events: I) -> Self
    where
        I: IntoIterator<Item = S>,
        S: Into<String>,
    {
        self.events = events.into_iter().map(Into::into).collect();
        self
    }

    /// Set the active flag.
    pub fn active(mut self, active: bool) -> Self {
        self.is_active = active;
        self
    }
}

/// Body for `POST /accounts/{account_id}/webhooks/endpoints` and
/// `PUT /accounts/{account_id}/webhooks/endpoints/{endpoint_id}`.
///
/// [`new`](Self::new) builds a creation body with the default event set;
/// [`Default`] builds an empty update body where only the fields set are sent.
///
/// # Request payload
///
/// ```json
/// {
///   "url": "https://hooks.example.invalid/assinafy",
///   "email": "user@example.invalid",
///   "events": ["document_ready", "signer_signed_document"],
///   "name": "ERP",
///   "is_active": true,
///   "signing_enabled": true
/// }
/// ```
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct WebhookEndpointBody {
    /// Destination URL; unique among the workspace's endpoints.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub url: Option<String>,
    /// Contact email for delivery-failure notices.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub email: Option<String>,
    /// Event names to deliver.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub events: Option<Vec<String>>,
    /// Label that tells endpoints apart.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub name: Option<String>,
    /// Whether events are delivered (the server defaults to `true`).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub is_active: Option<bool>,
    /// Sign deliveries with a Standard Webhooks signature (the server
    /// defaults to `false`). Enabling it generates a secret when the endpoint
    /// has none; disabling it discards the secret.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub signing_enabled: Option<bool>,
}

impl WebhookEndpointBody {
    /// Build an endpoint creation body with the default event set.
    pub fn new<U, E>(url: U, email: E) -> Self
    where
        U: Into<String>,
        E: Into<String>,
    {
        Self::default().url(url).email(email).events(DEFAULT_EVENTS)
    }

    /// Set the destination URL.
    pub fn url<S: Into<String>>(mut self, url: S) -> Self {
        self.url = Some(url.into());
        self
    }

    /// Set the contact email.
    pub fn email<S: Into<String>>(mut self, email: S) -> Self {
        self.email = Some(email.into());
        self
    }

    /// Replace the event set.
    pub fn events<I, S>(mut self, events: I) -> Self
    where
        I: IntoIterator<Item = S>,
        S: Into<String>,
    {
        self.events = Some(events.into_iter().map(Into::into).collect());
        self
    }

    /// Set the label.
    pub fn name<S: Into<String>>(mut self, name: S) -> Self {
        self.name = Some(name.into());
        self
    }

    /// Set the active flag.
    pub fn active(mut self, active: bool) -> Self {
        self.is_active = Some(active);
        self
    }

    /// Enable or disable delivery signing.
    pub fn signing_enabled(mut self, enabled: bool) -> Self {
        self.signing_enabled = Some(enabled);
        self
    }
}

/// Builder for `GET /accounts/{account_id}/webhooks`.
#[derive(Debug)]
pub struct ListWebhookDispatchesRequest<'a> {
    http: &'a HttpClient,
    account_id: &'a str,
    page: Option<u32>,
    per_page: Option<u32>,
    search: Option<String>,
    sort: Option<String>,
    endpoint_id: Option<String>,
    event: Option<String>,
    delivered: Option<bool>,
    from: Option<i64>,
    to: Option<i64>,
}

impl<'a> ListWebhookDispatchesRequest<'a> {
    /// 1-based page number.
    pub fn page(mut self, page: u32) -> Self {
        self.page = Some(page);
        self
    }

    /// Results per page.
    pub fn per_page(mut self, per_page: u32) -> Self {
        self.per_page = Some(per_page);
        self
    }

    /// Free-text search.
    pub fn search<S: Into<String>>(mut self, search: S) -> Self {
        self.search = Some(search.into());
        self
    }

    /// Sort expression.
    pub fn sort<S: Into<String>>(mut self, sort: S) -> Self {
        self.sort = Some(sort.into());
        self
    }

    /// Only deliveries to this webhook endpoint.
    pub fn endpoint_id<S: Into<String>>(mut self, endpoint_id: S) -> Self {
        self.endpoint_id = Some(endpoint_id.into());
        self
    }

    /// Filter by event name.
    pub fn event<S: Into<String>>(mut self, event: S) -> Self {
        self.event = Some(event.into());
        self
    }

    /// Filter by delivery status.
    pub fn delivered(mut self, delivered: bool) -> Self {
        self.delivered = Some(delivered);
        self
    }

    /// Filter dispatches created at or after this Unix timestamp.
    pub fn from_timestamp(mut self, from: i64) -> Self {
        self.from = Some(from);
        self
    }

    /// Filter dispatches created at or before this Unix timestamp.
    pub fn to_timestamp(mut self, to: i64) -> Self {
        self.to = Some(to);
        self
    }

    /// Execute the request.
    ///
    /// `GET /accounts/{account_id}/webhooks`.
    ///
    /// # Request payload
    ///
    /// No request body. Resource identifiers are supplied in the route. Query
    /// parameters are URL-encoded; only configured filters are sent.
    ///
    /// ```json
    /// {"page": 1, "per-page": 20, "search": "contract", "sort": "-created_at", "endpoint_id": "65f1c2a9b3e4d5f60718293a4b5c6d7e", "event": "document_ready", "delivered": true, "from": 1790812800, "to": 1793491200}
    /// ```
    ///
    /// # Response payload
    ///
    /// A paginated list of dispatches; each item's `payload` mirrors the JSON
    /// that was POSTed to the endpoint.
    ///
    /// ```json
    /// {
    ///   "status": 200,
    ///   "message": "",
    ///   "data": [
    ///     {
    ///       "id": "103a09cfce51319dd3b3f72ffcdf",
    ///       "event": "signature_requested",
    ///       "activity_id": 8629,
    ///       "endpoint_id": "65f1c2a9b3e4d5f60718293a4b5c6d7e",
    ///       "endpoint": "https://hooks.example.invalid/assinafy",
    ///       "payload": { "id": 8629, "event": "signature_requested", "object": {} },
    ///       "delivered": true,
    ///       "webhook_id": "9f86d081884c7d659a2feaa0c55ad015",
    ///       "attempt": 1,
    ///       "kind": "scheduled",
    ///       "will_retry": false,
    ///       "http_status": 200,
    ///       "response_body": "{\"ok\": true}",
    ///       "error": null,
    ///       "created_at": "2026-07-15T20:04:36Z",
    ///       "updated_at": "2026-07-15T20:04:36Z"
    ///     }
    ///   ]
    /// }
    /// ```
    pub async fn send(self) -> Result<Page<WebhookDispatch>> {
        let path = self.http.path(&["accounts", self.account_id, "webhooks"])?;
        let mut req = self.http.request(Method::GET, &path)?;
        let mut q: Vec<(&str, String)> = Vec::new();
        if let Some(v) = self.page {
            q.push(("page", v.to_string()));
        }
        if let Some(v) = self.per_page {
            q.push(("per-page", v.to_string()));
        }
        if let Some(v) = self.search {
            q.push(("search", v));
        }
        if let Some(v) = self.sort {
            q.push(("sort", v));
        }
        if let Some(v) = self.endpoint_id {
            q.push(("endpoint_id", v));
        }
        if let Some(v) = self.event {
            q.push(("event", v));
        }
        if let Some(v) = self.delivered {
            q.push(("delivered", v.to_string()));
        }
        if let Some(v) = self.from {
            q.push(("from", v.to_string()));
        }
        if let Some(v) = self.to {
            q.push(("to", v.to_string()));
        }
        if !q.is_empty() {
            req = req.query(&q);
        }
        self.http.send_paged(req).await
    }
}

/// Webhook endpoints for a specific account.
#[derive(Debug)]
pub struct WebhooksApi<'a> {
    http: &'a HttpClient,
    account_id: String,
}

impl<'a> WebhooksApi<'a> {
    pub(crate) fn new(http: &'a HttpClient, account_id: String) -> Self {
        Self { http, account_id }
    }

    /// Create or replace the account's oldest webhook endpoint.
    ///
    /// Accounts with several endpoints should use
    /// [`update_endpoint`](Self::update_endpoint).
    ///
    /// `PUT /accounts/{account_id}/webhooks/subscriptions`.
    ///
    /// # Request payload
    ///
    /// ```json
    /// {
    ///   "url": "https://hooks.example.invalid/assinafy",
    ///   "email": "user@example.invalid",
    ///   "events": ["document_ready", "signer_signed_document"],
    ///   "is_active": true
    /// }
    /// ```
    ///
    /// # Response payload
    ///
    /// ```json
    /// {
    ///   "status": 200,
    ///   "message": "",
    ///   "data": {
    ///     "events": ["document_ready", "signer_signed_document"],
    ///     "is_active": true,
    ///     "is_held": false,
    ///     "held_since": null,
    ///     "url": "https://hooks.example.invalid/assinafy",
    ///     "email": "user@example.invalid",
    ///     "updated_at": "2026-07-18T02:36:02Z"
    ///   }
    /// }
    /// ```
    pub async fn register(&self, body: &RegisterWebhookBody) -> Result<WebhookSubscription> {
        let path = self.http.path(&[
            "accounts",
            self.account_id.as_str(),
            "webhooks",
            "subscriptions",
        ])?;
        let req = self.http.request(Method::PUT, &path)?.json(body);
        self.http.send_envelope(req).await
    }

    /// Fetch the account's oldest webhook endpoint.
    ///
    /// Accounts with several endpoints should use
    /// [`list_endpoints`](Self::list_endpoints).
    ///
    /// `GET /accounts/{account_id}/webhooks/subscriptions`.
    ///
    /// # Request payload
    ///
    /// No request body. Resource identifiers are supplied in the route.
    ///
    /// # Response payload
    ///
    /// `data` is `null` when no subscription has been registered.
    ///
    /// ```json
    /// {
    ///   "status": 200,
    ///   "message": "",
    ///   "data": {
    ///     "events": ["document_ready", "signer_signed_document"],
    ///     "is_active": false,
    ///     "is_held": false,
    ///     "held_since": null,
    ///     "url": "https://hooks.example.invalid/assinafy",
    ///     "email": "user@example.invalid",
    ///     "updated_at": "2026-07-18T02:36:02Z"
    ///   }
    /// }
    /// ```
    pub async fn get_subscription(&self) -> Result<Option<WebhookSubscription>> {
        let path = self.http.path(&[
            "accounts",
            self.account_id.as_str(),
            "webhooks",
            "subscriptions",
        ])?;
        let req = self.http.request(Method::GET, &path)?;
        self.http.send_envelope(req).await
    }

    /// Stop delivering webhook events to the account's oldest endpoint without
    /// discarding it. Other endpoints are unaffected.
    ///
    /// `PUT /accounts/{account_id}/webhooks/inactivate`. This is the documented
    /// way to disable a webhook — the API has no delete-subscription route
    /// (a `DELETE` on `/accounts/{account_id}/webhooks/subscriptions` returns
    /// `404`). To re-enable
    /// delivery, call [`register`](Self::register) with `is_active = true`.
    ///
    /// # Request payload
    ///
    /// No request body. Resource identifiers are supplied in the route.
    ///
    /// # Response payload
    ///
    /// ```json
    /// {
    ///   "status": 200,
    ///   "message": "",
    ///   "data": {
    ///     "events": ["document_ready", "signer_signed_document"],
    ///     "is_active": false,
    ///     "is_held": false,
    ///     "held_since": null,
    ///     "url": "https://hooks.example.invalid/assinafy",
    ///     "email": "user@example.invalid",
    ///     "updated_at": "2026-07-18T02:36:02Z"
    ///   }
    /// }
    /// ```
    pub async fn inactivate(&self) -> Result<WebhookSubscription> {
        let path = self.http.path(&[
            "accounts",
            self.account_id.as_str(),
            "webhooks",
            "inactivate",
        ])?;
        let req = self.http.request(Method::PUT, &path)?;
        self.http.send_envelope(req).await
    }

    fn endpoint_path(&self, rest: &[&str]) -> Result<String> {
        let mut segments = vec![
            "accounts",
            self.account_id.as_str(),
            "webhooks",
            "endpoints",
        ];
        segments.extend_from_slice(rest);
        self.http.path(&segments)
    }

    /// List the account's webhook endpoints, oldest first.
    ///
    /// `GET /accounts/{account_id}/webhooks/endpoints`.
    ///
    /// # Request payload
    ///
    /// No request body. Resource identifiers are supplied in the route.
    ///
    /// # Response payload
    ///
    /// ```json
    /// {
    ///   "status": 200,
    ///   "message": "",
    ///   "data": [
    ///     {
    ///       "resource": "activity_event_subscription",
    ///       "id": "65f1c2a9b3e4d5f60718293a4b5c6d7e",
    ///       "name": "ERP",
    ///       "url": "https://hooks.example.invalid/assinafy",
    ///       "email": "user@example.invalid",
    ///       "events": ["document_ready", "signer_signed_document"],
    ///       "is_active": true,
    ///       "is_held": false,
    ///       "held_since": null,
    ///       "signing_enabled": true,
    ///       "created_at": "2026-10-01T12:00:00Z",
    ///       "updated_at": "2026-10-01T12:00:00Z"
    ///     }
    ///   ]
    /// }
    /// ```
    pub async fn list_endpoints(&self) -> Result<Vec<WebhookEndpoint>> {
        let req = self.http.request(Method::GET, &self.endpoint_path(&[])?)?;
        self.http.send_envelope(req).await
    }

    /// Find endpoints registered with exactly the supplied URL.
    ///
    /// `GET /accounts/{account_id}/webhooks/endpoints`. Uses the `account:read`
    /// OAuth scope. Returns an empty list when the URL is not registered.
    ///
    /// # Request payload
    ///
    /// No body. Query: `url=https://hooks.example.invalid/assinafy` (URL-encoded).
    ///
    /// # Response payload
    ///
    /// ```json
    /// {
    ///   "status": 200, "message": "",
    ///   "data": [{
    ///     "id": "endpoint-id", "name": "ERP",
    ///     "url": "https://hooks.example.invalid/assinafy", "email": "user@example.invalid",
    ///     "events": ["document_ready"], "is_active": true, "is_held": false,
    ///     "held_since": null, "signing_enabled": true,
    ///     "created_at": "2026-10-16T12:00:00Z", "updated_at": "2026-10-16T12:00:00Z"
    ///   }]
    /// }
    /// ```
    pub async fn list_endpoints_by_url(
        &self,
        url: impl AsRef<str>,
    ) -> Result<Vec<WebhookEndpoint>> {
        let req = self
            .http
            .request(Method::GET, &self.endpoint_path(&[])?)?
            .query(&[("url", url.as_ref())]);
        self.http.send_envelope(req).await
    }

    /// Register a webhook endpoint.
    ///
    /// `POST /accounts/{account_id}/webhooks/endpoints`. `url`, `email` and
    /// `events` are required. Accounts have one endpoint, or up to three on
    /// paid plans; past the limit the API answers `403`, and a `url` already
    /// used by another endpoint answers `400`. With `signing_enabled`, store
    /// [`WebhookEndpoint::secret`] securely from this response. OAuth applications
    /// cannot retrieve it later; if lost, delete and recreate that endpoint.
    ///
    /// # Request payload
    ///
    /// ```json
    /// {
    ///   "url": "https://hooks.example.invalid/assinafy",
    ///   "email": "user@example.invalid",
    ///   "events": ["document_ready", "signer_signed_document"],
    ///   "name": "ERP",
    ///   "is_active": true,
    ///   "signing_enabled": true
    /// }
    /// ```
    ///
    /// # Response payload
    ///
    /// ```json
    /// {
    ///   "status": 200,
    ///   "message": "",
    ///   "data": {
    ///     "resource": "activity_event_subscription",
    ///     "id": "65f1c2a9b3e4d5f60718293a4b5c6d7e",
    ///     "name": "ERP",
    ///     "url": "https://hooks.example.invalid/assinafy",
    ///     "email": "user@example.invalid",
    ///     "events": ["document_ready", "signer_signed_document"],
    ///     "is_active": true,
    ///     "is_held": false,
    ///     "held_since": null,
    ///     "signing_enabled": true,
    ///     "secret": "whsec_ZXhhbXBsZS1yZWRhY3RlZC1zZWNyZXQ=",
    ///     "created_at": "2026-10-01T12:00:00Z",
    ///     "updated_at": "2026-10-01T12:00:00Z"
    ///   }
    /// }
    /// ```
    pub async fn create_endpoint(&self, body: &WebhookEndpointBody) -> Result<WebhookEndpoint> {
        let req = self
            .http
            .request(Method::POST, &self.endpoint_path(&[])?)?
            .json(body);
        self.http.send_envelope(req).await
    }

    /// Fetch one webhook endpoint.
    ///
    /// `GET /accounts/{account_id}/webhooks/endpoints/{endpoint_id}`.
    ///
    /// # Request payload
    ///
    /// No request body. Resource identifiers are supplied in the route.
    ///
    /// # Response payload
    ///
    /// ```json
    /// {
    ///   "status": 200,
    ///   "message": "",
    ///   "data": {
    ///     "resource": "activity_event_subscription",
    ///     "id": "65f1c2a9b3e4d5f60718293a4b5c6d7e",
    ///     "name": "ERP",
    ///     "url": "https://hooks.example.invalid/assinafy",
    ///     "email": "user@example.invalid",
    ///     "events": ["document_ready", "signer_signed_document"],
    ///     "is_active": true,
    ///     "is_held": false,
    ///     "held_since": null,
    ///     "signing_enabled": true,
    ///     "created_at": "2026-10-01T12:00:00Z",
    ///     "updated_at": "2026-10-01T12:00:00Z"
    ///   }
    /// }
    /// ```
    pub async fn get_endpoint<S: AsRef<str>>(&self, endpoint_id: S) -> Result<WebhookEndpoint> {
        let path = self.endpoint_path(&[endpoint_id.as_ref()])?;
        let req = self.http.request(Method::GET, &path)?;
        self.http.send_envelope(req).await
    }

    /// Change a webhook endpoint; only the fields set are updated.
    ///
    /// `PUT /accounts/{account_id}/webhooks/endpoints/{endpoint_id}`.
    ///
    /// # Request payload
    ///
    /// ```json
    /// { "events": ["document_ready"], "is_active": false }
    /// ```
    ///
    /// # Response payload
    ///
    /// ```json
    /// {
    ///   "status": 200,
    ///   "message": "",
    ///   "data": {
    ///     "resource": "activity_event_subscription",
    ///     "id": "65f1c2a9b3e4d5f60718293a4b5c6d7e",
    ///     "name": "ERP",
    ///     "url": "https://hooks.example.invalid/assinafy",
    ///     "email": "user@example.invalid",
    ///     "events": ["document_ready", "signer_signed_document"],
    ///     "is_active": true,
    ///     "is_held": false,
    ///     "held_since": null,
    ///     "signing_enabled": true,
    ///     "created_at": "2026-10-01T12:00:00Z",
    ///     "updated_at": "2026-10-01T12:00:00Z"
    ///   }
    /// }
    /// ```
    pub async fn update_endpoint<S: AsRef<str>>(
        &self,
        endpoint_id: S,
        body: &WebhookEndpointBody,
    ) -> Result<WebhookEndpoint> {
        let path = self.endpoint_path(&[endpoint_id.as_ref()])?;
        let req = self.http.request(Method::PUT, &path)?.json(body);
        self.http.send_envelope(req).await
    }

    /// Delete a webhook endpoint and free its slot.
    ///
    /// `DELETE /accounts/{account_id}/webhooks/endpoints/{endpoint_id}`.
    ///
    /// # Request payload
    ///
    /// No request body. Resource identifiers are supplied in the route.
    ///
    /// # Response payload
    ///
    /// ```json
    /// { "status": 200, "message": "", "data": [] }
    /// ```
    pub async fn delete_endpoint<S: AsRef<str>>(&self, endpoint_id: S) -> Result<()> {
        let path = self.endpoint_path(&[endpoint_id.as_ref()])?;
        let req = self.http.request(Method::DELETE, &path)?;
        self.http.send_no_content(req).await
    }

    /// Delete the endpoint registered with exactly the supplied URL and free its slot.
    ///
    /// `DELETE /accounts/{account_id}/webhooks/endpoints`. Requires the
    /// `webhooks:write` OAuth scope. Returns `404` when the URL is not registered.
    ///
    /// # Request payload
    ///
    /// No body. Query: `url=https://hooks.example.invalid/assinafy` (URL-encoded).
    ///
    /// # Response payload
    ///
    /// ```json
    /// { "status": 200, "message": "", "data": [] }
    /// ```
    pub async fn delete_endpoint_by_url(&self, url: impl AsRef<str>) -> Result<()> {
        if url.as_ref().trim().is_empty() {
            return Err(Error::Validation(
                "webhook endpoint URL must not be empty".into(),
            ));
        }
        let req = self
            .http
            .request(Method::DELETE, &self.endpoint_path(&[])?)?
            .query(&[("url", url.as_ref())]);
        self.http.send_no_content(req).await
    }

    /// Read the secret that signs deliveries to an endpoint.
    ///
    /// `GET /accounts/{account_id}/webhooks/endpoints/{endpoint_id}/secret`.
    /// Answers `400` when signing is disabled. Not available to OAuth
    /// applications; use an API key.
    ///
    /// # Request payload
    ///
    /// No request body. Resource identifiers are supplied in the route.
    ///
    /// # Response payload
    ///
    /// ```json
    /// {
    ///   "status": 200,
    ///   "message": "",
    ///   "data": { "secret": "whsec_ZXhhbXBsZS1yZWRhY3RlZC1zZWNyZXQ=" }
    /// }
    /// ```
    pub async fn endpoint_secret<S: AsRef<str>>(
        &self,
        endpoint_id: S,
    ) -> Result<WebhookEndpointSecret> {
        let path = self.endpoint_path(&[endpoint_id.as_ref(), "secret"])?;
        let req = self.http.request(Method::GET, &path)?;
        self.http.send_envelope(req).await
    }

    /// Replace an endpoint's signing secret and return the new one.
    ///
    /// `POST /accounts/{account_id}/webhooks/endpoints/{endpoint_id}/secret/rotate`.
    /// The old secret stops working immediately. Answers `400` when signing is
    /// disabled. Not available to OAuth applications; use an API key.
    ///
    /// # Request payload
    ///
    /// No request body. Resource identifiers are supplied in the route.
    ///
    /// # Response payload
    ///
    /// ```json
    /// {
    ///   "status": 200,
    ///   "message": "",
    ///   "data": { "secret": "whsec_ZXhhbXBsZS1yZWRhY3RlZC1zZWNyZXQ=" }
    /// }
    /// ```
    pub async fn rotate_endpoint_secret<S: AsRef<str>>(
        &self,
        endpoint_id: S,
    ) -> Result<WebhookEndpointSecret> {
        let path = self.endpoint_path(&[endpoint_id.as_ref(), "secret", "rotate"])?;
        let req = self.http.request(Method::POST, &path)?;
        self.http.send_envelope(req).await
    }

    /// List supported webhook event types.
    ///
    /// `GET /webhooks/event-types`.
    ///
    /// # Request payload
    ///
    /// No request body. Resource identifiers are supplied in the route.
    ///
    /// # Response payload
    ///
    /// ```json
    /// {
    ///   "status": 200,
    ///   "message": "",
    ///   "data": [
    ///     {
    ///       "id": "document_uploaded",
    ///       "description": "Triggered when the User has uploaded a Document"
    ///     },
    ///     {
    ///       "id": "document_metadata_ready",
    ///       "description": "Triggered when the document is ready to be prepared."
    ///     }
    ///   ]
    /// }
    /// ```
    pub async fn event_types(&self) -> Result<Vec<WebhookEventTypeInfo>> {
        let req = self.http.request(Method::GET, "webhooks/event-types")?;
        self.http.send_envelope(req).await
    }

    /// List webhook dispatch history.
    ///
    /// `GET /accounts/{account_id}/webhooks`.
    ///
    /// # Request payload
    ///
    /// See [`ListWebhookDispatchesRequest::send`] for the route and query fields.
    ///
    /// # Response payload
    ///
    /// See [`ListWebhookDispatchesRequest::send`] for the JSON response example.
    pub fn list_dispatches(&self) -> ListWebhookDispatchesRequest<'_> {
        ListWebhookDispatchesRequest {
            http: self.http,
            account_id: &self.account_id,
            page: None,
            per_page: None,
            search: None,
            sort: None,
            endpoint_id: None,
            event: None,
            delivered: None,
            from: None,
            to: None,
        }
    }

    /// Retry one webhook dispatch.
    ///
    /// `POST /accounts/{account_id}/webhooks/{dispatch_id}/retry`.
    ///
    /// # Request payload
    ///
    /// No request body. Resource identifiers are supplied in the route.
    ///
    /// # Response payload
    ///
    /// The manual delivery attempt, with its attempt number and outcome.
    ///
    /// ```json
    /// {
    ///   "status": 200,
    ///   "message": "",
    ///   "data": {
    ///     "id": "103a09cfce51319dd3b3f72ffcdf",
    ///     "event": "signature_requested",
    ///     "activity_id": 8629,
    ///     "endpoint_id": "65f1c2a9b3e4d5f60718293a4b5c6d7e",
    ///     "endpoint": "https://hooks.example.invalid/assinafy",
    ///     "payload": { "id": 8629, "event": "signature_requested", "object": {} },
    ///     "delivered": true,
    ///     "webhook_id": "9f86d081884c7d659a2feaa0c55ad015",
    ///     "attempt": 2,
    ///     "kind": "manual",
    ///     "will_retry": false,
    ///     "http_status": 200,
    ///     "response_body": "{\"ok\": true}",
    ///     "error": null,
    ///     "created_at": "2026-07-15T20:04:36Z",
    ///     "updated_at": "2026-07-15T20:04:36Z"
    ///   }
    /// }
    /// ```
    pub async fn retry_dispatch<S: AsRef<str>>(&self, dispatch_id: S) -> Result<WebhookDispatch> {
        let path = self.http.path(&[
            "accounts",
            self.account_id.as_str(),
            "webhooks",
            dispatch_id.as_ref(),
            "retry",
        ])?;
        let req = self.http.request(Method::POST, &path)?;
        self.http.send_envelope(req).await
    }
}

/// How far a delivery's `webhook-timestamp` may be from the local clock
/// before [`verify_webhook_signature`] rejects it as a replay.
pub const WEBHOOK_TIMESTAMP_TOLERANCE: Duration = Duration::from_secs(300);

/// Verify a signed webhook delivery (Standard Webhooks).
///
/// Pass the endpoint's `whsec_…` secret (from
/// [`WebhooksApi::endpoint_secret`]), the `webhook-id`, `webhook-timestamp`
/// and `webhook-signature` request headers, and the **raw** request body
/// exactly as received. Returns `Ok(())` when one of the space-separated
/// `v1,<signature>` entries matches the HMAC-SHA256 of
/// `{webhook-id}.{webhook-timestamp}.{body}` and the timestamp is within
/// [`WEBHOOK_TIMESTAMP_TOLERANCE`] of the local clock; otherwise
/// [`Error::Validation`]. The comparison is constant-time.
///
/// # Request payload
///
/// The delivery as received by your endpoint:
///
/// ```text
/// POST /webhooks/assinafy HTTP/1.1
/// Content-Type: application/json
/// webhook-id: msg_2mDQ9JcP1d3r7Uy0yWzFqH
/// webhook-timestamp: 1790812800
/// webhook-signature: v1,K5oZfzN95Z9UVu1EsfQmfVNQhnkZ2pj9o9NDN/H/pI4=
///
/// {"id":8629,"event":"document_ready","account_id":"acc_1234567890abcdef12345678",...}
/// ```
pub fn verify_webhook_signature(
    secret: &str,
    webhook_id: &str,
    webhook_timestamp: &str,
    webhook_signature: &str,
    body: &[u8],
) -> Result<()> {
    let now = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map_err(|_| Error::Validation("system clock is before the Unix epoch".into()))?
        .as_secs();
    verify_at(
        secret,
        webhook_id,
        webhook_timestamp,
        webhook_signature,
        body,
        now,
    )
}

fn verify_at(
    secret: &str,
    webhook_id: &str,
    webhook_timestamp: &str,
    webhook_signature: &str,
    body: &[u8],
    now: u64,
) -> Result<()> {
    let invalid = |reason: &str| Error::Validation(format!("webhook signature: {reason}"));
    let key = secret
        .strip_prefix("whsec_")
        .and_then(|key| BASE64.decode(key).ok())
        .filter(|key| !key.is_empty())
        .ok_or_else(|| invalid("secret is not a whsec_ base64 key"))?;
    let timestamp: u64 = webhook_timestamp
        .trim()
        .parse()
        .map_err(|_| invalid("timestamp is not a Unix time"))?;
    if now.abs_diff(timestamp) > WEBHOOK_TIMESTAMP_TOLERANCE.as_secs() {
        return Err(invalid("timestamp is outside the tolerance"));
    }
    let mut mac = Hmac::<Sha256>::new_from_slice(&key).map_err(|_| invalid("invalid key"))?;
    mac.update(webhook_id.as_bytes());
    mac.update(b".");
    mac.update(webhook_timestamp.as_bytes());
    mac.update(b".");
    mac.update(body);
    let matched = webhook_signature
        .split_whitespace()
        .filter_map(|entry| entry.strip_prefix("v1,"))
        .filter_map(|signature| BASE64.decode(signature).ok())
        .any(|signature| mac.clone().verify_slice(&signature).is_ok());
    if matched {
        Ok(())
    } else {
        Err(invalid("no matching v1 signature"))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    // Standard Webhooks reference vector (standard-webhooks/standard-webhooks,
    // libraries/javascript/src/index.test.ts).
    const SECRET: &str = "whsec_MfKQ9r8GKYqrTwjUPD8ILPZIo2LaLaSw";
    const ID: &str = "msg_p5jXN8AQM9LWM0D4loKWxJek";
    const TIMESTAMP: &str = "1614265330";
    const BODY: &[u8] = br#"{"test": 2432232314}"#;
    const SIGNATURE: &str = "v1,g0hM9SsE+OTPJTGt/tmIKtSyZlE3uFJELVlNIOLJ1OE=";
    const NOW: u64 = 1_614_265_330;

    #[test]
    fn accepts_reference_signature_among_several() {
        let header = format!("v1,bm90LXRoaXMtb25l {SIGNATURE} v2,ignored");
        verify_at(SECRET, ID, TIMESTAMP, &header, BODY, NOW + 299).unwrap();
    }

    #[test]
    fn rejects_tampering_replay_and_bad_secret() {
        assert!(verify_at(SECRET, ID, TIMESTAMP, SIGNATURE, b"{}", NOW).is_err());
        assert!(verify_at(SECRET, "msg_other", TIMESTAMP, SIGNATURE, BODY, NOW).is_err());
        assert!(verify_at(SECRET, ID, TIMESTAMP, SIGNATURE, BODY, NOW + 301).is_err());
        assert!(verify_at(SECRET, ID, TIMESTAMP, SIGNATURE, BODY, NOW - 301).is_err());
        assert!(
            verify_at(
                "MfKQ9r8GKYqrTwjUPD8ILPZIo2LaLaSw",
                ID,
                TIMESTAMP,
                SIGNATURE,
                BODY,
                NOW
            )
            .is_err()
        );
        assert!(verify_at(SECRET, ID, TIMESTAMP, "", BODY, NOW).is_err());
    }

    #[test]
    fn rejects_a_signature_made_with_an_empty_key() {
        let mut mac = Hmac::<Sha256>::new_from_slice(b"").unwrap();
        mac.update(format!("{ID}.{TIMESTAMP}.").as_bytes());
        mac.update(BODY);
        let signature = format!("v1,{}", BASE64.encode(mac.finalize().into_bytes()));
        assert!(verify_at("whsec_", ID, TIMESTAMP, &signature, BODY, NOW).is_err());
    }
}
