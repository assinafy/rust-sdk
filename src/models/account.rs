//! Account / workspace models.

use serde::{Deserialize, Deserializer, Serialize};

/// Who is shown as the sender of account signature-request notifications.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum NotificationSenderType {
    /// Notifications are sent on behalf of the individual user.
    User,
    /// Notifications are sent on behalf of the account/workspace.
    Account,
}

/// Limits imposed by a workspace's current plan.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[non_exhaustive]
pub struct AccountLimits {
    /// Maximum number of webhook endpoints the workspace may create.
    #[serde(default)]
    pub webhook_endpoints: Option<u32>,
}

/// A workspace / billing account that owns documents, signers, tags, etc.
///
/// Returned by the account endpoints. The exact field set depends on the
/// endpoint: `GET /v1/accounts` (list) populates [`roles`](Account::roles) and
/// [`is_delete_allowed`](Account::is_delete_allowed), whereas
/// `GET /v1/accounts/{id}` populates [`primary_color`](Account::primary_color)
/// and [`secondary_color`](Account::secondary_color). All environment-specific
/// fields are optional so a single type deserializes either shape.
///
/// # List payload (`GET /v1/accounts`)
///
/// ```json
/// {
///   "id": "acc_1234567890abcdef12345678",
///   "name": "Example Workspace",
///   "roles": ["owner"],
///   "is_delete_allowed": true,
///   "created_at": "2026-05-12T18:05:11Z"
/// }
/// ```
///
/// # Account payload (`GET /v1/accounts/{accountId}`)
///
/// ```json
/// {
///   "resource": "account",
///   "id": "acc_1234567890abcdef12345678",
///   "name": "Example Workspace",
///   "primary_color": null,
///   "secondary_color": null,
///   "notification_sender_type": "User",
///   "limits": { "webhook_endpoints": 1 },
///   "created_at": "2026-05-12T18:05:11Z"
/// }
/// ```
#[derive(Debug, Clone, Serialize, Deserialize)]
#[non_exhaustive]
pub struct Account {
    /// Resource discriminator (normally `"account"`).
    #[serde(default)]
    pub resource: Option<String>,
    /// Account identifier.
    pub id: String,
    /// Display name.
    pub name: String,
    /// Roles held by the authenticated user (e.g. `"owner"`). Populated by the
    /// list endpoint.
    #[serde(default)]
    pub roles: Vec<String>,
    /// Whether the user may delete this account. Populated by the list
    /// endpoint.
    #[serde(default)]
    pub is_delete_allowed: bool,
    /// Primary brand color as a 6-character hex string without a leading `#`
    /// (e.g. `"2072b9"`), when set. Populated by the by-id endpoint.
    #[serde(default)]
    pub primary_color: Option<String>,
    /// Secondary brand color as a 6-character hex string without a leading `#`,
    /// when set. Populated by the by-id endpoint.
    #[serde(default)]
    pub secondary_color: Option<String>,
    /// Actor shown as the sender of signature-request notifications.
    #[serde(default)]
    pub notification_sender_type: Option<NotificationSenderType>,
    /// Limits of the workspace's current plan. Returned only by the by-id
    /// account endpoint.
    #[serde(default)]
    pub limits: Option<AccountLimits>,
    /// Capabilities returned with `expand=signing_features`.
    #[serde(default)]
    pub signing_features: Option<SigningFeatures>,
    /// ISO-8601 creation timestamp.
    #[serde(default)]
    pub created_at: Option<String>,
}

/// Branding/theme information for an account.
///
/// Returned by `GET /v1/accounts/{accountId}/theme`.
///
/// # Example payload
///
/// ```json
/// {
///   "account_name": "Example Workspace",
///   "primary_color": "2072b9",
///   "secondary_color": "ffffff",
///   "logo": null
/// }
/// ```
#[derive(Debug, Clone, Serialize, Deserialize)]
#[non_exhaustive]
pub struct AccountTheme {
    /// Account display name.
    #[serde(default)]
    pub account_name: Option<String>,
    /// Primary color as a 6-character hex string without a leading `#`.
    #[serde(default)]
    pub primary_color: Option<String>,
    /// Secondary color as a 6-character hex string without a leading `#`.
    #[serde(default)]
    pub secondary_color: Option<String>,
    /// URL of the account logo image, when one has been uploaded.
    #[serde(default)]
    pub logo: Option<String>,
}

