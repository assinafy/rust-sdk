//! List and execute deferred sends. Create them through assignments or envelopes.

use reqwest::Method;
use serde::{Deserialize, Serialize};

use crate::Result;
use crate::http::HttpClient;
use crate::models::ScheduledSend;
use crate::pagination::Page;

/// A scheduled time accepted by the API. Its one-hour to 90-day window is
/// validated by the server against its clock.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(untagged)]
pub enum ScheduledSendTime {
    /// ISO 8601 timestamp, preferably with an explicit offset.
    DateTime(String),
    /// Unix time in seconds.
    Unix(i64),
}

impl From<String> for ScheduledSendTime {
    fn from(value: String) -> Self {
        Self::DateTime(value)
    }
}

impl From<&str> for ScheduledSendTime {
    fn from(value: &str) -> Self {
        Self::DateTime(value.into())
    }
}

impl From<i64> for ScheduledSendTime {
    fn from(value: i64) -> Self {
        Self::Unix(value)
    }
}

/// Scheduled sends belonging to a workspace. All routes require
/// `documents:write` for OAuth, including listing.
#[derive(Debug)]
pub struct ScheduledSendsApi<'a> {
    http: &'a HttpClient,
    account_id: String,
}

impl<'a> ScheduledSendsApi<'a> {
    pub(crate) fn new(http: &'a HttpClient, account_id: String) -> Self {
        Self { http, account_id }
    }

    /// Build a list request, newest first, including every lifecycle state.
    /// See [`ListScheduledSendsRequest::send`] for the route and payloads.
    pub fn list(&self) -> ListScheduledSendsRequest<'_> {
        ListScheduledSendsRequest {
            http: self.http,
            account_id: &self.account_id,
            document_id: None,
            page: None,
            per_page: None,
        }
    }

    /// Cancel a pending send
    /// (`PUT /accounts/{accountId}/scheduled-sends/{scheduledSendId}/cancel`).
    /// Nothing is sent or charged. Any other state returns `409`.
    ///
    /// # Request payload
    /// No body or query parameters.
    ///
    /// # Response payload
    /// ```json
    /// {"status":200,"message":"","data":{"resource":"scheduled_send","id":"scheduled_123","type":"assignment","document_ids":["document_123"],"status":"cancelled","send_at":"2026-12-01T12:00:00Z","assignment_id":null,"envelope_id":null,"failure_reason":null,"created_at":"2026-11-24T12:00:00Z","processed_at":"2026-11-24T12:05:00Z"}}
    /// ```
    pub async fn cancel(&self, scheduled_send_id: &str) -> Result<ScheduledSend> {
        let path = self.http.path(&[
            "accounts",
            &self.account_id,
            "scheduled-sends",
            scheduled_send_id,
            "cancel",
        ])?;
        let req = self.http.request_workspace(Method::PUT, &path)?;
        self.http.send_envelope(req).await
    }

    /// Execute a pending send now
    /// (`PUT /accounts/{accountId}/scheduled-sends/{scheduledSendId}/send`).
    /// Uses the scheduling member's permissions and current prices. A creation
    /// refusal leaves the send pending; cancel it to prevent execution later.
    /// An already processed send returns `409`. Read after an uncertain response.
    ///
    /// # Request payload
    /// No body or query parameters.
    ///
    /// # Response payload
    /// ```json
    /// {"status":200,"message":"","data":{"resource":"scheduled_send","id":"scheduled_123","type":"assignment","document_ids":["document_123"],"status":"sent","send_at":"2026-12-01T12:00:00Z","assignment_id":"assignment_123","envelope_id":null,"failure_reason":null,"created_at":"2026-11-24T12:00:00Z","processed_at":"2026-11-24T12:05:00Z"}}
    /// ```
    pub async fn send_now(&self, scheduled_send_id: &str) -> Result<ScheduledSend> {
        let path = self.http.path(&[
            "accounts",
            &self.account_id,
            "scheduled-sends",
            scheduled_send_id,
            "send",
        ])?;
        let req = self.http.request_workspace(Method::PUT, &path)?;
        self.http.send_envelope(req).await
    }
}

/// Paged scheduled sends, optionally restricted to one document.
#[derive(Debug)]
pub struct ListScheduledSendsRequest<'a> {
    http: &'a HttpClient,
    account_id: &'a str,
    document_id: Option<String>,
    page: Option<u32>,
    per_page: Option<u32>,
}

impl ListScheduledSendsRequest<'_> {
    /// Restrict the list to sends containing this document.
    pub fn document_id(mut self, id: impl Into<String>) -> Self {
        self.document_id = Some(id.into());
        self
    }
    /// Select a one-based page.
    pub fn page(mut self, page: u32) -> Self {
        self.page = Some(page);
        self
    }
    /// Set the page size (default 20, capped at 50 by the API).
    pub fn per_page(mut self, count: u32) -> Self {
        self.per_page = Some(count);
        self
    }

    /// List scheduled sends (`GET /accounts/{accountId}/scheduled-sends`).
    /// Requires `documents:write`, since failures can reveal account balances.
    ///
    /// # Request payload
    /// Logical query parameters:
    /// ```json
    /// {"document_id":"document_123","page":1,"per-page":20}
    /// ```
    ///
    /// # Response payload
    /// ```json
    /// {"status":200,"message":"","data":[{"resource":"scheduled_send","id":"scheduled_123","type":"assignment","document_ids":["document_123"],"status":"pending","send_at":"2026-12-01T12:00:00Z","assignment_id":null,"envelope_id":null,"failure_reason":null,"created_at":"2026-11-24T12:00:00Z","processed_at":null}]}
    /// ```
    /// Pagination metadata is returned in `X-Pagination-*` headers.
    pub async fn send(self) -> Result<Page<ScheduledSend>> {
        let path = self
            .http
            .path(&["accounts", self.account_id, "scheduled-sends"])?;
        let mut query = Vec::new();
        if let Some(id) = self.document_id {
            query.push(("document_id", id));
        }
        if let Some(page) = self.page {
            query.push(("page", page.to_string()));
        }
        if let Some(count) = self.per_page {
            query.push(("per-page", count.to_string()));
        }
        let req = self
            .http
            .request_workspace(Method::GET, &path)?
            .query(&query);
        self.http.send_paged(req).await
    }
}
