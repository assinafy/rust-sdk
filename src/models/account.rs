//! Account / workspace models.

use serde::{Deserialize, Serialize};

/// Who is shown as the sender of account signature-request notifications.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum NotificationSenderType {
    /// Notifications are sent on behalf of the individual user.
    User,
    /// Notifications are sent on behalf of the account/workspace.
    Account,
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
/// # Example payload (`GET /v1/accounts/{accountId}`)
///
/// ```json
/// {
///   "resource": "account",
///   "id": "acc_1234567890abcdef12345678",
///   "name": "Acme Inc.",
///   "primary_color": null,
///   "secondary_color": null,
///   "notification_sender_type": "User",
///   "roles": ["owner"],
///   "is_delete_allowed": true,
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
///   "account_name": "Acme Inc.",
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
///   "signature_requests_email": 55,
///   "signature_requests_whatsapp": 18,
///   "signature_requests_viewed": 44,
///   "signature_requests_completed": 52,
///   "documents_certified": 30
/// }
/// ```
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
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
    /// Signature requests sent by e-mail during the period.
    pub signature_requests_email: u64,
    /// Signature requests sent through WhatsApp during the period.
    pub signature_requests_whatsapp: u64,
    /// Signature requests whose document was first viewed during the period.
    pub signature_requests_viewed: u64,
    /// Signature requests completed by individual signers during the period.
    pub signature_requests_completed: u64,
    /// Documents certified during the period.
    pub documents_certified: u64,
}
