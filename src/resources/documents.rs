//! Document endpoints.

use std::path::Path;

use bytes::Bytes;
use reqwest::Method;
use reqwest::multipart::{Form, Part};

use crate::error::{Error, Result};
use crate::http::HttpClient;
use crate::models::{ArtifactName, Document, DocumentStatusInfo, DocumentVerification};
use crate::pagination::Page;

/// Builder for `GET /accounts/{account_id}/documents`.
#[derive(Debug, Default, Clone)]
pub struct ListDocumentsRequest {
    page: Option<u32>,
    per_page: Option<u32>,
    search: Option<String>,
    sort: Option<String>,
    status: Option<String>,
    tags: Vec<String>,
    method: Option<String>,
}

impl ListDocumentsRequest {
    /// 1-based page number.
    pub fn page(mut self, page: u32) -> Self {
        self.page = Some(page);
        self
    }

    /// Results per page (server caps at 100).
    pub fn per_page(mut self, per_page: u32) -> Self {
        self.per_page = Some(per_page);
        self
    }

    /// Free-text search term.
    pub fn search<S: Into<String>>(mut self, term: S) -> Self {
        self.search = Some(term.into());
        self
    }

    /// Sort expression (e.g. `"-created_at"`).
    pub fn sort<S: Into<String>>(mut self, sort: S) -> Self {
        self.sort = Some(sort.into());
        self
    }

    /// Filter by status code.
    pub fn status<S: Into<String>>(mut self, status: S) -> Self {
        self.status = Some(status.into());
        self
    }

    /// Filter by one or more tag identifiers.
    pub fn tags<I, S>(mut self, tags: I) -> Self
    where
        I: IntoIterator<Item = S>,
        S: Into<String>,
    {
        self.tags = tags.into_iter().map(Into::into).collect();
        self
    }

    /// Filter by assignment method.
    pub fn method<S: Into<String>>(mut self, method: S) -> Self {
        self.method = Some(method.into());
        self
    }

    fn into_query(self) -> Vec<(&'static str, String)> {
        let mut q = Vec::new();
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
        if let Some(v) = self.status {
            q.push(("status", v));
        }
        if !self.tags.is_empty() {
            q.push(("tags", self.tags.join(",")));
        }
        if let Some(v) = self.method {
            q.push(("method", v));
        }
        q
    }
}

/// Builder for `GET /accounts/{account_id}/documents/search` — the lightweight
/// document search endpoint.
#[derive(Debug, Clone)]
pub struct SearchDocumentsRequest {
    search: String,
    status: Option<String>,
    page: Option<u32>,
    per_page: Option<u32>,
}

impl SearchDocumentsRequest {
    /// Build a search request for the given free-text term.
    pub fn new<S: Into<String>>(term: S) -> Self {
        Self {
            search: term.into(),
            status: None,
            page: None,
            per_page: None,
        }
    }

    /// Filter by status code.
    pub fn status<S: Into<String>>(mut self, status: S) -> Self {
        self.status = Some(status.into());
        self
    }

    /// 1-based page number.
    pub fn page(mut self, page: u32) -> Self {
        self.page = Some(page);
        self
    }

    /// Results per page (server caps at 100).
    pub fn per_page(mut self, per_page: u32) -> Self {
        self.per_page = Some(per_page);
        self
    }

    fn into_query(self) -> Vec<(&'static str, String)> {
        let mut q = vec![("search", self.search)];
        if let Some(v) = self.status {
            q.push(("status", v));
        }
        if let Some(v) = self.page {
            q.push(("page", v.to_string()));
        }
        if let Some(v) = self.per_page {
            q.push(("per-page", v.to_string()));
        }
        q
    }
}

/// Body for `POST /accounts/{account_id}/documents` (multipart/form-data).
///
/// The API requires a `file` part containing the PDF (≤ 25 MB, ≤ 2000 pages).
pub struct UploadDocumentRequest {
    filename: String,
    mime: String,
    bytes: Bytes,
}

impl UploadDocumentRequest {
    /// Construct from in-memory bytes.
    pub fn from_bytes<S: Into<String>>(filename: S, bytes: impl Into<Bytes>) -> Self {
        UploadDocumentRequest {
            filename: filename.into(),
            mime: "application/pdf".to_string(),
            bytes: bytes.into(),
        }
    }

    /// Construct by reading a local file.
    pub async fn from_path<P: AsRef<Path>>(path: P) -> Result<Self> {
        let p = path.as_ref();
        let filename = p
            .file_name()
            .and_then(|n| n.to_str())
            .ok_or_else(|| Error::Config(format!("invalid filename: {}", p.display())))?
            .to_owned();
        let bytes = tokio::fs::read(p).await?;
        Ok(UploadDocumentRequest::from_bytes(filename, bytes))
    }

