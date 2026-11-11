//! Account / workspace endpoints.
//!
//! Two handles cover this surface:
//!
//! * [`AccountsApi`] — account-level operations (`client.accounts_api()`):
//!   list the accounts the credential can see and create new accounts.
//! * [`AccountApi`] — operations scoped to a single account
//!   (`client.account(account_id)`): fetch, update, delete, theme, and the
//!   account logo, and document statistics.

use bytes::Bytes;
use reqwest::Method;
use reqwest::multipart::{Form, Part};
use serde::{Deserialize, Serialize};

use crate::error::{Error, Result};
use crate::http::HttpClient;
use crate::models::{Account, AccountTheme, DocumentStatsRow};

pub use crate::models::NotificationSenderType;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "lowercase")]
enum DocumentStatsGranularity {
    Monthly,
    Daily,
}

/// Query for an account or authenticated-user document statistics series.
///
/// Use [`monthly`](Self::monthly) for the last 12 zero-filled months, most
/// recent first, or [`daily`](Self::daily) for every zero-filled day in one
/// month. The daily constructor validates the API's required `YYYY-MM` value
/// before any request is sent.
///
/// # Monthly request parameters
///
/// ```json
/// { "granularity": "monthly" }
/// ```
///
/// # Daily request parameters
///
/// ```json
/// { "granularity": "daily", "month": "2026-06" }
/// ```
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct DocumentStatsQuery {
    granularity: DocumentStatsGranularity,
    #[serde(skip_serializing_if = "Option::is_none")]
    month: Option<String>,
}

impl DocumentStatsQuery {
    /// Request the last 12 monthly rows, most recent first.
    pub fn monthly() -> Self {
        Self {
            granularity: DocumentStatsGranularity::Monthly,
            month: None,
        }
    }

    /// Request all daily rows in `month`.
    ///
    /// Returns [`Error::Config`] unless `month` is exactly `YYYY-MM` with a
    /// numeric year and a month between `01` and `12`.
    pub fn daily(month: impl Into<String>) -> Result<Self> {
        let month = month.into();
        if !valid_stats_month(&month) {
            return Err(Error::Config(
                "document statistics month must use YYYY-MM with a month from 01 to 12".to_owned(),
            ));
        }

        Ok(Self {
            granularity: DocumentStatsGranularity::Daily,
            month: Some(month),
        })
    }
}

impl Default for DocumentStatsQuery {
    fn default() -> Self {
        Self::monthly()
    }
}

fn valid_stats_month(value: &str) -> bool {
    let bytes = value.as_bytes();
    bytes.len() == 7
        && bytes[4] == b'-'
        && bytes[..4].iter().all(u8::is_ascii_digit)
        && bytes[5..].iter().all(u8::is_ascii_digit)
        && (1..=12).contains(&((bytes[5] - b'0') * 10 + bytes[6] - b'0'))
}

/// Body for `POST /accounts` (create account).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CreateAccountBody {
    /// Display name for the new workspace account.
    pub name: String,
    /// Who appears as the notification sender. Defaults to `User` server-side
    /// when omitted.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub notification_sender_type: Option<NotificationSenderType>,
}

impl CreateAccountBody {
    /// Build a create-account request with just the required name.
    pub fn new<S: Into<String>>(name: S) -> Self {
        Self {
            name: name.into(),
            notification_sender_type: None,
        }
    }

    /// Set the notification sender type.
    pub fn notification_sender_type(mut self, kind: NotificationSenderType) -> Self {
        self.notification_sender_type = Some(kind);
        self
    }
}

/// Body for `PUT /accounts/{account_id}` (update account). Omitted fields are
/// left unchanged server-side.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct UpdateAccountBody {
    /// New display name.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub name: Option<String>,
    /// New notification sender type.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub notification_sender_type: Option<NotificationSenderType>,
}

impl UpdateAccountBody {
    /// New empty update body.
    pub fn new() -> Self {
        Self::default()
    }

    /// Set the display name.
    pub fn name<S: Into<String>>(mut self, name: S) -> Self {
        self.name = Some(name.into());
        self
    }

    /// Set the notification sender type.
    pub fn notification_sender_type(mut self, kind: NotificationSenderType) -> Self {
        self.notification_sender_type = Some(kind);
        self
    }
}

/// Multipart body for uploading an account logo (`POST /accounts/{id}/logo`).
pub struct UploadLogoRequest {
    filename: String,
    mime: String,
    bytes: Bytes,
}

