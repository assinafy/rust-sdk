//! Document models.

use std::collections::BTreeMap;
use std::fmt;

use serde::{Deserialize, Serialize};

use super::assignment::{Assignment, AssignmentSigner};
use super::tag::Tag;

/// A document stored in an account.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[non_exhaustive]
pub struct Document {
    /// Resource discriminator (always `"document"` when present).
    #[serde(default)]
    pub resource: Option<String>,
    /// Document identifier.
    pub id: String,
    /// Owning account identifier.
    #[serde(default)]
    pub account_id: Option<String>,
    /// If created from a template, the template identifier.
    #[serde(default)]
    pub template_id: Option<String>,
    /// Display name.
    pub name: String,
    /// Current status. Unknown server-side values are preserved via
    /// [`DocumentStatus::Unknown`].
    pub status: DocumentStatus,
    /// Downloadable artifacts keyed by name (`"original"`, `"certificated"`,
    /// `"certificate-page"`, `"bundle"`, `"thumbnail"`, etc.).
    #[serde(default)]
    pub artifacts: BTreeMap<String, String>,
    /// Pages within the document.
    #[serde(default)]
    pub pages: Vec<DocumentPage>,
    /// Expanded assignment, when one exists.
    #[serde(default)]
    pub assignment: Option<Assignment>,
    /// Signer represented by the current signer access code, when returned by
    /// signer-facing document endpoints.
    #[serde(default)]
    pub current_signer: Option<AssignmentSigner>,
    /// Whether the document is closed (signed, declined, expired, etc.).
    #[serde(default)]
    pub is_closed: bool,
    /// Signing URL for the document (when applicable).
    #[serde(default)]
    pub signing_url: Option<String>,
    /// Decline reason supplied by the signer or user, if any.
    #[serde(default)]
    pub decline_reason: Option<String>,
    /// Identity of the signer or user that declined the document.
    #[serde(default)]
    pub declined_by: Option<DeclinedBy>,
    /// Tags attached to the document.
    #[serde(default)]
    pub tags: Vec<Tag>,
    /// Creation timestamp (ISO-8601 string or Unix epoch number).
    #[serde(default)]
    pub created_at: Option<serde_json::Value>,
    /// Last-modification timestamp.
    #[serde(default)]
    pub updated_at: Option<serde_json::Value>,
}

/// A single page within a document.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[non_exhaustive]
pub struct DocumentPage {
    /// Page identifier.
    pub id: String,
    /// 1-based page number.
    pub number: u32,
    /// Height in pixels.
    pub height: u32,
    /// Width in pixels.
    pub width: u32,
    /// URL to download the page as JPEG.
    #[serde(default)]
    pub download_url: Option<String>,
}

/// Identifies the actor that declined a document.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[non_exhaustive]
pub struct DeclinedBy {
    /// Resource discriminator (always `"signer"` when present).
    #[serde(default)]
    pub resource: Option<String>,
    /// Identifier of the declining signer/user.
    pub id: String,
    /// Full name, when available.
    #[serde(default)]
    pub full_name: Option<String>,
    /// Email, when available.
    #[serde(default)]
    pub email: Option<String>,
    /// WhatsApp phone number in E.164 format, when available.
    #[serde(default)]
    pub whatsapp_phone_number: Option<String>,
    /// Whether the signer has accepted the platform terms.
    #[serde(default)]
    pub has_accepted_terms: bool,
}

/// Well-known artifact names. Custom names are accepted via
/// [`ArtifactName::Other`].
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ArtifactName {
    /// Original uploaded file.
    Original,
    /// Final certificated PDF (signed + audit trail).
    Certificated,
    /// Standalone certificate page.
    CertificatePage,
    /// PAdES-compliant signed PDF.
    Pades,
    /// Bundle containing the certificated PDF and audit trail.
    Bundle,
    /// Thumbnail image.
    Thumbnail,
    /// Custom artifact name.
    Other(String),
}

impl ArtifactName {
    /// Returns the wire-format string used in URLs.
    pub fn as_str(&self) -> &str {
        match self {
            ArtifactName::Original => "original",
            ArtifactName::Certificated => "certificated",
            ArtifactName::CertificatePage => "certificate-page",
            ArtifactName::Pades => "pades",
            ArtifactName::Bundle => "bundle",
            ArtifactName::Thumbnail => "thumbnail",
            ArtifactName::Other(s) => s.as_str(),
        }
    }
}

impl fmt::Display for ArtifactName {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.as_str())
    }
}

impl From<&str> for ArtifactName {
    fn from(s: &str) -> Self {
        match s {
            "original" => ArtifactName::Original,
            "certificated" => ArtifactName::Certificated,
            "certificate-page" => ArtifactName::CertificatePage,
            "pades" => ArtifactName::Pades,
            "bundle" => ArtifactName::Bundle,
            "thumbnail" => ArtifactName::Thumbnail,
            other => ArtifactName::Other(other.to_owned()),
        }
    }
}

impl From<String> for ArtifactName {
    fn from(s: String) -> Self {
        ArtifactName::from(s.as_str())
    }
}

