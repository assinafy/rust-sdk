//! Document endpoints.

use std::path::Path;

use bytes::Bytes;
use reqwest::Method;
use reqwest::multipart::{Form, Part};

use crate::error::{Error, Result};
use crate::http::HttpClient;
use crate::models::{ArtifactName, Document, DocumentStatusInfo};
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
        let part = Part::bytes(self.bytes.to_vec())
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
    pub async fn statuses(&self) -> Result<Vec<DocumentStatusInfo>> {
        let req = self.http.request(Method::GET, "documents/statuses")?;
        self.http.send_envelope(req).await
    }

    /// List documents in an account.
    ///
    /// `GET /accounts/{account_id}/documents`.
    pub async fn list<S: AsRef<str>>(
        &self,
        account_id: S,
        req: ListDocumentsRequest,
    ) -> Result<Page<Document>> {
        let path = format!("accounts/{}/documents", account_id.as_ref());
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
    pub async fn upload<S: AsRef<str>>(
        &self,
        account_id: S,
        upload: UploadDocumentRequest,
    ) -> Result<Document> {
        let path = format!("accounts/{}/documents", account_id.as_ref());
        let form = upload.into_form()?;
        let req = self.http.request(Method::POST, &path)?.multipart(form);
        self.http.send_data(req).await
    }

    /// Retrieve a document.
    ///
    /// `GET /documents/{document_id}`.
    pub async fn get<S: AsRef<str>>(&self, document_id: S) -> Result<Document> {
        let path = format!("documents/{}", document_id.as_ref());
        let req = self.http.request(Method::GET, &path)?;
        self.http.send_envelope(req).await
    }

    /// Delete a document.
    ///
    /// `DELETE /documents/{documentId}`.
    pub async fn delete<S: AsRef<str>>(&self, document_id: S) -> Result<()> {
        let path = format!("documents/{}", document_id.as_ref());
        let req = self.http.request(Method::DELETE, &path)?;
        self.http.send_no_content(req).await
    }

    /// Download an artifact (raw bytes + content type).
    ///
    /// `GET /documents/{document_id}/download/{artifact_name}`.
    pub async fn download_artifact<S: AsRef<str>>(
        &self,
        document_id: S,
        artifact: impl Into<ArtifactName>,
    ) -> Result<(Bytes, String)> {
        let artifact: ArtifactName = artifact.into();
        let path = format!(
            "documents/{}/download/{}",
            document_id.as_ref(),
            artifact.as_str()
        );
        let req = self.http.request(Method::GET, &path)?;
        let (bytes, headers) = self.http.send_bytes(req).await?;
        let content_type = headers
            .get(reqwest::header::CONTENT_TYPE)
            .and_then(|v| v.to_str().ok())
            .unwrap_or("application/octet-stream")
            .to_owned();
        Ok((bytes, content_type))
    }

    /// Download the preview thumbnail (PNG or JPEG bytes).
    ///
    /// `GET /documents/{document_id}/thumbnail`.
    pub async fn download_thumbnail<S: AsRef<str>>(
        &self,
        document_id: S,
    ) -> Result<(Bytes, String)> {
        let path = format!("documents/{}/thumbnail", document_id.as_ref());
        let req = self.http.request(Method::GET, &path)?;
        let (bytes, headers) = self.http.send_bytes(req).await?;
        let content_type = headers
            .get(reqwest::header::CONTENT_TYPE)
            .and_then(|v| v.to_str().ok())
            .unwrap_or("application/octet-stream")
            .to_owned();
        Ok((bytes, content_type))
    }

    /// Download a single document page as JPEG.
    ///
    /// `GET /documents/{document_id}/pages/{page_id}/download`.
    pub async fn download_page<D: AsRef<str>, P: AsRef<str>>(
        &self,
        document_id: D,
        page_id: P,
    ) -> Result<(Bytes, String)> {
        let path = format!(
            "documents/{}/pages/{}/download",
            document_id.as_ref(),
            page_id.as_ref()
        );
        let req = self.http.request(Method::GET, &path)?;
        let (bytes, headers) = self.http.send_bytes(req).await?;
        let content_type = headers
            .get(reqwest::header::CONTENT_TYPE)
            .and_then(|v| v.to_str().ok())
            .unwrap_or("application/octet-stream")
            .to_owned();
        Ok((bytes, content_type))
    }

    /// Verify the authenticity of a document by its signature hash.
    ///
    /// `GET /documents/{signature_hash}/verify`. Requires no authentication.
    pub async fn verify<S: AsRef<str>>(&self, signature_hash: S) -> Result<serde_json::Value> {
        let path = format!("documents/{}/verify", signature_hash.as_ref());
        let req = self.http.request(Method::GET, &path)?;
        self.http.send_envelope(req).await
    }
}
