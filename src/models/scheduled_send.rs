//! Deferred assignment and envelope creation.

use serde::{Deserialize, Serialize};

/// What a scheduled send creates.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
#[non_exhaustive]
pub enum ScheduledSendType {
    /// One document's assignment.
    Assignment,
    /// A multi-document envelope.
    Envelope,
    /// A future server value.
    #[serde(untagged)]
    Unknown(String),
}

/// Lifecycle of a scheduled send.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
#[non_exhaustive]
pub enum ScheduledSendStatus {
    /// Waiting until its scheduled time or an explicit send-now request.
    Pending,
    /// The stored creation request is being executed.
    Processing,
    /// Creation succeeded; invitation delivery is tracked separately.
    Sent,
    /// Cancelled before processing.
    Cancelled,
    /// Creation failed; inspect `failure_reason`.
    Failed,
    /// A future server value.
    #[serde(untagged)]
    Unknown(String),
}

/// A stored assignment or envelope request, executed as the scheduling member.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[non_exhaustive]
pub struct ScheduledSend {
    /// Resource discriminator.
    #[serde(default)]
    pub resource: Option<String>,
    /// Scheduled send identifier.
    pub id: String,
    /// Resource this send creates.
    #[serde(rename = "type")]
    pub kind: ScheduledSendType,
    /// Documents to send; deleted documents are omitted.
    pub document_ids: Vec<String>,
    /// Current execution state.
    pub status: ScheduledSendStatus,
    /// Scheduled time in ISO 8601 format.
    pub send_at: String,
    /// Assignment created by a sent assignment request.
    #[serde(default)]
    pub assignment_id: Option<String>,
    /// Envelope created by a sent envelope request; null after deletion.
    #[serde(default)]
    pub envelope_id: Option<String>,
    /// Human-readable reason for a background failure.
    #[serde(default)]
    pub failure_reason: Option<String>,
    /// Creation time in ISO 8601 format.
    pub created_at: String,
    /// Time sent, cancelled or failed.
    #[serde(default)]
    pub processed_at: Option<String>,
}
