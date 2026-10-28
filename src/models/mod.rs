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
pub mod contract;
pub mod cost;
pub mod document;
pub mod field;
pub mod identity;
pub mod oauth;
pub mod signer;
pub mod tag;
pub mod template;
pub mod user;
pub mod webhook;

pub use account::{Account, AccountLimits, AccountTheme, DocumentStatsRow, NotificationSenderType};
pub use activity::{Activity, ActivityOrigin};
pub use assignment::{
    Assignment, AssignmentItem, AssignmentItemField, AssignmentItemSigner, AssignmentMethod,
    AssignmentSigner, AssignmentSummary, AssignmentSummarySigner, CopyReceiver, NotificationEvent,
    NotificationHistoryEntry, NotificationMethod, NotificationStatus, ResendCostBreakdownItem,
    ResendCostEstimate, ResendNotificationResult, SignDocumentItem, SigningUrl, VerificationMethod,
    WhatsAppButton, WhatsAppNotification,
};
pub use contract::{
    ContractAmendmentStatus, ContractExternalReference, ContractRelationType, ContractStatus,
    DocumentContract, DocumentContractAmendment, DocumentContractDetails,
};
pub use cost::{BlockingReason, CostBreakdownItem, CostEstimate};
pub use document::{
    ArtifactName, DeclinedBy, Document, DocumentPage, DocumentStatus, DocumentStatusInfo,
    DocumentVerification, PublicDocument,
};
pub use field::{FieldDefinition, FieldType, FieldValidationResult};
pub use identity::{
    IdentityReviewCheck, IdentityReviewDocument, IdentityReviewExpected, IdentityReviewMatches,
    IdentityReviewReason, IdentityReviewer, IdentityVerification, IdentityVerificationSession,
    IdentityVerificationStatus, SignerIdentityReview,
};
pub use oauth::{AuthorizationServerMetadata, ProtectedResourceMetadata, TokenResponse, UserInfo};
pub use signer::{Signer, SignerSelf, SignerType};
pub use tag::Tag;
pub use template::{
    Template, TemplateField, TemplatePage, TemplateRole, TemplateStatus, TemplateTagRef,
};
pub use user::{
    LoginResult, MfaMethod, MfaStatus, NotificationPreferences, RecoveryCodes, SelfUser,
    TotpEnrollment, UserAccount, UserProfile,
};
pub use webhook::{
    WebhookDispatch, WebhookDispatchKind, WebhookEndpoint, WebhookEndpointSecret, WebhookEvent,
    WebhookEventTypeInfo, WebhookSubscription,
};
