//! Template endpoints.
//!
//! Templates are reusable documents with predefined roles and field
//! placements. Only three template operations are published in the official
//! OpenAPI reference — [`list`](TemplatesApi::list),
//! [`create_document`](TemplatesApi::create_document) and
//! [`estimate_cost`](TemplatesApi::estimate_cost). The remaining CRUD methods
//! ([`create`](TemplatesApi::create), [`get`](TemplatesApi::get),
//! [`update`](TemplatesApi::update), [`delete`](TemplatesApi::delete) and
//! [`download_page`](TemplatesApi::download_page)) target routes that are **not
//! part of the published spec** but are implemented by the live API; they were
//! verified against the sandbox and are provided for completeness. Their
//! request/response shapes may change without notice.

use std::path::Path;

use bytes::Bytes;
use reqwest::Method;
use reqwest::multipart::{Form, Part};
use serde::{Deserialize, Serialize};

use crate::error::{Error, Result};
use crate::http::HttpClient;
use crate::models::{CostEstimate, Document, Template};
use crate::pagination::Page;

/// Multipart body for creating a template from a source file
/// (`POST /accounts/{account_id}/templates`).
pub struct CreateTemplateRequest {
    filename: String,
    mime: String,
    bytes: Bytes,
}

impl CreateTemplateRequest {
    /// Construct from in-memory bytes (a PDF).
    pub fn from_bytes<S: Into<String>>(filename: S, bytes: impl Into<Bytes>) -> Self {
        Self {
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
        Ok(Self::from_bytes(filename, bytes))
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

/// Builder for `GET /accounts/{account_id}/templates`.
#[derive(Debug)]
pub struct ListTemplatesRequest<'a> {
    http: &'a HttpClient,
    account_id: &'a str,
    page: Option<u32>,
    per_page: Option<u32>,
    search: Option<String>,
    sort: Option<String>,
    status: Option<String>,
    tags: Vec<String>,
}

impl<'a> ListTemplatesRequest<'a> {
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

    /// Filter by template status.
    pub fn status<S: Into<String>>(mut self, status: S) -> Self {
        self.status = Some(status.into());
        self
    }

    /// Filter by tag IDs.
    pub fn tags<I, S>(mut self, tags: I) -> Self
    where
        I: IntoIterator<Item = S>,
        S: Into<String>,
    {
        self.tags = tags.into_iter().map(Into::into).collect();
        self
    }

    /// Execute the request.
    ///
    /// `GET /accounts/{account_id}/templates`. Pagination is carried in
    /// `X-Pagination-*` response headers; the body is the enveloped array
    /// (`default_document_tags` is omitted from list items).
    ///
    /// # Response payload
    ///
    /// ```json
    /// { "status": 200, "message": "", "data": [
    ///   {
    ///     "resource": "template",
    ///     "id": "103b03b8e5f14a2c9d7e0011a2b3",
    ///     "name": "service-agreement.pdf",
    ///     "document_name": "Service Agreement",
    ///     "message": "Please review and sign.",
    ///     "status": "ready",
    ///     "pages": [
    ///       { "id": "103b03b8c1a0", "number": 1, "height": 1651, "width": 1275,
    ///         "download_url": "https://sandbox.assinafy.com.br/v1/accounts/102d25a4.../templates/103b03b8.../pages/103b03b8.../download" }
    ///     ],
    ///     "roles": [
    ///       { "id": "fa8c14f32d732271e071998246e", "name": "Signer",
    ///         "assignment_type": "Signer",
    ///         "created_at": "2026-07-19T14:56:54Z", "updated_at": "2026-07-19T14:56:54Z" }
    ///     ],
    ///     "tags": [],
    ///     "created_at": "2026-07-19T14:56:54Z",
    ///     "updated_at": "2026-07-19T14:56:56Z"
    ///   }
    /// ] }
    /// ```
    pub async fn send(self) -> Result<Page<Template>> {
        let path = format!("accounts/{}/templates", self.account_id);
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
        if let Some(v) = self.status {
            q.push(("status", v));
        }
        if !self.tags.is_empty() {
            q.push(("tags", self.tags.join(",")));
        }
        if !q.is_empty() {
            req = req.query(&q);
        }
        self.http.send_paged(req).await
    }
}

/// Per-role signer binding emitted when creating a document from a template.
///
/// # Request payload
///
/// One entry in the `signers[]` array of the create/estimate body. `role_id`
/// is always required; `id` binds an existing signer (required by the create
/// endpoint), while inline signer fields resolve one by name/email.
///
/// ```json
/// {
///   "role_id": "fa8c14f32d732271e071998246e",
///   "id": "fa8c140cb49b79f940aab95fddd",
///   "verification_method": "Email",
///   "notification_methods": ["Email"],
///   "step": 1
/// }
/// ```
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TemplateDocumentSigner {
    /// Role identifier from the template.
    pub role_id: String,
    /// Existing signer identifier to bind to the role.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub id: Option<String>,
    /// Full name when creating or resolving a signer inline.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub full_name: Option<String>,
    /// Email address when creating or resolving a signer inline.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub email: Option<String>,
    /// WhatsApp phone number when creating or resolving a signer inline.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub whatsapp_phone_number: Option<String>,
    /// Verification method override.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub verification_method: Option<crate::models::VerificationMethod>,
    /// Notification method overrides.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub notification_methods: Option<Vec<crate::models::NotificationMethod>>,
    /// Optional signing step for sequential signing.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub step: Option<u32>,
}

impl TemplateDocumentSigner {
    /// Reference only a template role.
    ///
    /// This is useful for cost-estimate requests where the API only requires
    /// `role_id` and optional verification or notification methods.
    pub fn role<R: Into<String>>(role_id: R) -> Self {
        Self {
            role_id: role_id.into(),
            id: None,
            full_name: None,
            email: None,
            whatsapp_phone_number: None,
            verification_method: None,
            notification_methods: None,
            step: None,
        }
    }

    /// Bind an existing signer to a template role.
    pub fn existing<R, S>(role_id: R, signer_id: S) -> Self
    where
        R: Into<String>,
        S: Into<String>,
    {
        Self {
            role_id: role_id.into(),
            id: Some(signer_id.into()),
            full_name: None,
            email: None,
            whatsapp_phone_number: None,
            verification_method: None,
            notification_methods: None,
            step: None,
        }
    }

    /// Create an inline signer for a template role.
    pub fn inline<R, N>(role_id: R, full_name: N) -> Self
    where
        R: Into<String>,
        N: Into<String>,
    {
        Self {
            role_id: role_id.into(),
            id: None,
            full_name: Some(full_name.into()),
            email: None,
            whatsapp_phone_number: None,
            verification_method: None,
            notification_methods: None,
            step: None,
        }
    }

    /// Set the email address.
    pub fn email<S: Into<String>>(mut self, email: S) -> Self {
        self.email = Some(email.into());
        self
    }

    /// Set the WhatsApp phone number.
    pub fn whatsapp<S: Into<String>>(mut self, phone: S) -> Self {
        self.whatsapp_phone_number = Some(phone.into());
        self
    }

    /// Set the verification method.
    pub fn verification_method(mut self, method: crate::models::VerificationMethod) -> Self {
        self.verification_method = Some(method);
        self
    }

    /// Set notification methods.
    pub fn notification_methods(mut self, methods: Vec<crate::models::NotificationMethod>) -> Self {
        self.notification_methods = Some(methods);
        self
    }

    /// Set the sequential signing step.
    pub fn step(mut self, step: u32) -> Self {
        self.step = Some(step);
        self
    }
}

/// Backward-compatible alias for older role-binding terminology.
pub type TemplateRoleBinding = TemplateDocumentSigner;

/// A single editor-filled field value supplied when creating a document from a
/// template (`editor_fields[]`).
///
/// # Request payload
///
/// One entry in the `editor_fields[]` array of the create body.
///
/// ```json
/// { "field_id": "fa8c14f3af99d2846d1789de4ba", "value": "Acme Inc." }
/// ```
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EditorField {
    /// Field identifier, matching the `field_id` of an editor field on the
    /// template.
    pub field_id: String,
    /// Value to assign to the field. Usually a string, but the API accepts any
    /// JSON scalar.
    pub value: serde_json::Value,
}

impl EditorField {
    /// Build an editor field value.
    pub fn new<F: Into<String>>(field_id: F, value: impl Into<serde_json::Value>) -> Self {
        Self {
            field_id: field_id.into(),
            value: value.into(),
        }
    }
}

/// Body for `POST /accounts/{account_id}/templates/{template_id}/documents`.
///
/// # Request payload
///
/// ```json
/// {
///   "name": "Service Agreement — Acme",
///   "message": "Please review and sign.",
///   "expires_at": "2026-08-30T23:59:00Z",
///   "signers": [
///     { "role_id": "fa8c14f32d732271e071998246e", "id": "fa8c140cb49b79f940aab95fddd", "step": 1 }
///   ],
///   "editor_fields": [
///     { "field_id": "fa8c14f3af99d2846d1789de4ba", "value": "Acme Inc." }
///   ],
///   "tags": ["contracts"]
/// }
/// ```
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct CreateDocumentFromTemplateBody {
    /// Optional document name (overrides the template default).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub name: Option<String>,
    /// Optional invitation message.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub message: Option<String>,
    /// Optional expiration timestamp in ISO 8601 format. By default the
    /// document does not expire.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub expires_at: Option<String>,
    /// Role → signer bindings (one entry per template role).
    #[serde(skip_serializing_if = "Vec::is_empty", default)]
    pub signers: Vec<TemplateDocumentSigner>,
    /// Editor-filled field values.
    #[serde(skip_serializing_if = "Vec::is_empty", default)]
    pub editor_fields: Vec<EditorField>,
    /// Tag names to attach to the generated document. Names that do not exist
    /// yet are auto-created; the template's default-document-tags are always
    /// applied and merged on top.
    #[serde(skip_serializing_if = "Vec::is_empty", default)]
    pub tags: Vec<String>,
}

impl CreateDocumentFromTemplateBody {
    /// Set the document name override.
    pub fn name<S: Into<String>>(mut self, name: S) -> Self {
        self.name = Some(name.into());
        self
    }

    /// Set the invitation message.
    pub fn message<S: Into<String>>(mut self, message: S) -> Self {
        self.message = Some(message.into());
        self
    }

    /// Set the expiration timestamp (ISO 8601, the documented `expires_at`
    /// field).
    pub fn expires_at<S: Into<String>>(mut self, expires_at: S) -> Self {
        self.expires_at = Some(expires_at.into());
        self
    }

    /// Set the role bindings.
    pub fn roles(mut self, bindings: Vec<TemplateRoleBinding>) -> Self {
        self.signers = bindings;
        self
    }

    /// Set signer bindings.
    pub fn signers(mut self, signers: Vec<TemplateDocumentSigner>) -> Self {
        self.signers = signers;
        self
    }

    /// Set editor-filled field values.
    pub fn editor_fields<I>(mut self, fields: I) -> Self
    where
        I: IntoIterator<Item = EditorField>,
    {
        self.editor_fields = fields.into_iter().collect();
        self
    }

    /// Set the tag names to apply to the generated document.
    pub fn tags<I, S>(mut self, tags: I) -> Self
    where
        I: IntoIterator<Item = S>,
        S: Into<String>,
    {
        self.tags = tags.into_iter().map(Into::into).collect();
        self
    }
}

/// Body for the estimate-cost endpoint — same shape as
/// [`CreateDocumentFromTemplateBody`].
pub type EstimateTemplateCostBody = CreateDocumentFromTemplateBody;

/// Template endpoints for a specific account.
#[derive(Debug)]
pub struct TemplatesApi<'a> {
    http: &'a HttpClient,
    account_id: String,
}

impl<'a> TemplatesApi<'a> {
    pub(crate) fn new(http: &'a HttpClient, account_id: String) -> Self {
        Self { http, account_id }
    }