/// Document lifecycle status.
///
/// Unknown variants the API may add are surfaced through
/// [`DocumentStatus::Unknown`] rather than failing deserialization.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum DocumentStatus {
    /// Upload still in progress.
    Uploading,
    /// Upload complete; processing has not started.
    Uploaded,
    /// Metadata extraction underway.
    MetadataProcessing,
    /// Metadata extracted; document ready to be assigned.
    MetadataReady,
    /// Signature deadline reached.
    Expired,
    /// Final certificate being generated.
    Certificating,
    /// Document is fully signed and certificated.
    Certificated,
    /// At least one signer declined.
    RejectedBySigner,
    /// Awaiting signatures from one or more signers.
    PendingSignature,
    /// Cancelled by the document owner.
    RejectedByUser,
    /// Processing failed.
    Failed,
    /// Any value the SDK does not yet model.
    #[serde(untagged)]
    Unknown(String),
}

impl DocumentStatus {
    /// Returns the wire-format string.
    pub fn as_str(&self) -> &str {
        match self {
            DocumentStatus::Uploading => "uploading",
            DocumentStatus::Uploaded => "uploaded",
            DocumentStatus::MetadataProcessing => "metadata_processing",
            DocumentStatus::MetadataReady => "metadata_ready",
            DocumentStatus::Expired => "expired",
            DocumentStatus::Certificating => "certificating",
            DocumentStatus::Certificated => "certificated",
            DocumentStatus::RejectedBySigner => "rejected_by_signer",
            DocumentStatus::PendingSignature => "pending_signature",
            DocumentStatus::RejectedByUser => "rejected_by_user",
            DocumentStatus::Failed => "failed",
            DocumentStatus::Unknown(s) => s.as_str(),
        }
    }
}

impl fmt::Display for DocumentStatus {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.as_str())
    }
}

/// One entry from `GET /documents/statuses`.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[non_exhaustive]
pub struct DocumentStatusInfo {
    /// Status code.
    pub code: DocumentStatus,
    /// Whether documents in this status may be deleted.
    pub deletable: bool,
}

/// Result of `GET /documents/{signature_hash}/verify`.
///
/// When the hash is not found (or the document is not signed), [`is_valid`] is
/// `false` and the descriptive fields are `None`. Note that the API returns
/// `page_count` and `signer_count` as **strings** (e.g. `"1"`).
///
/// [`is_valid`]: DocumentVerification::is_valid
#[derive(Debug, Clone, Serialize, Deserialize)]
#[non_exhaustive]
pub struct DocumentVerification {
    /// The signature hash that was looked up (echoed back).
    #[serde(default)]
    pub hash: Option<String>,
    /// Document identifier, when the hash resolves to a certificated document.
    #[serde(default)]
    pub id: Option<String>,
    /// Document status, when found.
    #[serde(default)]
    pub status: Option<String>,
    /// Page count as reported by the API (a string such as `"1"`), when found.
    #[serde(default)]
    pub page_count: Option<String>,
    /// Signer count as reported by the API (a string such as `"1"`), when found.
    #[serde(default)]
    pub signer_count: Option<String>,
    /// Number of signers that have completed signing, when found.
    #[serde(default)]
    pub completed_count: Option<u32>,
    /// Completion timestamp (ISO-8601), when found.
    #[serde(default)]
    pub completed_at: Option<String>,
    /// Timestamp the verification was performed (ISO-8601).
    #[serde(default)]
    pub verified_at: Option<String>,
    /// Whether the document is a valid, signed Assinafy document.
    pub is_valid: bool,
    /// Human-readable detail. Empty on success; explains the failure otherwise.
    #[serde(default)]
    pub message: String,
}

/// Public, unauthenticated view of a document returned by
/// `GET /public/documents/{document_id}`.
///
/// The current API documents the full [`Document`] shape. All fields other
/// than `id` and `name` remain optional here because older deployments return
/// only `resource`, `id`, `name`, `page_count`, and `created_by`.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[non_exhaustive]
pub struct PublicDocument {
    /// Resource discriminator (always `"document"` when present).
    #[serde(default)]
    pub resource: Option<String>,
    /// Document identifier.
    pub id: String,
    /// Owning account identifier, when included.
    #[serde(default)]
    pub account_id: Option<String>,
    /// Source template identifier, when included.
    #[serde(default)]
    pub template_id: Option<String>,
    /// Document file name.
    pub name: String,
    /// Current document status, when included.
    #[serde(default)]
    pub status: Option<DocumentStatus>,
    /// Downloadable artifacts keyed by artifact name.
    #[serde(default)]
    pub artifacts: BTreeMap<String, String>,
    /// Pages within the document.
    #[serde(default)]
    pub pages: Vec<DocumentPage>,
    /// Expanded assignment, when included.
    #[serde(default)]
    pub assignment: Option<Assignment>,
    /// Whether the document is closed, when included.
    #[serde(default)]
    pub is_closed: Option<bool>,
    /// Signing URL, when available.
    #[serde(default)]
    pub signing_url: Option<String>,
    /// Decline reason, when the document was declined.
    #[serde(default)]
    pub decline_reason: Option<String>,
    /// Identity of the signer or user that declined the document.
    #[serde(default)]
    pub declined_by: Option<DeclinedBy>,
    /// Tags attached to the document.
    #[serde(default)]
    pub tags: Vec<Tag>,
    /// Creation timestamp (ISO-8601 string or Unix epoch number).
    #[serde(default)]
    pub created_at: Option<serde_json::Value>,
    /// Last-modification timestamp.
    #[serde(default)]
    pub updated_at: Option<serde_json::Value>,
    /// Page count as reported by the API (a string such as `"1"`).
    ///
    /// This legacy field is absent from the current document schema.
    #[serde(default)]
    pub page_count: Option<String>,
    /// Display name of the user that created the document.
    ///
    /// This legacy field is absent from the current document schema.
    #[serde(default)]
    pub created_by: Option<String>,
}
