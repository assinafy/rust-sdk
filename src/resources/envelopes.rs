//! Sending, tracking and maintaining document envelopes.

use std::collections::HashSet;

use reqwest::Method;
use serde::{Deserialize, Serialize};

use crate::error::{Error, Result};
use crate::http::HttpClient;
use crate::models::{
    ArtifactName, AssignmentMethod, CostEstimate, DocumentEnvelope, EnvelopeActivity,
    EnvelopeCancellation, EnvelopeReminderResult, NotificationMethod, VerificationMethod,
};
use crate::pagination::Page;

use super::AssignmentEntry;
use super::assignments::validate_credit_ceiling;

/// A signer shared by the documents of an envelope; envelopes have no signing steps.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EnvelopeSignerConfig {
    /// Existing signer ID.
    pub id: String,
    /// Identity verification method; omitted defaults to Email.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub verification_method: Option<VerificationMethod>,
    /// Exactly one compatible notification method, or omit for inference.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub notification_methods: Option<Vec<NotificationMethod>>,
    /// Sign inside an iframe on each envelope document, including later additions.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub embedded: Option<bool>,
}

impl EnvelopeSignerConfig {
    /// Use an existing signer with default verification and notification.
    pub fn new(id: impl Into<String>) -> Self {
        Self {
            id: id.into(),
            verification_method: None,
            notification_methods: None,
            embedded: None,
        }
    }

    /// Set the identity verification method.
    pub fn verification_method(mut self, method: VerificationMethod) -> Self {
        self.verification_method = Some(method);
        self
    }

    /// Set the single notification channel.
    pub fn notification_methods(mut self, methods: Vec<NotificationMethod>) -> Self {
        self.notification_methods = Some(methods);
        self
    }

    /// Enable or disable embedded signing (Email, Whatsapp or Bypass only).
    pub fn embedded(mut self, embedded: bool) -> Self {
        self.embedded = Some(embedded);
        self
    }
}

/// One processed, unassigned document to include in an envelope.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EnvelopeDocumentBody {
    /// Existing document ID, in `metadata_ready` state with no assignment.
    pub id: String,
    /// Signers of this document, each listed once.
    pub signer_ids: Vec<String>,
    /// Signing method; omitted defaults to virtual.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub method: Option<AssignmentMethod>,
    /// Page field placements, required for collect documents.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub entries: Option<Vec<AssignmentEntry>>,
    /// ISO 8601 date, Unix seconds or null for no expiration.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub expires_at: Option<serde_json::Value>,
    /// Existing signer IDs receiving only a completed copy.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub copy_receiver_ids: Option<Vec<String>>,
}

impl EnvelopeDocumentBody {
    /// Include a document for these signers.
    pub fn new<I, S>(id: impl Into<String>, signer_ids: I) -> Self
    where
        I: IntoIterator<Item = S>,
        S: Into<String>,
    {
        Self {
            id: id.into(),
            signer_ids: signer_ids.into_iter().map(Into::into).collect(),
            method: None,
            entries: None,
            expires_at: None,
            copy_receiver_ids: None,
        }
    }

    /// Set the signing method.
    pub fn method(mut self, method: AssignmentMethod) -> Self {
        self.method = Some(method);
        self
    }

    /// Set field placements for a collect document.
    pub fn entries(mut self, entries: Vec<AssignmentEntry>) -> Self {
        self.entries = Some(entries);
        self
    }

    /// Set an ISO 8601 string, Unix-second number, or null expiration.
    pub fn expires_at(mut self, expiration: impl Into<serde_json::Value>) -> Self {
        self.expires_at = Some(expiration.into());
        self
    }

    /// Set copy-only recipients.
    pub fn copy_receivers<I, S>(mut self, ids: I) -> Self
    where
        I: IntoIterator<Item = S>,
        S: Into<String>,
    {
        self.copy_receiver_ids = Some(ids.into_iter().map(Into::into).collect());
        self
    }

