//! Tag model.

use serde::{Deserialize, Serialize};

/// A tag that can be applied to documents and templates.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[non_exhaustive]
pub struct Tag {
    /// Resource discriminator (always `"tag"` when present).
    #[serde(default)]
    pub resource: Option<String>,
    /// Tag identifier.
    pub id: String,
    /// Display name. Unique per workspace (case-insensitive).
    pub name: String,
    /// 6-character hex color (no leading `#`).
    #[serde(default)]
    pub color: Option<String>,
    /// ISO-8601 creation timestamp.
    #[serde(default)]
    pub created_at: Option<String>,
    /// ISO-8601 modification timestamp.
    #[serde(default)]
    pub updated_at: Option<String>,
}
