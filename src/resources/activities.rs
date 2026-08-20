//! Document activity log endpoints.

use reqwest::Method;

use crate::error::Result;
use crate::http::HttpClient;
use crate::models::Activity;
use crate::pagination::Page;

/// Builder for `GET /documents/{documentId}/activities`.
///
/// The production contract has no pagination parameters. The optional page and
/// sort controls are retained for older deployments that support them.
#[derive(Debug, Clone)]
pub struct ListActivitiesRequest<'a> {
    http: &'a HttpClient,
    document_id: String,
    page: Option<u32>,
    per_page: Option<u32>,
    sort: Option<String>,
}

impl<'a> ListActivitiesRequest<'a> {
    /// Legacy 1-based page number.
    pub fn page(mut self, page: u32) -> Self {
        self.page = Some(page);
        self
    }

    /// Legacy results-per-page control.
    pub fn per_page(mut self, per_page: u32) -> Self {
        self.per_page = Some(per_page);
        self
    }

    /// Legacy sort expression (e.g. `"-created_at"`).
    pub fn sort<S: Into<String>>(mut self, sort: S) -> Self {
        self.sort = Some(sort.into());
        self
    }

    /// Execute the request.
    ///
    /// `GET /documents/{documentId}/activities`.
    ///
    /// # Response payload
    ///
    /// ```json
    /// {
    ///   "status": 200,
    ///   "message": "",
    ///   "data": [
    ///     {
    ///       "id": 15442,
    ///       "event": "document_metadata_ready",
    ///       "message": "Documento processado.",
    ///       "payload": {},
    ///       "origin": null,
    ///       "created_at": "2026-07-20T16:30:23Z"
    ///     },
    ///     {
    ///       "id": 15441,
    ///       "event": "document_uploaded",
    ///       "message": "Documento criado.",
    ///       "payload": null,
    ///       "origin": { "ip": "203.0.113.10", "user-agent": "curl/8.7.1" },
    ///       "created_at": "2026-07-20T16:30:21Z"
    ///     }
    ///   ]
    /// }
    /// ```
    ///
    /// The current production response is a flat array. The SDK wraps it in a
    /// [`Page`] so older deployments can retain any pagination headers they
    /// return; production responses have empty pagination metadata.
    pub async fn send(self) -> Result<Page<Activity>> {
        let path = format!("documents/{}/activities", self.document_id);
        let mut req = self.http.request(Method::GET, &path)?;
        let mut q: Vec<(&str, String)> = Vec::new();
        if let Some(v) = self.page {
            q.push(("page", v.to_string()));
        }
        if let Some(v) = self.per_page {
            q.push(("per-page", v.to_string()));
        }
        if let Some(v) = self.sort {
            q.push(("sort", v));
        }
        if !q.is_empty() {
            req = req.query(&q);
        }
        self.http.send_paged(req).await
    }
}

/// Activity-log endpoints.
#[derive(Debug)]
pub struct ActivitiesApi<'a> {
    http: &'a HttpClient,
}

impl<'a> ActivitiesApi<'a> {
    pub(crate) fn new(http: &'a HttpClient) -> Self {
        Self { http }
    }

    /// List activities for a document.
    ///
    /// `GET /documents/{documentId}/activities`.
    pub fn list<S: Into<String>>(&self, document_id: S) -> ListActivitiesRequest<'_> {
        ListActivitiesRequest {
            http: self.http,
            document_id: document_id.into(),
            page: None,
            per_page: None,
            sort: None,
        }
    }
}
