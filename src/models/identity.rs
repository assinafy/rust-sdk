//! Identity document and face verification, hosted capture sessions and account review.

use std::fmt;

use serde::{Deserialize, Serialize};

use super::VerificationMethod;

/// State of a signer's identity document verification.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[non_exhaustive]
pub enum IdentityVerificationStatus {
    /// The signer has an open capture session.
    Pending,
    /// The account must review the evidence before certification.
    InReview,
    /// The identity was approved by the provider or account reviewer.
    Approved,
    /// The identity was declined; a new document and assignment are required.
    Declined,
    /// The capture link expired before submission; a new session may be opened.
    Expired,
    /// Verification was abandoned; a new document and assignment are required.
    Abandoned,
    /// A server value not yet modeled by the SDK.
    #[serde(untagged)]
    Other(String),
}

/// Current state returned by `GET /signers/identity-verification`.
#[derive(Clone, Serialize, Deserialize)]
#[non_exhaustive]
pub struct IdentityVerification {
    /// Whether a session can be started: no session exists, or the last one expired.
    #[serde(default)]
    pub can_start: bool,
    /// Latest provider state, or `None` before a session exists.
    #[serde(default)]
    pub status: Option<IdentityVerificationStatus>,
    /// Hosted capture bearer URL while pending. Open it unchanged, including its
    /// fragment; never log it or persist it in browser storage.
    #[serde(default)]
    pub url: Option<String>,
    /// Capture-link expiration (ISO-8601), when supplied by the provider.
    #[serde(default)]
    pub expires_at: Option<String>,
    /// Verification completion time (ISO-8601), when available.
    #[serde(default)]
    pub completed_at: Option<String>,
}

impl fmt::Debug for IdentityVerification {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("IdentityVerification")
            .field("can_start", &self.can_start)
            .field("status", &self.status)
            .field("url", &self.url.as_ref().map(|_| "**redacted**"))
            .field("expires_at", &self.expires_at)
            .field("completed_at", &self.completed_at)
            .finish()
    }
}

/// Hosted capture session returned by `POST /signers/identity-verification`.
#[derive(Clone, Serialize, Deserialize)]
#[non_exhaustive]
pub struct IdentityVerificationSession {
    /// Session identifier.
    pub id: String,
    /// Hosted capture bearer URL. Open it unchanged, including its fragment;
    /// never log it or persist it in browser storage.
    pub url: String,
    /// Capture-link expiration (ISO-8601), when supplied by the provider.
    #[serde(default)]
    pub expires_at: Option<String>,
}

impl fmt::Debug for IdentityVerificationSession {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("IdentityVerificationSession")
            .field("id", &self.id)
            .field("url", &"**redacted**")
            .field("expires_at", &self.expires_at)
            .finish()
    }
}

/// Account review of a signer's identity document verification.
///
/// While `InReview`, the signer can sign but document certification waits for
/// approval. Evidence images are available only until the review is decided.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[non_exhaustive]
pub struct SignerIdentityReview {
    /// Verification identifier.
    pub id: String,
    /// Latest verification state.
    pub status: IdentityVerificationStatus,
    /// Provider name, such as `AssinafyVerify`.
    #[serde(default)]
    pub provider: Option<String>,
    /// Verification method used by this signer.
    #[serde(default)]
    pub verification_method: Option<VerificationMethod>,
    /// Reasons for review in the provider's order, including unknown codes.
    #[serde(default)]
    pub reason_codes: Vec<IdentityReviewReason>,
    /// Whether a summary of provider evidence is available.
    #[serde(default)]
    pub evidence_available: bool,
    /// Identity registered for the signer.
    #[serde(default)]
    pub expected: Option<IdentityReviewExpected>,
    /// Identity read from the submitted document.
    #[serde(default)]
    pub document: Option<IdentityReviewDocument>,
    /// Identity match results.
    #[serde(default)]
    pub matches: Option<IdentityReviewMatches>,
    /// Individual provider checks.
    #[serde(default)]
    pub checks: Vec<IdentityReviewCheck>,
    /// Evidence image names in display order: `document_front`, `document_back`,
    /// `document_pdf`, `face`. Empty after the review is decided.
    #[serde(default)]
    pub images: Vec<String>,
    /// Ruleset used to evaluate the identity.
    #[serde(default)]
    pub ruleset_version: Option<String>,
    /// SHA-256 digest of the evidence summary.
    #[serde(default)]
    pub evidence_sha256: Option<String>,
    /// Account user who made the review decision.
    #[serde(default)]
    pub reviewed_by: Option<IdentityReviewer>,
    /// Review decision time (ISO-8601).
    #[serde(default)]
    pub reviewed_at: Option<String>,
    /// Private reviewer note, never shown to the signer or copy receivers.
    #[serde(default)]
    pub review_note: Option<String>,
    /// Verification completion time (ISO-8601).
    #[serde(default)]
    pub completed_at: Option<String>,
}

/// Reason a provider sent an identity verification for account review.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[non_exhaustive]
pub struct IdentityReviewReason {
    /// Stable reason code, such as `NAME_MISMATCH`.
    pub code: String,
    /// Human-readable reason, or the code itself when unknown.
    pub description: String,
}

/// Identity the signer was registered with.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[non_exhaustive]
pub struct IdentityReviewExpected {
    /// Registered name.
    #[serde(default)]
    pub name: Option<String>,
    /// Registered CPF.
    #[serde(default)]
    pub cpf: Option<String>,
}

/// Identity read from the submitted identity document.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[non_exhaustive]
pub struct IdentityReviewDocument {
    /// Document type, such as `CNH`.
    #[serde(default, rename = "type")]
    pub kind: Option<String>,
    /// Name read from the document.
    #[serde(default)]
    pub name: Option<String>,
    /// CPF read from the document.
    #[serde(default)]
    pub cpf: Option<String>,
}

/// Results of matching registered and submitted identity data.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[non_exhaustive]
pub struct IdentityReviewMatches {
    /// Whether the CPFs match, when checked.
    #[serde(default)]
    pub cpf: Option<bool>,
    /// Name similarity from zero to one, when checked.
    #[serde(default)]
    pub name: Option<f64>,
    /// Whether the birth dates match, when checked.
    #[serde(default)]
    pub birth_date: Option<bool>,
}

/// Individual verification check reported by the provider.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[non_exhaustive]
pub struct IdentityReviewCheck {
    /// Check name.
    pub name: String,
    /// Result, such as `pass`, `review`, `skipped`, or a failure result.
    pub result: String,
    /// Provider score, when available.
    #[serde(default)]
    pub score: Option<f64>,
    /// Provider reason codes.
    #[serde(default)]
    pub reasons: Vec<String>,
}

/// Account user who decided an identity review.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[non_exhaustive]
pub struct IdentityReviewer {
    /// User identifier.
    pub id: String,
    /// User display name.
    pub name: String,
}
