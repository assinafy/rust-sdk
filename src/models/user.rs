//! User and login-result models.

use serde::{Deserialize, Serialize};

use super::account::Account;

/// Authenticated user profile as returned by `POST /login`.
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
            id: ua.id,
            name: ua.name,
            roles: ua.roles,
            is_delete_allowed: ua.is_delete_allowed,
            primary_color: None,
            secondary_color: None,
            created_at: Some(ua.created_at),
        }
    }
}

/// Payload returned by [`AuthApi::login`](crate::resources::AuthApi::login).
#[derive(Debug, Clone, Serialize, Deserialize)]
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
