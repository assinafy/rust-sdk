//! Public, unauthenticated endpoints.

use reqwest::Method;
use serde::{Deserialize, Serialize};

use crate::error::{Error, Result};
use crate::http::HttpClient;
use crate::models::PublicDocument;

/// Body for `PUT /public/documents/{document_id}/send-token`.
///
/// Both fields are required: `channel` is always `"email"` and `recipient` is
/// the email address of a signer on the document. Build it with
/// [`email`](Self::email).
///
/// # Request payload
///
/// ```json
/// { "channel": "email", "recipient": "user@example.invalid" }
/// ```
///
/// ```
/// # use assinafy::resources::SendTokenBody;
/// assert_eq!(
///     serde_json::to_value(SendTokenBody::email("user@example.invalid")).unwrap(),
///     serde_json::json!({ "recipient": "user@example.invalid", "channel": "email" })
/// );
/// ```
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SendTokenBody {
    /// Email address that should receive the token. It must belong to a
    /// signer on the document.
    pub recipient: String,
    /// Delivery channel. The API accepts only `"email"`.
    pub channel: String,
}

impl SendTokenBody {
    /// Build a request for an explicit recipient and channel.
    ///
    /// Prefer [`email`](Self::email): the API accepts only the `"email"`
    /// channel, and [`PublicApi::send_token`] refuses any other.
    pub fn new<R: Into<String>, C: Into<String>>(recipient: R, channel: C) -> Self {
        Self {
            recipient: recipient.into(),
            channel: channel.into(),
        }
    }

    /// Deliver the token to an email address.
    pub fn email<S: Into<String>>(recipient: S) -> Self {
        Self::new(recipient, "email")
    }

    /// Deliver the token over WhatsApp.
    ///
    /// The send-token route accepts only the `"email"` channel, so a body
    /// built here is refused by [`PublicApi::send_token`] with
    /// [`Error::Validation`].
    #[deprecated(
        since = "3.6.1",
        note = "the send-token route accepts only the email channel; use SendTokenBody::email"
    )]
    pub fn whatsapp<S: Into<String>>(recipient: S) -> Self {
        Self::new(recipient, "whatsapp")
    }

    fn validate(&self) -> Result<()> {
        if self.channel != "email" {
            return Err(Error::Validation(
                "send-token accepts only the \"email\" channel".into(),
            ));
        }
        if self.recipient.trim().is_empty() {
            return Err(Error::Validation(
                "send-token requires the signer's email address as recipient".into(),
            ));
        }
        Ok(())
    }
}

/// Response `data` of `PUT /public/documents/{document_id}/send-token`,
/// returned by [`PublicApi::send_token_with_result`].
///
/// # Response payload
///
/// ```json
/// {
///   "document": {
///     "resource": "document",
///     "id": "103b03b95d7951922360a1626727",
///     "name": "contract.pdf",
///     "page_count": 2,
///     "created_by": "Maria Silva"
///   },
///   "channel": "email",
///   "recipient": "user@example.invalid"
/// }
/// ```
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[non_exhaustive]
pub struct SendTokenResult {
    /// The document the token grants access to: `resource`, `id`, `name`,
    /// `page_count` and `created_by`.
    #[serde(default)]
    pub document: Option<PublicDocument>,
    /// Delivery channel used (`"email"`).
    #[serde(default)]
    pub channel: Option<String>,
    /// Email address that received the token.
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
    ///     "resource": "document",
    ///     "id": "103b03b95d7951922360a1626727",
    ///     "account_id": "acc_1234567890abcdef12345678",
    ///     "template_id": null,
    ///     "name": "test.pdf",
    ///     "status": "metadata_ready",
    ///     "artifacts": { "original": "https://files.example.invalid/original.pdf" },
    ///     "is_closed": false,
    ///     "signing_url": "https://sign.example.invalid/103b03b95d7951922360a1626727",
    ///     "verification_code": null,
    ///     "verification_url": null,
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

