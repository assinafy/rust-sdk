//! API-key management for the authenticated user.

use std::fmt;

use reqwest::Method;
use serde::{Deserialize, Serialize};

use crate::error::Result;
use crate::http::HttpClient;

/// Payload returned by [`ApiKeysApi::create`] and [`ApiKeysApi::get`].
///
/// On creation the `api_key` field contains the full key once. Subsequent
/// `get` calls return a masked value.
#[derive(Clone, Deserialize)]
#[non_exhaustive]
pub struct ApiKeyResponse {
    /// The API key (full on creation; masked on retrieval).
    ///
    /// This is `None` when the user has not generated an API key yet.
    pub api_key: Option<String>,
}

impl fmt::Debug for ApiKeyResponse {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("ApiKeyResponse")
            .field("api_key", &self.api_key.as_ref().map(|_| "**redacted**"))
            .finish()
    }
}

/// Body for `POST /users/api-keys`.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CreateApiKeyBody {
    /// Current user password. Required before rotating the personal API key.
    pub password: String,
}

impl CreateApiKeyBody {
    /// Build an API-key creation request.
    pub fn new<S: Into<String>>(password: S) -> Self {
        Self {
            password: password.into(),
        }
    }
}

/// API-key endpoints under `/users/api-keys`.
#[derive(Debug)]
pub struct ApiKeysApi<'a> {
    http: &'a HttpClient,
}

impl<'a> ApiKeysApi<'a> {
    pub(crate) fn new(http: &'a HttpClient) -> Self {
        Self { http }
    }

    /// Generate a brand new API key, invalidating any previous one.
    ///
    /// `POST /users/api-keys`.
    pub async fn create(&self, body: &CreateApiKeyBody) -> Result<ApiKeyResponse> {
        let req = self
            .http
            .request(Method::POST, "users/api-keys")?
            .json(body);
        self.http.send_envelope(req).await
    }

    /// Retrieve the (masked) current API key.
    ///
    /// `GET /users/api-keys`.
    pub async fn get(&self) -> Result<ApiKeyResponse> {
        let req = self.http.request(Method::GET, "users/api-keys")?;
        self.http.send_envelope(req).await
    }

    /// Delete the current API key.
    ///
    /// `DELETE /users/api-keys`.
    pub async fn delete(&self) -> Result<()> {
        let req = self.http.request(Method::DELETE, "users/api-keys")?;
        self.http.send_no_content(req).await
    }
}