impl UploadLogoRequest {
    /// Construct from in-memory image bytes (PNG or JPEG).
    pub fn from_bytes<S: Into<String>>(filename: S, bytes: impl Into<Bytes>) -> Self {
        Self {
            filename: filename.into(),
            mime: "image/png".to_string(),
            bytes: bytes.into(),
        }
    }

    /// Construct by reading a local image file.
    pub async fn from_path<P: AsRef<std::path::Path>>(path: P) -> Result<Self> {
        let p = path.as_ref();
        let filename = p
            .file_name()
            .and_then(|n| n.to_str())
            .ok_or_else(|| Error::Config(format!("invalid filename: {}", p.display())))?
            .to_owned();
        let bytes = tokio::fs::read(p).await?;
        Ok(Self::from_bytes(filename, bytes))
    }

    /// Override the MIME type (defaults to `image/png`).
    pub fn content_type<S: Into<String>>(mut self, mime: S) -> Self {
        self.mime = mime.into();
        self
    }

    fn into_form(self) -> Result<Form> {
        let part = Part::stream(self.bytes)
            .file_name(self.filename)
            .mime_str(&self.mime)
            .map_err(|e| Error::Config(format!("invalid mime `{}`: {e}", self.mime)))?;
        Ok(Form::new().part("file", part))
    }
}

/// Account-level endpoints (not scoped to a single account).
///
/// Obtain one with [`Client::accounts_api`](crate::Client::accounts_api).
#[derive(Debug)]
pub struct AccountsApi<'a> {
    http: &'a HttpClient,
}

impl<'a> AccountsApi<'a> {
    pub(crate) fn new(http: &'a HttpClient) -> Self {
        Self { http }
    }

    /// List the accounts the authenticated user belongs to.
    ///
    /// `GET /accounts`. Returns a flat array (this endpoint is not paginated).
    ///
    /// # Request payload
    ///
    /// No request body or query parameters. The credential determines which
    /// accounts are visible.
    ///
    /// # Response payload
    ///
    /// ```json
    /// { "status": 200, "message": "", "data": [
    ///   {
    ///     "id": "102d25a4a1b2c3d4e5f60718",
    ///     "name": "Example Workspace",
    ///     "roles": ["owner"],
    ///     "is_delete_allowed": true,
    ///     "created_at": "2026-05-12T18:05:11Z"
    ///   }
    /// ]}
    /// ```
    pub async fn list(&self) -> Result<Vec<Account>> {
        let req = self.http.request(Method::GET, "accounts")?;
        self.http.send_envelope(req).await
    }

    /// Create a new workspace account owned by the authenticated user.
    ///
    /// `POST /accounts`.
    ///
    /// # Request payload
    ///
    /// ```json
    /// { "name": "Example Workspace", "notification_sender_type": "Account" }
    /// ```
    ///
    /// # Response payload
    ///
    /// ```json
    /// { "status": 200, "message": "", "data": {
    ///   "resource": "account",
    ///   "id": "102d25a4a1b2c3d4e5f60718",
    ///   "name": "Example Workspace",
    ///   "primary_color": null,
    ///   "secondary_color": null,
    ///   "notification_sender_type": "Account",
    ///   "roles": ["owner"],
    ///   "is_delete_allowed": true,
    ///   "created_at": "2026-05-12T18:05:11Z"
    /// } }
    /// ```
    pub async fn create(&self, body: &CreateAccountBody) -> Result<Account> {
        let req = self.http.request(Method::POST, "accounts")?.json(body);
        self.http.send_envelope(req).await
    }
}

/// Endpoints scoped to a single account.
///
/// Obtain one with [`Client::account`](crate::Client::account).
#[derive(Debug)]
pub struct AccountApi<'a> {
    http: &'a HttpClient,
    account_id: String,
}

impl<'a> AccountApi<'a> {
    pub(crate) fn new(http: &'a HttpClient, account_id: String) -> Self {
        Self { http, account_id }
    }