    /// List templates.
    pub fn list(&self) -> ListTemplatesRequest<'_> {
        ListTemplatesRequest {
            http: self.http,
            account_id: &self.account_id,
            page: None,
            per_page: None,
            search: None,
            sort: None,
            status: None,
            tags: Vec::new(),
        }
    }

    /// Retrieve a template by ID.
    ///
    /// `GET /accounts/{account_id}/templates/{template_id}`. **Not part of the
    /// published spec** — verified against the live API. The single-template
    /// response additionally includes `default_document_tags` and per-page
    /// `fields`.
    pub async fn get<S: AsRef<str>>(&self, template_id: S) -> Result<Template> {
        let path = format!(
            "accounts/{}/templates/{}",
            self.account_id,
            template_id.as_ref()
        );
        let req = self.http.request(Method::GET, &path)?;
        self.http.send_envelope(req).await
    }

    /// Create a template from a source file.
    ///
    /// `POST /accounts/{account_id}/templates` (multipart/form-data, `file`
    /// part). **Not part of the published spec** — verified against the live
    /// API, which requires multipart form data and rejects JSON bodies.
    ///
    /// ```no_run
    /// # use assinafy::{Client, resources::CreateTemplateRequest};
    /// # async fn run(client: Client) -> assinafy::Result<()> {
    /// let file = CreateTemplateRequest::from_path("agreement.pdf").await?;
    /// let template = client.templates("acc_123").create(file).await?;
    /// # Ok(()) }
    /// ```
    ///
    /// # Response payload
    ///
    /// ```json
    /// { "status": 200, "message": "", "data": {
    ///   "resource": "template", "id": "103b03b8...", "name": "agreement.pdf",
    ///   "document_name": "agreement.pdf", "status": "Uploaded",
    ///   "pages": [], "roles": [ { "id": "…", "name": "TemplateEditor", "assignment_type": "Editor" } ],
    ///   "tags": [] } }
    /// ```
    pub async fn create(&self, file: CreateTemplateRequest) -> Result<Template> {
        let path = format!("accounts/{}/templates", self.account_id);
        let form = file.into_form()?;
        let req = self.http.request(Method::POST, &path)?.multipart(form);
        self.http.send_data(req).await
    }

