//! Assignment (signature-request) models.

use std::fmt;

use serde::{Deserialize, Deserializer, Serialize};

/// Delivery method for an assignment.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum AssignmentMethod {
    /// Signers are notified and sign remotely.
    Virtual,
    /// Signers sign in-person on the document owner's device.
    Collect,
}

impl AssignmentMethod {
    /// Wire-format string.
    pub fn as_str(&self) -> &'static str {
        match self {
            AssignmentMethod::Virtual => "virtual",
            AssignmentMethod::Collect => "collect",
        }
    }
}

impl fmt::Display for AssignmentMethod {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.as_str())
    }
}

/// Verification method applied to a signer.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum VerificationMethod {
    /// Email code.
    Email,
    /// WhatsApp code.
    Whatsapp,
    /// ICP-Brasil digital certificate.
    DigitalCertificate,
    /// Skip verification (only allowed in specific configurations).
    Bypass,
    /// Any value the SDK does not yet model.
    #[serde(untagged)]
    Other(String),
}

impl VerificationMethod {
    /// Wire-format string.
    pub fn as_str(&self) -> &str {
        match self {
            VerificationMethod::Email => "Email",
            VerificationMethod::Whatsapp => "Whatsapp",
            VerificationMethod::DigitalCertificate => "DigitalCertificate",
            VerificationMethod::Bypass => "Bypass",
            VerificationMethod::Other(s) => s.as_str(),
        }
    }
}

/// Notification channel.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum NotificationMethod {
    /// Email channel.
    Email,
    /// WhatsApp channel.
    Whatsapp,
    /// Any value the SDK does not yet model.
    #[serde(untagged)]
    Other(String),
}

impl NotificationMethod {
    /// Wire-format string.
    pub fn as_str(&self) -> &str {
        match self {
            NotificationMethod::Email => "Email",
            NotificationMethod::Whatsapp => "Whatsapp",
            NotificationMethod::Other(s) => s.as_str(),
        }
    }
}

/// Status of a notification delivery attempt.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum NotificationStatus {
    /// Notification was successfully sent.
    Sent,
    /// Notification delivery failed.
    Failed,
    /// Any value the SDK does not yet model.
    #[serde(untagged)]
    Other(String),
}

/// Notification event type.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum NotificationEvent {
    /// Initial signature request dispatched.
    SignatureRequest,
    /// Document close to expiring.
    DocumentAboutToExpire,
    /// Document has expired.
    DocumentExpired,
    /// Document was cancelled by the owner.
    DocumentCanceled,
    /// Document was declined by a signer.
    DocumentDeclined,
    /// Signed copy delivered.
    SignedDelivery,
    /// Any value the SDK does not yet model.
    #[serde(untagged)]
    Unknown(String),
}

/// Single notification delivery attempt for a signer.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[non_exhaustive]
pub struct NotificationHistoryEntry {
    /// Event triggering the notification.
    pub event: NotificationEvent,
    /// Delivery status.
    pub status: NotificationStatus,
    /// Provider error code (when failed).
    #[serde(default)]
    pub error_code: Option<String>,
    /// Provider error message (when failed).
    #[serde(default)]
    pub error_message: Option<String>,
    /// Timestamp the notification was sent (when status is `Sent`).
    #[serde(default)]
    pub sent_at: Option<String>,
    /// Timestamp the notification failed (when status is `Failed`).
    #[serde(default)]
    pub failed_at: Option<String>,
}

/// Signer assigned to an assignment.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[non_exhaustive]
pub struct AssignmentSigner {
    /// Signer identifier.
    pub id: String,
    /// Full name.
    pub full_name: String,
    /// Email address.
    #[serde(default)]
    pub email: Option<String>,
    /// WhatsApp phone number (E.164).
    #[serde(default)]
    pub whatsapp_phone_number: Option<String>,
    /// Whether the signer has accepted the terms.
    #[serde(default)]
    pub has_accepted_terms: bool,
    /// Method used to verify the signer's identity.
    #[serde(default)]
    pub verification_method: Option<VerificationMethod>,
    /// Notification channels used to reach the signer.
    #[serde(default)]
    pub notification_methods: Option<Vec<NotificationMethod>>,
    /// 1-based signing order, when sequential signing is in effect.
    #[serde(default)]
    pub step: Option<u32>,
    /// Whether the initial notification was dispatched.
    #[serde(default)]
    pub notified: Option<bool>,
    /// Whether all items assigned to this signer are completed.
    #[serde(default)]
    pub completed: Option<bool>,
    /// Delivery history for notifications sent to this signer.
    #[serde(default, deserialize_with = "null_as_default")]
    pub notification_history: Vec<NotificationHistoryEntry>,
}

