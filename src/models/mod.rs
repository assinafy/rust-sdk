//! Strongly-typed representations of every resource the API returns.
//!
//! Every model derives [`Serialize`](serde::Serialize) and
//! [`Deserialize`](serde::Deserialize) and enumerates every known field
//! explicitly (rather than a generic map); most structs are also marked
//! `#[non_exhaustive]` and use `#[serde(default)]` on optional fields, so
//! new server fields can't break downstream matches or struct literals.
//!
//! Where the API returns a discriminated string, an enum is provided
//! (e.g. [`DocumentStatus`]). Any future variant the server adds is
//! preserved rather than causing a hard parse failure — the catch-all
//! variant is spelled `Unknown(String)` on some enums (e.g.
//! [`DocumentStatus`], [`TemplateStatus`]) and `Other(String)` on others
//! (e.g. [`ArtifactName`], [`BlockingReason`]); check the individual enum's
//! docs for its exact spelling.

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

pub use account::{Account, AccountTheme, DocumentStatsRow, NotificationSenderType};
pub use activity::{Activity, ActivityOrigin};
pub use assignment::{
    Assignment, AssignmentItem, AssignmentItemField, AssignmentItemSigner, AssignmentMethod,
    AssignmentSigner, AssignmentSummary, AssignmentSummarySigner, CopyReceiver, NotificationEvent,
    NotificationHistoryEntry, NotificationMethod, NotificationStatus, ResendCostBreakdownItem,
    ResendCostEstimate, ResendNotificationResult, SignDocumentItem, SigningUrl, VerificationMethod,
    WhatsAppButton, WhatsAppNotification,
};
pub use cost::{BlockingReason, CostBreakdownItem, CostEstimate};
pub use document::{
    ArtifactName, DeclinedBy, Document, DocumentPage, DocumentStatus, DocumentStatusInfo,
    DocumentVerification, PublicDocument,
};
pub use field::{FieldDefinition, FieldType, FieldValidationResult};
pub use signer::{Signer, SignerSelf, SignerType};
pub use tag::Tag;
pub use template::{
    Template, TemplateField, TemplatePage, TemplateRole, TemplateStatus, TemplateTagRef,
};
pub use user::{LoginResult, NotificationPreferences, SelfUser, UserAccount, UserProfile};
pub use webhook::{WebhookDispatch, WebhookEventTypeInfo, WebhookSubscription};