    /// Email a signer access token to a signer of the document.
    ///
    /// `PUT /public/documents/{document_id}/send-token`. Requires no
    /// authentication. The recipient must be the email address of a signer on
    /// the document; an unknown document answers `404`. Use
    /// [`send_token_with_result`](Self::send_token_with_result) to read the
    /// response `data`.
    ///
    /// # Errors
    ///
    /// [`Error::Validation`], before any request, when the channel is not
    /// `"email"` or the recipient is empty.
    ///
    /// # Request payload
    ///
    /// ```json
    /// { "channel": "email", "recipient": "user@example.invalid" }
    /// ```
    ///
    /// # Response payload
    ///
    /// ```json
    /// {
    ///   "status": 200,
    ///   "message": "",
    ///   "data": {
    ///     "document": { "resource": "document", "id": "103b03b95d7951922360a1626727", "name": "contract.pdf", "page_count": 2, "created_by": "Maria Silva" },
    ///     "channel": "email",
    ///     "recipient": "user@example.invalid"
    ///   }
    /// }
    /// ```
    pub async fn send_token<S: AsRef<str>>(
        &self,
        document_id: S,
        body: &SendTokenBody,
    ) -> Result<()> {
        self.send_token_with_result(document_id, body)
            .await
            .map(drop)
    }

    /// Email a signer access token and return the response `data`.
    ///
    /// `PUT /public/documents/{document_id}/send-token`. Same request, rules
    /// and errors as [`send_token`](Self::send_token).
    ///
    /// # Request payload
    ///
    /// ```json
    /// { "channel": "email", "recipient": "user@example.invalid" }
    /// ```
    ///
    /// # Response payload
    ///
    /// ```json
    /// {
    ///   "status": 200,
    ///   "message": "",
    ///   "data": {
    ///     "document": { "resource": "document", "id": "103b03b95d7951922360a1626727", "name": "contract.pdf", "page_count": 2, "created_by": "Maria Silva" },
    ///     "channel": "email",
    ///     "recipient": "user@example.invalid"
    ///   }
    /// }
    /// ```
    pub async fn send_token_with_result<S: AsRef<str>>(
        &self,
        document_id: S,
        body: &SendTokenBody,
    ) -> Result<SendTokenResult> {
        body.validate()?;
        let path = self
            .http
            .path(&["public", "documents", document_id.as_ref(), "send-token"])?;
        let req = self.http.request_public(Method::PUT, &path)?.json(body);
        self.http.send_envelope(req).await
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn send_token_body_is_the_email_channel_and_a_recipient() {
        let body = SendTokenBody::email("user@example.invalid");
        assert_eq!(
            serde_json::to_value(&body).unwrap(),
            serde_json::json!({ "channel": "email", "recipient": "user@example.invalid" })
        );
        assert!(body.validate().is_ok());
    }

    #[test]
    #[allow(deprecated)]
    fn send_token_refuses_other_channels_and_empty_recipients() {
        for body in [
            SendTokenBody::whatsapp("+5511999999999"),
            SendTokenBody::new("user@example.invalid", "sms"),
            SendTokenBody::email(" "),
        ] {
            assert!(matches!(body.validate(), Err(Error::Validation(_))));
        }
    }

    #[test]
    fn send_token_result_decodes_the_documented_data() {
        let result: SendTokenResult = serde_json::from_value(serde_json::json!({
            "document": { "resource": "document", "id": "doc-1", "name": "contract.pdf", "page_count": 2, "created_by": "Maria Silva" },
            "channel": "email",
            "recipient": "user@example.invalid"
        }))
        .unwrap();
        assert_eq!(result.channel.as_deref(), Some("email"));
        assert_eq!(result.recipient.as_deref(), Some("user@example.invalid"));
        assert_eq!(result.document.unwrap().id, "doc-1");
    }
}