fn null_as_default<'de, D, T>(deserializer: D) -> Result<T, D::Error>
where
    D: Deserializer<'de>,
    T: Deserialize<'de> + Default,
{
    Ok(Option::<T>::deserialize(deserializer)?.unwrap_or_default())
}

/// Copy-only recipient who receives the final signed document.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[non_exhaustive]
pub struct CopyReceiver {
    /// Recipient identifier.
    pub id: String,
    /// Full name.
    pub full_name: String,
    /// Email address.
    #[serde(default)]
    pub email: Option<String>,
    /// WhatsApp phone number (E.164).
    #[serde(default)]
    pub whatsapp_phone_number: Option<String>,
    /// Whether the recipient has accepted the terms.
    #[serde(default)]
    pub has_accepted_terms: bool,
}

/// Signer summary embedded in an [`AssignmentItem`].
///
/// Item payloads are intentionally smaller than the full assignment signer
/// payload, so every field except the identifier is optional.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[non_exhaustive]
pub struct AssignmentItemSigner {
    /// Signer identifier.
    pub id: String,
    /// Full name, when included.
    #[serde(default)]
    pub full_name: Option<String>,
    /// Email address, when included.
    #[serde(default)]
    pub email: Option<String>,
    /// WhatsApp phone number, when included.
    #[serde(default)]
    pub whatsapp_phone_number: Option<String>,
}

/// Field summary embedded in an [`AssignmentItem`].
#[derive(Debug, Clone, Serialize, Deserialize)]
#[non_exhaustive]
pub struct AssignmentItemField {
    /// Field identifier.
    pub id: String,
    /// Human-readable field name, when included.
    #[serde(default)]
    pub name: Option<String>,
    /// Field type, such as `signature` or `virtual`.
    #[serde(default, rename = "type")]
    pub kind: Option<String>,
}

/// A single item within an assignment (signature placeholder, field, etc.).
#[derive(Debug, Clone, Serialize, Deserialize)]
#[non_exhaustive]
pub struct AssignmentItem {
    /// Item identifier.
    pub id: String,
    /// Page the item lives on, if any.
    #[serde(default)]
    pub page: Option<crate::models::document::DocumentPage>,
    /// Signer the item is assigned to.
    #[serde(default)]
    pub signer: Option<AssignmentItemSigner>,
    /// Field definition, if applicable.
    #[serde(default)]
    pub field: Option<AssignmentItemField>,
    /// Display settings for the item. Genuinely polymorphic on the wire
    /// (an array when unset, an object when configured), so it stays
    /// untyped.
    #[serde(default)]
    pub display_settings: Option<serde_json::Value>,
    /// Captured value.
    #[serde(default)]
    pub value: Option<serde_json::Value>,
    /// Whether the item is completed.
    #[serde(default)]
    pub completed: bool,
}

/// Pre-built direct signing URL for one of the assignment signers.
#[derive(Clone, Serialize, Deserialize)]
#[non_exhaustive]
pub struct SigningUrl {
    /// Signer identifier.
    pub signer_id: String,
    /// Direct signing URL.
    pub url: String,
}

impl fmt::Debug for SigningUrl {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("SigningUrl")
            .field("signer_id", &self.signer_id)
            .field("url", &"**redacted**")
            .finish()
    }
}

/// Aggregated completion summary returned with an assignment.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[non_exhaustive]
pub struct AssignmentSummary {
    /// Total signers in the assignment.
    pub signer_count: u32,
    /// Number of signers that have completed all their items.
    pub completed_count: u32,
    /// Per-signer summary.
    #[serde(default)]
    pub signers: Vec<AssignmentSummarySigner>,
}

/// A single signer entry inside [`AssignmentSummary`].
#[derive(Debug, Clone, Serialize, Deserialize)]
#[non_exhaustive]
pub struct AssignmentSummarySigner {
    /// Signer identifier.
    pub id: String,
    /// Full name.
    pub full_name: String,
    /// Email address.
    #[serde(default)]
    pub email: Option<String>,
    /// WhatsApp phone number (E.164), when the signer has one.
    #[serde(default)]
    pub whatsapp_phone_number: Option<String>,
    /// Whether the signer accepted the terms.
    #[serde(default)]
    pub has_accepted_terms: bool,
    /// Whether the signer has completed all of their items.
    pub completed: bool,
}

