//! Account / workspace models.

use serde::{Deserialize, Serialize};

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
///   "id": "102d25a489f34a275d31a16045fd",
///   "name": "Acme Inc.",
///   "primary_color": null,
///   "secondary_color": null,
///   "created_at": "2026-05-12T18:05:11Z"
/// }
/// ```
#[derive(Debug, Clone, Serialize, Deserialize)]
#[non_exhaustive]
pub struct Account {
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
