//! Public, unauthenticated endpoints.

use reqwest::Method;
use serde::{Deserialize, Serialize};

use crate::error::Result;
use crate::http::HttpClient;
use crate::models::PublicDocument;

/// Body for `PUT /public/documents/{document_id}/send-token`.
#[derive(Clone, Serialize, Deserialize)]
pub struct SendTokenBody {
    /// Email address that should receive the token.
    pub email: String,
}

impl SendTokenBody {
    /// Build an email token request.
    pub fn new<S: Into<String>>(email: S) -> Self {
        Self {
            email: email.into(),
        }
    }

    /// Build an email token request.
    pub fn email<S: Into<String>>(email: S) -> Self {
        Self::new(email)
    }
}

/// Compatibility body for
/// `PUT /public/documents/{document_id}/send-token`.
///
/// Use [`SendTokenBody`] unless the target deployment requires an explicit
/// recipient and channel:
///
/// ```json
/// { "recipient": "user@example.invalid", "channel": "email" }
/// ```
#[derive(Clone, Serialize, Deserialize)]
pub struct LegacySendTokenBody {
    /// Address or phone number that should receive the token.
    pub recipient: String,
    /// Delivery channel understood by the compatibility endpoint.
    pub channel: String,
}

impl LegacySendTokenBody {
    /// Build a compatibility token request for an explicit channel.
    #[deprecated(note = "compatibility only; use SendTokenBody")]
    pub fn new<R: Into<String>, C: Into<String>>(recipient: R, channel: C) -> Self {
        Self {
            recipient: recipient.into(),
            channel: channel.into(),
        }
    }

    /// Build a compatibility email-token request.
    #[deprecated(note = "compatibility only; use SendTokenBody::email")]
    pub fn email<S: Into<String>>(recipient: S) -> Self {
        Self {
            recipient: recipient.into(),
            channel: "email".to_owned(),
        }
    }
}

/// Legacy payload used by older deployments of
/// `PUT /public/documents/{document_id}/send-token`.
///
/// The current API defines only a generic success envelope and
/// [`PublicApi::send_token`] therefore returns `()`. This model remains public
/// so applications that deserialize a legacy response themselves do not lose
/// source compatibility.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[non_exhaustive]
pub struct SendTokenResult {
    /// Public document metadata, when returned by the API.
    #[serde(default)]
    pub document: Option<PublicDocument>,
    /// Delivery channel used, when returned.
    #[serde(default)]
    pub channel: Option<String>,
    /// Recipient that received the token, when returned.
    #[serde(default)]
    pub recipient: Option<String>,
}

/// Public endpoints that do not require authentication.
#[derive(Debug)]
pub struct PublicApi<'a> {
    http: &'a HttpClient,
}

impl<'a> PublicApi<'a> {
    pub(crate) fn new(http: &'a HttpClient) -> Self {
        Self { http }
    }

    /// Retrieve public-facing document metadata.
    ///
    /// `GET /public/documents/{document_id}`. Requires no authentication.
    ///
    /// # Response payload
    ///
    /// ```json
    /// {
    ///   "status": 200,
    ///   "message": "",
    ///   "data": {
    ///     "resource": "document",
    ///     "id": "103b03b95d7951922360a1626727",
    ///     "account_id": "acc_1234567890abcdef12345678",
    ///     "template_id": null,
    ///     "name": "test.pdf",
    ///     "status": "metadata_ready",
    ///     "artifacts": { "original": "https://files.example.invalid/original.pdf" },
    ///     "is_closed": false,
    ///     "signing_url": "https://sign.example.invalid/103b03b95d7951922360a1626727",
    ///     "decline_reason": null,
    ///     "declined_by": null,
    ///     "tags": [],
    ///     "assignment": null,
    ///     "pages": [],
    ///     "created_at": "2026-08-20T12:00:00Z",
    ///     "updated_at": "2026-08-20T12:01:00Z"
    ///   }
    /// }
    /// ```
    ///
    /// Older deployments may instead return the reduced legacy fields
    /// `page_count` and `created_by`; [`PublicDocument`] accepts both shapes.
    pub async fn document<S: AsRef<str>>(&self, document_id: S) -> Result<PublicDocument> {
        let path = self
            .http
            .path(&["public", "documents", document_id.as_ref()])?;
        let req = self.http.request_public(Method::GET, &path)?;
        self.http.send_envelope(req).await
    }

    /// Send a signer access token to the signer by email.
    ///
    /// `PUT /public/documents/{document_id}/send-token`. The email address must
    /// belong to a signer on the document.
    ///
    /// # Request payload
    ///
    /// ```json
    /// { "email": "user@example.invalid" }
    /// ```
    ///
    /// # Response payload
    ///
    /// ```json
    /// { "status": 200, "message": "" }
    /// ```
    pub async fn send_token<S: AsRef<str>>(
        &self,
        document_id: S,
        body: &SendTokenBody,
    ) -> Result<()> {
        self.send_token_payload(document_id.as_ref(), body).await
    }

    /// Send a signer access token using the compatibility payload.
    ///
    /// `PUT /public/documents/{document_id}/send-token`. This compatibility
    /// method requires no authentication and sends the recipient/channel
    /// request shape. Prefer [`Self::send_token`] unless it is required.
    ///
    /// # Request payload
    ///
    /// ```json
    /// { "recipient": "user@example.invalid", "channel": "email" }
    /// ```
    ///
    /// # Response payload
    ///
    /// ```json
    /// { "status": 200, "message": "", "data": [] }
    /// ```
    #[deprecated(note = "compatibility only; use PublicApi::send_token")]
    pub async fn send_token_legacy<S: AsRef<str>>(
        &self,
        document_id: S,
        body: &LegacySendTokenBody,
    ) -> Result<()> {
        self.send_token_payload(document_id.as_ref(), body).await
    }

    async fn send_token_payload<T: Serialize + ?Sized>(
        &self,
        document_id: &str,
        body: &T,
    ) -> Result<()> {
        let path = self
            .http
            .path(&["public", "documents", document_id, "send-token"])?;
        let req = self.http.request_public(Method::PUT, &path)?.json(body);
        self.http.send_no_content(req).await
    }
}

#[cfg(test)]
mod tests {
    use super::{LegacySendTokenBody, SendTokenBody};

    #[test]
    #[allow(deprecated)]
    fn production_and_legacy_send_token_bodies_stay_distinct() {
        let production =
            serde_json::to_value(SendTokenBody::email("user@example.invalid")).unwrap();
        let legacy =
            serde_json::to_value(LegacySendTokenBody::email("user@example.invalid")).unwrap();

        assert_eq!(
            production,
            serde_json::json!({ "email": "user@example.invalid" })
        );
        assert_eq!(
            legacy,
            serde_json::json!({
                "recipient": "user@example.invalid",
                "channel": "email"
            })
        );
    }
}