    fn validate(&self) -> Result<()> {
        if self.id.trim().is_empty()
            || self.signer_ids.is_empty()
            || self.signer_ids.iter().any(|id| id.trim().is_empty())
            || self.signer_ids.iter().collect::<HashSet<_>>().len() != self.signer_ids.len()
        {
            return Err(Error::Validation(
                "an envelope document requires an ID and distinct non-empty signer IDs".into(),
            ));
        }
        if self
            .expires_at
            .as_ref()
            .is_some_and(|v| !(v.is_string() || v.is_number() || v.is_null()))
        {
            return Err(Error::Validation(
                "expires_at must be text, a number or null".into(),
            ));
        }
        if self.method == Some(AssignmentMethod::Collect) {
            let entries = self.entries.as_deref().unwrap_or_default();
            if entries.is_empty()
                || entries.iter().any(|entry| {
                    entry.page_id.trim().is_empty()
                        || entry.fields.is_empty()
                        || entry.fields.iter().any(|field| {
                            field.field_id.trim().is_empty()
                                || !field
                                    .display_settings
                                    .as_ref()
                                    .is_some_and(serde_json::Value::is_object)
                        })
                })
            {
                return Err(Error::Validation(
                    "collect documents require pages with fields and display_settings objects"
                        .into(),
                ));
            }
            let placed: HashSet<_> = entries
                .iter()
                .flat_map(|entry| entry.fields.iter().map(|field| &field.signer_id))
                .collect();
            if placed != self.signer_ids.iter().collect() {
                return Err(Error::Validation(
                    "collect field signers must exactly match signer_ids".into(),
                ));
            }
        }
        Ok(())
    }
}

/// Body for creating an envelope and estimating its cost.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CreateEnvelopeBody {
    /// Invitation name, 1–255 characters and no `<`.
    pub name: String,
    /// All signers, once each; 1–20 entries, each used by a document.
    pub signers: Vec<EnvelopeSignerConfig>,
    /// Documents in presentation order; 1–20 entries.
    pub documents: Vec<EnvelopeDocumentBody>,
    /// Optional invitation message.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub message: Option<String>,
    /// Maximum credits creation may spend; ignored when estimating.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub max_total_credits: Option<f64>,
}

impl CreateEnvelopeBody {
    /// Create an envelope body using existing signers and processed documents.
    pub fn new(
        name: impl Into<String>,
        signers: Vec<EnvelopeSignerConfig>,
        documents: Vec<EnvelopeDocumentBody>,
    ) -> Self {
        Self {
            name: name.into(),
            signers,
            documents,
            message: None,
            max_total_credits: None,
        }
    }

    /// Set the invitation message.
    pub fn message(mut self, message: impl Into<String>) -> Self {
        self.message = Some(message.into());
        self
    }

    /// Refuse creation when its current total credit cost exceeds this ceiling.
    pub fn max_total_credits(mut self, credits: f64) -> Self {
        self.max_total_credits = Some(credits);
        self
    }

    fn validate(&self) -> Result<()> {
        validate_name(&self.name)?;
        if !(1..=20).contains(&self.signers.len()) || !(1..=20).contains(&self.documents.len()) {
            return Err(Error::Validation(
                "envelopes require 1–20 signers and 1–20 documents".into(),
            ));
        }
        let signer_ids: HashSet<_> = self.signers.iter().map(|s| &s.id).collect();
        if signer_ids.len() != self.signers.len()
            || signer_ids.iter().any(|id| id.trim().is_empty())
        {
            return Err(Error::Validation(
                "envelope signer IDs must be distinct and non-empty".into(),
            ));
        }
        if self
            .documents
            .iter()
            .map(|d| &d.id)
            .collect::<HashSet<_>>()
            .len()
            != self.documents.len()
        {
            return Err(Error::Validation(
                "envelope document IDs must be distinct".into(),
            ));
        }
        let used: HashSet<_> = self.documents.iter().flat_map(|d| &d.signer_ids).collect();
        if used != signer_ids {
            return Err(Error::Validation("every envelope signer must appear on a document and every document signer must be listed".into()));
        }
        for document in &self.documents {
            document.validate()?;
            let certificate = self.signers.iter().any(|s| {
                document.signer_ids.contains(&s.id)
                    && s.verification_method
                        .as_ref()
                        .is_some_and(|m| m.as_str() == "DigitalCertificate")
            });
            if certificate
                && (document.method == Some(AssignmentMethod::Collect)
                    || document.signer_ids.len() != 1)
            {
                return Err(Error::Validation(
                    "digital-certificate signers must sign virtual documents alone".into(),
                ));
            }
        }
        let fields: usize = self
            .documents
            .iter()
            .map(|d| {
                if d.method == Some(AssignmentMethod::Collect) {
                    d.entries.iter().flatten().map(|e| e.fields.len()).sum()
                } else {
                    d.signer_ids.len()
                }
            })
            .sum();
        let copies: usize = self
            .documents
            .iter()
            .map(|d| d.copy_receiver_ids.as_ref().map_or(0, Vec::len))
            .sum();
        if fields > 2000 || copies > 100 {
            return Err(Error::Validation(
                "an envelope accepts at most 2,000 fields and 100 copy receivers".into(),
            ));
        }
        for signer in &self.signers {
            if let Some(notifications) = &signer.notification_methods {
                if notifications.len() != 1 {
                    return Err(Error::Validation(
                        "each envelope signer requires exactly one notification method".into(),
                    ));
                }
                if let Some(verification) = &signer.verification_method {
                    let notification = notifications[0].as_str();
                    if matches!(verification.as_str(), "Email" | "Whatsapp")
                        && verification.as_str() != notification
                    {
                        return Err(Error::Validation("Email and Whatsapp verification require the matching notification method".into()));
                    }
                }
            }
            if signer.embedded == Some(true)
                && signer
                    .verification_method
                    .as_ref()
                    .is_some_and(|m| !matches!(m.as_str(), "Email" | "Whatsapp" | "Bypass"))
            {
                return Err(Error::Validation(
                    "embedded signers require Email, Whatsapp or Bypass verification".into(),
                ));
            }
        }
        Ok(())
    }
}

