//! Template models.

use std::fmt;

use serde::{Deserialize, Serialize};

use super::tag::Tag;

/// Template lifecycle status.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum TemplateStatus {
    /// Upload in progress.
    #[serde(alias = "Uploading")]
    Uploading,
    /// Upload complete; processing not started.
    #[serde(alias = "Uploaded")]
    Uploaded,
    /// Server-side processing in progress.
    #[serde(alias = "Processing")]
    Processing,
    /// Ready to instantiate documents from.
    #[serde(alias = "Ready")]
    Ready,
    /// Processing failed.
    #[serde(alias = "Failed")]
    Failed,
    /// Any value the SDK does not yet model.
    #[serde(untagged)]
    Unknown(String),
}

impl fmt::Display for TemplateStatus {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let s = match self {
            TemplateStatus::Uploading => "uploading",
            TemplateStatus::Uploaded => "uploaded",
            TemplateStatus::Processing => "processing",
            TemplateStatus::Ready => "ready",
            TemplateStatus::Failed => "failed",
            TemplateStatus::Unknown(s) => s.as_str(),
        };
        f.write_str(s)
    }
}

/// A reusable document template.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[non_exhaustive]
pub struct Template {
    /// Resource discriminator (always `"template"` when present).
    #[serde(default)]
    pub resource: Option<String>,
    /// Template identifier.
    pub id: String,
    /// Template (PDF) filename.
    pub name: String,
    /// Default document name when creating a document from this template.
    #[serde(default)]
    pub document_name: Option<String>,
    /// Default invitation message.
    #[serde(default)]
    pub message: Option<String>,
    /// Lifecycle status.
    pub status: TemplateStatus,
    /// Pages within the template.
    #[serde(default)]
    pub pages: Vec<TemplatePage>,
    /// Roles defined on the template.
    #[serde(default)]
    pub roles: Vec<TemplateRole>,
    /// Tags attached to the template itself.
    #[serde(default)]
    pub tags: Vec<Tag>,
    /// Tags that will be automatically applied to documents created from this
    /// template. Only returned by the single-template endpoint.
    #[serde(default)]
    pub default_document_tags: Vec<TemplateTagRef>,
    /// Creation timestamp.
    #[serde(default)]
    pub created_at: Option<String>,
    /// Last modification timestamp.
    #[serde(default)]
    pub updated_at: Option<String>,
}

/// A page within a template.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[non_exhaustive]
pub struct TemplatePage {
    /// Page identifier.
    pub id: String,
    /// 1-based page number.
    pub number: u32,
    /// Page height in pixels.
    pub height: u32,
    /// Page width in pixels.
    pub width: u32,
    /// Pre-signed download URL.
    #[serde(default)]
    pub download_url: Option<String>,
    /// Field placements on this page.
    #[serde(default)]
    pub fields: Vec<TemplateField>,
}

/// A field placement on a template page.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[non_exhaustive]
pub struct TemplateField {
    /// Placement identifier.
    pub id: String,
    /// Field type identifier (signature, initial, text, etc.).
    pub field_id: String,
    /// Role identifier assigned to this field.
    pub role_id: String,
    /// Field label.
    pub label: String,
    /// Free-form display settings.
    #[serde(default)]
    pub display_settings: Option<serde_json::Value>,
    /// Creation timestamp.
    #[serde(default)]
    pub created_at: Option<String>,
    /// Last modification timestamp.
    #[serde(default)]
    pub updated_at: Option<String>,
}

/// A role defined on a template.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[non_exhaustive]
pub struct TemplateRole {
    /// Role identifier.
    pub id: String,
    /// Display name.
    pub name: String,
    /// Assignment type (`"Editor"`, etc.).
    pub assignment_type: String,
    /// Creation timestamp.
    #[serde(default)]
    pub created_at: Option<String>,
    /// Last modification timestamp.
    #[serde(default)]
    pub updated_at: Option<String>,
}

/// Reference to a tag included in a template's default document tags.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[non_exhaustive]
pub struct TemplateTagRef {
    /// Tag identifier.
    pub id: String,
    /// Tag name.
    pub name: String,
    /// Tag color, when included by the API.
    #[serde(default)]
    pub color: Option<String>,
}
