//! Endpoints for the authenticated user under `/users`.

use reqwest::Method;
use serde::{Deserialize, Serialize};

use crate::error::{Error, Result};
use crate::http::HttpClient;
use crate::models::{
    DocumentStatsRow, MfaStatus, NotificationPreferences, RecoveryCodes, SelfUser, TotpEnrollment,
    UserProfile,
};

use super::accounts::DocumentStatsQuery;

/// Partial body for `PUT /users/self/notification-preferences`.
///
/// Set only the preferences that should change; omitted preferences retain
/// their current values. The API rejects an empty object, so
/// [`UsersApi::update_notification_preferences`] validates that at least one
/// setter was used before sending the request.
///
/// # Example request payload
///
/// ```json
/// {
///   "DocumentCompleted": false,
///   "SignerDeclined": true
/// }
/// ```
#[derive(Debug, Clone, Default, Serialize)]
pub struct UpdateNotificationPreferencesBody {
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
}

impl UpdateNotificationPreferencesBody {
    /// Create an empty partial update. Set at least one preference before
    /// passing it to [`UsersApi::update_notification_preferences`].
    pub fn new() -> Self {
        Self::default()
    }

    /// Enable or disable the document-completed e-mail.
    pub fn document_completed(mut self, enabled: bool) -> Self {
        self.document_completed = Some(enabled);
        self
    }

    /// Enable or disable the signer-declined e-mail.
    pub fn signer_declined(mut self, enabled: bool) -> Self {
        self.signer_declined = Some(enabled);
        self
    }

    /// Enable or disable the document-cancelled e-mail.
    pub fn document_cancelled(mut self, enabled: bool) -> Self {
        self.document_cancelled = Some(enabled);
        self
    }

    /// Enable or disable the document-about-to-expire e-mail.
    pub fn document_about_to_expire(mut self, enabled: bool) -> Self {
        self.document_about_to_expire = Some(enabled);
        self
    }

    /// Enable or disable the document-expired e-mail.
    pub fn document_expired(mut self, enabled: bool) -> Self {
        self.document_expired = Some(enabled);
        self
    }

    /// Enable or disable the document-expiration-reset e-mail.
    pub fn document_expiration_reset(mut self, enabled: bool) -> Self {
        self.document_expiration_reset = Some(enabled);
        self
    }

    /// Enable or disable the document-processing-failed e-mail.
    pub fn document_processing_failed(mut self, enabled: bool) -> Self {
        self.document_processing_failed = Some(enabled);
        self
    }

    /// Enable or disable the template-processing-failed e-mail.
    pub fn template_processing_failed(mut self, enabled: bool) -> Self {
        self.template_processing_failed = Some(enabled);
        self
    }

    /// Enable or disable the signer-WhatsApp-failed e-mail.
    pub fn signer_whatsapp_failed(mut self, enabled: bool) -> Self {
        self.signer_whatsapp_failed = Some(enabled);
        self
    }

    /// Whether this update contains no preferences.
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
    }
}

/// Body for `PUT /users/self/mfa/totp/confirm`.
///
/// `id` and `code` (a live code from the newly enrolled device) are required.
/// When the enrollment replaces a confirmed method of the same type, also
/// re-authenticate with [`password`](Self::password) or
/// [`reauth_code`](Self::reauth_code). Holds secrets, so it has no `Debug`.
///
/// # Request payload
///
/// ```json
/// { "id": "a1b2c3d4e5f60718293a4b5c6d7e8f90", "code": "123456", "password": "example-redacted-password" }
/// ```
#[derive(Clone, Serialize)]
pub struct ConfirmTotpBody {
    id: String,
    code: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    password: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    reauth_code: Option<String>,
}

impl ConfirmTotpBody {
    /// Confirm enrollment `id` with a live `code` from the new device.
    pub fn new<I: Into<String>, C: Into<String>>(id: I, code: C) -> Self {
        Self {
            id: id.into(),
            code: code.into(),
            password: None,
            reauth_code: None,
        }
    }

    /// Re-authenticate with the current password (needed only when replacing
    /// a confirmed method).
    pub fn password<S: Into<String>>(mut self, password: S) -> Self {
        self.password = Some(password.into());
        self
    }

    /// Re-authenticate with a live code from the current device or an
    /// unused recovery code (needed only when replacing a confirmed method).
    pub fn reauth_code<S: Into<String>>(mut self, code: S) -> Self {
        self.reauth_code = Some(code.into());
        self
    }
}

/// Re-authentication body for `POST /users/self/mfa/recovery-codes` and
/// `DELETE /users/self/mfa/{id}`: the current password, or a live
/// authenticator code or unused recovery code (which is then consumed). Holds
/// a secret, so it has no `Debug`.
///
/// # Request payload
///
/// ```json
/// { "code": "123456" }
/// ```
#[derive(Clone, Serialize)]
pub struct MfaReauthBody {
    #[serde(skip_serializing_if = "Option::is_none")]
    password: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    code: Option<String>,
}