fn validate_name(name: &str) -> Result<()> {
    if name.is_empty() || name.chars().count() > 255 || name.contains('<') {
        return Err(Error::Validation(
            "envelope name must contain 1–255 characters and no '<'".into(),
        ));
    }
    Ok(())
}

/// Body for adding a document to an existing envelope.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AddEnvelopeDocumentBody {
    /// The document and existing envelope signers that should sign it.
    #[serde(flatten)]
    pub document: EnvelopeDocumentBody,
    /// Maximum credits this additional document may spend.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub max_total_credits: Option<f64>,
}

impl AddEnvelopeDocumentBody {
    /// Add a prepared document for existing envelope signers.
    pub fn new(document: EnvelopeDocumentBody) -> Self {
        Self {
            document,
            max_total_credits: None,
        }
    }

    /// Set the ceiling for this addition's digital-certificate charges.
    pub fn max_total_credits(mut self, credits: f64) -> Self {
        self.max_total_credits = Some(credits);
        self
    }
}

/// Partial envelope changes; omitted fields remain unchanged.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct UpdateEnvelopeBody {
    /// Replacement invitation name.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub name: Option<String>,
    /// Omitted: unchanged; null: clear; string: replace the message.
    #[serde(
        default,
        skip_serializing_if = "Option::is_none",
        deserialize_with = "deserialize_message"
    )]
    pub message: Option<Option<String>>,
    /// Every current document ID, exactly once, in its new order.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub document_ids: Option<Vec<String>>,
}

fn deserialize_message<'de, D: serde::Deserializer<'de>>(
    deserializer: D,
) -> std::result::Result<Option<Option<String>>, D::Error> {
    Option::<String>::deserialize(deserializer).map(Some)
}

impl UpdateEnvelopeBody {
    /// Rename the envelope.
    pub fn name(mut self, name: impl Into<String>) -> Self {
        self.name = Some(name.into());
        self
    }

    /// Replace the invitation message.
    pub fn message(mut self, message: impl Into<String>) -> Self {
        self.message = Some(Some(message.into()));
        self
    }

    /// Clear the invitation message by sending JSON null.
    pub fn clear_message(mut self) -> Self {
        self.message = Some(None);
        self
    }

    /// Reorder every current document.
    pub fn document_ids<I, S>(mut self, ids: I) -> Self
    where
        I: IntoIterator<Item = S>,
        S: Into<String>,
    {
        self.document_ids = Some(ids.into_iter().map(Into::into).collect());
        self
    }
}

/// Envelope operations within one workspace.
#[derive(Debug)]
pub struct EnvelopesApi<'a> {
    http: &'a HttpClient,
    account_id: String,
}