    /// Override the MIME type (defaults to `application/pdf`).
    pub fn content_type<S: Into<String>>(mut self, mime: S) -> Self {
        self.mime = mime.into();
        self
    }

    fn into_form(self) -> Result<Form> {
        let part = Part::stream(self.bytes)
            .file_name(self.filename)
            .mime_str(&self.mime)
            .map_err(|e| Error::Config(format!("invalid mime `{}`: {e}", self.mime)))?;
        Ok(Form::new().part("file", part))
    }
}

/// Document endpoints.
#[derive(Debug)]
pub struct DocumentsApi<'a> {
    http: &'a HttpClient,
}

impl<'a> DocumentsApi<'a> {
    pub(crate) fn new(http: &'a HttpClient) -> Self {
        Self { http }
    }

    /// List all known document statuses.
    ///
    /// `GET /documents/statuses`.
    ///
    /// # Response payload
    ///
    /// ```json
    /// { "status": 200, "message": "", "data": [
    ///   { "code": "metadata_ready", "deletable": true },
    ///   { "code": "uploaded", "deletable": false }
    /// ]}
    /// ```
    pub async fn statuses(&self) -> Result<Vec<DocumentStatusInfo>> {
        let req = self.http.request(Method::GET, "documents/statuses")?;
        self.http.send_envelope(req).await
    }

    /// List documents in an account.
    ///
    /// `GET /accounts/{account_id}/documents`. Paginated via `X-Pagination-*`
    /// response headers; `data` is a bare array of documents.
    ///
    /// # Response payload
    ///
    /// ```json
    /// { "status": 200, "message": "", "data": [
    ///   { "id": "103acccd24234c07858ffddf6d84", "account_id": "102d25a4a1b2c3d4e5f60718", "template_id": null,
    ///     "name": "contract.pdf", "status": "metadata_ready",
    ///     "artifacts": {
    ///       "original": "https://api.example.invalid/v1/documents/103acccd24234c07858ffddf6d84/download/original",
    ///       "thumbnail": "https://api.example.invalid/v1/documents/103acccd24234c07858ffddf6d84/thumbnail" },
    ///     "is_closed": false, "signing_url": "https://app.example.invalid/sign/103acccd24234c07858ffddf6d84", "decline_reason": null,
    ///     "declined_by": null, "tags": [], "created_at": "2026-07-19T14:56:54Z",
    ///     "updated_at": "2026-07-19T14:56:56Z", "assignment": null,
    ///     "pages": [ { "id": "103acccd7080eeee6abb709dfa0e", "number": 1, "width": 1275, "height": 1651,
    ///       "download_url": "https://api.example.invalid/v1/documents/103acccd24234c07858ffddf6d84/pages/103acccd7080eeee6abb709dfa0e/download" } ] }
    /// ]}
    /// ```
    pub async fn list<S: AsRef<str>>(
        &self,
        account_id: S,
        req: ListDocumentsRequest,
    ) -> Result<Page<Document>> {
        let path = self
            .http
            .path(&["accounts", account_id.as_ref(), "documents"])?;
        let query = req.into_query();
        let mut request = self.http.request(Method::GET, &path)?;
        if !query.is_empty() {
            request = request.query(&query);
        }
        self.http.send_paged(request).await
    }

    /// Upload a new document.
    ///
    /// `POST /accounts/{account_id}/documents`. Accepts either an enveloped or
    /// a direct response so the SDK keeps working if the API ever returns the
    /// document object without the `{ status, message, data }` wrapper.
    ///
    /// The request is `multipart/form-data` with a single `file` part (the PDF);
    /// there is no JSON request body. A freshly uploaded document is returned in
    /// status `"uploaded"` with only the `original` artifact and empty `pages`;
    /// the `thumbnail` artifact and `pages` appear once processing completes.
    ///
    /// # Response payload
    ///
    /// ```json
    /// { "status": 200, "message": "", "data": {
    ///   "resource": "document", "id": "103b03a4e5f14a2c9d7e0011a2b3", "account_id": "102d25a4a1b2c3d4e5f60718",
    ///   "template_id": null, "name": "contract.pdf", "status": "uploaded",
    ///   "artifacts": { "original": "https://api.example.invalid/v1/documents/103b03a4e5f14a2c9d7e0011a2b3/download/original" }, "is_closed": false,
    ///   "signing_url": "https://app.example.invalid/sign/103b03a4e5f14a2c9d7e0011a2b3", "decline_reason": null, "declined_by": null,
    ///   "tags": [], "created_at": "2026-07-20T16:30:21Z",
    ///   "updated_at": "2026-07-20T16:30:21Z", "pages": [] } }
    /// ```
    pub async fn upload<S: AsRef<str>>(
        &self,
        account_id: S,
        upload: UploadDocumentRequest,
    ) -> Result<Document> {
        let path = self
            .http
            .path(&["accounts", account_id.as_ref(), "documents"])?;
        let form = upload.into_form()?;
        let req = self.http.request(Method::POST, &path)?.multipart(form);
        self.http.send_data(req).await
    }