    /// Rename/update a template's metadata.
    ///
    /// `PUT /accounts/{account_id}/templates/{template_id}`. **Not part of the
    /// published spec** — verified against the live API. Accepts any
    /// serializable JSON body (the live API honors `{ "name": "…" }`).
    ///
    /// # Request payload
    ///
    /// ```json
    /// { "name": "Updated template name" }
    /// ```
    pub async fn update<S, B>(&self, template_id: S, body: &B) -> Result<Template>
    where
        S: AsRef<str>,
        B: Serialize + ?Sized,
    {
        let path = format!(
            "accounts/{}/templates/{}",
            self.account_id,
            template_id.as_ref()
        );
        let req = self.http.request(Method::PUT, &path)?.json(body);
        self.http.send_data(req).await
    }

    /// Delete a template.
    ///
    /// `DELETE /accounts/{account_id}/templates/{template_id}`. **Not part of
    /// the published spec** — verified against the live API (returns a `200`
    /// envelope; a subsequent fetch returns `404`).
    pub async fn delete<S: AsRef<str>>(&self, template_id: S) -> Result<()> {
        let path = format!(
            "accounts/{}/templates/{}",
            self.account_id,
            template_id.as_ref()
        );
        let req = self.http.request(Method::DELETE, &path)?;
        self.http.send_no_content(req).await
    }