    /// Retrieve this account.
    ///
    /// `GET /accounts/{account_id}`.
    ///
    /// # Request payload
    ///
    /// No request body. Resource identifiers are supplied in the route.
    ///
    /// # Response payload
    ///
    /// ```json
    /// { "status": 200, "message": "", "data": {
    ///   "resource": "account",
    ///   "id": "102d25a4a1b2c3d4e5f60718", "name": "Example Workspace",
    ///   "primary_color": null, "secondary_color": null,
    ///   "notification_sender_type": "User",
    ///   "limits": { "webhook_endpoints": 1 },
    ///   "created_at": "2026-05-12T18:05:11Z" } }
    /// ```
    pub async fn get(&self) -> Result<Account> {
        let path = self.http.path(&["accounts", self.account_id.as_str()])?;
        let req = self.http.request(Method::GET, &path)?;
        self.http.send_envelope(req).await
    }

    /// Update this account's profile.
    ///
    /// `PUT /accounts/{account_id}`.
    ///
    /// # Request payload
    ///
    /// ```json
    /// { "name": "New name", "notification_sender_type": "User" }
    /// ```
    ///
    /// # Response payload
    ///
    /// ```json
    /// { "status": 200, "message": "", "data": {
    ///   "resource": "account",
    ///   "id": "102d25a4a1b2c3d4e5f60718", "name": "New name",
    ///   "primary_color": null, "secondary_color": null,
    ///   "notification_sender_type": "User",
    ///   "limits": { "webhook_endpoints": 1 },
    ///   "created_at": "2026-05-12T18:05:11Z" } }
    /// ```
    pub async fn update(&self, body: &UpdateAccountBody) -> Result<Account> {
        let path = self.http.path(&["accounts", self.account_id.as_str()])?;
        let req = self.http.request(Method::PUT, &path)?.json(body);
        self.http.send_envelope(req).await
    }

    /// Delete this account.
    ///
    /// `DELETE /accounts/{account_id}`. By default this fails with `400` when
    /// the workspace still has an active paid subscription; the error's `data`
    /// lists the blockers. Use [`delete_forcing`](Self::delete_forcing) to
    /// cancel any active subscription and delete immediately.
    ///
    /// # Request payload
    ///
    /// No request body. Resource identifiers are supplied in the route.
    ///
    /// # Response payload
    ///
    /// ```json
    /// { "status": 200, "message": "", "data": [] }
    /// ```
    pub async fn delete(&self) -> Result<()> {
        self.delete_inner(false).await
    }

    /// Delete this account, cancelling any active paid subscription first.
    ///
    /// `DELETE /accounts/{account_id}` with body `{ "force": true }`.
    ///
    /// # Request payload
    ///
    /// ```json
    /// { "force": true }
    /// ```
    ///
    /// # Response payload
    ///
    /// ```json
    /// { "status": 200, "message": "", "data": [] }
    /// ```
    pub async fn delete_forcing(&self) -> Result<()> {
        self.delete_inner(true).await
    }

    async fn delete_inner(&self, force: bool) -> Result<()> {
        let path = self.http.path(&["accounts", self.account_id.as_str()])?;
        let mut req = self.http.request(Method::DELETE, &path)?;
        if force {
            req = req.json(&serde_json::json!({ "force": true }));
        }
        self.http.send_no_content(req).await
    }

    /// Retrieve the account theme (branding name, colors, and logo URL).
    ///
    /// `GET /accounts/{account_id}/theme`.
    ///
    /// # Request payload
    ///
    /// No request body. Resource identifiers are supplied in the route.
    ///
    /// # Response payload
    ///
    /// ```json
    /// { "status": 200, "message": "", "data": {
    ///   "account_name": "Example Workspace", "primary_color": "2072b9",
    ///   "secondary_color": "ffffff", "logo": null } }
    /// ```
    pub async fn theme(&self) -> Result<AccountTheme> {
        let path = self
            .http
            .path(&["accounts", self.account_id.as_str(), "theme"])?;
        let req = self.http.request(Method::GET, &path)?;
        self.http.send_envelope(req).await
    }