/// One period in an account or user document-funnel statistics series.
///
/// Returned by `GET /v1/accounts/{accountId}/stats` and
/// `GET /v1/users/self/stats`. Monthly responses use a `YYYY-MM` period;
/// daily responses use `YYYY-MM-DD`. Both series are zero-filled by the API.
///
/// # Example payload
///
/// ```json
/// {
///   "period": "2026-06",
///   "documents_uploaded": 42,
///   "documents_sent": 37,
///   "signature_requests": 61,
///   "signature_requests_notification_email": 55,
///   "signature_requests_notification_whatsapp": 18,
///   "signature_requests_notification_bypass": 3,
///   "signature_requests_verification_email": 48,
///   "signature_requests_verification_whatsapp": 6,
///   "signature_requests_verification_bypass": 3,
///   "signature_requests_verification_digital_certificate": 4,
///   "signature_requests_viewed": 44,
///   "signature_requests_completed": 52,
///   "documents_certified": 30
/// }
/// ```
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[non_exhaustive]
pub struct DocumentStatsRow {
    /// Period represented by this row: `YYYY-MM` or `YYYY-MM-DD`.
    pub period: String,
    /// Documents uploaded during the period.
    pub documents_uploaded: u64,
    /// Documents sent for signature during the period.
    pub documents_sent: u64,
    /// Total signature requests created during the period.
    pub signature_requests: u64,
    /// Requests for which e-mail was a notification channel.
    pub signature_requests_notification_email: u64,
    /// Requests for which WhatsApp was a notification channel.
    pub signature_requests_notification_whatsapp: u64,
    /// Requests sent without a notification (`Bypass`).
    pub signature_requests_notification_bypass: u64,
    /// Requests verified by an e-mail token.
    pub signature_requests_verification_email: u64,
    /// Requests verified by a WhatsApp token.
    pub signature_requests_verification_whatsapp: u64,
    /// Requests signed without token verification (`Bypass`).
    pub signature_requests_verification_bypass: u64,
    /// Requests verified with an ICP-Brasil digital certificate.
    pub signature_requests_verification_digital_certificate: u64,
    /// Compatibility alias for [`signature_requests_notification_email`](Self::signature_requests_notification_email).
    #[deprecated(note = "use signature_requests_notification_email")]
    #[serde(skip_serializing)]
    pub signature_requests_email: u64,
    /// Compatibility alias for [`signature_requests_notification_whatsapp`](Self::signature_requests_notification_whatsapp).
    #[deprecated(note = "use signature_requests_notification_whatsapp")]
    #[serde(skip_serializing)]
    pub signature_requests_whatsapp: u64,
    /// Signature requests whose document was first viewed during the period.
    pub signature_requests_viewed: u64,
    /// Signature requests completed by individual signers during the period.
    pub signature_requests_completed: u64,
    /// Documents certified during the period.
    pub documents_certified: u64,
}

#[derive(Deserialize)]
struct DocumentStatsRowWire {
    period: String,
    #[serde(default)]
    documents_uploaded: u64,
    #[serde(default)]
    documents_sent: u64,
    #[serde(default)]
    signature_requests: u64,
    #[serde(default)]
    signature_requests_notification_email: Option<u64>,
    #[serde(default)]
    signature_requests_notification_whatsapp: Option<u64>,
    #[serde(default)]
    signature_requests_notification_bypass: u64,
    #[serde(default)]
    signature_requests_verification_email: u64,
    #[serde(default)]
    signature_requests_verification_whatsapp: u64,
    #[serde(default)]
    signature_requests_verification_bypass: u64,
    #[serde(default)]
    signature_requests_verification_digital_certificate: u64,
    #[serde(default)]
    signature_requests_email: Option<u64>,
    #[serde(default)]
    signature_requests_whatsapp: Option<u64>,
    #[serde(default)]
    signature_requests_viewed: u64,
    #[serde(default)]
    signature_requests_completed: u64,
    #[serde(default)]
    documents_certified: u64,
}

impl<'de> Deserialize<'de> for DocumentStatsRow {
    #[allow(deprecated)]
    fn deserialize<D: Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        let wire = DocumentStatsRowWire::deserialize(deserializer)?;
        let notification_email = wire
            .signature_requests_notification_email
            .or(wire.signature_requests_email)
            .unwrap_or_default();
        let notification_whatsapp = wire
            .signature_requests_notification_whatsapp
            .or(wire.signature_requests_whatsapp)
            .unwrap_or_default();
        Ok(Self {
            period: wire.period,
            documents_uploaded: wire.documents_uploaded,
            documents_sent: wire.documents_sent,
            signature_requests: wire.signature_requests,
            signature_requests_notification_email: notification_email,
            signature_requests_notification_whatsapp: notification_whatsapp,
            signature_requests_notification_bypass: wire.signature_requests_notification_bypass,
            signature_requests_verification_email: wire.signature_requests_verification_email,
            signature_requests_verification_whatsapp: wire.signature_requests_verification_whatsapp,
            signature_requests_verification_bypass: wire.signature_requests_verification_bypass,
            signature_requests_verification_digital_certificate: wire
                .signature_requests_verification_digital_certificate,
            signature_requests_email: notification_email,
            signature_requests_whatsapp: notification_whatsapp,
            signature_requests_viewed: wire.signature_requests_viewed,
            signature_requests_completed: wire.signature_requests_completed,
            documents_certified: wire.documents_certified,
        })
    }
}

