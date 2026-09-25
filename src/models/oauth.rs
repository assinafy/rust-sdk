//! OAuth 2.1 and OpenID Connect models.
//!
//! These payloads are the one part of the API that is **not** wrapped in the
//! usual `{ status, message, data }` envelope: RFC 6749 §5.1, RFC 9728 and
//! OIDC Core §5.3.2 each mandate a flat JSON object, so a standard OAuth
//! client library finds `access_token`, `resource` or `sub` at the top level.

use std::fmt;

use serde::{Deserialize, Serialize};

/// Successful response from `POST /oauth/token`.
///
/// ```json
/// {
///   "access_token": "example-redacted-access-token",
///   "token_type": "Bearer",
///   "expires_in": 3600,
///   "refresh_token": "example-redacted-refresh-token",
///   "scope": "documents:read documents:write",
///   "id_token": "example-redacted-id-token"
/// }
/// ```
///
/// [`Debug`] redacts every token so a logged response cannot leak a
/// credential.
#[derive(Clone, Serialize, Deserialize)]
#[non_exhaustive]
pub struct TokenResponse {
    /// Bearer token to send as `Authorization: Bearer <token>`. Pass it to
    /// [`Auth::Bearer`](crate::Auth::Bearer).
    pub access_token: String,
    /// Token type; always `"Bearer"` in practice.
    #[serde(default)]
    pub token_type: String,
    /// Lifetime of `access_token` in seconds.
    #[serde(default)]
    pub expires_in: Option<u64>,
    /// Refresh token. Present only when the `offline_access` scope was both
    /// requested and consented; otherwise the user must re-authorize once
    /// `access_token` expires.
    ///
    /// Every refresh returns a new one and retires the one it was traded
    /// for: persist it before doing anything else with this response. It is
    /// valid for 30 days, and each refresh starts a fresh 30 days. On a
    /// refresh, [`OAuthApi::token`](crate::resources::OAuthApi::token) only
    /// succeeds when it is present and new.
    #[serde(default)]
    pub refresh_token: Option<String>,
    /// Space-separated scopes carried by `access_token`. `offline_access` is
    /// a request-time signal rather than a permission, so it never appears
    /// here even when it was requested: check
    /// [`refresh_token`](Self::refresh_token) to learn whether one was issued.
    #[serde(default)]
    pub scope: Option<String>,
    /// Signed OIDC ID token (RS256). Present only when the `openid` scope was
    /// granted.
    ///
    /// The SDK does not validate it. Before trusting its claims, verify the
    /// signature with the key from `jwks_uri` whose `kid` matches, and that
    /// `iss` is `https://auth.assinafy.com.br`, `aud` is your client id,
    /// `exp` is in the future and `nonce` matches the one you sent, if any.
    #[serde(default)]
    pub id_token: Option<String>,
}

impl TokenResponse {
    /// Iterates the granted scopes, splitting [`scope`](Self::scope) on
    /// whitespace as RFC 6749 §3.3 specifies.
    ///
    /// ```
    /// # use assinafy::models::TokenResponse;
    /// let token: TokenResponse = serde_json::from_str(
    ///     r#"{"access_token":"t","scope":"documents:read documents:write"}"#,
    /// )
    /// .unwrap();
    /// assert_eq!(
    ///     token.scopes().collect::<Vec<_>>(),
    ///     ["documents:read", "documents:write"]
    /// );
    /// ```
    pub fn scopes(&self) -> impl Iterator<Item = &str> {
        self.scope.as_deref().unwrap_or_default().split_whitespace()
    }

    /// Returns `true` when the token carries `scope`.
    pub fn has_scope(&self, scope: &str) -> bool {
        self.scopes().any(|granted| granted == scope)
    }
}

impl fmt::Debug for TokenResponse {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("TokenResponse")
            .field("access_token", &"**redacted**")
            .field("token_type", &self.token_type)
            .field("expires_in", &self.expires_in)
            .field(
                "refresh_token",
                &self.refresh_token.as_ref().map(|_| "**redacted**"),
            )
            .field("scope", &self.scope)
            .field("id_token", &self.id_token.as_ref().map(|_| "**redacted**"))
            .finish()
    }
}

/// Claims returned by `GET /oauth/userinfo` (OIDC Core §5.3.2).
///
/// ```json
/// {
///   "sub": "d6zqpbyog2v3xvxerwn8la94",
///   "name": "Maria Silva",
///   "email": "user@example.invalid",
///   "email_verified": true
/// }
/// ```
///
/// Only `sub` is always present: `name` requires the `profile` scope and
/// `email`/`email_verified` require the `email` scope.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[non_exhaustive]
pub struct UserInfo {
    /// Stable identifier of the user who authorized the token.
    pub sub: String,
    /// Full name; requires the `profile` scope.
    #[serde(default)]
    pub name: Option<String>,
    /// Email address; requires the `email` scope.
    #[serde(default)]
    pub email: Option<String>,
    /// Whether the email address is verified; requires the `email` scope.
    #[serde(default)]
    pub email_verified: Option<bool>,
}