impl<'a> EnvelopesApi<'a> {
    pub(crate) fn new(http: &'a HttpClient, account_id: String) -> Self {
        Self { http, account_id }
    }

    fn path(&self, suffix: &[&str]) -> Result<String> {
        let mut segments = vec!["accounts", self.account_id.as_str(), "envelopes"];
        segments.extend_from_slice(suffix);
        self.http.path(&segments)
    }

    /// List envelopes: `GET /accounts/{accountId}/envelopes`.
    ///
    /// # Request payload
    /// See [`ListEnvelopesRequest::send`] for query fields.
    ///
    /// # Response payload
    /// See [`ListEnvelopesRequest::send`] for the response example.
    pub fn list(&self) -> ListEnvelopesRequest<'_> {
        ListEnvelopesRequest {
            http: self.http,
            account_id: self.account_id.clone(),
            search: None,
            status: None,
            page: None,
            per_page: None,
        }
    }

    /// Create an envelope: `POST /accounts/{accountId}/envelopes`.
    ///
    /// Configure an idempotency key before sending to safely recover a lost response.
    ///
    /// # Request payload
    /// See [`CreateEnvelopeRequest::send`] for the JSON body and header.
    ///
    /// # Response payload
    /// See [`CreateEnvelopeRequest::send`] for the response example.
    pub fn create(&self, body: CreateEnvelopeBody) -> CreateEnvelopeRequest<'_> {
        CreateEnvelopeRequest {
            http: self.http,
            account_id: self.account_id.clone(),
            body,
            idempotency_key: None,
        }
    }

    /// Estimate creation cost: `POST /accounts/{accountId}/envelopes/estimate-cost`.
    ///
    /// Charges nothing and ignores the ceiling. The envelope consumes one
    /// document; notifications and biometric identity checks count once per signer,
    /// while digital-certificate signatures count on each document.
    ///
    /// # Request payload
    /// ```json
    /// {"name":"Contract","signers":[{"id":"signer_1"}],"documents":[{"id":"document_1","signer_ids":["signer_1"]}]}
    /// ```
    /// # Response payload
    /// ```json
    /// {"status":200,"message":"","data":{"documents":1,"credits":0,"needs_extra_document":false,"extra_document_cost":1,"total_credits":0,"breakdown":[],"document_balance":10,"credit_balance":10,"has_sufficient_resources":true,"blocking_reason":null,"message":null}}
    /// ```
    pub async fn estimate_cost(&self, body: &CreateEnvelopeBody) -> Result<CostEstimate> {
        body.validate()?;
        // The estimate ignores the spending ceiling; omit it even if non-finite.
        let mut body = body.clone();
        body.max_total_credits = None;
        let req = self
            .http
            .request_workspace(Method::POST, &self.path(&["estimate-cost"])?)?
            .json(&body);
        self.http.send_envelope(req).await
    }

    /// Read an envelope: `GET /accounts/{accountId}/envelopes/{envelopeId}`.
    ///
    /// # Request payload
    /// No body or query parameters.
    ///
    /// # Response payload
    /// ```json
    /// {"status":200,"message":"","data":{"resource":"envelope","id":"envelope_1","name":"Contract","message":null,"documents":[],"counts":null,"signing_urls":[],"created_at":"2026-11-04T12:00:00Z"}}
    /// ```
    pub async fn get(&self, envelope_id: &str) -> Result<DocumentEnvelope> {
        let req = self
            .http
            .request_workspace(Method::GET, &self.path(&[envelope_id])?)?;
        self.http.send_envelope(req).await
    }

    /// Update name, message or order: `PATCH /accounts/{accountId}/envelopes/{envelopeId}`.
    ///
    /// Sends no notifications. Future notices use the new values.
    ///
    /// # Request payload
    /// ```json
    /// {"name":"Updated contract","message":null,"document_ids":["document_2","document_1"]}
    /// ```
    /// # Response payload
    /// ```json
    /// {"status":200,"message":"","data":{"id":"envelope_1","name":"Updated contract","message":null,"documents":[],"counts":null,"signing_urls":[],"created_at":"2026-11-04T12:00:00Z"}}
    /// ```
    pub async fn update(
        &self,
        envelope_id: &str,
        body: &UpdateEnvelopeBody,
    ) -> Result<DocumentEnvelope> {
        if let Some(name) = &body.name {
            validate_name(name)?;
        }
        if let Some(ids) = &body.document_ids {
            if ids.is_empty()
                || ids.iter().any(|id| id.trim().is_empty())
                || ids.iter().collect::<HashSet<_>>().len() != ids.len()
            {
                return Err(Error::Validation(
                    "document_ids must contain distinct non-empty IDs".into(),
                ));
            }
        }
        let req = self
            .http
            .request_workspace(Method::PATCH, &self.path(&[envelope_id])?)?
            .json(body);
        self.http.send_envelope(req).await
    }

    /// Delete a closed envelope: `DELETE /accounts/{accountId}/envelopes/{envelopeId}`.
    ///
    /// Its documents and certificates remain. Open envelopes return `409`;
    /// cancel them first. Deletion also removes the creation key's association.
    ///
    /// # Request payload
    /// No body or query parameters.
    ///
    /// # Response payload
    /// ```json
    /// {"status":200,"message":"","data":[]}
    /// ```
    pub async fn delete(&self, envelope_id: &str) -> Result<()> {
        let req = self
            .http
            .request_workspace(Method::DELETE, &self.path(&[envelope_id])?)?;
        self.http.send_no_content(req).await
    }

    /// List the combined trail: `GET /accounts/{accountId}/envelopes/{envelopeId}/activities`.
    ///
    /// # Request payload
    /// See [`ListEnvelopeActivitiesRequest::send`] for query fields.
    ///
    /// # Response payload
    /// See [`ListEnvelopeActivitiesRequest::send`] for the response example.
    pub fn activities(&self, envelope_id: impl Into<String>) -> ListEnvelopeActivitiesRequest<'_> {
        ListEnvelopeActivitiesRequest {
            http: self.http,
            account_id: self.account_id.clone(),
            envelope_id: envelope_id.into(),
            sort: None,
            page: None,
            per_page: None,
        }
    }

    /// Download a ZIP: `GET /accounts/{accountId}/envelopes/{envelopeId}/download/{artifactName}`.
    ///
    /// Includes only documents that have the chosen artifact. Thumbnail is
    /// not available on this route. Returns bytes and response content type.
    ///
    /// # Request payload
    /// No body. Artifact is `original`, `certificated`, `certificate-page`, `pades` or `bundle`.
    ///
    /// # Response payload
    /// ```text
    /// Content-Type: application/zip
    /// <binary ZIP bytes>
    /// ```
    pub async fn download_artifact(
        &self,
        envelope_id: &str,
        artifact: ArtifactName,
    ) -> Result<(bytes::Bytes, String)> {
        if artifact == ArtifactName::Thumbnail {
            return Err(Error::Validation(
                "envelope downloads do not support thumbnails".into(),
            ));
        }
        let req = self.http.request_workspace(
            Method::GET,
            &self.path(&[envelope_id, "download", artifact.as_str()])?,
        )?;
        self.http.send_download(req).await
    }

    /// Cancel eligible documents: `PUT /accounts/{accountId}/envelopes/{envelopeId}/cancel`.
    ///
    /// Notifies the owner, signers and copy receivers. Closed documents and
    /// documents whose certification is queued or running remain unchanged.
    /// Safe to repeat; the response reports what this call changed.
    ///
    /// # Request payload
    /// No body or query parameters.
    ///
    /// # Response payload
    /// ```json
    /// {"status":200,"message":"","data":[{"id":"document_1","status":"rejected_by_user","cancelled":true}]}
    /// ```
    pub async fn cancel(&self, envelope_id: &str) -> Result<Vec<EnvelopeCancellation>> {
        let req = self
            .http
            .request_workspace(Method::PUT, &self.path(&[envelope_id, "cancel"])?)?;
        self.http.send_envelope(req).await
    }

    /// Remind a signer: `PUT /accounts/{accountId}/envelopes/{envelopeId}/signers/{signerId}/resend`.
    ///
    /// Sends one envelope reminder; paid notification channels are charged.
    ///
    /// # Request payload
    /// No body or query parameters.
    ///
    /// # Response payload
    /// ```json
    /// {"status":200,"message":"","data":{"is_sent":true,"envelope_id":"envelope_1","signer_id":"signer_1"}}
    /// ```
    pub async fn resend(
        &self,
        envelope_id: &str,
        signer_id: &str,
    ) -> Result<EnvelopeReminderResult> {
        let req = self.http.request_workspace(
            Method::PUT,
            &self.path(&[envelope_id, "signers", signer_id, "resend"])?,
        )?;
        self.http.send_envelope(req).await
    }

    /// Add a document: `POST /accounts/{accountId}/envelopes/{envelopeId}/documents`.
    ///
    /// Only existing envelope signers may join it. Its digital-certificate
    /// signatures are charged; selected signers receive an updated invitation.
    ///
    /// # Request payload
    /// ```json
    /// {"id":"document_2","signer_ids":["signer_1"],"max_total_credits":0}
    /// ```
    /// # Response payload
    /// ```json
    /// {"status":200,"message":"","data":{"id":"envelope_1","name":"Contract","message":null,"documents":[],"counts":null,"signing_urls":[],"created_at":"2026-11-04T12:00:00Z"}}
    /// ```
    pub async fn add_document(
        &self,
        envelope_id: &str,
        body: &AddEnvelopeDocumentBody,
    ) -> Result<DocumentEnvelope> {
        body.document.validate()?;
        validate_credit_ceiling(body.max_total_credits)?;
        let req = self
            .http
            .request_workspace(Method::POST, &self.path(&[envelope_id, "documents"])?)?
            .json(body);
        self.http.send_envelope(req).await
    }

    /// Cancel and detach an open document: `DELETE /accounts/{accountId}/envelopes/{envelopeId}/documents/{documentId}`.
    ///
    /// Notifies its owner, signers and copy receivers. Closed documents return `409`.
    ///
    /// # Request payload
    /// No body or query parameters.
    ///
    /// # Response payload
    /// ```json
    /// {"status":200,"message":"","data":{"id":"envelope_1","name":"Contract","message":null,"documents":[],"counts":null,"signing_urls":[],"created_at":"2026-11-04T12:00:00Z"}}
    /// ```
    pub async fn remove_document(
        &self,
        envelope_id: &str,
        document_id: &str,
    ) -> Result<DocumentEnvelope> {
        let req = self.http.request_workspace(
            Method::DELETE,
            &self.path(&[envelope_id, "documents", document_id])?,
        )?;
        self.http.send_envelope(req).await
    }
}