/// Workspace rules for document alerts across e-mail and push channels.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[non_exhaustive]
pub struct NotificationPolicyEvents {
    /// DocumentCompleted notification.
    #[serde(rename = "DocumentCompleted", default)]
    pub document_completed: Option<bool>,
    /// SignerDeclined notification.
    #[serde(rename = "SignerDeclined", default)]
    pub signer_declined: Option<bool>,
    /// DocumentCancelled notification.
    #[serde(rename = "DocumentCancelled", default)]
    pub document_cancelled: Option<bool>,
    /// DocumentAboutToExpire notification.
    #[serde(rename = "DocumentAboutToExpire", default)]
    pub document_about_to_expire: Option<bool>,
    /// DocumentExpired notification.
    #[serde(rename = "DocumentExpired", default)]
    pub document_expired: Option<bool>,
    /// DocumentExpirationReset notification.
    #[serde(rename = "DocumentExpirationReset", default)]
    pub document_expiration_reset: Option<bool>,
    /// DocumentProcessingFailed notification.
    #[serde(rename = "DocumentProcessingFailed", default)]
    pub document_processing_failed: Option<bool>,
    /// TemplateProcessingFailed notification.
    #[serde(rename = "TemplateProcessingFailed", default)]
    pub template_processing_failed: Option<bool>,
    /// SignerWhatsappFailed notification.
    #[serde(rename = "SignerWhatsappFailed", default)]
    pub signer_whatsapp_failed: Option<bool>,
    /// ContractRenewalNotice notification.
    #[serde(rename = "ContractRenewalNotice", default)]
    pub contract_renewal_notice: Option<bool>,
    /// ContractExpiring notification.
    #[serde(rename = "ContractExpiring", default)]
    pub contract_expiring: Option<bool>,
    /// ContractExpired notification.
    #[serde(rename = "ContractExpired", default)]
    pub contract_expired: Option<bool>,
    /// ContractRenewed notification.
    #[serde(rename = "ContractRenewed", default)]
    pub contract_renewed: Option<bool>,
    /// SignerViewed notification.
    #[serde(rename = "SignerViewed", default)]
    pub signer_viewed: Option<bool>,
    /// SignerSigned notification.
    #[serde(rename = "SignerSigned", default)]
    pub signer_signed: Option<bool>,
    /// Turns off the push only; the identity review e-mail is always sent.
    #[serde(rename = "SignerIdentityReviewRequired", default)]
    pub signer_identity_review_required: Option<bool>,
}

/// Workspace notification policy. Only owners may edit it.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[non_exhaustive]
pub struct NotificationPolicy {
    /// Whether the requesting user may edit this policy.
    #[serde(default)]
    pub editable: bool,
    /// Whether owners receive push alerts for every member's documents.
    #[serde(default)]
    pub owners_receive_all: bool,
    /// Workspace-wide event switches; identity-review e-mail is always sent.
    #[serde(default)]
    pub events: NotificationPolicyEvents,
}
/// Signing capabilities included in the workspace's current subscription.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[non_exhaustive]
pub struct SigningFeatures {
    /// ICP-Brasil certificate verification.
    #[serde(rename = "DigitalCertificate", default)]
    pub digital_certificate: bool,
    /// Identity document and face matching.
    #[serde(rename = "VerificationIdDocumentFaceMatchBasic", default)]
    pub id_document_face_match_basic: bool,
    /// WhatsApp notifications and verification.
    #[serde(rename = "WhatsappNotification", default)]
    pub whatsapp_notification: bool,
    /// Signing without a verification code.
    #[serde(rename = "VerificationBypass", default)]
    pub verification_bypass: bool,
    /// Embedded signing.
    #[serde(rename = "Embed", default)]
    pub embed: bool,
}

/// Remaining quantity and consumption for a document or credit pool.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[non_exhaustive]
pub struct Balance {
    /// Remaining quantity; `None` means unlimited.
    #[serde(default)]
    pub balance: Option<f64>,
    /// Cycle or pool allowance; `None` means unlimited or not cycle-limited.
    #[serde(default)]
    pub limit: Option<f64>,
    /// Quantity consumed in the current cycle.
    #[serde(default)]
    pub spent: f64,
}

/// Workspace document and credit pools.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[non_exhaustive]
pub struct AccountBalances {
    /// Document allowance and consumption.
    pub documents: Balance,
    /// Credit allowance and consumption.
    pub credits: Balance,
}

/// Workspace plan name, allowance-renewal date, and balances.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[non_exhaustive]
pub struct AccountUsage {
    /// Plan name; absent if there is no plan or it was retired.
    #[serde(default)]
    pub plan: Option<String>,
    /// Next document-allowance renewal date (`YYYY-MM-DD`), when applicable.
    #[serde(default)]
    pub documents_renewal_date: Option<String>,
    /// Document and credit balances.
    pub balances: AccountBalances,
}