    /// Download a rendered template page.
    ///
    /// `GET /accounts/{account_id}/templates/{template_id}/pages/{page_id}/download`.
    /// **Not part of the published spec** — the page `download_url` returned by
    /// [`get`](Self::get) points at this route.
    pub async fn download_page<T: AsRef<str>, P: AsRef<str>>(
        &self,
        template_id: T,
        page_id: P,
    ) -> Result<(Bytes, String)> {
        let path = format!(
            "accounts/{}/templates/{}/pages/{}/download",
            self.account_id,
            template_id.as_ref(),
            page_id.as_ref()
        );
        let req = self.http.request(Method::GET, &path)?;
        self.http.send_download(req).await
    }

    /// Create a document from a template.
    ///
    /// `POST /accounts/{account_id}/templates/{template_id}/documents`.
    ///
    /// # Request payload
    ///
    /// ```json
    /// {
    ///   "name": "Service Agreement — Acme",
    ///   "message": "Please review and sign.",
    ///   "expires_at": "2026-08-30T23:59:00Z",
    ///   "signers": [
    ///     { "role_id": "fa8c14f32d732271e071998246e", "id": "fa8c140cb49b79f940aab95fddd", "step": 1 }
    ///   ],
    ///   "editor_fields": [
    ///     { "field_id": "fa8c14f3af99d2846d1789de4ba", "value": "Acme Inc." }
    ///   ],
    ///   "tags": ["contracts"]
    /// }
    /// ```
    ///
    /// # Response payload
    ///
    /// The create response carries a `resource` discriminator; a freshly
    /// generated document starts in `uploaded` with only the `original`
    /// artifact and no pages yet.
    ///
    /// ```json
    /// { "status": 200, "message": "", "data": {
    ///   "resource": "document",
    ///   "id": "103acccd24234c07858ffddf6d84",
    ///   "account_id": "102d25a489f34a275d31a16045fd",
    ///   "template_id": "103b03b8e5f14a2c9d7e0011a2b3",
    ///   "name": "Service Agreement — Acme",
    ///   "status": "uploaded",
    ///   "artifacts": { "original": "https://sandbox.assinafy.com.br/v1/documents/103acccd.../download/original" },
    ///   "is_closed": false,
    ///   "signing_url": "https://app-sandbox.assinafy.com.br/sign/103acccd...",
    ///   "decline_reason": null,
    ///   "declined_by": null,
    ///   "tags": [],
    ///   "assignment": null,
    ///   "pages": [],
    ///   "created_at": "2026-07-19T14:56:54Z",
    ///   "updated_at": "2026-07-19T14:56:54Z"
    /// } }
    /// ```
    pub async fn create_document<S: AsRef<str>>(
        &self,
        template_id: S,
        body: &CreateDocumentFromTemplateBody,
    ) -> Result<Document> {
        let path = format!(
            "accounts/{}/templates/{}/documents",
            self.account_id,
            template_id.as_ref()
        );
        let req = self.http.request(Method::POST, &path)?.json(body);
        self.http.send_data(req).await
    }

