//! Tag management and document tagging endpoints.

use reqwest::Method;
use serde::{Deserialize, Serialize};

use crate::error::Result;
use crate::http::HttpClient;
use crate::models::Tag;
use crate::pagination::Page;

/// Body for `POST /accounts/{account_id}/tags`.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct CreateTagBody {
    /// Tag name (1–64 chars, trimmed; case-insensitively unique).
    pub name: String,
    /// 6-character hex color (with or without leading `#`), or `null`.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub color: Option<Option<String>>,
}

impl CreateTagBody {
    /// Build a body with just a name.
    pub fn new<S: Into<String>>(name: S) -> Self {
        Self {
            name: name.into(),
            color: None,
        }
    }

    /// Set the color.
    pub fn color<S: Into<String>>(mut self, color: S) -> Self {
        self.color = Some(Some(color.into()));
        self
    }

    /// Explicitly set the color to `null`.
    pub fn no_color(mut self) -> Self {
        self.color = Some(None);
        self
    }
}

/// Body for `PUT /accounts/{account_id}/tags/{tag_id}`.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct UpdateTagBody {
    /// New name. Omit to leave unchanged.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub name: Option<String>,
    /// New color. Omit to leave unchanged; pass `null` to clear.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub color: Option<Option<String>>,
}

impl UpdateTagBody {
    /// New empty update body. Use the builder methods to set fields.
    pub fn new() -> Self {
        Self::default()
    }

    /// Set the name.
    pub fn name<S: Into<String>>(mut self, name: S) -> Self {
        self.name = Some(name.into());
        self
    }

    /// Set the color.
    pub fn color<S: Into<String>>(mut self, color: S) -> Self {
        self.color = Some(Some(color.into()));
        self
    }

    /// Clear the tag color.
    pub fn clear_color(mut self) -> Self {
        self.color = Some(None);
        self
    }
}

/// Builder for `GET /accounts/{account_id}/tags`.
#[derive(Debug)]
pub struct ListTagsRequest<'a> {
    http: &'a HttpClient,
    account_id: &'a str,
    page: Option<u32>,
    per_page: Option<u32>,
    search: Option<String>,
    sort: Option<String>,
}

impl<'a> ListTagsRequest<'a> {
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
    pub fn search<S: Into<String>>(mut self, term: S) -> Self {
        self.search = Some(term.into());
        self
    }

    /// Sort expression.
    pub fn sort<S: Into<String>>(mut self, sort: S) -> Self {
        self.sort = Some(sort.into());
        self
    }

    /// Execute the request.
    pub async fn send(self) -> Result<Page<Tag>> {
        let path = format!("accounts/{}/tags", self.account_id);
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
        if !q.is_empty() {
            req = req.query(&q);
        }
        self.http.send_paged(req).await
    }
}

/// Tag endpoints for a specific account.
#[derive(Debug)]
pub struct TagsApi<'a> {
    http: &'a HttpClient,
    account_id: String,
}

impl<'a> TagsApi<'a> {
    pub(crate) fn new(http: &'a HttpClient, account_id: String) -> Self {
        Self { http, account_id }
    }

    /// List tags in the account.
    pub fn list(&self) -> ListTagsRequest<'_> {
        ListTagsRequest {
            http: self.http,
            account_id: &self.account_id,
            page: None,
            per_page: None,
            search: None,
            sort: None,
        }
    }

    /// Create a tag.
    ///
    /// `POST /accounts/{account_id}/tags`.
    pub async fn create(&self, body: &CreateTagBody) -> Result<Tag> {
        let path = format!("accounts/{}/tags", self.account_id);
        let req = self.http.request(Method::POST, &path)?.json(body);
        self.http.send_envelope(req).await
    }

    /// Update a tag.
    ///
    /// `PUT /accounts/{account_id}/tags/{tag_id}`.
    pub async fn update<S: AsRef<str>>(&self, tag_id: S, body: &UpdateTagBody) -> Result<Tag> {
        let path = format!("accounts/{}/tags/{}", self.account_id, tag_id.as_ref());
        let req = self.http.request(Method::PUT, &path)?.json(body);
        self.http.send_envelope(req).await
    }

    /// Delete a tag.
    ///
    /// `DELETE /accounts/{account_id}/tags/{tag_id}`.
    pub async fn delete<S: AsRef<str>>(&self, tag_id: S) -> Result<()> {
        self.delete_with_force(tag_id, false).await
    }

    /// Delete a tag with an explicit `force` option.
    ///
    /// `DELETE /accounts/{account_id}/tags/{tag_id}?force=true`.
    pub async fn delete_with_force<S: AsRef<str>>(&self, tag_id: S, force: bool) -> Result<()> {
        let path = format!("accounts/{}/tags/{}", self.account_id, tag_id.as_ref());
        let mut req = self.http.request(Method::DELETE, &path)?;
        if force {
            req = req.query(&[("force", true)]);
        }
        self.http.send_no_content(req).await
    }

    /// List tags currently attached to a document.
    ///
    /// `GET /accounts/{account_id}/documents/{document_id}/tags`.
    pub async fn list_for_document<S: AsRef<str>>(&self, document_id: S) -> Result<Vec<Tag>> {
        let path = format!(
            "accounts/{}/documents/{}/tags",
            self.account_id,
            document_id.as_ref()
        );
        let req = self.http.request(Method::GET, &path)?;
        self.http.send_envelope(req).await
    }

    /// Append tag names to a document (idempotent on existing tags).
    ///
    /// `POST /accounts/{account_id}/documents/{document_id}/tags`.
    pub async fn add_to_document<D: AsRef<str>, I, S>(
        &self,
        document_id: D,
        tags: I,
    ) -> Result<Vec<Tag>>
    where
        I: IntoIterator<Item = S>,
        S: Into<String>,
    {
        let tags: Vec<String> = tags.into_iter().map(Into::into).collect();
        let path = format!(
            "accounts/{}/documents/{}/tags",
            self.account_id,
            document_id.as_ref()
        );
        let req = self
            .http
            .request(Method::POST, &path)?
            .json(&serde_json::json!({ "tags": tags }));
        self.http.send_envelope(req).await
    }

    /// Replace the full set of tag names on a document.
    ///
    /// `PUT /accounts/{account_id}/documents/{document_id}/tags`.
    pub async fn set_on_document<D: AsRef<str>, I, S>(
        &self,
        document_id: D,
        tags: I,
    ) -> Result<Vec<Tag>>
    where
        I: IntoIterator<Item = S>,
        S: Into<String>,
    {
        let tags: Vec<String> = tags.into_iter().map(Into::into).collect();
        let path = format!(
            "accounts/{}/documents/{}/tags",
            self.account_id,
            document_id.as_ref()
        );
        let req = self
            .http
            .request(Method::PUT, &path)?
            .json(&serde_json::json!({ "tags": tags }));
        self.http.send_envelope(req).await
    }

    /// Remove a single tag from a document.
    ///
    /// `DELETE /accounts/{account_id}/documents/{document_id}/tags/{tag_id}`.
    pub async fn remove_from_document<D: AsRef<str>, T: AsRef<str>>(
        &self,
        document_id: D,
        tag_id: T,
    ) -> Result<()> {
        let path = format!(
            "accounts/{}/documents/{}/tags/{}",
            self.account_id,
            document_id.as_ref(),
            tag_id.as_ref()
        );
        let req = self.http.request(Method::DELETE, &path)?;
        self.http.send_no_content(req).await
    }
}
