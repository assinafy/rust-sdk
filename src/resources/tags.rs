//! Tag management and document tagging endpoints.

use reqwest::Method;
use serde::{Deserialize, Serialize};

use crate::error::Result;
use crate::http::HttpClient;
use crate::models::Tag;
use crate::pagination::Page;

#[derive(Deserialize)]
struct DeleteTagResult {
    deleted: bool,
}

#[derive(Deserialize)]
struct DetachTagResult {
    detached: bool,
}

/// Body for `POST /accounts/{account_id}/tags`.
///
/// # Request payload
///
/// ```json
/// {
///   "name": "Contracts",
///   "color": "ff8800"
/// }
/// ```
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
///
/// # Request payload
///
/// ```json
/// {
///   "name": "Signed Contracts",
///   "color": "00aa55"
/// }
/// ```
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
///
/// Search is supported everywhere. Pagination and sorting are compatibility
/// controls that deployments may ignore.
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
    /// Compatibility 1-based page number.
    pub fn page(mut self, page: u32) -> Self {
        self.page = Some(page);
        self
    }

    /// Compatibility results-per-page control.
    pub fn per_page(mut self, per_page: u32) -> Self {
        self.per_page = Some(per_page);
        self
    }

    /// Free-text search.
    pub fn search<S: Into<String>>(mut self, term: S) -> Self {
        self.search = Some(term.into());
        self
    }

    /// Compatibility sort expression.
    pub fn sort<S: Into<String>>(mut self, sort: S) -> Self {
        self.sort = Some(sort.into());
        self
    }

    /// Execute the request.
    ///
    /// # Response payload
    ///
    /// ```json
    /// {
    ///   "status": 200,
    ///   "message": "",
    ///   "data": [
    ///     {
    ///       "id": "103aa221874346e6b3de41688526",
    ///       "name": "Contracts",
    ///       "color": null,
    ///       "created_at": "2026-07-18T19:03:45Z",
    ///       "updated_at": "2026-07-18T19:03:45Z"
    ///     }
    ///   ]
    /// }
    /// ```
    pub async fn send(self) -> Result<Page<Tag>> {
        let path = self.http.path(&["accounts", self.account_id, "tags"])?;
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
    ///
    /// `GET /accounts/{account_id}/tags`. The returned builder documents the
    /// optional query parameters and complete response payload in
    /// [`ListTagsRequest::send`].
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
    ///
    /// # Request payload
    ///
    /// ```json
    /// {
    ///   "name": "Contracts",
    ///   "color": "ff8800"
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
    ///     "resource": "tag",
    ///     "id": "103aa221874346e6b3de41688526",
    ///     "name": "Contracts",
    ///     "color": "ff8800",
    ///     "created_at": "2026-07-18T19:03:45Z",
    ///     "updated_at": "2026-07-18T19:03:45Z"
    ///   }
    /// }
    /// ```
    pub async fn create(&self, body: &CreateTagBody) -> Result<Tag> {
        let path = self
            .http
            .path(&["accounts", self.account_id.as_str(), "tags"])?;
        let req = self.http.request(Method::POST, &path)?.json(body);
        self.http.send_envelope(req).await
    }

    /// Update a tag.
    ///
    /// `PUT /accounts/{account_id}/tags/{tag_id}`.
    ///
    /// # Request payload
    ///
    /// ```json
    /// {
    ///   "name": "Signed Contracts",
    ///   "color": "00aa55"
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
    ///     "id": "103aa221874346e6b3de41688526",
    ///     "name": "Signed Contracts",
    ///     "color": "00aa55",
    ///     "created_at": "2026-07-18T19:03:45Z",
    ///     "updated_at": "2026-07-20T16:30:27Z"
    ///   }
    /// }
    /// ```
    pub async fn update<S: AsRef<str>>(&self, tag_id: S, body: &UpdateTagBody) -> Result<Tag> {
        let path = self.http.path(&[
            "accounts",
            self.account_id.as_str(),
            "tags",
            tag_id.as_ref(),
        ])?;
        let req = self.http.request(Method::PUT, &path)?.json(body);
        self.http.send_envelope(req).await
    }

    /// Delete a tag.
    ///
    /// `DELETE /accounts/{account_id}/tags/{tag_id}`.
    ///
    /// # Response payload
    ///
    /// ```json
    /// {
    ///   "status": 200,
    ///   "message": "",
    ///   "data": {
    ///     "deleted": true
    ///   }
    /// }
    /// ```
    pub async fn delete<S: AsRef<str>>(&self, tag_id: S) -> Result<bool> {
        self.delete_with_force(tag_id, false).await
    }

    /// Delete a tag with an explicit `force` option.
    ///
    /// `DELETE /accounts/{account_id}/tags/{tag_id}?force=true`.
    ///
    /// # Response payload
    ///
    /// ```json
    /// {
    ///   "status": 200,
    ///   "message": "",
    ///   "data": {
    ///     "deleted": true
    ///   }
    /// }
    /// ```
    pub async fn delete_with_force<S: AsRef<str>>(&self, tag_id: S, force: bool) -> Result<bool> {
        let path = self.http.path(&[
            "accounts",
            self.account_id.as_str(),
            "tags",
            tag_id.as_ref(),
        ])?;
        let mut req = self.http.request(Method::DELETE, &path)?;
        if force {
            req = req.query(&[("force", true)]);
        }
        let result: DeleteTagResult = self.http.send_envelope(req).await?;
        Ok(result.deleted)
    }

    /// List tags currently attached to a document.
    ///
    /// `GET /accounts/{account_id}/documents/{document_id}/tags`.
    ///
    /// # Response payload
    ///
    /// ```json
    /// {
    ///   "status": 200,
    ///   "message": "",
    ///   "data": [
    ///     {
    ///       "id": "103b03a53c0b5c0ddd885c0391c8",
    ///       "name": "Contracts",
    ///       "color": "ff8800",
    ///       "created_at": "2026-07-20T16:30:27Z",
    ///       "updated_at": "2026-07-20T16:30:27Z"
    ///     }
    ///   ]
    /// }
    /// ```
    pub async fn list_for_document<S: AsRef<str>>(&self, document_id: S) -> Result<Vec<Tag>> {
        let path = self.http.path(&[
            "accounts",
            self.account_id.as_str(),
            "documents",
            document_id.as_ref(),
            "tags",
        ])?;
        let req = self.http.request(Method::GET, &path)?;
        self.http.send_envelope(req).await
    }

    /// Append tags to a document by **name** (idempotent on existing tags).
    ///
    /// `POST /accounts/{account_id}/documents/{document_id}/tags`.
    ///
    /// Each entry is resolved by **name** using a case-insensitive match against
    /// the account's tags. A missing name creates a tag. Passing an identifier
    /// instead creates a tag whose name is that identifier.
    /// [`TagsApi::remove_from_document`] is the only one of these four
    /// document-tag operations that takes a real tag id.
    ///
    /// # Request payload
    ///
    /// ```json
    /// {
    ///   "tags": ["Contracts", "Urgent"]
    /// }
    /// ```
    ///
    /// # Response payload
    ///
    /// ```json
    /// {
    ///   "status": 200,
    ///   "message": "",
    ///   "data": [
    ///     {
    ///       "id": "103b03a53c0b5c0ddd885c0391c8",
    ///       "name": "Contracts",
    ///       "color": "ff8800",
    ///       "created_at": "2026-07-20T16:30:27Z",
    ///       "updated_at": "2026-07-20T16:30:27Z"
    ///     }
    ///   ]
    /// }
    /// ```
    pub async fn add_to_document<D: AsRef<str>, I, S>(
        &self,
        document_id: D,
        tags: I,
    ) -> Result<Vec<Tag>>
    where
        I: IntoIterator<Item = S>,
        S: Into<String>,
    {
        self.write_document_tags(Method::POST, document_id, tags)
            .await
    }

    /// Replace the full set of tags on a document, addressed by **name**.
    ///
    /// `PUT /accounts/{account_id}/documents/{document_id}/tags`.
    ///
    /// Same name-upsert contract as [`TagsApi::add_to_document`] — see that
    /// method's doc comment for details.
    ///
    /// # Request payload
    ///
    /// ```json
    /// {
    ///   "tags": ["Contracts", "Urgent"]
    /// }
    /// ```
    ///
    /// # Response payload
    ///
    /// ```json
    /// {
    ///   "status": 200,
    ///   "message": "",
    ///   "data": [
    ///     {
    ///       "id": "103b03a53c0b5c0ddd885c0391c8",
    ///       "name": "Contracts",
    ///       "color": "ff8800",
    ///       "created_at": "2026-07-20T16:30:27Z",
    ///       "updated_at": "2026-07-20T16:30:27Z"
    ///     }
    ///   ]
    /// }
    /// ```
    pub async fn set_on_document<D: AsRef<str>, I, S>(
        &self,
        document_id: D,
        tags: I,
    ) -> Result<Vec<Tag>>
    where
        I: IntoIterator<Item = S>,
        S: Into<String>,
    {
        self.write_document_tags(Method::PUT, document_id, tags)
            .await
    }

    /// Shared body for [`TagsApi::add_to_document`] / [`TagsApi::set_on_document`],
    /// which differ only in HTTP method (`POST` appends, `PUT` replaces).
    async fn write_document_tags<D: AsRef<str>, I, S>(
        &self,
        method: Method,
        document_id: D,
        tags: I,
    ) -> Result<Vec<Tag>>
    where
        I: IntoIterator<Item = S>,
        S: Into<String>,
    {
        let tags: Vec<String> = tags.into_iter().map(Into::into).collect();
        let path = self.http.path(&[
            "accounts",
            self.account_id.as_str(),
            "documents",
            document_id.as_ref(),
            "tags",
        ])?;
        let req = self
            .http
            .request(method, &path)?
            .json(&serde_json::json!({ "tags": tags }));
        self.http.send_envelope(req).await
    }

    /// Remove a single tag from a document.
    ///
    /// `DELETE /accounts/{account_id}/documents/{document_id}/tags/{tag_id}`.
    ///
    /// # Response payload
    ///
    /// ```json
    /// {
    ///   "status": 200,
    ///   "message": "",
    ///   "data": {
    ///     "detached": true
    ///   }
    /// }
    /// ```
    pub async fn remove_from_document<D: AsRef<str>, T: AsRef<str>>(
        &self,
        document_id: D,
        tag_id: T,
    ) -> Result<bool> {
        let path = self.http.path(&[
            "accounts",
            self.account_id.as_str(),
            "documents",
            document_id.as_ref(),
            "tags",
            tag_id.as_ref(),
        ])?;
        let req = self.http.request(Method::DELETE, &path)?;
        let result: DetachTagResult = self.http.send_envelope(req).await?;
        Ok(result.detached)
    }
}