    /// Estimate the cost of creating a document from a template.
    ///
    /// `POST /accounts/{account_id}/templates/{template_id}/documents/estimate-cost`.
    /// Only `role_id` is required per signer here (editor roles are ignored for
    /// the cost calculation).
    ///
    /// # Request payload
    ///
    /// ```json
    /// {
    ///   "signers": [
    ///     { "role_id": "fa8c14f32d732271e071998246e", "verification_method": "Whatsapp", "notification_methods": ["Whatsapp"] }
    ///   ]
    /// }
    /// ```
    ///
    /// # Response payload
    ///
    /// ```json
    /// { "status": 200, "message": "", "data": {
    ///   "documents": 1,
    ///   "credits": 0,
    ///   "needs_extra_document": false,
    ///   "extra_document_cost": 0,
    ///   "total_credits": 0,
    ///   "breakdown": [],
    ///   "document_balance": 80,
    ///   "credit_balance": 0,
    ///   "has_sufficient_resources": true,
    ///   "blocking_reason": null,
    ///   "message": null
    /// } }
    /// ```
    pub async fn estimate_cost<S: AsRef<str>>(
        &self,
        template_id: S,
        body: &EstimateTemplateCostBody,
    ) -> Result<CostEstimate> {
        let path = format!(
            "accounts/{}/templates/{}/documents/estimate-cost",
            self.account_id,
            template_id.as_ref()
        );
        let req = self.http.request(Method::POST, &path)?.json(body);
        self.http.send_envelope(req).await
    }
}
