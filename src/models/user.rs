//! User and login-result models.

use std::fmt;

use serde::{Deserialize, Serialize};

use super::account::Account;

/// Authenticated user profile returned by `POST /login` and
/// `GET /v1/users/self`.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
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

/// Payload returned by [`AuthApi::login`](crate::resources::AuthApi::login),
/// [`AuthApi::social_login`](crate::resources::AuthApi::social_login) and
/// [`AuthApi::verify_mfa`](crate::resources::AuthApi::verify_mfa).
///
/// When the user has two-factor authentication enabled, login answers with an
/// `mfa_token` instead of an access token: `access_token` is then empty and the
/// token comes from [`AuthApi::verify_mfa`](crate::resources::AuthApi::verify_mfa).
#[derive(Clone, Serialize, Deserialize)]
#[non_exhaustive]
pub struct LoginResult {
    /// JWT bearer token. Pass to [`Client::with_auth`](crate::Client::with_auth)
    /// or [`ClientBuilder::bearer`](crate::ClientBuilder::bearer) to make
    /// authenticated requests. Empty when [`mfa_token`](Self::mfa_token) is set.
    #[serde(default)]
    pub access_token: String,
    /// Authenticated user.
    #[serde(default)]
    pub user: UserProfile,
    /// Accounts the user has access to.
    #[serde(default)]
    pub accounts: Vec<UserAccount>,
    /// Single-use two-factor challenge returned by login when the user has
    /// two-factor authentication enabled. Expires 5 minutes after login.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub mfa_token: Option<String>,
}

impl fmt::Debug for LoginResult {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("LoginResult")
            .field("access_token", &"**redacted**")
            .field("user", &self.user)
            .field("accounts", &self.accounts)
            .field(
                "mfa_token",
                &self.mfa_token.as_ref().map(|_| "**redacted**"),
            )
            .finish()
    }
}

/// Response `data` of `GET /users/self/mfa`: the user's enrolled two-factor
/// methods and how many recovery codes remain unused.
///
/// # Response payload
///
/// ```json
/// {
///   "methods": [
///     {
///       "id": "a1b2c3d4e5f60718293a4b5c6d7e8f90",
///       "type": "Totp",
///       "label": "My phone",
///       "confirmed_at": "2026-09-09T14:21:03Z",
///       "last_used_at": "2026-09-09T18:02:44Z"
///     }
///   ],
///   "recovery_codes_remaining": 10
/// }
/// ```
#[derive(Debug, Clone, Serialize, Deserialize)]
#[non_exhaustive]
pub struct MfaStatus {
    /// Enrolled methods.
    #[serde(default)]
    pub methods: Vec<MfaMethod>,
    /// Recovery codes still unused.
    #[serde(default)]
    pub recovery_codes_remaining: u32,
}

/// One enrolled two-factor method.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[non_exhaustive]
pub struct MfaMethod {
    /// Method ID, used by [`UsersApi::remove_mfa_method`](crate::resources::UsersApi::remove_mfa_method).
    pub id: String,
    /// Method type, e.g. `"Totp"`.
    #[serde(rename = "type")]
    pub kind: String,
    /// Label chosen at enrollment.
    #[serde(default)]
    pub label: Option<String>,
    /// When the enrollment was confirmed (ISO 8601).
    #[serde(default)]
    pub confirmed_at: Option<String>,
    /// When the method last completed a login (ISO 8601).
    #[serde(default)]
    pub last_used_at: Option<String>,
}

/// Response `data` of `POST /users/self/mfa/totp`: an unconfirmed
/// authenticator enrollment. The secret is returned only once. `Debug`
/// redacts `secret` and `provisioning_uri`.
///
/// # Response payload
///
/// ```json
/// {
///   "id": "a1b2c3d4e5f60718293a4b5c6d7e8f90",
///   "secret": "GEZDGNBVGY3TQOJQGEZDGNBVGY3TQOJQ",
///   "provisioning_uri": "otpauth://totp/user%40example.invalid?issuer=Assinafy&secret=GEZDGNBVGY3TQOJQGEZDGNBVGY3TQOJQ"
/// }
/// ```
#[derive(Clone, Serialize, Deserialize)]
#[non_exhaustive]
pub struct TotpEnrollment {
    /// Method ID to confirm with [`UsersApi::confirm_totp_enrollment`](crate::resources::UsersApi::confirm_totp_enrollment).
    pub id: String,
    /// Base32 shared secret for the authenticator app.
    pub secret: String,
    /// `otpauth://` URI, usually rendered as a QR code.
    pub provisioning_uri: String,
}

impl fmt::Debug for TotpEnrollment {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("TotpEnrollment")
            .field("id", &self.id)
            .field("secret", &"**redacted**")
            .field("provisioning_uri", &"**redacted**")
            .finish()
    }
}

/// Response `data` carrying freshly issued recovery codes, which the API shows
/// only once. `Debug` redacts the codes.
///
/// # Response payload
///
/// ```json
/// { "recovery_codes": ["ABCD-EFGH-JKMN", "PQRS-TUVW-XYZ2"] }
/// ```
#[derive(Clone, Serialize, Deserialize)]
#[non_exhaustive]
pub struct RecoveryCodes {
    /// One-time recovery codes.
    #[serde(default)]
    pub recovery_codes: Vec<String>,
}

impl fmt::Debug for RecoveryCodes {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("RecoveryCodes")
            .field(
                "recovery_codes",
                &format_args!("[{} redacted]", self.recovery_codes.len()),
            )
            .finish()
    }
}