    /// Retrieve this account's precomputed document-funnel statistics.
    ///
    /// `GET /accounts/{account_id}/stats`. Monthly queries return the last 12
    /// months, most recent first. Daily queries return every day in the
    /// requested month. The API zero-fills both series. This endpoint is
    /// available on production only.
    ///
    /// # Request parameters
    ///
    /// ```json
    /// { "granularity": "daily", "month": "2026-06" }
    /// ```
    ///
    /// # Request payload
    ///
    /// No request body. Resource identifiers are supplied in the route. Query
    /// parameters are URL-encoded; only configured filters are sent.
    ///
    /// ```json
    /// { "granularity": "daily", "month": "2026-06" }
    /// ```
    ///
    /// # Response payload
    ///
    /// ```json
    /// { "status": 200, "message": "", "data": [{
    ///   "period": "2026-06-01",
    ///   "documents_uploaded": 4,
    ///   "documents_sent": 3,
    ///   "signature_requests": 6,
    ///   "signature_requests_notification_email": 5,
    ///   "signature_requests_notification_whatsapp": 1,
    ///   "signature_requests_notification_bypass": 0,
    ///   "signature_requests_verification_email": 5,
    ///   "signature_requests_verification_whatsapp": 1,
    ///   "signature_requests_verification_bypass": 0,
    ///   "signature_requests_verification_digital_certificate": 0,
    ///   "signature_requests_viewed": 4,
    ///   "signature_requests_completed": 5,
    ///   "documents_certified": 3
    /// }] }
    /// ```
    pub async fn stats(&self, query: &DocumentStatsQuery) -> Result<Vec<DocumentStatsRow>> {
        let path = self
            .http
            .path(&["accounts", self.account_id.as_str(), "stats"])?;
        let req = self.http.request(Method::GET, &path)?.query(query);
        self.http.send_envelope(req).await
    }

    /// Download the account logo image bytes and its content type.
    ///
    /// `GET /accounts/{account_id}/logo`. Returns an [`Error::Api`] with status
    /// `404` when no logo has been uploaded.
    ///
    /// # Response
    ///
    /// The response is the raw logo image (`image/png` or `image/jpeg`), not a
    /// JSON envelope. The returned tuple contains the bytes and the response
    /// `Content-Type` header.
    ///
    /// # Request payload
    ///
    /// No request body. Resource identifiers are supplied in the route.
    ///
    /// # Response payload
    ///
    /// Raw binary bytes; the response is not a JSON envelope. The returned tuple
    /// contains the bytes and the server's `Content-Type` (or
    /// `application/octet-stream` when absent).
    ///
    /// ```text
    /// HTTP/1.1 200 OK
    /// Content-Type: image/png
    ///
    /// <artifact bytes>
    /// ```
    pub async fn download_logo(&self) -> Result<(Bytes, String)> {
        let path = self
            .http
            .path(&["accounts", self.account_id.as_str(), "logo"])?;
        let req = self.http.request(Method::GET, &path)?;
        self.http.send_download(req).await
    }

    /// Upload or replace the account logo image.
    ///
    /// `POST /accounts/{account_id}/logo` (multipart/form-data, `file` part).
    ///
    /// ```no_run
    /// # use assinafy::{Client, resources::UploadLogoRequest};
    /// # async fn run(client: Client) -> assinafy::Result<()> {
    /// let logo = UploadLogoRequest::from_path("logo.png").await?;
    /// client.account("acc_123").upload_logo(logo).await?;
    /// # Ok(()) }
    /// ```
    ///
    /// # Request payload
    ///
    /// Multipart form (`multipart/form-data`) with a binary `file` part.
    ///
    /// ```text
    /// Content-Disposition: form-data; name="file"; filename="example.png"
    /// Content-Type: image/png
    ///
    /// <file bytes>
    /// ```
    ///
    /// # Response payload
    ///
    /// ```json
    /// { "status": 200, "message": "" }
    /// ```
    pub async fn upload_logo(&self, logo: UploadLogoRequest) -> Result<()> {
        let path = self
            .http
            .path(&["accounts", self.account_id.as_str(), "logo"])?;
        let form = logo.into_form()?;
        let req = self.http.request(Method::POST, &path)?.multipart(form);
        self.http.send_no_content(req).await
    }

    /// Delete the account logo image.
    ///
    /// `DELETE /accounts/{account_id}/logo`.
    ///
    /// # Request payload
    ///
    /// No request body. Resource identifiers are supplied in the route.
    ///
    /// # Response payload
    ///
    /// ```json
    /// { "status": 200, "message": "" }
    /// ```
    pub async fn delete_logo(&self) -> Result<()> {
        let path = self
            .http
            .path(&["accounts", self.account_id.as_str(), "logo"])?;
        let req = self.http.request(Method::DELETE, &path)?;
        self.http.send_no_content(req).await
    }
}

