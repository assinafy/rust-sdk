//! Embedded signing setup and session endpoints.

use reqwest::Method;
use serde::{Deserialize, Serialize};
use url::Url;

use crate::error::{Error, Result};
use crate::http::HttpClient;
use crate::models::{EmbedOrigin, EmbedSession, EmbedSessionExchange};

/// Body for creating or replacing an allowed embed origin.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EmbedOriginBody {
    /// Exact HTTPS origin, or HTTP on localhost or 127.0.0.1.
    pub origin: String,
}

impl EmbedOriginBody {
    /// Set the origin. The endpoint validates it before sending.
    pub fn new(origin: impl Into<String>) -> Self {
        Self {
            origin: origin.into(),
        }
    }

    fn validate(&self) -> Result<()> {
        let invalid = || {
            Error::Validation("embed origin must be an exact HTTPS origin (HTTP only on localhost or 127.0.0.1), without credentials, wildcard, path, query or fragment".into())
        };
        let raw_authority = self.origin.split_once("://").ok_or_else(invalid)?.1;
        let suffix = raw_authority
            .find(['/', '?', '#'])
            .map(|index| &raw_authority[index..])
            .unwrap_or("");
        if !matches!(suffix, "" | "/")
            || self.origin.chars().any(|c| c.is_whitespace() || c == '\\')
        {
            return Err(invalid());
        }
        let url = Url::parse(&self.origin).map_err(|_| invalid())?;
        let host = url.host_str().ok_or_else(invalid)?;
        if !(url.scheme() == "https"
            || url.scheme() == "http" && matches!(host, "localhost" | "127.0.0.1"))
            || !url.username().is_empty()
            || url.password().is_some()
            || host.contains('*')
            || url.path() != "/"
            || url.query().is_some()
            || url.fragment().is_some()
        {
            return Err(invalid());
        }
        Ok(())
    }
}

/// Optional parent-page context supplied by the hosted embed page.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct ExchangeEmbedSessionBody {
    /// Origin of the page hosting the iframe.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub parent_origin: Option<String>,
}

impl ExchangeEmbedSessionBody {
    /// Record the parent origin.
    pub fn parent_origin(mut self, origin: impl Into<String>) -> Self {
        self.parent_origin = Some(origin.into());
        self
    }
}

/// Workspace origin configuration and short-lived embedded signing sessions.
///
/// Origin management is unavailable to OAuth applications. Session creation
/// requires an embedded signer and an eligible workspace plan. Exchange is
/// for Assinafy's hosted embed page; integrations normally only mint a URL.
#[derive(Debug)]
pub struct EmbeddedSigningApi<'a> {
    http: &'a HttpClient,
}

impl<'a> EmbeddedSigningApi<'a> {
    pub(crate) fn new(http: &'a HttpClient) -> Self {
        Self { http }
    }

    /// List allowed origins, oldest first: `GET /accounts/{accountId}/embed-origins`.
    ///
    /// # Request payload
    /// No body or query parameters.
    ///
    /// # Response payload
    /// ```json
    /// {"status":200,"message":"","data":[{"id":"origin_1","origin":"https://app.example.invalid","created_at":"2026-11-04T12:00:00Z"}]}
    /// ```
    pub async fn list_origins(&self, account_id: &str) -> Result<Vec<EmbedOrigin>> {
        let path = self.http.path(&["accounts", account_id, "embed-origins"])?;
        let req = self.http.request_workspace(Method::GET, &path)?;
        self.http.send_envelope(req).await
    }

    /// Add an origin: `POST /accounts/{accountId}/embed-origins`.
    ///
    /// A workspace accepts up to ten distinct origins. The server normalizes
    /// host casing, the trailing slash and default ports.
    ///
    /// # Request payload
    /// ```json
    /// {"origin":"https://app.example.invalid"}
    /// ```
    /// # Response payload
    /// ```json
    /// {"status":200,"message":"","data":{"id":"origin_1","origin":"https://app.example.invalid","created_at":"2026-11-04T12:00:00Z"}}
    /// ```
    pub async fn create_origin(
        &self,
        account_id: &str,
        body: &EmbedOriginBody,
    ) -> Result<EmbedOrigin> {
        body.validate()?;
        let path = self.http.path(&["accounts", account_id, "embed-origins"])?;
        let req = self.http.request_workspace(Method::POST, &path)?.json(body);
        self.http.send_envelope(req).await
    }