/// A signature request attached to a document.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[non_exhaustive]
pub struct Assignment {
    /// Resource discriminator (always `"assignment"` when present).
    #[serde(default)]
    pub resource: Option<String>,
    /// Assignment identifier.
    pub id: String,
    /// Compatibility document identifier, when returned by a deployment.
    #[serde(default)]
    pub document_id: Option<String>,
    /// Email of the user that created the assignment.
    #[serde(default)]
    pub sender_email: Option<String>,
    /// Delivery method.
    #[serde(default)]
    pub method: Option<AssignmentMethod>,
    /// Compatibility assignment status, when returned by a deployment.
    #[serde(default)]
    pub status: Option<String>,
    /// Expiration timestamp using the compatibility field name. Prefer
    /// [`expires_at`](Self::expires_at).
    #[serde(default)]
    pub expiration: Option<String>,
    /// Expiry timestamp (ISO-8601).
    #[serde(default)]
    pub expires_at: Option<String>,
    /// Optional invitation message.
    #[serde(default)]
    pub message: Option<String>,
    /// Signers assigned to the document.
    #[serde(default)]
    pub signers: Vec<AssignmentSigner>,
    /// Copy-only recipients.
    #[serde(default)]
    pub copy_receivers: Vec<CopyReceiver>,
    /// Assignment items.
    #[serde(default)]
    pub items: Vec<AssignmentItem>,
    /// Completion summary.
    #[serde(default)]
    pub summary: Option<AssignmentSummary>,
    /// Direct signing URLs.
    #[serde(default)]
    pub signing_urls: Vec<SigningUrl>,
    /// Compatibility completion timestamp, when returned by a deployment.
    #[serde(default)]
    pub completed_at: Option<serde_json::Value>,
    /// Compatibility creation timestamp, when returned by a deployment.
    #[serde(default)]
    pub created_at: Option<serde_json::Value>,
    /// Compatibility last-modification timestamp, when returned by a
    /// deployment.
    #[serde(default)]
    pub updated_at: Option<serde_json::Value>,
}

/// Response returned by assignment notification resend endpoints.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[non_exhaustive]
pub struct ResendNotificationResult {
    /// Whether a notification was sent.
    #[serde(default)]
    pub is_sent: bool,
    /// Document identifier.
    #[serde(default)]
    pub document_id: Option<String>,
    /// Signer identifier.
    #[serde(default)]
    pub signer_id: Option<String>,
}

/// Backward-compatible name for a resend cost line.
pub type ResendCostBreakdownItem = crate::models::cost::CostBreakdownItem;

/// Backward-compatible name for the standard cost-estimate response.
pub type ResendCostEstimate = crate::models::cost::CostEstimate;

/// One filled field submitted by the signer-facing sign endpoint.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SignDocumentItem {
    /// Assignment item identifier.
    #[serde(rename = "itemId")]
    pub item_id: String,
    /// Field definition identifier.
    #[serde(rename = "fieldId")]
    pub field_id: String,
    /// Document page identifier.
    #[serde(rename = "pageId")]
    pub page_id: String,
    /// Value supplied by the signer.
    pub value: String,
}

impl SignDocumentItem {
    /// Build one signer-filled item.
    pub fn new<I, F, P, V>(item_id: I, field_id: F, page_id: P, value: V) -> Self
    where
        I: Into<String>,
        F: Into<String>,
        P: Into<String>,
        V: Into<String>,
    {
        Self {
            item_id: item_id.into(),
            field_id: field_id.into(),
            page_id: page_id.into(),
            value: value.into(),
        }
    }
}

/// WhatsApp message metadata returned for assignment notifications.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[non_exhaustive]
pub struct WhatsAppNotification {
    /// Sent timestamp.
    pub sent_at: serde_json::Value,
    /// Message header.
    pub header: String,
    /// Message body.
    pub body: String,
    /// Buttons shown in the message.
    #[serde(default)]
    pub buttons: Vec<WhatsAppButton>,
    /// Destination phone number.
    pub phone_number: String,
    /// Signer identifier.
    pub signer_id: String,
}

/// Button embedded in a WhatsApp notification.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[non_exhaustive]
pub struct WhatsAppButton {
    /// Button text.
    pub text: String,
    /// Button URL, when present.
    #[serde(default)]
    pub url: Option<String>,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn assignment_signer_accepts_nullable_notification_history() {
        let signer: AssignmentSigner = serde_json::from_value(serde_json::json!({
            "id": "signer-id",
            "full_name": "Example Signer",
            "notification_history": null
        }))
        .unwrap();
        assert!(signer.notification_history.is_empty());
    }
}
