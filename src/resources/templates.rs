//! Template endpoints.

use bytes::Bytes;
use reqwest::Method;
use serde::{Deserialize, Serialize};

use crate::error::Result;
use crate::http::HttpClient;
use crate::models::{CostEstimate, Document, Template};
use crate::pagination::Page;

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
    /// `GET /accounts/{account_id}/templates/{template_id}`.
    pub async fn get<S: AsRef<str>>(&self, template_id: S) -> Result<Template> {
        let path = format!(
            "accounts/{}/templates/{}",
            self.account_id,
            template_id.as_ref()
        );
        let req = self.http.request(Method::GET, &path)?;
        self.http.send_envelope(req).await
    }

    /// Create a template.
    ///
    /// `POST /accounts/{account_id}/templates`.
    ///
    /// The public template object reference names this endpoint, but the API
    /// page does not publish a dedicated request-body table. This method accepts
    /// any serializable JSON body so callers can pass the fields enabled for
    /// their Assinafy account.
    pub async fn create<B>(&self, body: &B) -> Result<Template>
    where
        B: Serialize + ?Sized,
    {
        let path = format!("accounts/{}/templates", self.account_id);
        let req = self.http.request(Method::POST, &path)?.json(body);
        self.http.send_data(req).await
    }

    /// Update a template.
    ///
    /// `PUT /accounts/{account_id}/templates/{template_id}`.
    ///
    /// The API reference documents the returned template object but not a fixed
    /// body schema, so this accepts any serializable JSON body.
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

    /// Download a rendered template page.
    ///
    /// `GET /accounts/{account_id}/templates/{template_id}/pages/{page_id}/download`.
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