impl MfaReauthBody {
    /// Re-authenticate with the current password.
    pub fn password<S: Into<String>>(password: S) -> Self {
        Self {
            password: Some(password.into()),
            code: None,
        }
    }

    /// Re-authenticate with a live authenticator code or a recovery code.
    pub fn code<S: Into<String>>(code: S) -> Self {
        Self {
            password: None,
            code: Some(code.into()),
        }
    }
}

#[derive(Deserialize)]
struct MfaRemoval {
    is_mfa_enabled: bool,
}

#[derive(Debug, Deserialize)]
#[serde(untagged)]
enum SelfUserResponse {
    Profile(UserProfile),
    Legacy(SelfUser),
}

impl SelfUserResponse {
    fn into_profile(self) -> UserProfile {
        match self {
            Self::Profile(profile) => profile,
            Self::Legacy(response) => response.user,
        }
    }
}

/// User endpoints scoped to the authenticated credential.
#[derive(Debug)]
pub struct UsersApi<'a> {
    http: &'a HttpClient,
}

impl<'a> UsersApi<'a> {
    pub(crate) fn new(http: &'a HttpClient) -> Self {
        Self { http }
    }

    /// Retrieve the authenticated user's profile.
    ///
    /// `GET /users/self`. The SDK accepts either a direct profile or the
    /// compatibility [`SelfUser`] data shape and consistently returns only the
    /// profile.
    ///
    /// # Request payload
    ///
    /// No request body. Resource identifiers are supplied in the route.
    ///
    /// # Response payload
    ///
    /// ```json
    /// {
    ///   "status": 200,
    ///   "message": "",
    ///   "data": {
    ///     "id": "user_123",
    ///     "name": "Example User",
    ///     "email": "redacted",
    ///     "telephone": null,
    ///     "government_id": null,
    ///     "is_email_verified": true,
    ///     "has_accepted_terms": true,
    ///     "created_at": "2026-05-12T18:05:11Z",
    ///     "to_be_deleted_at": null
    ///   }
    /// }
    /// ```
    pub async fn me(&self) -> Result<UserProfile> {
        let req = self.http.request(Method::GET, "users/self")?;
        let response: SelfUserResponse = self.http.send_envelope(req).await?;
        Ok(response.into_profile())
    }

    /// Retrieve document-funnel statistics summed across all accounts the
    /// authenticated user currently belongs to.
    ///
    /// `GET /users/self/stats`. Monthly queries return the last 12 months,
    /// most recent first. Daily queries return every day in the requested
    /// month. The API zero-fills both series. This endpoint is available on
    /// production only.
    ///
    /// # Request parameters
    ///
    /// ```json
    /// { "granularity": "monthly" }
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
    ///   "period": "2026-06",
    ///   "documents_uploaded": 42,
    ///   "documents_sent": 37,
    ///   "signature_requests": 61,
    ///   "signature_requests_notification_email": 55,
    ///   "signature_requests_notification_whatsapp": 18,
    ///   "signature_requests_notification_bypass": 3,
    ///   "signature_requests_verification_email": 48,
    ///   "signature_requests_verification_whatsapp": 6,
    ///   "signature_requests_verification_bypass": 3,
    ///   "signature_requests_verification_digital_certificate": 4,
    ///   "signature_requests_viewed": 44,
    ///   "signature_requests_completed": 52,
    ///   "documents_certified": 30
    /// }] }
    /// ```
    pub async fn stats(&self, query: &DocumentStatsQuery) -> Result<Vec<DocumentStatsRow>> {
        let req = self
            .http
            .request(Method::GET, "users/self/stats")?
            .query(query);
        self.http.send_envelope(req).await
    }

    /// Retrieve all nine owner-facing document e-mail preferences.
    ///
    /// `GET /users/self/notification-preferences`. All keys are always
    /// returned; each defaults to `true` until changed. This endpoint is
    /// available on production only.
    ///
    /// # Request payload
    ///
    /// No request body. Resource identifiers are supplied in the route.
    ///
    /// # Response payload
    ///
    /// ```json
    /// { "status": 200, "message": "", "data": {
    ///   "DocumentCompleted": true,
    ///   "SignerDeclined": true,
    ///   "DocumentCancelled": true,
    ///   "DocumentAboutToExpire": true,
    ///   "DocumentExpired": true,
    ///   "DocumentExpirationReset": true,
    ///   "DocumentProcessingFailed": true,
    ///   "TemplateProcessingFailed": true,
    ///   "SignerWhatsappFailed": true
    /// } }
    /// ```
    pub async fn notification_preferences(&self) -> Result<NotificationPreferences> {
        let req = self
            .http
            .request(Method::GET, "users/self/notification-preferences")?;
        self.http.send_envelope(req).await
    }

