//! Assignment (signature-request) endpoints.

use reqwest::Method;
use serde::{Deserialize, Serialize};

use crate::error::Result;
use crate::http::HttpClient;
use crate::models::{
    Assignment, AssignmentMethod, CostEstimate, NotificationMethod, ResendCostEstimate,
    ResendNotificationResult, SignDocumentItem, VerificationMethod, WhatsAppNotification,
};

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
    /// Default verification method when no per-signer override is set.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub verification_method: Option<VerificationMethod>,
    /// Default notification methods when no per-signer override is set.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub notification_methods: Option<Vec<NotificationMethod>>,
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
            verification_method: None,
            notification_methods: None,
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
    pub fn with_signers(mut self, signers: Vec<CreateAssignmentSigner>) -> Self {
        self.signers = Some(signers);
        self.signer_ids.clear();
        self
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

    /// Default verification method.
    pub fn verification_method(mut self, method: VerificationMethod) -> Self {
        self.verification_method = Some(method);
        self
    }

    /// Default notification methods.
    pub fn notification_methods(mut self, methods: Vec<NotificationMethod>) -> Self {
        self.notification_methods = Some(methods);
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
/// Accepts the same fields as a full create request; the server inspects them
/// without actually dispatching anything.
pub type EstimateAssignmentCostBody = CreateAssignmentBody;

/// Assignment endpoints.
#[derive(Debug)]
pub struct AssignmentsApi<'a> {
    http: &'a HttpClient,
}

impl<'a> AssignmentsApi<'a> {
    pub(crate) fn new(http: &'a HttpClient) -> Self {
        Self { http }
    }

    /// Request signatures from one or more signers.
    ///
    /// `POST /documents/{document_id}/assignments`.
    pub async fn create<S: AsRef<str>>(
        &self,
        document_id: S,
        body: &CreateAssignmentBody,
    ) -> Result<Assignment> {
        let path = format!("documents/{}/assignments", document_id.as_ref());
        let req = self.http.request(Method::POST, &path)?.json(body);
        self.http.send_data(req).await
    }

    /// Estimate the cost of an assignment without creating it.
    ///
    /// `POST /documents/{document_id}/assignments/estimate-cost`.
    pub async fn estimate_cost<S: AsRef<str>>(
        &self,
        document_id: S,
        body: &EstimateAssignmentCostBody,
    ) -> Result<CostEstimate> {
        let path = format!(
            "documents/{}/assignments/estimate-cost",
            document_id.as_ref()
        );
        let req = self.http.request(Method::POST, &path)?.json(body);
        self.http.send_envelope(req).await
    }

    /// Extend an assignment's expiration deadline.
    ///
    /// `PUT /documents/{document_id}/assignments/{assignmentId}/reset-expiration`.
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

    /// Re-send notifications to the assignment's outstanding signers.
    ///
    /// `PUT /documents/{document_id}/assignments/{assignmentId}/resend`.
    pub async fn resend<D: AsRef<str>, A: AsRef<str>>(
        &self,
        document_id: D,
        assignment_id: A,
    ) -> Result<Assignment> {
        let path = format!(
            "documents/{}/assignments/{}/resend",
            document_id.as_ref(),
            assignment_id.as_ref()
        );
        let req = self.http.request(Method::PUT, &path)?;
        self.http.send_envelope(req).await
    }

    /// Re-send notifications to a specific signer.
    ///
    /// `PUT /documents/{document_id}/assignments/{assignment_id}/signers/{signer_id}/resend`.
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
    pub async fn estimate_resend_cost<D: AsRef<str>, A: AsRef<str>, S: AsRef<str>>(
        &self,
        document_id: D,
        assignment_id: A,
        signer_id: S,
    ) -> Result<ResendCostEstimate> {
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
    pub async fn sign<D: AsRef<str>, A: AsRef<str>, I>(
        &self,
        document_id: D,
        assignment_id: A,
        items: I,
    ) -> Result<()>
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
        self.http.send_no_content(req).await
    }

    /// Reject a document assignment on behalf of a signer access-code flow.
    ///
    /// `PUT /documents/{document_id}/assignments/{assignment_id}/reject`.
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
    use super::reset_expiration_payload;

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
}
