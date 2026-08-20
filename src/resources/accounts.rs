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
    /// # Response payload
    ///
    /// ```json
    /// { "status": 200, "message": "", "data": [
    ///   { "id": "102d25a4...", "name": "Acme Inc.", "roles": ["owner"],
    ///     "is_delete_allowed": true, "created_at": "2026-05-12T18:05:11Z" }
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
    /// { "name": "Acme Inc.", "notification_sender_type": "Account" }
    /// ```
    ///
    /// # Response payload
    ///
    /// ```json
    /// { "status": 200, "message": "", "data": { "id": "…", "name": "Acme Inc.", "created_at": "…" } }
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
    /// # Response payload
    ///
    /// ```json
    /// { "status": 200, "message": "", "data": {
    ///   "id": "102d25a4...", "name": "Acme Inc.",
    ///   "primary_color": null, "secondary_color": null,
    ///   "created_at": "2026-05-12T18:05:11Z" } }
    /// ```
    pub async fn get(&self) -> Result<Account> {
        let path = format!("accounts/{}", self.account_id);
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
    ///   "id": "102d25a4...", "name": "New name",
    ///   "primary_color": null, "secondary_color": null,
    ///   "created_at": "2026-05-12T18:05:11Z" } }
    /// ```
    pub async fn update(&self, body: &UpdateAccountBody) -> Result<Account> {
        let path = format!("accounts/{}", self.account_id);
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
        let path = format!("accounts/{}", self.account_id);
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
    /// # Response payload
    ///
    /// ```json
    /// { "status": 200, "message": "", "data": {
    ///   "account_name": "Acme Inc.", "primary_color": "2072b9",
    ///   "secondary_color": "ffffff", "logo": null } }
    /// ```
    pub async fn theme(&self) -> Result<AccountTheme> {
        let path = format!("accounts/{}/theme", self.account_id);
        let req = self.http.request(Method::GET, &path)?;
        self.http.send_envelope(req).await
    }

    /// Retrieve this account's precomputed document-funnel statistics.
    ///
    /// `GET /accounts/{account_id}/stats`. Monthly queries return the last 12
    /// months, most recent first. Daily queries return every day in the
    /// requested month. The API zero-fills both series.
    ///
    /// # Request parameters
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
    ///   "signature_requests_email": 5,
    ///   "signature_requests_whatsapp": 1,
    ///   "signature_requests_viewed": 4,
    ///   "signature_requests_completed": 5,
    ///   "documents_certified": 3
    /// }] }
    /// ```
    pub async fn stats(&self, query: &DocumentStatsQuery) -> Result<Vec<DocumentStatsRow>> {
        let path = format!("accounts/{}/stats", self.account_id);
        let req = self.http.request(Method::GET, &path)?.query(query);
        self.http.send_envelope(req).await
    }

    /// Download the account logo image bytes and its content type.
    ///
    /// `GET /accounts/{account_id}/logo`. Returns an [`Error::Api`] with status
    /// `404` when no logo has been uploaded.
    pub async fn download_logo(&self) -> Result<(Bytes, String)> {
        let path = format!("accounts/{}/logo", self.account_id);
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
    /// # Response payload
    ///
    /// ```json
    /// { "status": 200, "message": "" }
    /// ```
    pub async fn upload_logo(&self, logo: UploadLogoRequest) -> Result<()> {
        let path = format!("accounts/{}/logo", self.account_id);
        let form = logo.into_form()?;
        let req = self.http.request(Method::POST, &path)?.multipart(form);
        self.http.send_no_content(req).await
    }

    /// Delete the account logo image.
    ///
    /// `DELETE /accounts/{account_id}/logo`.
    ///
    /// # Response payload
    ///
    /// ```json
    /// { "status": 200, "message": "" }
    /// ```
    pub async fn delete_logo(&self) -> Result<()> {
        let path = format!("accounts/{}/logo", self.account_id);
        let req = self.http.request(Method::DELETE, &path)?;
        self.http.send_no_content(req).await
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
            "signature_requests_email": 55,
            "signature_requests_whatsapp": 18,
            "signature_requests_viewed": 44,
            "signature_requests_completed": 52,
            "documents_certified": 30
        }))
        .unwrap();

        assert_eq!(row.period, "2026-06");
        assert_eq!(row.signature_requests, 61);
        assert_eq!(row.documents_certified, 30);
    }
}
