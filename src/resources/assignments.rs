//! Assignment (signature-request) endpoints.

use reqwest::Method;
use serde::{Deserialize, Serialize};

use crate::error::{Error, Result};
use crate::http::HttpClient;
use crate::models::{
    Assignment, AssignmentMethod, CostEstimate, NotificationMethod, ResendNotificationResult,
    SignDocumentItem, VerificationMethod, WhatsAppNotification,
};
use crate::pagination::Page;

fn reset_expiration_payload(new_expires_at: Option<&str>) -> serde_json::Value {
    serde_json::json!({ "expires_at": new_expires_at })
}

/// Per-signer override used when creating an assignment.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct CreateAssignmentSigner {
    /// Signer identifier.
    #[serde(skip_serializing_if = "String::is_empty", default)]
    pub id: String,
    /// Optional 1-based sequential step.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub step: Option<u32>,
    /// Override verification method for this signer.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub verification_method: Option<VerificationMethod>,
    /// Override notification methods for this signer.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub notification_methods: Option<Vec<NotificationMethod>>,
}

impl CreateAssignmentSigner {
    /// Convenience constructor with just the signer id.
    pub fn new<S: Into<String>>(signer_id: S) -> Self {
        Self {
            id: signer_id.into(),
            ..Default::default()
        }
    }

    /// Set or replace the signer id.
    pub fn id<S: Into<String>>(mut self, signer_id: S) -> Self {
        self.id = signer_id.into();
        self
    }

    /// Set the sequential signing step.
    pub fn step(mut self, step: u32) -> Self {
        self.step = Some(step);
        self
    }

    /// Set the verification method for this signer.
    pub fn verification_method(mut self, method: VerificationMethod) -> Self {
        self.verification_method = Some(method);
        self
    }

    /// Set notification methods for this signer.
    pub fn notification_methods(mut self, methods: Vec<NotificationMethod>) -> Self {
        self.notification_methods = Some(methods);
        self
    }
}

/// `POST /documents/{document_id}/assignments` request body.
///
/// New integrations should use `signers`. The `signer_ids` field remains
/// available for legacy deployments that still accept that documented alias.
///
/// # Request payload
///
/// ```json
/// {
///   "method": "virtual",
///   "signers": [
///     { "id": "19e6b92e7895332ed9708535d8c", "step": 1,
///       "verification_method": "Email", "notification_methods": ["Email"] }
///   ],
///   "message": "Please review and sign this contract.",
///   "expires_at": "2026-12-31T23:59:59Z",
///   "copy_receivers": []
/// }
/// ```
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CreateAssignmentBody {
    /// Delivery method.
    pub method: AssignmentMethod,
    /// Bulk list of signer identifiers (use when no overrides are needed).
    #[serde(
        rename = "signer_ids",
        alias = "signerIds",
        skip_serializing_if = "Vec::is_empty",
        default
    )]
    pub signer_ids: Vec<String>,
    /// Per-signer configurations (when overrides are needed).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub signers: Option<Vec<CreateAssignmentSigner>>,
    /// Field placements for collect assignments.
    #[serde(skip_serializing_if = "Vec::is_empty", default)]
    pub entries: Vec<AssignmentEntry>,
    /// Optional invitation message.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub message: Option<String>,
    /// Legacy expiration timestamp field accepted by the API.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub expiration: Option<String>,
    /// Optional expiration timestamp (ISO-8601).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub expires_at: Option<String>,
    /// Optional list of copy-receivers (IDs of signers that only receive
    /// the final certificated copy).
    #[serde(
        rename = "copy_receivers",
        skip_serializing_if = "Vec::is_empty",
        default
    )]
    pub copy_receiver_ids: Vec<String>,
}

impl CreateAssignmentBody {
    /// Build a bulk request with one or more signer IDs.
    ///
    /// The IDs are serialized using the current `signers: [{ id }]` request
    /// shape. Use [`CreateAssignmentBody::legacy_signer_ids`] only when an
    /// older deployment specifically requires `signer_ids`.
    pub fn new<I, S>(method: AssignmentMethod, signer_ids: I) -> Self
    where
        I: IntoIterator<Item = S>,
        S: Into<String>,
    {
        let signers = signer_ids
            .into_iter()
            .map(CreateAssignmentSigner::new)
            .collect::<Vec<_>>();
        Self::from_signers(method, signers)
    }

