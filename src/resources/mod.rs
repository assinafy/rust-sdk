//! API resource handles, one per logical area of the Assinafy API.
//!
//! Resource handles are created by calling the corresponding accessor on
//! [`Client`](crate::Client). They borrow from the client and are cheap to
//! create per call:
//!
//! ```no_run
//! # use assinafy::Client;
//! # async fn run() -> assinafy::Result<()> {
//! let client = Client::from_api_key("k")?;
//! let signers_api = client.signers("acc_123");
//! let _list = signers_api.list().send().await?;
//! # Ok(()) }
//! ```

mod accounts;
mod activities;
mod api_keys;
mod assignments;
mod auth;
mod documents;
mod fields;
mod oauth;
mod public;
mod signer_self;
mod signers;
mod tags;
mod templates;
mod users;
mod webhooks;

pub use accounts::{
    AccountApi, AccountsApi, CreateAccountBody, DocumentStatsQuery, NotificationSenderType,
    UpdateAccountBody, UploadLogoRequest,
};
pub use activities::{ActivitiesApi, ListActivitiesRequest};
pub use api_keys::{ApiKeyResponse, ApiKeysApi, CreateApiKeyBody};
pub use assignments::{
    AssignmentEntry, AssignmentField, AssignmentsApi, CreateAssignmentBody, CreateAssignmentSigner,
    EstimateAssignmentCostBody, IdentityReviewBody, IdentityReviewDecision, ListAssignmentsRequest,
};
pub use auth::{
    AuthApi, ChangePasswordBody, EmailResult, LinkSocialLoginBody, LoginBody,
    RequestPasswordResetBody, ResetPasswordBody, SocialLoginBody, VerifyMfaBody,
};
pub use documents::{
    DocumentsApi, ListDocumentsRequest, SearchDocumentsRequest, UploadDocumentRequest,
};
pub use fields::{
    CreateFieldBody, FieldsApi, ListFieldsRequest, UpdateFieldBody, ValidateFieldEntry,
};
pub use oauth::{
    AuthorizationRequest, OAuthApi, PkceChallenge, RevokeRequest, TokenRequest, scope,
};
pub use public::{PublicApi, SendTokenBody, SendTokenResult};
pub use signer_self::{
    ConfirmSignerDataBody, DeclineMultipleDocumentsBody, ListSignerDocumentsRequest,
    SignMultipleDocumentsBody, SignerSelfApi, VerifyCodeBody,
};
pub use signers::{CreateSignerBody, ListSignersRequest, SignersApi, UpdateSignerBody};
pub use tags::{CreateTagBody, ListTagsRequest, TagsApi, UpdateTagBody};
pub use templates::{
    CreateDocumentFromTemplateBody, CreateTemplateRequest, EditorField, EstimateTemplateCostBody,
    ListTemplatesRequest, TemplateDocumentSigner, TemplateRoleBinding, TemplatesApi,
};
pub use users::{ConfirmTotpBody, MfaReauthBody, UpdateNotificationPreferencesBody, UsersApi};
pub use webhooks::{
    ListWebhookDispatchesRequest, RegisterWebhookBody, WEBHOOK_TIMESTAMP_TOLERANCE,
    WebhookEndpointBody, WebhooksApi, verify_webhook_signature,
};