    /// Search documents in an account (lightweight).
    ///
    /// `GET /accounts/{account_id}/documents/search`. Like [`list`](Self::list)
    /// this is paginated via `X-Pagination-*` headers, but it is optimised for
    /// free-text lookups and returns a trimmed document shape.
    ///
    /// # Response payload
    ///
    /// ```json
    /// { "status": 200, "message": "", "data": [
    ///   { "id": "103acccd24234c07858ffddf6d84", "name": "contract.pdf", "status": "metadata_ready",
    ///     "account_id": "102d25a4a1b2c3d4e5f60718",
    ///     "artifacts": { "original": "https://api.example.invalid/v1/documents/103acccd24234c07858ffddf6d84/download/original" }, "tags": [] }
    /// ]}
    /// ```
    pub async fn search<S: AsRef<str>>(
        &self,
        account_id: S,
        req: SearchDocumentsRequest,
    ) -> Result<Page<Document>> {
        let path = self
            .http
            .path(&["accounts", account_id.as_ref(), "documents", "search"])?;
        let request = self
            .http
            .request(Method::GET, &path)?
            .query(&req.into_query());
        self.http.send_paged(request).await
    }

    /// Retrieve a document.
    ///
    /// `GET /documents/{document_id}`.
    ///
    /// # Response payload
    ///
    /// ```json
    /// { "status": 200, "message": "", "data": {
    ///   "id": "103acccd24234c07858ffddf6d84", "account_id": "102d25a4a1b2c3d4e5f60718", "template_id": null,
    ///   "name": "contract.pdf", "status": "metadata_ready",
    ///   "artifacts": {
    ///     "original": "https://api.example.invalid/v1/documents/103acccd24234c07858ffddf6d84/download/original",
    ///     "thumbnail": "https://api.example.invalid/v1/documents/103acccd24234c07858ffddf6d84/thumbnail" },
    ///   "is_closed": false, "tags": [], "assignment": null,
    ///   "pages": [ { "id": "103acccd7080eeee6abb709dfa0e", "number": 1, "width": 1275, "height": 1651,
    ///     "download_url": "https://api.example.invalid/v1/documents/103acccd24234c07858ffddf6d84/pages/103acccd7080eeee6abb709dfa0e/download" } ] } }
    /// ```
    pub async fn get<S: AsRef<str>>(&self, document_id: S) -> Result<Document> {
        let path = self.http.path(&["documents", document_id.as_ref()])?;
        let req = self.http.request(Method::GET, &path)?;
        self.http.send_envelope(req).await
    }

    /// Rename a document.
    ///
    /// `PATCH /documents/{document_id}` with body `{ "name": "New name.pdf" }`
    /// (`name` is required, max 255 chars). Returns the updated document.
    ///
    /// # Request payload
    ///
    /// ```json
    /// { "name": "Signed service agreement.pdf" }
    /// ```
    ///
    /// # Response payload
    ///
    /// ```json
    /// { "status": 200, "message": "", "data": {
    ///   "resource": "document", "id": "103e4af8c99c19de9cabda1d22c4",
    ///   "account_id": "acc_1234567890abcdef12345678", "template_id": null,
    ///   "name": "Signed service agreement.pdf", "status": "metadata_ready",
    ///   "artifacts": {
    ///     "original": "https://api.example.invalid/v1/documents/103e4af8c99c19de9cabda1d22c4/download/original",
    ///     "thumbnail": "https://api.example.invalid/v1/documents/103e4af8c99c19de9cabda1d22c4/thumbnail" },
    ///   "is_closed": false, "tags": [], "assignment": null, "pages": [] } }
    /// ```
    pub async fn rename<S: AsRef<str>, N: Into<String>>(
        &self,
        document_id: S,
        name: N,
    ) -> Result<Document> {
        let path = self.http.path(&["documents", document_id.as_ref()])?;
        let req = self
            .http
            .request(Method::PATCH, &path)?
            .json(&serde_json::json!({ "name": name.into() }));
        self.http.send_envelope(req).await
    }