    /// Build a request from per-signer configuration objects.
    pub fn from_signers<I>(method: AssignmentMethod, signers: I) -> Self
    where
        I: IntoIterator<Item = CreateAssignmentSigner>,
    {
        let signers = signers.into_iter().collect::<Vec<_>>();
        Self {
            method,
            signer_ids: Vec::new(),
            signers: (!signers.is_empty()).then_some(signers),
            entries: Vec::new(),
            message: None,
            expiration: None,
            expires_at: None,
            copy_receiver_ids: Vec::new(),
        }
    }

    /// Set the optional invitation message.
    pub fn message<S: Into<String>>(mut self, message: S) -> Self {
        self.message = Some(message.into());
        self
    }

    /// Set the expiration timestamp.
    pub fn expires_at<S: Into<String>>(mut self, ts: S) -> Self {
        self.expires_at = Some(ts.into());
        self
    }

    /// Set the legacy `expiration` timestamp used by collect assignment docs.
    pub fn expiration<S: Into<String>>(mut self, ts: S) -> Self {
        self.expiration = Some(ts.into());
        self
    }

    /// Use per-signer overrides instead of a bulk list.
    ///
    /// Equivalent to [`signers`](Self::signers), which accepts any
    /// `IntoIterator` rather than requiring a `Vec`.
    pub fn with_signers(self, signers: Vec<CreateAssignmentSigner>) -> Self {
        self.signers(signers)
    }

    /// Set per-signer overrides.
    pub fn signers<I>(mut self, signers: I) -> Self
    where
        I: IntoIterator<Item = CreateAssignmentSigner>,
    {
        let signers = signers.into_iter().collect::<Vec<_>>();
        self.signers = Some(signers);
        self.signer_ids.clear();
        self
    }

    /// Send signer IDs using the legacy `signer_ids` request field.
    pub fn legacy_signer_ids<I, S>(mut self, signer_ids: I) -> Self
    where
        I: IntoIterator<Item = S>,
        S: Into<String>,
    {
        self.signer_ids = signer_ids.into_iter().map(Into::into).collect();
        self.signers = None;
        self
    }

    /// Set collect-assignment entries.
    pub fn entries(mut self, entries: Vec<AssignmentEntry>) -> Self {
        self.entries = entries;
        self
    }

    /// Add copy-only recipients.
    pub fn copy_receivers<I, S>(mut self, ids: I) -> Self
    where
        I: IntoIterator<Item = S>,
        S: Into<String>,
    {
        self.copy_receiver_ids = ids.into_iter().map(Into::into).collect();
        self
    }
}

fn validate_create_assignment(body: &CreateAssignmentBody) -> Result<()> {
    if let Some(signers) = &body.signers {
        if signers.is_empty() {
            return Err(Error::Config(
                "assignment creation requires at least one signer".into(),
            ));
        }
        if signers.iter().any(|signer| signer.id.is_empty()) {
            return Err(Error::Config(
                "every assignment signer must have an id".into(),
            ));
        }
    } else if body.signer_ids.is_empty() {
        return Err(Error::Config(
            "assignment creation requires at least one signer".into(),
        ));
    }
    Ok(())
}

/// Entry used to place fields on a document page for collect assignments.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AssignmentEntry {
    /// Document page identifier.
    pub page_id: String,
    /// Fields placed on this page.
    pub fields: Vec<AssignmentField>,
}

impl AssignmentEntry {
    /// Build an assignment entry.
    pub fn new<S: Into<String>>(page_id: S, fields: Vec<AssignmentField>) -> Self {
        Self {
            page_id: page_id.into(),
            fields,
        }
    }
}

