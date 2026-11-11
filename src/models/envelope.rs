//! Documents sent together in an envelope.

use std::collections::HashMap;
use std::fmt;

use serde::{Deserialize, Serialize};

use super::{Activity, DocumentStatus, SigningUrl};

/// An envelope of independently signed and certificated documents.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[non_exhaustive]
pub struct DocumentEnvelope {
    /// Resource discriminator, present on single-resource responses.
    #[serde(default)]
    pub resource: Option<String>,
    /// Envelope identifier.
    pub id: String,
    /// Name used in invitations.
    pub name: String,
    /// Optional invitation message.
    #[serde(default)]
    pub message: Option<String>,
    /// Documents in their presentation order.
    pub documents: Vec<EnvelopeDocument>,
    /// Document counts by status; null when no documents remain.
    #[serde(default)]
    pub counts: Option<HashMap<String, u32>>,
    /// One invitation entry per signer; embedded signers have no public link.
    pub signing_urls: Vec<SigningUrl>,
    /// Creation time in ISO 8601 format.
    pub created_at: String,
}

/// A document listed within an envelope.
#[derive(Clone, Serialize, Deserialize)]
#[non_exhaustive]
pub struct EnvelopeDocument {
    /// Document identifier.
    pub id: String,
    /// One-based position in the envelope.
    pub position: u32,
    /// Document name.
    pub name: String,
    /// Current document status.
    pub status: DocumentStatus,
    /// Download URLs keyed by artifact name; lists include only the thumbnail.
    pub artifacts: HashMap<String, String>,
    /// Signers of this document.
    pub signers: Vec<EnvelopeSigner>,
}

impl fmt::Debug for EnvelopeDocument {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("EnvelopeDocument")
            .field("id", &self.id)
            .field("position", &self.position)
            .field("name", &self.name)
            .field("status", &self.status)
            .field("artifacts", &"**redacted**")
            .field("signers", &self.signers)
            .finish()
    }
}

/// Completion state of a signer on one envelope document.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[non_exhaustive]
pub struct EnvelopeSigner {
    /// Signer identifier.
    pub id: String,
    /// Signer's full name.
    pub full_name: String,
    /// Email address, when supplied.
    #[serde(default)]
    pub email: Option<String>,
    /// WhatsApp number, when supplied.
    #[serde(default)]
    pub whatsapp_phone_number: Option<String>,
    /// Whether this document was signed.
    pub completed: bool,
    /// Signing time, or null until signed.
    #[serde(default)]
    pub signed_at: Option<String>,
}

/// A document activity in an envelope's combined trail.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[non_exhaustive]
pub struct EnvelopeActivity {
    /// The underlying activity and its original wire fields.
    #[serde(flatten)]
    pub activity: Activity,
    /// Identifier of the document the event concerns.
    pub document_id: String,
    /// Name of the document the event concerns.
    pub document_name: String,
}

/// Outcome of cancelling one document in an envelope.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[non_exhaustive]
pub struct EnvelopeCancellation {
    /// Document identifier.
    pub id: String,
    /// Document status after the call.
    pub status: DocumentStatus,
    /// Whether this call cancelled the document.
    pub cancelled: bool,
}

/// Outcome of reminding an envelope signer.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[non_exhaustive]
pub struct EnvelopeReminderResult {
    /// Whether all configured notification methods sent the reminder.
    pub is_sent: bool,
    /// Envelope identifier.
    pub envelope_id: String,
    /// Signer identifier.
    pub signer_id: String,
}
