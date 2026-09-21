//! Public, unauthenticated endpoints.

use reqwest::Method;
use serde::{Deserialize, Serialize};

use crate::error::Result;
use crate::http::HttpClient;
use crate::models::PublicDocument;

/// Body for `PUT /public/documents/{document_id}/send-token`.
///
/// Both fields are required by the live API. The published OpenAPI document
/// describes a single `email` field instead; that shape is rejected with
/// `400 O atributo "channel" é obrigatório.` on production,
/// so the SDK follows the live contract.
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
    /// Address or phone number that should receive the token. It must belong
    /// to a signer on the document.
    pub recipient: String,
    /// Delivery channel — `"email"` or `"whatsapp"`.
    pub channel: String,
}

impl SendTokenBody {
    /// Build a request for an explicit recipient and channel.
    ///
    /// Prefer [`email`](Self::email) or [`whatsapp`](Self::whatsapp) unless
    /// the channel is only known at runtime.
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

    /// Deliver the token over WhatsApp, to a phone number in E.164 format.
    pub fn whatsapp<S: Into<String>>(recipient: S) -> Self {
        Self::new(recipient, "whatsapp")
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

    /// Send a signer access token to the signer, by email or WhatsApp.
    ///
    /// `PUT /public/documents/{document_id}/send-token`. Requires no
    /// authentication. The recipient must belong to a signer on the document;
    /// an unknown document answers `404 Documento não encontrado.`
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
    /// { "status": 200, "message": "" }
    /// ```
    pub async fn send_token<S: AsRef<str>>(
        &self,
        document_id: S,
        body: &SendTokenBody,
    ) -> Result<()> {
        let path = self
            .http
            .path(&["public", "documents", document_id.as_ref(), "send-token"])?;
        let req = self.http.request_public(Method::PUT, &path)?.json(body);
        self.http.send_no_content(req).await
    }
}

#[cfg(test)]
mod tests {
    use super::SendTokenBody;

    #[test]
    fn send_token_bodies_carry_the_channel_the_api_requires() {
        // Omitting `channel` is what the published spec describes, and what
        // the live API rejects with `400 O atributo "channel" é obrigatório.`
        for (body, expected_channel) in [
            (SendTokenBody::email("user@example.invalid"), "email"),
            (SendTokenBody::whatsapp("+5511999999999"), "whatsapp"),
        ] {
            let json = serde_json::to_value(&body).unwrap();
            assert_eq!(json["channel"], expected_channel);
            assert_eq!(json["recipient"], body.recipient);
            assert_eq!(json.as_object().unwrap().len(), 2);
        }
    }
}