/// Field placement used in a collect assignment entry.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AssignmentField {
    /// Signer identifier.
    pub signer_id: String,
    /// Field definition identifier.
    pub field_id: String,
    /// Visual placement settings.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub display_settings: Option<serde_json::Value>,
}

impl AssignmentField {
    /// Build a field placement.
    pub fn new<S, F>(signer_id: S, field_id: F) -> Self
    where
        S: Into<String>,
        F: Into<String>,
    {
        Self {
            signer_id: signer_id.into(),
            field_id: field_id.into(),
            display_settings: None,
        }
    }

    /// Set visual placement settings.
    pub fn display_settings(mut self, settings: impl Into<serde_json::Value>) -> Self {
        self.display_settings = Some(settings.into());
        self
    }
}

/// Body for `POST /documents/{document_id}/assignments/estimate-cost`.
///
/// Kept as an alias for source compatibility. Before sending, the SDK projects
/// it onto the endpoint's documented pricing fields (`method`, signer methods,
/// and `entries`), so signer IDs and create-only metadata are never emitted.
pub type EstimateAssignmentCostBody = CreateAssignmentBody;

#[derive(Serialize)]
struct EstimateAssignmentSignerPayload<'a> {
    #[serde(skip_serializing_if = "Option::is_none")]
    verification_method: Option<&'a VerificationMethod>,
    #[serde(skip_serializing_if = "Option::is_none")]
    notification_methods: Option<&'a Vec<NotificationMethod>>,
}

#[derive(Serialize)]
struct EstimateAssignmentPayload<'a> {
    method: AssignmentMethod,
    #[serde(skip_serializing_if = "Option::is_none")]
    signers: Option<Vec<EstimateAssignmentSignerPayload<'a>>>,
    #[serde(skip_serializing_if = "Vec::is_empty")]
    entries: &'a Vec<AssignmentEntry>,
}

impl<'a> From<&'a EstimateAssignmentCostBody> for EstimateAssignmentPayload<'a> {
    fn from(body: &'a EstimateAssignmentCostBody) -> Self {
        let signers = body.signers.as_ref().map(|signers| {
            signers
                .iter()
                .map(|signer| EstimateAssignmentSignerPayload {
                    verification_method: signer.verification_method.as_ref(),
                    notification_methods: signer.notification_methods.as_ref(),
                })
                .collect()
        });
        Self {
            method: body.method,
            signers,
            entries: &body.entries,
        }
    }
}

/// Builder for `GET /assignments`.
#[derive(Debug)]
pub struct ListAssignmentsRequest<'a> {
    http: &'a HttpClient,
    account_id: Option<String>,
    page: Option<u32>,
    per_page: Option<u32>,
}

impl<'a> ListAssignmentsRequest<'a> {
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

    /// Execute the request.
    ///
    /// `GET /assignments` with optional `page` and `per-page` query
    /// parameters. The SDK also sends the sandbox-compatible `accountId`
    /// parameter when the request came from [`AssignmentsApi::list`].
    ///
    /// # Response payload
    ///
    /// ```json
    /// { "status": 200, "message": "", "data": [
    ///   { "id": "103033c9d2cec233bf65eea04999", "sender_email": "user@example.invalid",
    ///     "method": "virtual", "expires_at": null, "message": "Please sign",
    ///     "copy_receivers": [], "items": [],
    ///     "signers": [ { "id": "19e6b92e7895332ed9708535d8c", "full_name": "Ada Lovelace",
    ///       "email": "user@example.invalid", "whatsapp_phone_number": null,
    ///       "has_accepted_terms": false, "completed": false, "notification_history": [],
    ///       "verification_method": "Email", "notification_methods": ["Email"],
    ///       "step": 1, "notified": true } ],
    ///     "summary": { "signer_count": 1, "completed_count": 0, "signers": [] },
    ///     "signing_urls": [ { "signer_id": "19e6b92e7895332ed9708535d8c", "url": "https://…" } ] }
    /// ]}
    /// ```
    pub async fn send(self) -> Result<Page<Assignment>> {
        let mut query: Vec<(&str, String)> = Vec::with_capacity(3);
        if let Some(account_id) = self.account_id {
            query.push(("accountId", account_id));
        }
        if let Some(v) = self.page {
            query.push(("page", v.to_string()));
        }
        if let Some(v) = self.per_page {
            query.push(("per-page", v.to_string()));
        }
        let mut req = self.http.request(Method::GET, "assignments")?;
        if !query.is_empty() {
            req = req.query(&query);
        }
        self.http.send_paged(req).await
    }
}