/// Partial workspace event policy. Omitted keys remain unchanged.
#[derive(Debug, Clone, Default, Serialize)]
pub struct UpdateNotificationPolicyEvents {
    #[serde(rename = "DocumentCompleted", skip_serializing_if = "Option::is_none")]
    document_completed: Option<bool>,
    #[serde(rename = "SignerDeclined", skip_serializing_if = "Option::is_none")]
    signer_declined: Option<bool>,
    #[serde(rename = "DocumentCancelled", skip_serializing_if = "Option::is_none")]
    document_cancelled: Option<bool>,
    #[serde(
        rename = "DocumentAboutToExpire",
        skip_serializing_if = "Option::is_none"
    )]
    document_about_to_expire: Option<bool>,
    #[serde(rename = "DocumentExpired", skip_serializing_if = "Option::is_none")]
    document_expired: Option<bool>,
    #[serde(
        rename = "DocumentExpirationReset",
        skip_serializing_if = "Option::is_none"
    )]
    document_expiration_reset: Option<bool>,
    #[serde(
        rename = "DocumentProcessingFailed",
        skip_serializing_if = "Option::is_none"
    )]
    document_processing_failed: Option<bool>,
    #[serde(
        rename = "TemplateProcessingFailed",
        skip_serializing_if = "Option::is_none"
    )]
    template_processing_failed: Option<bool>,
    #[serde(
        rename = "SignerWhatsappFailed",
        skip_serializing_if = "Option::is_none"
    )]
    signer_whatsapp_failed: Option<bool>,
    #[serde(
        rename = "ContractRenewalNotice",
        skip_serializing_if = "Option::is_none"
    )]
    contract_renewal_notice: Option<bool>,
    #[serde(rename = "ContractExpiring", skip_serializing_if = "Option::is_none")]
    contract_expiring: Option<bool>,
    #[serde(rename = "ContractExpired", skip_serializing_if = "Option::is_none")]
    contract_expired: Option<bool>,
    #[serde(rename = "ContractRenewed", skip_serializing_if = "Option::is_none")]
    contract_renewed: Option<bool>,
    #[serde(rename = "SignerViewed", skip_serializing_if = "Option::is_none")]
    signer_viewed: Option<bool>,
    #[serde(rename = "SignerSigned", skip_serializing_if = "Option::is_none")]
    signer_signed: Option<bool>,
    #[serde(
        rename = "SignerIdentityReviewRequired",
        skip_serializing_if = "Option::is_none"
    )]
    signer_identity_review_required: Option<bool>,
}
impl UpdateNotificationPolicyEvents {
    /// Start an empty partial update.
    pub fn new() -> Self {
        Self::default()
    }
    /// Enable or disable DocumentCompleted.
    pub fn document_completed(mut self, enabled: bool) -> Self {
        self.document_completed = Some(enabled);
        self
    }
    /// Enable or disable SignerDeclined.
    pub fn signer_declined(mut self, enabled: bool) -> Self {
        self.signer_declined = Some(enabled);
        self
    }
    /// Enable or disable DocumentCancelled.
    pub fn document_cancelled(mut self, enabled: bool) -> Self {
        self.document_cancelled = Some(enabled);
        self
    }
    /// Enable or disable DocumentAboutToExpire.
    pub fn document_about_to_expire(mut self, enabled: bool) -> Self {
        self.document_about_to_expire = Some(enabled);
        self
    }
    /// Enable or disable DocumentExpired.
    pub fn document_expired(mut self, enabled: bool) -> Self {
        self.document_expired = Some(enabled);
        self
    }
    /// Enable or disable DocumentExpirationReset.
    pub fn document_expiration_reset(mut self, enabled: bool) -> Self {
        self.document_expiration_reset = Some(enabled);
        self
    }
    /// Enable or disable DocumentProcessingFailed.
    pub fn document_processing_failed(mut self, enabled: bool) -> Self {
        self.document_processing_failed = Some(enabled);
        self
    }
    /// Enable or disable TemplateProcessingFailed.
    pub fn template_processing_failed(mut self, enabled: bool) -> Self {
        self.template_processing_failed = Some(enabled);
        self
    }
    /// Enable or disable SignerWhatsappFailed.
    pub fn signer_whatsapp_failed(mut self, enabled: bool) -> Self {
        self.signer_whatsapp_failed = Some(enabled);
        self
    }
    /// Enable or disable ContractRenewalNotice.
    pub fn contract_renewal_notice(mut self, enabled: bool) -> Self {
        self.contract_renewal_notice = Some(enabled);
        self
    }
    /// Enable or disable ContractExpiring.
    pub fn contract_expiring(mut self, enabled: bool) -> Self {
        self.contract_expiring = Some(enabled);
        self
    }
    /// Enable or disable ContractExpired.
    pub fn contract_expired(mut self, enabled: bool) -> Self {
        self.contract_expired = Some(enabled);
        self
    }
    /// Enable or disable ContractRenewed.
    pub fn contract_renewed(mut self, enabled: bool) -> Self {
        self.contract_renewed = Some(enabled);
        self
    }
    /// Enable or disable SignerViewed.
    pub fn signer_viewed(mut self, enabled: bool) -> Self {
        self.signer_viewed = Some(enabled);
        self
    }
    /// Enable or disable SignerSigned.
    pub fn signer_signed(mut self, enabled: bool) -> Self {
        self.signer_signed = Some(enabled);
        self
    }
    /// Enable or disable SignerIdentityReviewRequired.
    pub fn signer_identity_review_required(mut self, enabled: bool) -> Self {
        self.signer_identity_review_required = Some(enabled);
        self
    }
    /// Whether no preference was supplied.
    pub fn is_empty(&self) -> bool {
        self.document_completed.is_none()
            && self.signer_declined.is_none()
            && self.document_cancelled.is_none()
            && self.document_about_to_expire.is_none()
            && self.document_expired.is_none()
            && self.document_expiration_reset.is_none()
            && self.document_processing_failed.is_none()
            && self.template_processing_failed.is_none()
            && self.signer_whatsapp_failed.is_none()
            && self.contract_renewal_notice.is_none()
            && self.contract_expiring.is_none()
            && self.contract_expired.is_none()
            && self.contract_renewed.is_none()
            && self.signer_viewed.is_none()
            && self.signer_signed.is_none()
            && self.signer_identity_review_required.is_none()
    }
}