/// Creation request with an optional workspace-scoped idempotency key.
#[derive(Debug)]
pub struct CreateEnvelopeRequest<'a> {
    http: &'a HttpClient,
    account_id: String,
    body: CreateEnvelopeBody,
    idempotency_key: Option<String>,
}

impl CreateEnvelopeRequest<'_> {
    /// Set a case-sensitive key, unique for each intended envelope.
    ///
    /// Reusing it returns the original envelope without creating or charging
    /// again, regardless of the new body. Keys do not expire while associated.
    pub fn idempotency_key(mut self, key: impl Into<String>) -> Self {
        self.idempotency_key = Some(key.into());
        self
    }

    /// Execute `POST /accounts/{accountId}/envelopes`.
    ///
    /// A successful response confirms creation, not delivery of every invitation.
    /// Inspect signer notification histories for delivery failures.
    ///
    /// # Request payload
    /// Optional `Idempotency-Key` header, plus:
    /// ```json
    /// {"name":"Contract","signers":[{"id":"signer_1","verification_method":"Email","notification_methods":["Email"]}],"documents":[{"id":"document_1","signer_ids":["signer_1"]}],"max_total_credits":0}
    /// ```
    /// # Response payload
    /// ```json
    /// {"status":200,"message":"","data":{"id":"envelope_1","name":"Contract","message":null,"documents":[],"counts":null,"signing_urls":[],"created_at":"2026-11-04T12:00:00Z"}}
    /// ```
    pub async fn send(self) -> Result<DocumentEnvelope> {
        self.body.validate()?;
        validate_credit_ceiling(self.body.max_total_credits)?;
        let path = self
            .http
            .path(&["accounts", &self.account_id, "envelopes"])?;
        let mut req = self
            .http
            .request_workspace(Method::POST, &path)?
            .json(&self.body);
        if let Some(key) = self.idempotency_key {
            req = req.header("Idempotency-Key", key);
        }
        self.http.send_envelope(req).await
    }
}