/// Assignment endpoints.
#[derive(Debug)]
pub struct AssignmentsApi<'a> {
    http: &'a HttpClient,
}

impl<'a> AssignmentsApi<'a> {
    pub(crate) fn new(http: &'a HttpClient) -> Self {
        Self { http }
    }

    /// List assignments using the sandbox-compatible account context.
    ///
    /// `GET /assignments?accountId={account_id}`. Returns a builder that adds
    /// optional pagination and sends the request. The `accountId` query
    /// parameter is required by the sandbox API. Production callers should
    /// prefer [`Self::list_current`], which follows the published contract.
    ///
    /// # Response payload
    ///
    /// ```json
    /// { "status": 200, "message": "", "data": [
    ///   { "id": "103033c9...", "sender_email": "user@example.invalid", "method": "virtual",
    ///     "expires_at": null, "message": "Please sign", "copy_receivers": [],
    ///     "signers": [ { "id": "19e6b9...", "full_name": "Ada", "email": "user@example.invalid",
    ///       "step": 1, "completed": false, "verification_method": "Email",
    ///       "notification_methods": ["Email"], "notified": true } ],
    ///     "summary": { "signer_count": 1, "completed_count": 0, "signers": [ … ] },
    ///     "signing_urls": [ { "signer_id": "19e6b9...", "url": "https://…" } ] }
    /// ]}
    /// ```
    pub fn list<S: Into<String>>(&self, account_id: S) -> ListAssignmentsRequest<'_> {
        ListAssignmentsRequest {
            http: self.http,
            account_id: Some(account_id.into()),
            page: None,
            per_page: None,
        }
    }

    /// List assignments for the authenticated user's current account.
    ///
    /// `GET /assignments`, with only the documented pagination parameters.
    pub fn list_current(&self) -> ListAssignmentsRequest<'_> {
        ListAssignmentsRequest {
            http: self.http,
            account_id: None,
            page: None,
            per_page: None,
        }
    }

    /// Request signatures from one or more signers.
    ///
    /// `POST /documents/{document_id}/assignments`.
    ///
    /// # Request payload
    ///
    /// ```json
    /// {
    ///   "method": "virtual",
    ///   "signers": [
    ///     { "id": "19e6b92e7895332ed9708535d8c", "step": 1,
    ///       "verification_method": "Email", "notification_methods": ["Email"] }
    ///   ],
    ///   "message": "Please review and sign this contract.",
    ///   "expires_at": "2026-12-31T23:59:59Z"
    /// }
    /// ```
    ///
    /// # Response payload
    ///
    /// ```json
    /// { "status": 200, "message": "", "data": {
    ///   "resource": "assignment", "id": "103033c9d2cec233bf65eea04999",
    ///   "sender_email": "user@example.invalid", "method": "virtual", "expires_at": null,
    ///   "message": "Please review and sign this contract.", "copy_receivers": [],
    ///   "signers": [ { "id": "19e6b92e7895332ed9708535d8c", "full_name": "Ada Lovelace",
    ///     "email": "user@example.invalid", "whatsapp_phone_number": null,
    ///     "has_accepted_terms": false, "completed": false, "notification_history": [],
    ///     "verification_method": "Email", "notification_methods": ["Email"],
    ///     "step": 1, "notified": true } ],
    ///   "items": [ { "id": "103033c9d33326458deb74fc3052", "page": null,
    ///     "signer": { "id": "19e6b92e7895332ed9708535d8c" },
    ///     "field": { "id": "102d25a48bc7357b93f9b8e01b24", "type": "virtual" },
    ///     "display_settings": [], "value": null, "completed": false } ],
    ///   "summary": { "signer_count": 1, "completed_count": 0, "signers": [] },
    ///   "signing_urls": [ { "signer_id": "19e6b92e7895332ed9708535d8c",
    ///     "url": "https://app-sandbox.assinafy.com.br/sign/103033c950d865a248a11c5cf96c" } ] }
    /// }
    /// ```
    pub async fn create<S: AsRef<str>>(
        &self,
        document_id: S,
        body: &CreateAssignmentBody,
    ) -> Result<Assignment> {
        validate_create_assignment(body)?;
        let path = format!("documents/{}/assignments", document_id.as_ref());
        let req = self.http.request(Method::POST, &path)?.json(body);
        self.http.send_data(req).await
    }

    /// Estimate the cost of an assignment without creating it.
    ///
    /// `POST /documents/{document_id}/assignments/estimate-cost`.
    ///
    /// # Request payload
    ///
    /// ```json
    /// {
    ///   "method": "virtual",
    ///   "signers": [
    ///     { "verification_method": "Whatsapp", "notification_methods": ["Whatsapp"] }
    ///   ]
    /// }
    /// ```
    ///
    /// # Response payload
    ///
    /// ```json
    /// { "status": 200, "message": "", "data": {
    ///   "documents": 1, "credits": 0, "needs_extra_document": false,
    ///   "extra_document_cost": 0, "total_credits": 0, "breakdown": [],
    ///   "document_balance": 80, "credit_balance": 0, "has_sufficient_resources": true,
    ///   "blocking_reason": null, "message": null } }
    /// ```
    pub async fn estimate_cost<S: AsRef<str>>(
        &self,
        document_id: S,
        body: &EstimateAssignmentCostBody,
    ) -> Result<CostEstimate> {
        let path = format!(
            "documents/{}/assignments/estimate-cost",
            document_id.as_ref()
        );
        let payload = EstimateAssignmentPayload::from(body);
        let req = self.http.request(Method::POST, &path)?.json(&payload);
        self.http.send_envelope(req).await
    }

    /// Extend an assignment's expiration deadline.
    ///
    /// `PUT /documents/{document_id}/assignments/{assignmentId}/reset-expiration`.
    /// Pass `Some(timestamp)` for the production contract. `None` retains the
    /// live-verified legacy sandbox behavior of sending `expires_at: null`.
    ///
    /// # Request payload
    ///
    /// ```json
    /// { "expires_at": "2026-12-31T23:59:59Z" }
    /// ```
    ///
    /// # Response payload
    ///
    /// ```json
    /// { "status": 200, "message": "", "data": {
    ///   "id": "103033c9d2cec233bf65eea04999", "sender_email": "user@example.invalid",
    ///   "method": "virtual", "expires_at": "2026-12-31T23:59:59Z",
    ///   "message": "Please sign", "copy_receivers": [], "signers": [], "items": [],
    ///   "summary": { "signer_count": 1, "completed_count": 0, "signers": [] },
    ///   "signing_urls": [ { "signer_id": "19e6b92e7895332ed9708535d8c", "url": "https://…" } ] } }
    /// ```
    pub async fn reset_expiration<D: AsRef<str>, A: AsRef<str>>(
        &self,
        document_id: D,
        assignment_id: A,
        new_expires_at: Option<&str>,
    ) -> Result<Assignment> {
        let path = format!(
            "documents/{}/assignments/{}/reset-expiration",
            document_id.as_ref(),
            assignment_id.as_ref()
        );
        let req = self
            .http
            .request(Method::PUT, &path)?
            .json(&reset_expiration_payload(new_expires_at));
        self.http.send_envelope(req).await
    }

    /// Extend an assignment using the production API's non-null timestamp.
    ///
    /// `PUT /documents/{document_id}/assignments/{assignmentId}/reset-expiration`
    /// with `{ "expires_at": "..." }`. Returns the complete [`Assignment`]
    /// payload documented by [`Self::reset_expiration`].
    pub async fn reset_expiration_at<D: AsRef<str>, A: AsRef<str>>(
        &self,
        document_id: D,
        assignment_id: A,
        new_expires_at: &str,
    ) -> Result<Assignment> {
        self.reset_expiration(document_id, assignment_id, Some(new_expires_at))
            .await
    }

    /// Re-send the signature-request notification to a specific signer.
    ///
    /// `PUT /documents/{document_id}/assignments/{assignment_id}/signers/{signer_id}/resend`.
    ///
    /// # Response payload
    ///
    /// ```json
    /// { "status": 200, "message": "", "data": {
    ///   "is_sent": true, "document_id": "103b041f599e7dcdbbbf3cb9382d",
    ///   "signer_id": "103b041f4a97d41ca92debc3a4de" } }
    /// ```
    pub async fn resend_to_signer<D: AsRef<str>, A: AsRef<str>, S: AsRef<str>>(
        &self,
        document_id: D,
        assignment_id: A,
        signer_id: S,
    ) -> Result<ResendNotificationResult> {
        let path = format!(
            "documents/{}/assignments/{}/signers/{}/resend",
            document_id.as_ref(),
            assignment_id.as_ref(),
            signer_id.as_ref()
        );
        let req = self.http.request(Method::PUT, &path)?;
        self.http.send_envelope(req).await
    }

    /// Estimate the cost of re-sending a notification to one signer.
    ///
    /// `POST /documents/{document_id}/assignments/{assignment_id}/signers/{signer_id}/estimate-resend-cost`.
    ///
    /// # Response payload
    ///
    /// ```json
    /// { "status": 200, "message": "", "data": {
    ///   "documents": 1, "credits": 0, "needs_extra_document": false,
    ///   "extra_document_cost": 0, "total_credits": 0,
    ///   "breakdown": [], "document_balance": 80, "credit_balance": 0,
    ///   "has_sufficient_resources": true, "blocking_reason": null,
    ///   "message": null } }
    /// ```
    pub async fn estimate_resend_cost<D: AsRef<str>, A: AsRef<str>, S: AsRef<str>>(
        &self,
        document_id: D,
        assignment_id: A,
        signer_id: S,
    ) -> Result<CostEstimate> {
        let path = format!(
            "documents/{}/assignments/{}/signers/{}/estimate-resend-cost",
            document_id.as_ref(),
            assignment_id.as_ref(),
            signer_id.as_ref()
        );
        let req = self.http.request(Method::POST, &path)?;
        self.http.send_envelope(req).await
    }

    /// List WhatsApp notifications for an assignment.
    ///
    /// `GET /documents/{document_id}/assignments/{assignment_id}/whatsapp-notifications`.
    ///
    /// # Response payload
    ///
    /// ```json
    /// { "status": 200, "message": "", "data": [
    ///   { "sent_at": 1710000000,
    ///     "header": "Documento para assinatura: Contrato de Servico",
    ///     "body": "Olá, você tem um documento para assinar.",
    ///     "buttons": [ { "text": "Abrir documento",
    ///       "url": "https://app-sandbox.assinafy.com.br/sign/103033c950d865a248a11c5cf96c" } ],
    ///     "phone_number": "+5511999990001", "signer_id": "103033c9cd9426bbbb78eccd2c79" } ]
    /// }
    /// ```
    pub async fn whatsapp_notifications<D: AsRef<str>, A: AsRef<str>>(
        &self,
        document_id: D,
        assignment_id: A,
    ) -> Result<Vec<WhatsAppNotification>> {
        let path = format!(
            "documents/{}/assignments/{}/whatsapp-notifications",
            document_id.as_ref(),
            assignment_id.as_ref()
        );
        let req = self.http.request(Method::GET, &path)?;
        self.http.send_envelope(req).await
    }

    /// Sign a document assignment on behalf of a signer access-code flow.
    ///
    /// `POST /documents/{document_id}/assignments/{assignment_id}`.
    ///
    /// # Request payload
    ///
    /// ```json
    /// [
    ///   { "itemId": "103033c9d33326458deb74fc3052",
    ///     "fieldId": "102d25a48bc7357b93f9b8e01b24",
    ///     "pageId": "615213ed81b071f4293b2fc2", "value": "Signed by Ada Lovelace" }
    /// ]
    /// ```
    ///
    /// # Response payload
    ///
    /// The API defines the result as an object without fixed properties, so
    /// the SDK preserves it as [`serde_json::Value`].
    ///
    /// ```json
    /// { "status": 200, "message": "", "data": {} }
    /// ```
    pub async fn sign<D: AsRef<str>, A: AsRef<str>, I>(
        &self,
        document_id: D,
        assignment_id: A,
        items: I,
    ) -> Result<serde_json::Value>
    where
        I: IntoIterator<Item = SignDocumentItem>,
    {
        let path = format!(
            "documents/{}/assignments/{}",
            document_id.as_ref(),
            assignment_id.as_ref()
        );
        let items: Vec<SignDocumentItem> = items.into_iter().collect();
        let req = self.http.request(Method::POST, &path)?.json(&items);
        self.http.send_envelope(req).await
    }

    /// Reject a document assignment on behalf of a signer access-code flow.
    ///
    /// `PUT /documents/{document_id}/assignments/{assignment_id}/reject`.
    ///
    /// # Request payload
    ///
    /// ```json
    /// { "decline_reason": "I do not agree with clause 2." }
    /// ```
    ///
    /// # Response payload
    ///
    /// ```json
    /// { "status": 200, "message": "", "data": [] }
    /// ```
    pub async fn reject<D: AsRef<str>, A: AsRef<str>, R: AsRef<str>>(
        &self,
        document_id: D,
        assignment_id: A,
        reason: R,
    ) -> Result<()> {
        let path = format!(
            "documents/{}/assignments/{}/reject",
            document_id.as_ref(),
            assignment_id.as_ref()
        );
        let req = self
            .http
            .request(Method::PUT, &path)?
            .json(&serde_json::json!({ "decline_reason": reason.as_ref() }));
        self.http.send_no_content(req).await
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn reset_expiration_none_serializes_null() {
        assert_eq!(
            reset_expiration_payload(None),
            serde_json::json!({ "expires_at": null })
        );
    }

    #[test]
    fn reset_expiration_some_serializes_timestamp() {
        assert_eq!(
            reset_expiration_payload(Some("2026-06-01T12:00:00Z")),
            serde_json::json!({ "expires_at": "2026-06-01T12:00:00Z" })
        );
    }

    #[test]
    fn estimate_payload_strips_create_only_fields_and_signer_ids() {
        let body = CreateAssignmentBody::new(AssignmentMethod::Virtual, ["signer-id"])
            .message("not part of pricing")
            .expires_at("2026-12-31T23:59:59Z")
            .copy_receivers(["copy-id"])
            .signers([CreateAssignmentSigner::new("signer-id")
                .verification_method(VerificationMethod::Whatsapp)
                .notification_methods(vec![NotificationMethod::Whatsapp])]);

        assert_eq!(
            serde_json::to_value(EstimateAssignmentPayload::from(&body)).unwrap(),
            serde_json::json!({
                "method": "virtual",
                "signers": [{
                    "verification_method": "Whatsapp",
                    "notification_methods": ["Whatsapp"]
                }]
            })
        );
    }

    #[test]
    fn assignment_create_requires_signers_with_ids() {
        let empty = CreateAssignmentBody::from_signers(AssignmentMethod::Virtual, []);
        assert!(matches!(
            validate_create_assignment(&empty),
            Err(Error::Config(_))
        ));

        let missing_id = CreateAssignmentBody::from_signers(
            AssignmentMethod::Virtual,
            [CreateAssignmentSigner::default()],
        );
        assert!(matches!(
            validate_create_assignment(&missing_id),
            Err(Error::Config(_))
        ));

        let valid = CreateAssignmentBody::new(AssignmentMethod::Virtual, ["signer-id"]);
        assert!(validate_create_assignment(&valid).is_ok());

        let legacy = CreateAssignmentBody::from_signers(AssignmentMethod::Virtual, [])
            .legacy_signer_ids(["signer-id"]);
        assert!(validate_create_assignment(&legacy).is_ok());
    }
}
