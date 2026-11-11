//! Embedded signing origins and short-lived sessions.

use std::fmt;

use serde::{Deserialize, Serialize};

use super::VerificationMethod;

/// An origin allowed to host the workspace's signing iframe.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[non_exhaustive]
pub struct EmbedOrigin {
    /// Origin entry identifier.
    pub id: String,
    /// Exact scheme, host and optional non-default port.
    pub origin: String,
    /// Creation time in ISO 8601 format.
    pub created_at: String,
}

/// A single-use signing URL, valid for five minutes.
#[derive(Clone, Serialize, Deserialize)]
#[non_exhaustive]
pub struct EmbedSession {
    /// Secret URL to load unchanged in an iframe on an allowed origin.
    pub url: String,
    /// Expiry time in ISO 8601 format.
    pub expires_at: String,
}

impl fmt::Debug for EmbedSession {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("EmbedSession")
            .field("url", &"**redacted**")
            .field("expires_at", &self.expires_at)
            .finish()
    }
}

/// Credentials returned only to the hosted embed page after token exchange.
#[derive(Clone, Serialize, Deserialize)]
#[non_exhaustive]
pub struct EmbedSessionExchange {
    /// Signer credential; do not log it or persist it in browser storage.
    pub signer_access_code: String,
    /// Signer identifier.
    pub signer_id: String,
    /// Document identifier.
    pub document_id: String,
    /// Verification method for this signer.
    pub verification_method: VerificationMethod,
    /// Origins allowed to host the iframe.
    pub embed_origins: Vec<String>,
}

impl fmt::Debug for EmbedSessionExchange {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("EmbedSessionExchange")
            .field("signer_access_code", &"**redacted**")
            .field("signer_id", &self.signer_id)
            .field("document_id", &self.document_id)
            .field("verification_method", &self.verification_method)
            .field("embed_origins", &self.embed_origins)
            .finish()
    }
}
