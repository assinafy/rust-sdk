//! User and login-result models.

use std::fmt;

use serde::{Deserialize, Serialize};

use super::account::Account;

/// Authenticated user profile returned by `POST /login` and
/// `GET /v1/users/self`.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[non_exhaustive]
pub struct UserProfile {
    /// Server-assigned identifier.
    pub id: String,
    /// Display name.
    pub name: String,
    /// Email address.
    pub email: String,
    /// Optional phone number.
    #[serde(default)]
    pub telephone: Option<String>,
    /// Optional government ID (e.g. CPF/CNPJ).
    #[serde(default)]
    pub government_id: Option<String>,
    /// Whether the user verified their email.
    #[serde(default)]
    pub is_email_verified: bool,
    /// Whether the user has accepted the terms of use.
    #[serde(default)]
    pub has_accepted_terms: bool,
    /// Whether the user has a password set (as opposed to social login only).
    #[serde(default)]
    pub is_password_set: bool,
    /// ISO-8601 creation timestamp.
    pub created_at: String,
    /// If non-null, the date the user requested account deletion.
    #[serde(default)]
    pub to_be_deleted_at: Option<String>,
}

/// Account membership emitted by `POST /login` alongside the user profile.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[non_exhaustive]
pub struct UserAccount {
    /// Account identifier.
    pub id: String,
    /// Account name.
    pub name: String,
    /// Roles the authenticated user holds on this account (e.g. `["owner"]`).
    #[serde(default)]
    pub roles: Vec<String>,
    /// Whether the user may delete this account.
    #[serde(default)]
    pub is_delete_allowed: bool,
    /// ISO-8601 creation timestamp.
    pub created_at: String,
}

impl From<UserAccount> for Account {
    fn from(ua: UserAccount) -> Self {
        Account {
            resource: None,
            id: ua.id,
            name: ua.name,
            roles: ua.roles,
            is_delete_allowed: ua.is_delete_allowed,
            primary_color: None,
            secondary_color: None,
            notification_sender_type: None,
            created_at: Some(ua.created_at),
        }
    }
}

/// Compatibility data shape accepted by
/// [`UsersApi::me`](crate::resources::UsersApi::me).
///
/// This type remains public for callers that decode the wrapped [`SelfUser`]
/// shape; `UsersApi::me` normalizes direct and wrapped profiles to
/// `UserProfile`.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[non_exhaustive]
pub struct SelfUser {
    /// Authenticated user.
    pub user: UserProfile,
    /// Accounts the user has access to.
    #[serde(default)]
    pub accounts: Vec<UserAccount>,
}

/// Owner-facing document e-mail preferences for the authenticated user.
///
/// Returned in full by both notification-preference endpoints. `true` means
/// the corresponding e-mail is enabled across every account the user belongs
/// to. Account and security e-mails are not configurable and are not included.
///
/// # Example payload
///
/// ```json
/// {
///   "DocumentCompleted": true,
///   "SignerDeclined": true,
///   "DocumentCancelled": true,
///   "DocumentAboutToExpire": true,
///   "DocumentExpired": true,
///   "DocumentExpirationReset": true,
///   "DocumentProcessingFailed": true,
///   "TemplateProcessingFailed": true,
///   "SignerWhatsappFailed": true
/// }
/// ```
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[non_exhaustive]
pub struct NotificationPreferences {
    /// Every signer has signed and the document is certified.
    #[serde(rename = "DocumentCompleted")]
    pub document_completed: bool,
    /// A signer declined to sign.
    #[serde(rename = "SignerDeclined")]
    pub signer_declined: bool,
    /// The document was cancelled.
    #[serde(rename = "DocumentCancelled")]
    pub document_cancelled: bool,
    /// The signature deadline is approaching.
    #[serde(rename = "DocumentAboutToExpire")]
    pub document_about_to_expire: bool,
    /// The signature deadline passed.
    #[serde(rename = "DocumentExpired")]
    pub document_expired: bool,
    /// The signature deadline was extended.
    #[serde(rename = "DocumentExpirationReset")]
    pub document_expiration_reset: bool,
    /// An uploaded document could not be processed.
    #[serde(rename = "DocumentProcessingFailed")]
    pub document_processing_failed: bool,
    /// A template could not be processed.
    #[serde(rename = "TemplateProcessingFailed")]
    pub template_processing_failed: bool,
    /// A WhatsApp notification to a signer could not be delivered.
    #[serde(rename = "SignerWhatsappFailed")]
    pub signer_whatsapp_failed: bool,
}

/// Payload returned by [`AuthApi::login`](crate::resources::AuthApi::login).
#[derive(Clone, Serialize, Deserialize)]
#[non_exhaustive]
pub struct LoginResult {
    /// JWT bearer token. Pass to [`Client::with_auth`](crate::Client::with_auth)
    /// or [`ClientBuilder::bearer`](crate::ClientBuilder::bearer) to make
    /// authenticated requests.
    pub access_token: String,
    /// Authenticated user.
    pub user: UserProfile,
    /// Accounts the user has access to.
    #[serde(default)]
    pub accounts: Vec<UserAccount>,
}

impl fmt::Debug for LoginResult {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("LoginResult")
            .field("access_token", &"**redacted**")
            .field("user", &self.user)
            .field("accounts", &self.accounts)
            .finish()
    }
}