/// Envelope list filters and pagination.
#[derive(Debug)]
pub struct ListEnvelopesRequest<'a> {
    http: &'a HttpClient,
    account_id: String,
    search: Option<String>,
    status: Option<String>,
    page: Option<u32>,
    per_page: Option<u32>,
}

impl ListEnvelopesRequest<'_> {
    /// Match part of the envelope name.
    pub fn search(mut self, search: impl Into<String>) -> Self {
        self.search = Some(search.into());
        self
    }
    /// Filter by `open` or `closed`.
    pub fn status(mut self, status: impl Into<String>) -> Self {
        self.status = Some(status.into());
        self
    }
    /// Set the one-based page.
    pub fn page(mut self, page: u32) -> Self {
        self.page = Some(page);
        self
    }
    /// Set results per page.
    pub fn per_page(mut self, count: u32) -> Self {
        self.per_page = Some(count);
        self
    }

    /// Execute `GET /accounts/{accountId}/envelopes`, most recent first.
    ///
    /// # Request payload
    /// No body. Optional query fields:
    /// ```json
    /// {"search":"Contract","status":"open","page":1,"per-page":20}
    /// ```
    /// # Response payload
    /// ```json
    /// {"status":200,"message":"","data":[{"id":"envelope_1","name":"Contract","message":null,"documents":[],"counts":null,"signing_urls":[],"created_at":"2026-11-04T12:00:00Z"}]}
    /// ```
    pub async fn send(self) -> Result<Page<DocumentEnvelope>> {
        if self
            .status
            .as_deref()
            .is_some_and(|status| !matches!(status, "open" | "closed"))
        {
            return Err(Error::Validation(
                "envelope status must be open or closed".into(),
            ));
        }
        let path = self
            .http
            .path(&["accounts", &self.account_id, "envelopes"])?;
        let mut query = Vec::new();
        if let Some(v) = self.search {
            query.push(("search", v));
        }
        if let Some(v) = self.status {
            query.push(("status", v));
        }
        if let Some(v) = self.page {
            query.push(("page", v.to_string()));
        }
        if let Some(v) = self.per_page {
            query.push(("per-page", v.to_string()));
        }
        let req = self
            .http
            .request_workspace(Method::GET, &path)?
            .query(&query);
        self.http.send_paged(req).await
    }
}

