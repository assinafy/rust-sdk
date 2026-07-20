//! Webhook subscription and dispatch endpoints.

use reqwest::Method;
use serde::{Deserialize, Serialize};

use crate::error::Result;
use crate::http::HttpClient;
use crate::models::{WebhookDispatch, WebhookEventTypeInfo, WebhookSubscription};
use crate::pagination::Page;

const DEFAULT_EVENTS: [&str; 5] = [
    "document_ready",
    "document_prepared",
    "signer_signed_document",
    "signer_rejected_document",
    "document_processing_failed",
];

/// Body for `PUT /accounts/{account_id}/webhooks/subscriptions`.
///
/// # Request payload
///
/// ```json
/// {
///   "url": "https://hooks.example.com/assinafy",
///   "email": "bill@febacapital.com",
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
            events: DEFAULT_EVENTS
                .iter()
                .map(|event| (*event).to_string())
                .collect(),
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

/// Builder for `GET /accounts/{account_id}/webhooks`.
#[derive(Debug)]
pub struct ListWebhookDispatchesRequest<'a> {
    http: &'a HttpClient,
    account_id: &'a str,
    page: Option<u32>,
    per_page: Option<u32>,
    search: Option<String>,
    sort: Option<String>,
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
    ///       "endpoint": "https://httpbin.org/anything",
    ///       "payload": { "id": 8629, "event": "signature_requested", "object": {} },
    ///       "delivered": true,
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
        let path = format!("accounts/{}/webhooks", self.account_id);
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

    /// Create or replace the account webhook subscription.
    ///
    /// `PUT /accounts/{account_id}/webhooks/subscriptions`.
    ///
    /// # Request payload
    ///
    /// ```json
    /// {
    ///   "url": "https://hooks.example.com/assinafy",
    ///   "email": "bill@febacapital.com",
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
    ///     "url": "https://hooks.example.com/assinafy",
    ///     "email": "bill@febacapital.com",
    ///     "updated_at": "2026-07-18T02:36:02Z"
    ///   }
    /// }
    /// ```
    pub async fn register(&self, body: &RegisterWebhookBody) -> Result<WebhookSubscription> {
        let path = format!("accounts/{}/webhooks/subscriptions", self.account_id);
        let req = self.http.request(Method::PUT, &path)?.json(body);
        self.http.send_envelope(req).await
    }

    /// Fetch the current webhook subscription.
    ///
    /// `GET /accounts/{account_id}/webhooks/subscriptions`.
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
    ///     "url": "https://hooks.example.com/assinafy",
    ///     "email": "bill@febacapital.com",
    ///     "updated_at": "2026-07-18T02:36:02Z"
    ///   }
    /// }
    /// ```
    pub async fn get_subscription(&self) -> Result<Option<WebhookSubscription>> {
        let path = format!("accounts/{}/webhooks/subscriptions", self.account_id);
        let req = self.http.request(Method::GET, &path)?;
        self.http.send_envelope(req).await
    }

    /// Stop delivering webhook events without discarding the subscription.
    ///
    /// `PUT /accounts/{account_id}/webhooks/inactivate`. This is the documented
    /// way to disable a webhook — the API has no delete-subscription route
    /// (a `DELETE` on `.../webhooks/subscriptions` returns `404`). To re-enable
    /// delivery, call [`register`](Self::register) with `is_active = true`.
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
    ///     "url": "https://hooks.example.com/assinafy",
    ///     "email": "bill@febacapital.com",
    ///     "updated_at": "2026-07-18T02:36:02Z"
    ///   }
    /// }
    /// ```
    pub async fn inactivate(&self) -> Result<WebhookSubscription> {
        let path = format!("accounts/{}/webhooks/inactivate", self.account_id);
        let req = self.http.request(Method::PUT, &path)?;
        self.http.send_envelope(req).await
    }

    /// List supported webhook event types.
    ///
    /// `GET /webhooks/event-types`.
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
    pub fn list_dispatches(&self) -> ListWebhookDispatchesRequest<'_> {
        ListWebhookDispatchesRequest {
            http: self.http,
            account_id: &self.account_id,
            page: None,
            per_page: None,
            search: None,
            sort: None,
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
    /// # Response payload
    ///
    /// The re-delivered dispatch, with refreshed `delivered`, `http_status`,
    /// `response_body`, `error`, and `updated_at`.
    ///
    /// ```json
    /// {
    ///   "status": 200,
    ///   "message": "",
    ///   "data": {
    ///     "id": "103a09cfce51319dd3b3f72ffcdf",
    ///     "event": "signature_requested",
    ///     "activity_id": 8629,
    ///     "endpoint": "https://httpbin.org/anything",
    ///     "payload": { "id": 8629, "event": "signature_requested", "object": {} },
    ///     "delivered": true,
    ///     "http_status": 200,
    ///     "response_body": "{\"ok\": true}",
    ///     "error": null,
    ///     "created_at": "2026-07-15T20:04:36Z",
    ///     "updated_at": "2026-07-15T20:04:36Z"
    ///   }
    /// }
    /// ```
    pub async fn retry_dispatch<S: AsRef<str>>(&self, dispatch_id: S) -> Result<WebhookDispatch> {
        let path = format!(
            "accounts/{}/webhooks/{}/retry",
            self.account_id,
            dispatch_id.as_ref()
        );
        let req = self.http.request(Method::POST, &path)?;
        self.http.send_envelope(req).await
    }
}
