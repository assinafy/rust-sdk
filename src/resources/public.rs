//! Public, unauthenticated endpoints.

use reqwest::Method;
use serde::{Deserialize, Serialize};

use crate::error::Result;
use crate::http::HttpClient;
use crate::models::PublicDocument;

/// Body for `PUT /public/documents/{document_id}/send-token`.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SendTokenBody {
    /// Email address or WhatsApp phone number that should receive the token.
    pub recipient: String,
    /// Delivery channel, usually `"email"` or `"whatsapp"`.
    pub channel: String,
}

impl SendTokenBody {
    /// Build a token request for a recipient and channel.
    pub fn new<R, C>(recipient: R, channel: C) -> Self
    where
        R: Into<String>,
        C: Into<String>,
    {
        Self {
            recipient: recipient.into(),
            channel: channel.into(),
        }
    }

    /// Build an email token request.
    pub fn email<S: Into<String>>(recipient: S) -> Self {
        Self::new(recipient, "email")
    }

    /// Build a WhatsApp token request.
    pub fn whatsapp<S: Into<String>>(recipient: S) -> Self {
        Self::new(recipient, "whatsapp")
    }
}

/// Payload returned by `PUT /public/documents/{document_id}/send-token`.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[non_exhaustive]
pub struct SendTokenResult {
    /// Public document metadata returned by the API.
    pub document: PublicDocument,
    /// Delivery channel used.
    pub channel: String,
    /// Recipient that received the token.
    pub recipient: String,
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
    pub async fn document<S: AsRef<str>>(&self, document_id: S) -> Result<PublicDocument> {
        let path = format!("public/documents/{}", document_id.as_ref());
        let req = self.http.request(Method::GET, &path)?;
        self.http.send_envelope(req).await
    }

    /// Send a signer access token by email or WhatsApp.
    ///
    /// `PUT /public/documents/{document_id}/send-token`.
    pub async fn send_token<S: AsRef<str>>(
        &self,
        document_id: S,
        body: &SendTokenBody,
    ) -> Result<SendTokenResult> {
        let path = format!("public/documents/{}/send-token", document_id.as_ref());
        let req = self.http.request(Method::PUT, &path)?.json(body);
        self.http.send_envelope(req).await
    }
}