/// Envelope activity ordering and pagination.
#[derive(Debug)]
pub struct ListEnvelopeActivitiesRequest<'a> {
    http: &'a HttpClient,
    account_id: String,
    envelope_id: String,
    sort: Option<String>,
    page: Option<u32>,
    per_page: Option<u32>,
}

impl ListEnvelopeActivitiesRequest<'_> {
    /// Use `-id` (newest first) or `id` (oldest first).
    pub fn sort(mut self, sort: impl Into<String>) -> Self {
        self.sort = Some(sort.into());
        self
    }
    /// Set the one-based page.
    pub fn page(mut self, page: u32) -> Self {
        self.page = Some(page);
        self
    }
    /// Set results per page (server default: twenty).
    pub fn per_page(mut self, count: u32) -> Self {
        self.per_page = Some(count);
        self
    }

    /// Execute `GET /accounts/{accountId}/envelopes/{envelopeId}/activities`.
    ///
    /// Includes activities of detached documents while those documents exist.
    ///
    /// # Request payload
    /// No body. Optional query fields:
    /// ```json
    /// {"sort":"-id","page":1,"per-page":20}
    /// ```
    /// # Response payload
    /// ```json
    /// {"status":200,"message":"","data":[{"id":1,"event":"document_uploaded","message":"Document uploaded","payload":[],"origin":null,"created_at":"2026-11-04T12:00:00Z","document_id":"document_1","document_name":"Contract.pdf"}]}
    /// ```
    pub async fn send(self) -> Result<Page<EnvelopeActivity>> {
        let path = self.http.path(&[
            "accounts",
            &self.account_id,
            "envelopes",
            &self.envelope_id,
            "activities",
        ])?;
        let mut query = Vec::new();
        if let Some(v) = self.sort {
            query.push(("sort", v));
        }
        if let Some(v) = self.page {
            query.push(("page", v.to_string()));
        }
        if let Some(v) = self.per_page {
            query.push(("per-page", v.to_string()));
        }
        let req = self
            .http
            .request_workspace(Method::GET, &path)?
            .query(&query);
        self.http.send_paged(req).await
    }
}