/// Partial workspace notification policy; set events and/or the owner switch.
#[derive(Debug, Clone, Default, Serialize)]
pub struct UpdateNotificationPolicyBody {
    #[serde(skip_serializing_if = "Option::is_none")]
    events: Option<UpdateNotificationPolicyEvents>,
    #[serde(skip_serializing_if = "Option::is_none")]
    owners_receive_all: Option<bool>,
}
impl UpdateNotificationPolicyBody {
    /// Start an empty partial update.
    pub fn new() -> Self {
        Self::default()
    }
    /// Change selected event rules.
    pub fn events(mut self, events: UpdateNotificationPolicyEvents) -> Self {
        self.events = Some(events);
        self
    }
    /// Set whether owners receive all members' push alerts.
    pub fn owners_receive_all(mut self, enabled: bool) -> Self {
        self.owners_receive_all = Some(enabled);
        self
    }
    /// Whether this update contains no effective changes.
    pub fn is_empty(&self) -> bool {
        self.owners_receive_all.is_none()
            && self
                .events
                .as_ref()
                .is_none_or(UpdateNotificationPolicyEvents::is_empty)
    }
}

impl AccountApi<'_> {
    /// Read account capabilities. Requires `account:read` for OAuth apps.
    /// `GET /accounts/{account_id}?expand=signing_features`.
    /// # Request payload
    /// ```json
    /// {"expand":"signing_features"}
    /// ```
    /// # Response payload
    /// ```json
    /// {"status":200,"message":"","data":{"id":"account-id","name":"Example Workspace","signing_features":{"DigitalCertificate":true,"VerificationIdDocumentFaceMatchBasic":false,"WhatsappNotification":true,"VerificationBypass":false,"Embed":false}}}
    /// ```
    pub async fn get_with_signing_features(&self) -> Result<Account> {
        let path = self.http.path(&["accounts", &self.account_id])?;
        let req = self
            .http
            .request(Method::GET, &path)?
            .query(&[("expand", "signing_features")]);
        self.http.send_envelope(req).await
    }
    /// Read workspace notification rules. Third-party OAuth apps receive `403`.
    /// `GET /accounts/{account_id}/notification-policy`.
    /// # Request payload
    /// No request body or query parameters.
    /// # Response payload
    /// ```json
    /// {"status":200,"message":"","data":{"editable":true,"owners_receive_all":false,"events":{"SignerViewed":false}}}
    /// ```
    pub async fn notification_policy(&self) -> Result<crate::models::account::NotificationPolicy> {
        let path = self
            .http
            .path(&["accounts", &self.account_id, "notification-policy"])?;
        let req = self.http.request(Method::GET, &path)?;
        self.http.send_envelope(req).await
    }
    /// Update workspace notification rules. Only owners may write; third-party OAuth apps receive `403`.
    /// `PUT /accounts/{account_id}/notification-policy`.
    /// # Request payload
    /// ```json
    /// {"events":{"SignerViewed":false},"owners_receive_all":true}
    /// ```
    /// # Response payload
    /// ```json
    /// {"status":200,"message":"","data":{"editable":true,"owners_receive_all":true,"events":{"SignerViewed":false}}}
    /// ```
    pub async fn update_notification_policy(
        &self,
        body: &UpdateNotificationPolicyBody,
    ) -> Result<crate::models::account::NotificationPolicy> {
        if body.is_empty() {
            return Err(Error::Validation(
                "notification policy update must not be empty".into(),
            ));
        }
        let path = self
            .http
            .path(&["accounts", &self.account_id, "notification-policy"])?;
        let req = self.http.request(Method::PUT, &path)?.json(body);
        self.http.send_envelope(req).await
    }
}