/// Protected-resource metadata from
/// `GET /.well-known/oauth-protected-resource` (RFC 9728).
///
/// ```json
/// {
///   "resource": "https://api.assinafy.com.br",
///   "authorization_servers": ["https://auth.assinafy.com.br"],
///   "scopes_supported": ["documents:read", "documents:write"],
///   "bearer_methods_supported": ["header"]
/// }
/// ```
///
/// `scopes_supported` deliberately omits `offline_access`: requesting a
/// refresh token is a client concern, not something the resource is protected
/// by.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[non_exhaustive]
pub struct ProtectedResourceMetadata {
    /// Canonical identifier of this API as an OAuth resource. Send it as the
    /// RFC 8707 `resource` indicator.
    pub resource: String,
    /// Authorization servers that can issue tokens for this resource. Fetch
    /// the first entry's RFC 8414 metadata to continue discovery.
    #[serde(default)]
    pub authorization_servers: Vec<String>,
    /// Scopes this resource accepts.
    #[serde(default)]
    pub scopes_supported: Vec<String>,
    /// How a bearer token may be presented (`"header"`).
    #[serde(default)]
    pub bearer_methods_supported: Vec<String>,
}

/// Authorization-server metadata from
/// `GET {issuer}/.well-known/oauth-authorization-server` (RFC 8414).
///
/// ```json
/// {
///   "issuer": "https://auth.assinafy.com.br",
///   "authorization_endpoint": "https://auth.assinafy.com.br/oauth/authorize",
///   "token_endpoint": "https://api.assinafy.com.br/v1/oauth/token",
///   "revocation_endpoint": "https://api.assinafy.com.br/v1/oauth/revoke",
///   "userinfo_endpoint": "https://api.assinafy.com.br/v1/oauth/userinfo",
///   "jwks_uri": "https://auth.assinafy.com.br/.well-known/jwks.json",
///   "scopes_supported": ["documents:read", "openid"],
///   "response_types_supported": ["code"],
///   "grant_types_supported": ["authorization_code", "refresh_token"],
///   "code_challenge_methods_supported": ["S256"],
///   "token_endpoint_auth_methods_supported": ["client_secret_post", "none"],
///   "authorization_response_iss_parameter_supported": true,
///   "client_id_metadata_document_supported": true
/// }
/// ```
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[non_exhaustive]
pub struct AuthorizationServerMetadata {
    /// Issuer identifier.
    pub issuer: String,
    /// Browser-facing endpoint the user is redirected to. Build the redirect
    /// with [`AuthorizationRequest`](crate::resources::AuthorizationRequest).
    pub authorization_endpoint: String,
    /// Endpoint that exchanges a code or refresh token for an access token.
    pub token_endpoint: String,
    /// Token-revocation endpoint (RFC 7009).
    #[serde(default)]
    pub revocation_endpoint: Option<String>,
    /// OIDC userinfo endpoint.
    #[serde(default)]
    pub userinfo_endpoint: Option<String>,
    /// JSON Web Key Set used to verify `id_token` signatures.
    #[serde(default)]
    pub jwks_uri: Option<String>,
    /// Scopes the server can issue.
    #[serde(default)]
    pub scopes_supported: Vec<String>,
    /// Supported `response_type` values (`"code"`).
    #[serde(default)]
    pub response_types_supported: Vec<String>,
    /// Supported grant types.
    #[serde(default)]
    pub grant_types_supported: Vec<String>,
    /// Supported PKCE challenge methods (`"S256"`).
    #[serde(default)]
    pub code_challenge_methods_supported: Vec<String>,
    /// Supported client-authentication methods at the token endpoint.
    #[serde(default)]
    pub token_endpoint_auth_methods_supported: Vec<String>,
    /// Whether authorization responses carry the `iss` parameter (RFC 9207).
    #[serde(default)]
    pub authorization_response_iss_parameter_supported: bool,
    /// Whether a client may identify itself with a client-ID metadata
    /// document URL instead of a pre-registered identifier.
    #[serde(default)]
    pub client_id_metadata_document_supported: bool,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn token_response_debug_redacts_every_credential() {
        let token = TokenResponse {
            access_token: "sentinel-access".into(),
            token_type: "Bearer".into(),
            expires_in: Some(3600),
            refresh_token: Some("sentinel-refresh".into()),
            scope: Some("documents:read".into()),
            id_token: Some("sentinel-id".into()),
        };
        let rendered = format!("{token:?}");
        for secret in ["sentinel-access", "sentinel-refresh", "sentinel-id"] {
            assert!(!rendered.contains(secret), "{rendered} leaked {secret}");
        }
        assert!(rendered.contains("documents:read"));
    }

    #[test]
    fn token_response_decodes_a_minimal_body_and_reports_scopes() {
        let token: TokenResponse =
            serde_json::from_str(r#"{"access_token":"t","token_type":"Bearer"}"#).unwrap();
        assert_eq!(token.expires_in, None);
        assert_eq!(token.refresh_token, None);
        assert_eq!(token.scopes().count(), 0);
        assert!(!token.has_scope("documents:read"));
    }
}
