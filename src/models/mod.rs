//! Strongly-typed representations of every resource the API returns.
//!
//! Every model derives [`Serialize`](serde::Serialize) and
//! [`Deserialize`](serde::Deserialize); structs are exhaustive but use
//! `#[serde(default)]` on optional fields so that the API may add new fields
//! without breaking deserialization.
//!
//! Where the API returns a discriminated string, an enum is provided
//! (e.g. [`DocumentStatus`]). Any future variant the
//! server adds is preserved as the `Unknown(String)` enum variant rather than
//! causing a hard parse failure.

pub mod account;
pub mod activity;
pub mod assignment;
pub mod cost;
pub mod document;
pub mod field;
pub mod signer;
pub mod tag;
pub mod template;
pub mod user;
pub mod webhook;

pub use account::Account;
pub use activity::{Activity, ActivityOrigin};
pub use assignment::{
    Assignment, AssignmentItem, AssignmentMethod, AssignmentSigner, AssignmentSummary,
    AssignmentSummarySigner, CopyReceiver, NotificationEvent, NotificationHistoryEntry,
    NotificationMethod, NotificationStatus, ResendCostBreakdownItem, ResendCostEstimate,
    ResendNotificationResult, SignDocumentItem, SigningUrl, VerificationMethod, WhatsAppButton,
    WhatsAppNotification,
};
pub use cost::{BlockingReason, CostBreakdownItem, CostEstimate};
pub use document::{
    Artifact, ArtifactName, DeclinedBy, Document, DocumentPage, DocumentStatus, DocumentStatusInfo,
    DocumentVerification, PublicDocument,
};
pub use field::{FieldDefinition, FieldType, FieldValidationResult};
pub use signer::{Signer, SignerSelf, SignerType};
pub use tag::Tag;
pub use template::{
    Template, TemplateField, TemplatePage, TemplateRole, TemplateStatus, TemplateTagRef,
};
pub use user::{LoginResult, UserAccount, UserProfile};
pub use webhook::{WebhookDispatch, WebhookEventTypeInfo, WebhookSubscription};