    /// Update one or more owner-facing document e-mail preferences.
    ///
    /// `PUT /users/self/notification-preferences`. Omitted keys keep their
    /// current values. An empty update is rejected locally as
    /// [`Error::Config`]; the API likewise rejects empty objects, unknown keys,
    /// and non-boolean values. The full updated preference map is returned.
    /// This endpoint is available on production only.
    ///
    /// # Request payload
    ///
    /// ```json
    /// { "DocumentCompleted": false, "SignerDeclined": true }
    /// ```
    ///
    /// # Response payload
    ///
    /// ```json
    /// { "status": 200, "message": "", "data": {
    ///   "DocumentCompleted": false,
    ///   "SignerDeclined": true,
    ///   "DocumentCancelled": true,
    ///   "DocumentAboutToExpire": true,
    ///   "DocumentExpired": true,
    ///   "DocumentExpirationReset": true,
    ///   "DocumentProcessingFailed": true,
    ///   "TemplateProcessingFailed": true,
    ///   "SignerWhatsappFailed": true
    /// } }
    /// ```
    pub async fn update_notification_preferences(
        &self,
        body: &UpdateNotificationPreferencesBody,
    ) -> Result<NotificationPreferences> {
        if body.is_empty() {
            return Err(Error::Config(
                "notification preference update must contain at least one preference".to_owned(),
            ));
        }

        let req = self
            .http
            .request(Method::PUT, "users/self/notification-preferences")?
            .json(body);
        self.http.send_envelope(req).await
    }

    /// List the authenticated user's two-factor methods.
    ///
    /// `GET /users/self/mfa`.
    ///
    /// # Request payload
    ///
    /// No request body.
    ///
    /// # Response payload
    ///
    /// ```json
    /// {
    ///   "status": 200,
    ///   "message": "",
    ///   "data": {
    ///     "methods": [
    ///       { "id": "a1b2c3d4e5f60718293a4b5c6d7e8f90", "type": "Totp", "label": "My phone", "confirmed_at": "2026-09-09T14:21:03Z", "last_used_at": "2026-09-09T18:02:44Z" }
    ///     ],
    ///     "recovery_codes_remaining": 10
    ///   }
    /// }
    /// ```
    pub async fn mfa_methods(&self) -> Result<MfaStatus> {
        let req = self.http.request(Method::GET, "users/self/mfa")?;
        self.http.send_envelope(req).await
    }

    /// Start an authenticator (TOTP) enrollment. Two-factor authentication is
    /// not active until [`confirm_totp_enrollment`](Self::confirm_totp_enrollment).
    ///
    /// `POST /users/self/mfa/totp`. The secret is returned only by this call.
    ///
    /// # Request payload
    ///
    /// `label` is optional; `None` sends an empty object.
    ///
    /// ```json
    /// { "label": "My phone" }
    /// ```
    ///
    /// # Response payload
    ///
    /// ```json
    /// {
    ///   "status": 200,
    ///   "message": "",
    ///   "data": {
    ///     "id": "a1b2c3d4e5f60718293a4b5c6d7e8f90",
    ///     "secret": "GEZDGNBVGY3TQOJQGEZDGNBVGY3TQOJQ",
    ///     "provisioning_uri": "otpauth://totp/user%40example.invalid?issuer=Assinafy&secret=GEZDGNBVGY3TQOJQGEZDGNBVGY3TQOJQ"
    ///   }
    /// }
    /// ```
    pub async fn start_totp_enrollment(&self, label: Option<&str>) -> Result<TotpEnrollment> {
        let body = match label {
            Some(label) => serde_json::json!({ "label": label }),
            None => serde_json::json!({}),
        };
        let req = self
            .http
            .request(Method::POST, "users/self/mfa/totp")?
            .json(&body);
        self.http.send_envelope(req).await
    }

    /// Confirm an authenticator enrollment, enabling two-factor login, and
    /// receive the recovery codes (shown only once).
    ///
    /// `PUT /users/self/mfa/totp/confirm`.
    ///
    /// # Request payload
    ///
    /// ```json
    /// { "id": "a1b2c3d4e5f60718293a4b5c6d7e8f90", "code": "123456" }
    /// ```
    ///
    /// # Response payload
    ///
    /// ```json
    /// { "status": 200, "message": "", "data": { "recovery_codes": ["ABCD-EFGH-JKMN"] } }
    /// ```
    pub async fn confirm_totp_enrollment(&self, body: &ConfirmTotpBody) -> Result<RecoveryCodes> {
        let req = self
            .http
            .request(Method::PUT, "users/self/mfa/totp/confirm")?
            .json(body);
        self.http.send_envelope(req).await
    }

