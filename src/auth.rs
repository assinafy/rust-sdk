//! Credential handling.

use std::fmt;

use reqwest::RequestBuilder;
use reqwest::header::AUTHORIZATION;

/// Credential used to authenticate API requests.
#[derive(Clone, Default)]
pub enum Auth {
    /// API key sent as `X-Api-Key`. Use for server-to-server integrations.
    ApiKey(String),
    /// Bearer token sent as `Authorization: Bearer <token>`. Returned by
    /// [`AuthApi::login`](crate::resources::AuthApi::login).
    Bearer(String),
    /// Bearer token sent as the `?access-token=<token>` query parameter.
    ///
    /// Prefer [`Auth::Bearer`] for new integrations. This variant exists for
    /// endpoints and deployments that rely on the URL-parameter form documented
    /// by the API.
    AccessToken(String),
    /// Signer access code sent as the `?signer-access-code=<code>` query
    /// parameter. Used by signer-facing endpoints.
    AccessCode(String),
    /// No credential. Only valid for the handful of public endpoints
    /// (e.g. `/login`, `/documents/{hash}/verify`).
    #[default]
    None,
}

impl Auth {
    pub(crate) fn is_none(&self) -> bool {
        matches!(self, Self::None)
    }

    /// Apply the credential to a [`RequestBuilder`].
    pub(crate) fn apply(&self, req: RequestBuilder) -> RequestBuilder {
        match self {
            Auth::ApiKey(k) => req.header("X-Api-Key", k),
            Auth::Bearer(t) => req.header(AUTHORIZATION, format!("Bearer {t}")),
            Auth::AccessToken(t) => req.query(&[("access-token", t)]),
            Auth::AccessCode(c) => req.query(&[("signer-access-code", c)]),
            Auth::None => req,
        }
    }
}

impl fmt::Debug for Auth {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Auth::ApiKey(_) => f.write_str("ApiKey(**redacted**)"),
            Auth::Bearer(_) => f.write_str("Bearer(**redacted**)"),
            Auth::AccessToken(_) => f.write_str("AccessToken(**redacted**)"),
            Auth::AccessCode(_) => f.write_str("AccessCode(**redacted**)"),
            Auth::None => f.write_str("None"),
        }
    }
}
