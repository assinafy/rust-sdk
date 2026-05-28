//! Account / workspace model.

use serde::{Deserialize, Serialize};

/// A workspace / billing account that owns documents, signers, tags, etc.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[non_exhaustive]
pub struct Account {
    /// Account identifier.
    pub id: String,
    /// Display name.
    pub name: String,
    /// Roles held by the authenticated user (e.g. `"owner"`).
    #[serde(default)]
    pub roles: Vec<String>,
    /// Whether the user may delete this account.
    #[serde(default)]
    pub is_delete_allowed: bool,
    /// ISO-8601 creation timestamp.
    pub created_at: String,
}