    /// Issue ten new recovery codes, invalidating the previous set.
    ///
    /// `POST /users/self/mfa/recovery-codes`.
    ///
    /// # Request payload
    ///
    /// ```json
    /// { "password": "example-redacted-password" }
    /// ```
    ///
    /// # Response payload
    ///
    /// ```json
    /// { "status": 200, "message": "", "data": { "recovery_codes": ["ABCD-EFGH-JKMN"] } }
    /// ```
    pub async fn regenerate_recovery_codes(&self, body: &MfaReauthBody) -> Result<RecoveryCodes> {
        let req = self
            .http
            .request(Method::POST, "users/self/mfa/recovery-codes")?
            .json(body);
        self.http.send_envelope(req).await
    }

    /// Remove an enrolled two-factor method. Returns whether two-factor
    /// authentication is still enabled; removing the last method also
    /// discards the recovery codes.
    ///
    /// `DELETE /users/self/mfa/{method_id}`.
    ///
    /// # Request payload
    ///
    /// ```json
    /// { "code": "123456" }
    /// ```
    ///
    /// # Response payload
    ///
    /// ```json
    /// { "status": 200, "message": "", "data": { "is_mfa_enabled": false } }
    /// ```
    pub async fn remove_mfa_method<S: AsRef<str>>(
        &self,
        method_id: S,
        body: &MfaReauthBody,
    ) -> Result<bool> {
        let path = self
            .http
            .path(&["users", "self", "mfa", method_id.as_ref()])?;
        let req = self.http.request(Method::DELETE, &path)?.json(body);
        let removal: MfaRemoval = self.http.send_envelope(req).await?;
        Ok(removal.is_mfa_enabled)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn profile_json(id: &str) -> serde_json::Value {
        serde_json::json!({
            "id": id,
            "name": "Example User",
            "email": "redacted",
            "telephone": null,
            "government_id": null,
            "is_email_verified": true,
            "has_accepted_terms": true,
            "created_at": "2026-05-12T18:05:11Z",
            "to_be_deleted_at": null
        })
    }

    #[test]
    fn users_self_accepts_production_and_legacy_data_shapes() {
        let production: SelfUserResponse =
            serde_json::from_value(profile_json("production-user")).unwrap();
        assert_eq!(production.into_profile().id, "production-user");

        let legacy: SelfUserResponse = serde_json::from_value(serde_json::json!({
            "user": profile_json("legacy-user"),
            "accounts": []
        }))
        .unwrap();
        assert_eq!(legacy.into_profile().id, "legacy-user");
    }

    #[test]
    fn mfa_bodies_send_only_the_fields_set() {
        assert_eq!(
            serde_json::to_value(ConfirmTotpBody::new("method-id", "123456")).unwrap(),
            serde_json::json!({ "id": "method-id", "code": "123456" })
        );
        assert_eq!(
            serde_json::to_value(ConfirmTotpBody::new("method-id", "123456").reauth_code("654321"))
                .unwrap(),
            serde_json::json!({ "id": "method-id", "code": "123456", "reauth_code": "654321" })
        );
        assert_eq!(
            serde_json::to_value(MfaReauthBody::code("123456")).unwrap(),
            serde_json::json!({ "code": "123456" })
        );
        assert_eq!(
            serde_json::to_value(MfaReauthBody::password("example-redacted-password")).unwrap(),
            serde_json::json!({ "password": "example-redacted-password" })
        );
    }

    #[test]
    fn notification_preference_update_is_partial_and_rejects_empty_bodies() {
        assert!(UpdateNotificationPreferencesBody::new().is_empty());

        let body = UpdateNotificationPreferencesBody::new()
            .document_completed(false)
            .signer_whatsapp_failed(true);
        assert!(!body.is_empty());
        assert_eq!(
            serde_json::to_value(body).unwrap(),
            serde_json::json!({
                "DocumentCompleted": false,
                "SignerWhatsappFailed": true
            })
        );
    }

    #[test]
    fn notification_preferences_decode_all_nine_api_keys() {
        let preferences: NotificationPreferences = serde_json::from_value(serde_json::json!({
            "DocumentCompleted": false,
            "SignerDeclined": true,
            "DocumentCancelled": false,
            "DocumentAboutToExpire": true,
            "DocumentExpired": false,
            "DocumentExpirationReset": true,
            "DocumentProcessingFailed": false,
            "TemplateProcessingFailed": true,
            "SignerWhatsappFailed": false
        }))
        .unwrap();

        assert!(!preferences.document_completed);
        assert!(preferences.signer_declined);
        assert!(preferences.document_about_to_expire);
        assert!(preferences.document_expiration_reset);
        assert!(preferences.template_processing_failed);
        assert!(!preferences.signer_whatsapp_failed);
    }
}
