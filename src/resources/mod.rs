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

mod activities;
mod api_keys;
mod assignments;
mod auth;
mod documents;
mod fields;
mod public;
mod signer_self;
mod signers;
mod tags;
mod templates;
mod webhooks;

pub use activities::ActivitiesApi;
pub use api_keys::{ApiKeyResponse, ApiKeysApi, CreateApiKeyBody};
pub use assignments::{
    AssignmentEntry, AssignmentField, AssignmentsApi, CreateAssignmentBody, CreateAssignmentSigner,
    EstimateAssignmentCostBody,
};
pub use auth::{
    AuthApi, ChangePasswordBody, LoginBody, RequestPasswordResetBody, ResetPasswordBody,
    SocialLoginBody,
};
pub use documents::{DocumentsApi, ListDocumentsRequest, UploadDocumentRequest};
pub use fields::{
    CreateFieldBody, FieldsApi, ListFieldsRequest, UpdateFieldBody, ValidateFieldEntry,
};
pub use public::{PublicApi, SendTokenBody, SendTokenResult};
pub use signer_self::{
    ConfirmSignerDataBody, DeclineMultipleDocumentsBody, ListSignerDocumentsRequest,
    SignMultipleDocumentsBody, SignerSelfApi, VerifyCodeBody,
};
pub use signers::{CreateSignerBody, ListSignersRequest, SignersApi, UpdateSignerBody};
pub use tags::{CreateTagBody, ListTagsRequest, TagsApi, UpdateTagBody};
pub use templates::{
    CreateDocumentFromTemplateBody, EditorField, EstimateTemplateCostBody, ListTemplatesRequest,
    TemplateDocumentSigner, TemplateRoleBinding, TemplatesApi,
};
pub use webhooks::{ListWebhookDispatchesRequest, RegisterWebhookBody, WebhooksApi};
