//! Field-definition models.

use serde::{Deserialize, Serialize};

/// A workspace-scoped field definition.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[non_exhaustive]
pub struct FieldDefinition {
    /// Resource discriminator, when returned by the API.
    #[serde(default)]
    pub resource: Option<String>,
    /// Field identifier.
    pub id: String,
    /// Human-readable field name.
    pub name: String,
    /// Field type, such as `text`, `email`, `signature`, or `initial`.
    #[serde(rename = "type")]
    pub kind: String,
    /// Optional validation regular expression.
    #[serde(default)]
    pub regex: Option<String>,
    /// Whether this field is predefined by the platform.
    #[serde(default)]
    pub is_pre_defined: Option<bool>,
    /// Whether this field is active.
    #[serde(default)]
    pub is_active: bool,
    /// Whether this field is required when filled.
    #[serde(default)]
    pub is_required: Option<bool>,
    /// Whether this is a standard built-in field.
    #[serde(default)]
    pub is_standard: Option<bool>,
    /// Whether this field is read-only.
    #[serde(default)]
    pub is_read_only: Option<bool>,
    /// Whether this field is visible to signers.
    #[serde(default)]
    pub is_visible: Option<bool>,
}

/// Description of a supported field type.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[non_exhaustive]
pub struct FieldType {
    /// Stable field type identifier.
    #[serde(rename = "type")]
    pub kind: String,
    /// Display name.
    pub name: String,
}

/// Result returned by field-value validation endpoints.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[non_exhaustive]
pub struct FieldValidationResult {
    /// Field type, when included.
    #[serde(default, rename = "type")]
    pub kind: Option<String>,
    /// Field definition identifier, when included.
    #[serde(default)]
    pub field_id: Option<String>,
    /// Whether the value passed validation.
    pub success: bool,
    /// Validation error message. Empty when validation succeeds.
    #[serde(default)]
    pub error_message: String,
}