impl AccountApi<'_> {
    /// Read plan usage without prices or payment information.
    /// `GET /accounts/{account_id}/usage`. Third-party OAuth apps receive `403`.
    /// # Request payload
    /// No body or query parameters; the account identifier is supplied in the route.
    /// # Response payload
    /// ```json
    /// {"status":200,"message":"","data":{"plan":"Pro","documents_renewal_date":"2026-12-01","balances":{"documents":{"balance":100,"limit":100,"spent":0},"credits":{"balance":12.5,"limit":null,"spent":0.5}}}}
    /// ```
    pub async fn usage(&self) -> Result<crate::models::account::AccountUsage> {
        let path = self.http.path(&["accounts", &self.account_id, "usage"])?;
        let req = self.http.request(Method::GET, &path)?;
        self.http.send_envelope(req).await
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn document_stats_query_serializes_valid_monthly_and_daily_requests() {
        assert_eq!(
            serde_json::to_value(DocumentStatsQuery::monthly()).unwrap(),
            serde_json::json!({ "granularity": "monthly" })
        );
        assert_eq!(
            serde_json::to_value(DocumentStatsQuery::daily("2026-06").unwrap()).unwrap(),
            serde_json::json!({ "granularity": "daily", "month": "2026-06" })
        );
    }

    #[test]
    fn document_stats_query_rejects_invalid_daily_months() {
        for invalid in ["2026-00", "2026-13", "2026-6", "26-06", "202A-06"] {
            assert!(
                matches!(DocumentStatsQuery::daily(invalid), Err(Error::Config(_))),
                "accepted invalid month {invalid}"
            );
        }
    }

    #[test]
    fn document_stats_row_decodes_the_complete_api_shape() {
        let row: DocumentStatsRow = serde_json::from_value(serde_json::json!({
            "period": "2026-06",
            "documents_uploaded": 42,
            "documents_sent": 37,
            "signature_requests": 61,
            "signature_requests_notification_email": 55,
            "signature_requests_notification_whatsapp": 18,
            "signature_requests_notification_bypass": 3,
            "signature_requests_verification_email": 48,
            "signature_requests_verification_whatsapp": 6,
            "signature_requests_verification_bypass": 3,
            "signature_requests_verification_digital_certificate": 4,
            "signature_requests_viewed": 44,
            "signature_requests_completed": 52,
            "documents_certified": 30
        }))
        .unwrap();

        assert_eq!(row.period, "2026-06");
        assert_eq!(row.signature_requests, 61);
        assert_eq!(row.signature_requests_notification_email, 55);
        assert_eq!(row.signature_requests_verification_digital_certificate, 4);
        assert_eq!(row.documents_certified, 30);
    }

    #[test]
    #[allow(deprecated)]
    fn document_stats_row_accepts_legacy_notification_counter_names() {
        let row: DocumentStatsRow = serde_json::from_value(serde_json::json!({
            "period": "2026-06",
            "signature_requests_email": 5,
            "signature_requests_whatsapp": 1
        }))
        .unwrap();

        assert_eq!(row.signature_requests_notification_email, 5);
        assert_eq!(row.signature_requests_notification_whatsapp, 1);
        assert_eq!(row.signature_requests_email, 5);
        assert_eq!(row.signature_requests_whatsapp, 1);
    }
}