    /// Replace an origin, retaining its ID: `PUT /accounts/{accountId}/embed-origins/{originId}`.
    ///
    /// # Request payload
    /// ```json
    /// {"origin":"https://new.example.invalid"}
    /// ```
    /// # Response payload
    /// ```json
    /// {"status":200,"message":"","data":{"id":"origin_1","origin":"https://new.example.invalid","created_at":"2026-11-04T12:00:00Z"}}
    /// ```
    pub async fn update_origin(
        &self,
        account_id: &str,
        origin_id: &str,
        body: &EmbedOriginBody,
    ) -> Result<EmbedOrigin> {
        body.validate()?;
        let path = self
            .http
            .path(&["accounts", account_id, "embed-origins", origin_id])?;
        let req = self.http.request_workspace(Method::PUT, &path)?.json(body);
        self.http.send_envelope(req).await
    }

    /// Remove an origin: `DELETE /accounts/{accountId}/embed-origins/{originId}`.
    ///
    /// Removing the last origin prevents new embed sessions.
    ///
    /// # Request payload
    /// No body or query parameters.
    ///
    /// # Response payload
    /// ```json
    /// {"status":200,"message":"","data":[]}
    /// ```
    pub async fn delete_origin(&self, account_id: &str, origin_id: &str) -> Result<()> {
        let path = self
            .http
            .path(&["accounts", account_id, "embed-origins", origin_id])?;
        let req = self.http.request_workspace(Method::DELETE, &path)?;
        self.http.send_no_content(req).await
    }

    /// Mint a signing iframe URL: `POST /documents/{documentId}/assignments/{assignmentId}/signers/{signerId}/embed-sessions`.
    ///
    /// Open it within five minutes; opening spends the token. Mint a fresh
    /// session each time the page opens. At most thirty sessions per signer
    /// per hour are allowed. Never log the returned URL.
    ///
    /// # Request payload
    /// No body or query parameters.
    ///
    /// # Response payload
    /// ```json
    /// {"status":200,"message":"","data":{"url":"https://app.example.invalid/embed/example-redacted-token","expires_at":"2026-11-04T12:05:00Z"}}
    /// ```
    pub async fn create_session(
        &self,
        document_id: &str,
        assignment_id: &str,
        signer_id: &str,
    ) -> Result<EmbedSession> {
        let path = self.http.path(&[
            "documents",
            document_id,
            "assignments",
            assignment_id,
            "signers",
            signer_id,
            "embed-sessions",
        ])?;
        let req = self.http.request_workspace(Method::POST, &path)?;
        self.http.send_envelope(req).await
    }

    /// Exchange a token for the hosted embed page: `POST /embed/sessions/{token}/exchange`.
    ///
    /// Integrations normally load the minted URL instead. This public call
    /// uses only its token, never this client's configured credential, and
    /// never retries or follows redirects. A `409` also consumes the token;
    /// expired, unknown or already-used tokens return `410`.
    ///
    /// # Request payload
    /// Optional JSON body; pass `None` to send no body.
    /// ```json
    /// {"parent_origin":"https://app.example.invalid"}
    /// ```
    /// # Response payload
    /// ```json
    /// {"status":200,"message":"","data":{"signer_access_code":"example-redacted-code","signer_id":"signer_1","document_id":"document_1","verification_method":"Email","embed_origins":["https://app.example.invalid"]}}
    /// ```
    pub async fn exchange_session(
        &self,
        token: &str,
        body: Option<&ExchangeEmbedSessionBody>,
    ) -> Result<EmbedSessionExchange> {
        let path = self.http.path(&["embed", "sessions", token, "exchange"])?;
        let mut req = self.http.request_single_send_public(Method::POST, &path)?;
        if let Some(body) = body {
            req = req.json(body);
        }
        self.http.send_envelope(req).await
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn origins_require_exact_secure_origins() {
        for origin in [
            "https://app.example.invalid",
            "https://APP.example.invalid:443/",
            "http://localhost:3000",
            "http://127.0.0.1:3000",
        ] {
            assert!(EmbedOriginBody::new(origin).validate().is_ok(), "{origin}");
        }
        for origin in [
            "http://remote.example.invalid",
            "http://127.0.0.2",
            "http://[::1]",
            "https://*.example.invalid",
            "https://app.example.invalid/path",
            "https://app.example.invalid/path/..",
            "https://app.example.invalid?x=y",
            "https://app.example.invalid#t=secret",
            "https://user:secret@app.example.invalid",
            "not an origin",
        ] {
            assert!(EmbedOriginBody::new(origin).validate().is_err(), "{origin}");
        }
    }
}
