//! Signer models.

use serde::{Deserialize, Serialize};

/// Resource discriminator emitted by the API on signer payloads.
pub const SIGNER_RESOURCE: &str = "signer";

/// A person who can sign documents in an account.
///
/// Returned by every signer endpoint. The optional `resource` field is the
/// `"signer"` discriminator the API emits.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[non_exhaustive]
pub struct Signer {
    /// Resource type discriminator (always `"signer"` when present).
    #[serde(default)]
    pub resource: Option<String>,
    /// Server-assigned identifier.
    pub id: String,
    /// Full legal name.
    pub full_name: String,
    /// Optional email address (required for `Email` verification flows).
    #[serde(default)]
    pub email: Option<String>,
    /// Optional WhatsApp phone number in E.164 format (required for
    /// `Whatsapp` verification flows).
    #[serde(default)]
    pub whatsapp_phone_number: Option<String>,
    /// Whether the signer has accepted the platform terms.
    #[serde(default)]
    pub has_accepted_terms: bool,
}

/// Signature image type a signer can upload.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum SignerType {
    /// Full signature image.
    Signature,
    /// Initials image.
    Initial,
}

impl SignerType {
    /// Returns the URL path segment (`"signature"` or `"initial"`).
    pub fn as_str(&self) -> &'static str {
        match self {
            SignerType::Signature => "signature",
            SignerType::Initial => "initial",
        }
    }
}

/// Extended signer profile returned by `GET /signers/self`.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[non_exhaustive]
pub struct SignerSelf {
    /// Resource discriminator (always `"signer"`).
    #[serde(default)]
    pub resource: Option<String>,
    /// Signer identifier.
    pub id: String,
    /// Full legal name.
    pub full_name: String,
    /// Email address.
    #[serde(default)]
    pub email: Option<String>,
    /// WhatsApp phone number in E.164 format.
    #[serde(default)]
    pub whatsapp_phone_number: Option<String>,
    /// Whether the signer has accepted the platform terms.
    #[serde(default)]
    pub has_accepted_terms: bool,
    /// Whether the signer has uploaded a signature image.
    #[serde(default)]
    pub has_signature: bool,
    /// Whether the signer has uploaded an initials image.
    #[serde(default)]
    pub has_initial: bool,
    /// Whether the signer opted to reuse their saved signature/initials across
    /// documents. When `false`, the signer is prompted to draw a fresh
    /// signature for each document.
    #[serde(default)]
    pub is_signature_reusable: bool,
}