    /// Delete a document.
    ///
    /// `DELETE /documents/{documentId}`. Only documents in a deletable status
    /// (see [`statuses`](Self::statuses)) may be removed.
    ///
    /// # Response payload
    ///
    /// ```json
    /// { "status": 200, "message": "", "data": [] }
    /// ```
    pub async fn delete<S: AsRef<str>>(&self, document_id: S) -> Result<()> {
        let path = self.http.path(&["documents", document_id.as_ref()])?;
        let req = self.http.request(Method::DELETE, &path)?;
        self.http.send_no_content(req).await
    }

    /// Download an artifact (raw bytes + content type).
    ///
    /// `GET /documents/{document_id}/download/{artifact_name}`.
    ///
    /// [`ArtifactName::Thumbnail`] is transparently redirected to
    /// [`download_thumbnail`](Self::download_thumbnail) — the
    /// `download/{artifact_name}` route accepts `original`, `certificated`,
    /// `certificate-page`, `pades`, and `bundle`; thumbnails use the dedicated
    /// `/thumbnail` route.
    ///
    /// # Response
    ///
    /// The response body is the raw artifact bytes (for `original`,
    /// `Content-Type: application/pdf`), not a JSON envelope. The returned tuple
    /// carries the bytes and the response `Content-Type` header.
    pub async fn download_artifact<S: AsRef<str>>(
        &self,
        document_id: S,
        artifact: impl Into<ArtifactName>,
    ) -> Result<(Bytes, String)> {
        let artifact: ArtifactName = artifact.into();
        if artifact == ArtifactName::Thumbnail {
            return self.download_thumbnail(document_id).await;
        }
        let path = self.http.path(&[
            "documents",
            document_id.as_ref(),
            "download",
            artifact.as_str(),
        ])?;
        let req = self.http.request(Method::GET, &path)?;
        self.http.send_download(req).await
    }

    /// Download the preview thumbnail (PNG or JPEG bytes).
    ///
    /// `GET /documents/{document_id}/thumbnail`.
    ///
    /// # Response
    ///
    /// The response body is the raw image bytes (`image/png` or `image/jpeg`),
    /// not a JSON envelope. The returned tuple carries the bytes and the
    /// response `Content-Type` header.
    pub async fn download_thumbnail<S: AsRef<str>>(
        &self,
        document_id: S,
    ) -> Result<(Bytes, String)> {
        let path = self
            .http
            .path(&["documents", document_id.as_ref(), "thumbnail"])?;
        let req = self.http.request(Method::GET, &path)?;
        self.http.send_download(req).await
    }

    /// Download a single document page as JPEG.
    ///
    /// `GET /documents/{document_id}/pages/{page_id}/download`.
    ///
    /// # Response
    ///
    /// The response body is the raw page image bytes (`image/jpeg`), not a JSON
    /// envelope. The returned tuple carries the bytes and the response
    /// `Content-Type` header.
    pub async fn download_page<D: AsRef<str>, P: AsRef<str>>(
        &self,
        document_id: D,
        page_id: P,
    ) -> Result<(Bytes, String)> {
        let path = self.http.path(&[
            "documents",
            document_id.as_ref(),
            "pages",
            page_id.as_ref(),
            "download",
        ])?;
        let req = self.http.request(Method::GET, &path)?;
        self.http.send_download(req).await
    }

    /// Verify the authenticity of a document by its signature hash.
    ///
    /// `GET /documents/{signature_hash}/verify`. Requires no authentication.
    /// When the hash is unknown the call still succeeds with
    /// [`DocumentVerification::is_valid`] set to `false`. Note that
    /// `page_count` and `signer_count` are returned as strings.
    ///
    /// # Response payload
    ///
    /// ```json
    /// { "status": 200, "message": "", "data": {
    ///   "hash": "FE32EDDADE7CBDDCBB934E7402047450B0E59C02", "id": "103acccd24234c07858ffddf6d84",
    ///   "status": "certificated", "page_count": "1", "signer_count": "1",
    ///   "completed_count": 1, "completed_at": "2026-07-19T19:27:44Z",
    ///   "verified_at": "2026-07-19T19:27:46Z", "is_valid": true, "message": "" } }
    /// ```
    pub async fn verify<S: AsRef<str>>(&self, signature_hash: S) -> Result<DocumentVerification> {
        let path = self
            .http
            .path(&["documents", signature_hash.as_ref(), "verify"])?;
        let req = self.http.request_public(Method::GET, &path)?;
        self.http.send_envelope(req).await
    }
}
