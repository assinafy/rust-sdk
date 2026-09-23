//! OAuth 2.1 authorization-code flow with PKCE.

use std::fmt;

use reqwest::Method;
use serde::Serialize;
use url::Url;

use crate::config::validate_https_url;
use crate::error::{Error, Result};
use crate::http::HttpClient;
use crate::models::{
    AuthorizationServerMetadata, ProtectedResourceMetadata, TokenResponse, UserInfo,
};

/// Scopes a user can approve, as published by
/// [`ProtectedResourceMetadata::scopes_supported`].
///
/// Ask for the narrowest set the integration needs: a token that never writes
/// should not carry `documents:write`.
pub mod scope {
    /// Read documents, their pages, tags, signers, assignments and activity.
    pub const DOCUMENTS_READ: &str = "documents:read";
    /// Create, update and delete documents, and manage their signers,
    /// assignments and activity.
    pub const DOCUMENTS_WRITE: &str = "documents:write";
    /// Read reusable document templates, their pages, roles, fields and tags.
    pub const TEMPLATES_READ: &str = "templates:read";
    /// Create, update and delete templates, their pages, roles, fields and
    /// tags.
    pub const TEMPLATES_WRITE: &str = "templates:write";
    /// Read the workspace's profile, theme and logo.
    pub const ACCOUNT_READ: &str = "account:read";
    /// Configure and deactivate the workspace webhook subscription.
    pub const WEBHOOKS_WRITE: &str = "webhooks:write";
    /// Identify the authenticated user and enable `GET /oauth/userinfo`.
    pub const OPENID: &str = "openid";
    /// Include the user's name in the `id_token`/userinfo claims.
    pub const PROFILE: &str = "profile";
    /// Include the user's email and its verification status in the
    /// `id_token`/userinfo claims.
    pub const EMAIL: &str = "email";
    /// Request a refresh token, so the app keeps working after the user's
    /// session expires without prompting them again. Granted only to a client
    /// that explicitly asks for it, and never echoed back in the access
    /// token's own `scope`.
    pub const OFFLINE_ACCESS: &str = "offline_access";
}

/// RFC 7636 PKCE verifier and its S256 challenge.
///
/// PKCE is **mandatory**: the authorization server publishes `S256` as the
/// only supported challenge method. Keep the pair for the lifetime of one
/// authorization: the challenge goes to the authorization endpoint and the
/// verifier to the token endpoint, which rejects a mismatch with
/// `invalid_grant`.
///
/// ```
/// # use assinafy::resources::PkceChallenge;
/// // RFC 7636 Appendix B test vector.
/// let pkce = PkceChallenge::from_verifier("dBjftJeZ4CVP-mB92K27uhbUJU1p1r_wW1gFWFOEjXk").unwrap();
/// assert_eq!(pkce.challenge(), "E9Melhoa2OwvFrEMTJguCHaoeK1t8URWbuGJSstw-cM");
/// assert_eq!(PkceChallenge::METHOD, "S256");
/// ```
///
/// [`Debug`] redacts the verifier, which is a one-time secret.
#[derive(Clone)]
pub struct PkceChallenge {
    verifier: String,
    challenge: String,
}

impl PkceChallenge {
    /// The only challenge method this authorization server supports.
    pub const METHOD: &'static str = "S256";

    /// Generate a pair from 32 bytes of operating-system entropy, yielding
    /// the RFC 7636 recommended 43-character verifier.
    pub fn generate() -> Result<Self> {
        let mut entropy = [0_u8; 32];
        getrandom::fill(&mut entropy)
            .map_err(|e| Error::Config(format!("failed to read system entropy: {e}")))?;
        Self::from_verifier(base64url(&entropy))
    }

    /// Build a pair from a verifier you already hold — for instance one
    /// carried across processes between the redirect and the callback.
    ///
    /// The verifier must match the RFC 7636 grammar: 43 to 128 characters
    /// drawn from `A-Z`, `a-z`, `0-9`, `-`, `.`, `_` and `~`. The token
    /// endpoint rejects anything else with `invalid_grant`, so this checks it
    /// up front.
    pub fn from_verifier<S: Into<String>>(verifier: S) -> Result<Self> {
        let verifier = verifier.into();
        let valid = (43..=128).contains(&verifier.len())
            && verifier
                .bytes()
                .all(|b| b.is_ascii_alphanumeric() || matches!(b, b'-' | b'.' | b'_' | b'~'));
        if !valid {
            return Err(Error::Config(
                "PKCE code verifier must be 43-128 characters of [A-Za-z0-9-._~]".into(),
            ));
        }
        let challenge = base64url(&<sha2::Sha256 as sha2::Digest>::digest(verifier.as_bytes()));
        Ok(Self {
            verifier,
            challenge,
        })
    }

    /// The `code_verifier` sent to the token endpoint.
    pub fn verifier(&self) -> &str {
        &self.verifier
    }

    /// The `code_challenge` sent to the authorization endpoint.
    pub fn challenge(&self) -> &str {
        &self.challenge
    }
}

impl fmt::Debug for PkceChallenge {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("PkceChallenge")
            .field("verifier", &"**redacted**")
            .field("challenge", &self.challenge)
            .finish()
    }
}

/// Encode `input` as unpadded base64url (RFC 4648 §5), the encoding RFC 7636
/// mandates for both the verifier and the challenge.
fn base64url(input: &[u8]) -> String {
    const ALPHABET: &[u8; 64] = b"ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789-_";
    let mut out = String::with_capacity(input.len().div_ceil(3) * 4);
    for chunk in input.chunks(3) {
        let bits = u32::from(chunk[0]) << 16
            | u32::from(chunk.get(1).copied().unwrap_or(0)) << 8
            | u32::from(chunk.get(2).copied().unwrap_or(0));
        // A 3-byte chunk encodes to 4 characters, 2 bytes to 3, 1 byte to 2;
        // the padding RFC 4648 would add is omitted.
        for shift in [18, 12, 6, 0].iter().take(chunk.len() + 1) {
            out.push(ALPHABET[(bits >> shift & 0b111111) as usize] as char);
        }
    }
    out
}

/// Builder for the browser URL that starts the authorization-code flow.
///
/// This performs no request: [`url`](Self::url) returns the address to
/// redirect the user's browser to.
#[derive(Debug, Clone)]
pub struct AuthorizationRequest {
    client_id: String,
    redirect_uri: String,
    code_challenge: String,
    scopes: Vec<String>,
    state: Option<String>,
    resource: Option<String>,
}

impl AuthorizationRequest {
    /// Start an authorization request for `client_id`, returning the user to
    /// `redirect_uri` and binding the flow to `pkce`.
    pub fn new<C, R>(client_id: C, redirect_uri: R, pkce: &PkceChallenge) -> Self
    where
        C: Into<String>,
        R: Into<String>,
    {
        Self {
            client_id: client_id.into(),
            redirect_uri: redirect_uri.into(),
            code_challenge: pkce.challenge().to_owned(),
            scopes: Vec::new(),
            state: None,
            resource: None,
        }
    }

    /// Request one scope; call repeatedly to add more. See [`scope`].
    pub fn scope<S: Into<String>>(mut self, scope: S) -> Self {
        self.scopes.push(scope.into());
        self
    }

    /// Request several scopes at once, replacing any set so far.
    pub fn scopes<I, S>(mut self, scopes: I) -> Self
    where
        I: IntoIterator<Item = S>,
        S: Into<String>,
    {
        self.scopes = scopes.into_iter().map(Into::into).collect();
        self
    }

    /// Set the opaque `state` value echoed back to the redirect URI. Use it
    /// to bind the callback to the browser session that started the flow.
    pub fn state<S: Into<String>>(mut self, state: S) -> Self {
        self.state = Some(state.into());
        self
    }

    /// Set the RFC 8707 `resource` indicator, which must be the `resource`
    /// value from [`ProtectedResourceMetadata`]. When present here it must
    /// also be sent to the token endpoint via [`TokenRequest::resource`], or
    /// the exchange fails with `invalid_target`.
    pub fn resource<S: Into<String>>(mut self, resource: S) -> Self {
        self.resource = Some(resource.into());
        self
    }

    /// Build the redirect URL against `authorization_endpoint` — the value
    /// [`AuthorizationServerMetadata::authorization_endpoint`] published.
    ///
    /// The endpoint must be an HTTPS URL without a query string or fragment
    /// (loopback HTTP is accepted for local development).
    pub fn url(&self, authorization_endpoint: &str) -> Result<String> {
        let mut url = Url::parse(authorization_endpoint)
            .map_err(|e| Error::Config(format!("invalid OAuth authorization endpoint: {e}")))?;
        validate_https_url(&url)?;
        {
            let mut query = url.query_pairs_mut();
            query.append_pair("response_type", "code");
            query.append_pair("client_id", &self.client_id);
            query.append_pair("redirect_uri", &self.redirect_uri);
            query.append_pair("code_challenge", &self.code_challenge);
            query.append_pair("code_challenge_method", PkceChallenge::METHOD);
            if !self.scopes.is_empty() {
                query.append_pair("scope", &self.scopes.join(" "));
            }
            if let Some(state) = &self.state {
                query.append_pair("state", state);
            }
            if let Some(resource) = &self.resource {
                query.append_pair("resource", resource);
            }
        }
        Ok(url.into())
    }
}

/// `POST /oauth/token` request body (RFC 6749 §5.1).
///
/// Build it with [`authorization_code`](Self::authorization_code) or
/// [`refresh_token`](Self::refresh_token); [`Debug`] redacts every secret it
/// carries.
#[derive(Clone, Serialize)]
pub struct TokenRequest {
    grant_type: &'static str,
    #[serde(skip_serializing_if = "Option::is_none")]
    code: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    redirect_uri: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    code_verifier: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    refresh_token: Option<String>,
    client_id: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    client_secret: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    resource: Option<String>,
}

impl TokenRequest {
    /// Exchange the authorization code the browser returned for an access
    /// token.
    ///
    /// `redirect_uri` must be byte-identical to the one sent to the
    /// authorization endpoint, and `pkce` must be the pair whose challenge
    /// started the flow.
    pub fn authorization_code<C, O, R>(
        client_id: C,
        code: O,
        redirect_uri: R,
        pkce: &PkceChallenge,
    ) -> Self
    where
        C: Into<String>,
        O: Into<String>,
        R: Into<String>,
    {
        Self {
            grant_type: "authorization_code",
            code: Some(code.into()),
            redirect_uri: Some(redirect_uri.into()),
            code_verifier: Some(pkce.verifier().to_owned()),
            refresh_token: None,
            client_id: client_id.into(),
            client_secret: None,
            resource: None,
        }
    }

    /// Trade a refresh token for a fresh access token.
    ///
    /// Only a client that requested and was granted
    /// [`scope::OFFLINE_ACCESS`] holds one.
    pub fn refresh_token<C, T>(client_id: C, refresh_token: T) -> Self
    where
        C: Into<String>,
        T: Into<String>,
    {
        Self {
            grant_type: "refresh_token",
            code: None,
            redirect_uri: None,
            code_verifier: None,
            refresh_token: Some(refresh_token.into()),
            client_id: client_id.into(),
            client_secret: None,
            resource: None,
        }
    }

    /// Authenticate a confidential client. Public clients authenticate with
    /// PKCE alone and are never issued a secret.
    pub fn client_secret<S: Into<String>>(mut self, client_secret: S) -> Self {
        self.client_secret = Some(client_secret.into());
        self
    }

    /// Set the RFC 8707 `resource` indicator. It must match the value sent to
    /// the authorization endpoint, otherwise the exchange fails with
    /// `invalid_target`.
    pub fn resource<S: Into<String>>(mut self, resource: S) -> Self {
        self.resource = Some(resource.into());
        self
    }
}

impl fmt::Debug for TokenRequest {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("TokenRequest")
            .field("grant_type", &self.grant_type)
            .field("code", &self.code.as_ref().map(|_| "**redacted**"))
            .field("redirect_uri", &self.redirect_uri)
            .field(
                "code_verifier",
                &self.code_verifier.as_ref().map(|_| "**redacted**"),
            )
            .field(
                "refresh_token",
                &self.refresh_token.as_ref().map(|_| "**redacted**"),
            )
            .field("client_id", &self.client_id)
            .field(
                "client_secret",
                &self.client_secret.as_ref().map(|_| "**redacted**"),
            )
            .field("resource", &self.resource)
            .finish()
    }
}

/// `POST /oauth/revoke` request body (RFC 7009).
///
/// [`Debug`] redacts the token and the client secret.
#[derive(Clone, Serialize)]
pub struct RevokeRequest {
    token: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    token_type_hint: Option<&'static str>,
    client_id: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    client_secret: Option<String>,
}

impl RevokeRequest {
    /// Revoke an access token.
    pub fn access_token<C, T>(client_id: C, token: T) -> Self
    where
        C: Into<String>,
        T: Into<String>,
    {
        Self::new(client_id, token, Some("access_token"))
    }

    /// Revoke a refresh token, which also invalidates the access tokens
    /// issued from it.
    pub fn refresh_token<C, T>(client_id: C, token: T) -> Self
    where
        C: Into<String>,
        T: Into<String>,
    {
        Self::new(client_id, token, Some("refresh_token"))
    }

    fn new<C: Into<String>, T: Into<String>>(
        client_id: C,
        token: T,
        hint: Option<&'static str>,
    ) -> Self {
        Self {
            token: token.into(),
            token_type_hint: hint,
            client_id: client_id.into(),
            client_secret: None,
        }
    }

    /// Authenticate a confidential client.
    pub fn client_secret<S: Into<String>>(mut self, client_secret: S) -> Self {
        self.client_secret = Some(client_secret.into());
        self
    }
}

impl fmt::Debug for RevokeRequest {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("RevokeRequest")
            .field("token", &"**redacted**")
            .field("token_type_hint", &self.token_type_hint)
            .field("client_id", &self.client_id)
            .field(
                "client_secret",
                &self.client_secret.as_ref().map(|_| "**redacted**"),
            )
            .finish()
    }
}

/// OAuth 2.1 authorization-code flow with PKCE.
///
/// Use this when an application acts **in a user's workspace with that
/// user's permission**, as opposed to [`Auth::ApiKey`](crate::Auth::ApiKey)
/// and [`Auth::Bearer`](crate::Auth::Bearer), which authenticate the
/// workspace or the user directly. An OAuth access token carries only the
/// scopes the user approved, is scoped to one workspace, and can never reach
/// billing, account lifecycle, credential management or admin surfaces —
/// whatever its scopes.
///
/// The flow has five steps; the SDK covers all of them except the browser
/// redirect, which only the user's browser can perform:
///
/// 1. Discover the endpoints — [`OAuthApi::protected_resource_metadata`],
///    then [`OAuthApi::authorization_server_metadata`].
/// 2. Create a PKCE pair with [`PkceChallenge::generate`] and send the user
///    to the URL from [`AuthorizationRequest::url`].
/// 3. The user approves; the browser returns to your `redirect_uri` with
///    `?code=…&state=…`.
/// 4. Exchange the code with [`OAuthApi::token`] and
///    [`TokenRequest::authorization_code`].
/// 5. Call the API with [`Auth::Bearer`](crate::Auth::Bearer), refresh via
///    [`TokenRequest::refresh_token`], and revoke with [`OAuthApi::revoke`].
///
/// ```no_run
/// use assinafy::Client;
/// use assinafy::resources::{AuthorizationRequest, PkceChallenge, TokenRequest, scope};
///
/// # async fn run() -> assinafy::Result<()> {
/// let client = Client::builder().build()?;
///
/// // 1. Discovery.
/// let resource = client.oauth().protected_resource_metadata().await?;
/// let server = client
///     .oauth()
///     .authorization_server_metadata(&resource.authorization_servers[0])
///     .await?;
///
/// // 2. Send the user to the authorization endpoint.
/// let pkce = PkceChallenge::generate()?;
/// let authorize = AuthorizationRequest::new("my-client-id", "https://app.example/callback", &pkce)
///     .scopes([scope::DOCUMENTS_READ, scope::DOCUMENTS_WRITE])
///     .state("opaque-csrf-token")
///     .resource(&resource.resource)
///     .url(&server.authorization_endpoint)?;
/// println!("open {authorize}");
///
/// // 4. Exchange the code the browser came back with.
/// let token = client
///     .oauth()
///     .token(&TokenRequest::authorization_code(
///         "my-client-id",
///         "code-from-the-redirect",
///         "https://app.example/callback",
///         &pkce,
///     ))
///     .await?;
///
/// // 5. Use it.
/// let api = client.with_auth(assinafy::Auth::Bearer(token.access_token.clone()));
/// let who = api.oauth().userinfo().await?;
/// println!("acting for {}", who.sub);
/// # Ok(()) }
/// ```
///
/// # Availability
///
/// The OAuth endpoints are served by **production only**. Other deployments
/// answer `404 Página não encontrada.` and do not publish
/// `/.well-known/oauth-protected-resource` at all.
#[derive(Debug)]
pub struct OAuthApi<'a> {
    http: &'a HttpClient,
}

impl<'a> OAuthApi<'a> {
    pub(crate) fn new(http: &'a HttpClient) -> Self {
        Self { http }
    }

    /// Fetch this API's protected-resource metadata — step one of discovery.
    ///
    /// `GET /.well-known/oauth-protected-resource` (RFC 9728). The document
    /// lives at the API **origin**, not under the versioned base path, and is
    /// unauthenticated.
    ///
    /// # Response payload
    ///
    /// Per RFC 8615 the body is the bare metadata object, not this API's
    /// `{ status, message, data }` envelope.
    ///
    /// ```json
    /// {
    ///   "resource": "https://api.assinafy.com.br",
    ///   "authorization_servers": ["https://auth.assinafy.com.br"],
    ///   "scopes_supported": [
    ///     "documents:read",
    ///     "documents:write",
    ///     "templates:read",
    ///     "templates:write",
    ///     "account:read",
    ///     "openid",
    ///     "profile",
    ///     "email"
    ///   ],
    ///   "bearer_methods_supported": ["header"]
    /// }
    /// ```
    pub async fn protected_resource_metadata(&self) -> Result<ProtectedResourceMetadata> {
        let url = self.http.well_known_url("oauth-protected-resource")?;
        let req = self.http.request_absolute_public(Method::GET, url)?;
        self.http.send_data(req).await
    }

    /// Fetch the authorization server's metadata — step two of discovery.
    ///
    /// `GET {issuer}/.well-known/oauth-authorization-server` (RFC 8414). Pass
    /// an entry from
    /// [`ProtectedResourceMetadata::authorization_servers`]; the request goes
    /// to that host, which is **not** the API host, and carries no
    /// credentials.
    ///
    /// # Response payload
    ///
    /// ```json
    /// {
    ///   "issuer": "https://auth.assinafy.com.br",
    ///   "authorization_endpoint": "https://auth.assinafy.com.br/oauth/authorize",
    ///   "token_endpoint": "https://api.assinafy.com.br/v1/oauth/token",
    ///   "revocation_endpoint": "https://api.assinafy.com.br/v1/oauth/revoke",
    ///   "userinfo_endpoint": "https://api.assinafy.com.br/v1/oauth/userinfo",
    ///   "jwks_uri": "https://auth.assinafy.com.br/.well-known/jwks.json",
    ///   "scopes_supported": [
    ///     "documents:read",
    ///     "documents:write",
    ///     "templates:read",
    ///     "templates:write",
    ///     "account:read",
    ///     "openid",
    ///     "profile",
    ///     "email",
    ///     "offline_access"
    ///   ],
    ///   "response_types_supported": ["code"],
    ///   "grant_types_supported": ["authorization_code", "refresh_token"],
    ///   "code_challenge_methods_supported": ["S256"],
    ///   "token_endpoint_auth_methods_supported": ["client_secret_post", "none"],
    ///   "authorization_response_iss_parameter_supported": true,
    ///   "client_id_metadata_document_supported": true
    /// }
    /// ```
    pub async fn authorization_server_metadata<S: AsRef<str>>(
        &self,
        issuer: S,
    ) -> Result<AuthorizationServerMetadata> {
        let issuer = Url::parse(issuer.as_ref())
            .map_err(|e| Error::Config(format!("invalid OAuth issuer URL: {e}")))?;
        validate_https_url(&issuer)?;
        let url = issuer.join("/.well-known/oauth-authorization-server")?;
        let req = self.http.request_absolute_public(Method::GET, url)?;
        self.http.send_data(req).await
    }

    /// Exchange an authorization code or a refresh token for an access token.
    ///
    /// `POST /oauth/token`. Unauthenticated: the client identifies itself in
    /// the body, with PKCE (public clients) or a client secret (confidential
    /// clients).
    ///
    /// # Request payload
    ///
    /// ```json
    /// {
    ///   "grant_type": "authorization_code",
    ///   "code": "example-redacted-authorization-code",
    ///   "redirect_uri": "https://app.example.invalid/callback",
    ///   "code_verifier": "dBjftJeZ4CVP-mB92K27uhbUJU1p1r_wW1gFWFOEjXk",
    ///   "client_id": "my-client-id"
    /// }
    /// ```
    ///
    /// # Response payload
    ///
    /// Per RFC 6749 §5.1 the body is flat, not enveloped.
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
    /// # Errors
    ///
    /// Failures are flat too — `{ "error": "invalid_grant",
    /// "error_description": "…" }` — and surface as
    /// [`Error::Api`](crate::Error::Api) with the code available from
    /// [`ApiError::oauth_error`](crate::ApiError::oauth_error): `invalid_grant`
    /// (bad, expired, replayed or wrong-client code; a `code_verifier` outside
    /// the RFC 7636 grammar; a `redirect_uri` mismatch; a refresh token whose
    /// authorization no longer includes `offline_access`), `invalid_target`,
    /// `unsupported_grant_type`, or `invalid_client` on a `401`.
    pub async fn token(&self, body: &TokenRequest) -> Result<TokenResponse> {
        let req = self
            .http
            .request_public(Method::POST, "oauth/token")?
            .json(body);
        self.http.send_data(req).await
    }

    /// Revoke an access or refresh token.
    ///
    /// `POST /oauth/revoke` (RFC 7009). Every token outcome answers `200` —
    /// including a token that does not exist, is already revoked, or is
    /// malformed — so the endpoint cannot be used to probe whether a token
    /// exists. Only failed client authentication returns `401`
    /// `invalid_client`.
    ///
    /// # Request payload
    ///
    /// ```json
    /// {
    ///   "token": "example-redacted-refresh-token",
    ///   "token_type_hint": "refresh_token",
    ///   "client_id": "my-client-id"
    /// }
    /// ```
    ///
    /// # Response payload
    ///
    /// Empty.
    pub async fn revoke(&self, body: &RevokeRequest) -> Result<()> {
        let req = self
            .http
            .request_public(Method::POST, "oauth/revoke")?
            .json(body);
        self.http.send_no_content(req).await
    }

    /// Read the claims of the user who authorized the current token.
    ///
    /// `GET /oauth/userinfo`. Requires the [`scope::OPENID`] scope;
    /// [`name`](UserInfo::name) additionally requires [`scope::PROFILE`] and
    /// [`email`](UserInfo::email) requires [`scope::EMAIL`]. Call it on a
    /// client carrying the OAuth token — see
    /// [`Client::with_auth`](crate::Client::with_auth).
    ///
    /// # Response payload
    ///
    /// Per OIDC Core §5.3.2 the body is a flat claims object, not enveloped.
    ///
    /// ```json
    /// {
    ///   "sub": "d6zqpbyog2v3xvxerwn8la94",
    ///   "name": "Maria Silva",
    ///   "email": "user@example.invalid",
    ///   "email_verified": true
    /// }
    /// ```
    pub async fn userinfo(&self) -> Result<UserInfo> {
        let req = self.http.request(Method::GET, "oauth/userinfo")?;
        self.http.send_data(req).await
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn base64url_matches_rfc_4648_vectors_without_padding() {
        // RFC 4648 §10 vectors, base64url-encoded without `=` padding.
        for (input, expected) in [
            ("", ""),
            ("f", "Zg"),
            ("fo", "Zm8"),
            ("foo", "Zm9v"),
            ("foob", "Zm9vYg"),
            ("fooba", "Zm9vYmE"),
            ("foobar", "Zm9vYmFy"),
        ] {
            assert_eq!(base64url(input.as_bytes()), expected, "input {input:?}");
        }
        // The URL-safe alphabet uses `-` and `_` where base64 uses `+` and `/`.
        assert_eq!(base64url(&[0xfb, 0xff, 0xfe]), "-__-");
    }

    #[test]
    fn generated_pkce_pairs_are_valid_and_distinct() {
        let first = PkceChallenge::generate().unwrap();
        let second = PkceChallenge::generate().unwrap();
        assert_eq!(first.verifier().len(), 43);
        assert_eq!(first.challenge().len(), 43);
        assert_ne!(first.verifier(), second.verifier());
        // A generated verifier must round-trip through the RFC 7636 grammar.
        let rebuilt = PkceChallenge::from_verifier(first.verifier()).unwrap();
        assert_eq!(rebuilt.challenge(), first.challenge());
    }

    #[test]
    fn pkce_rejects_verifiers_outside_the_rfc_7636_grammar() {
        for invalid in [
            "too-short",
            &"a".repeat(42),
            &"a".repeat(129),
            &format!("{}+", "a".repeat(42)),
            &format!("{} ", "a".repeat(42)),
        ] {
            assert!(
                matches!(PkceChallenge::from_verifier(invalid), Err(Error::Config(_))),
                "accepted invalid verifier {invalid:?}"
            );
        }
        assert!(PkceChallenge::from_verifier("a".repeat(43)).is_ok());
    }

    #[test]
    fn pkce_debug_redacts_the_verifier() {
        let pkce = PkceChallenge::from_verifier("a".repeat(43)).unwrap();
        let rendered = format!("{pkce:?}");
        assert!(!rendered.contains(pkce.verifier()));
        assert!(rendered.contains(pkce.challenge()));
    }

    #[test]
    fn authorization_url_carries_every_required_parameter() {
        let pkce =
            PkceChallenge::from_verifier("dBjftJeZ4CVP-mB92K27uhbUJU1p1r_wW1gFWFOEjXk").unwrap();
        let url = AuthorizationRequest::new("client-id", "https://app.example.invalid/cb", &pkce)
            .scopes([scope::DOCUMENTS_READ, scope::WEBHOOKS_WRITE, scope::OPENID])
            .state("csrf-state")
            .resource("https://api.assinafy.com.br")
            .url("https://auth.assinafy.com.br/oauth/authorize")
            .unwrap();
        let parsed = Url::parse(&url).unwrap();
        let query: std::collections::BTreeMap<_, _> = parsed.query_pairs().into_owned().collect();
        assert_eq!(parsed.path(), "/oauth/authorize");
        assert_eq!(query["response_type"], "code");
        assert_eq!(query["client_id"], "client-id");
        assert_eq!(query["redirect_uri"], "https://app.example.invalid/cb");
        assert_eq!(query["code_challenge"], pkce.challenge());
        assert_eq!(query["code_challenge_method"], "S256");
        assert_eq!(query["scope"], "documents:read webhooks:write openid");
        assert_eq!(query["state"], "csrf-state");
        assert_eq!(query["resource"], "https://api.assinafy.com.br");
    }

    #[test]
    fn authorization_url_rejects_non_https_endpoints() {
        let pkce = PkceChallenge::from_verifier("a".repeat(43)).unwrap();
        let request =
            AuthorizationRequest::new("client-id", "https://app.example.invalid/cb", &pkce);
        for endpoint in [
            "http://auth.example.invalid/oauth/authorize",
            "ftp://auth.example.invalid/oauth/authorize",
            "not-a-url",
        ] {
            assert!(
                matches!(request.url(endpoint), Err(Error::Config(_))),
                "accepted endpoint {endpoint}"
            );
        }
        // Loopback HTTP stays available for local development.
        assert!(request.url("http://127.0.0.1:8080/authorize").is_ok());
    }

    #[test]
    fn token_requests_serialize_only_the_fields_their_grant_uses() {
        let pkce =
            PkceChallenge::from_verifier("dBjftJeZ4CVP-mB92K27uhbUJU1p1r_wW1gFWFOEjXk").unwrap();
        let code = TokenRequest::authorization_code(
            "client-id",
            "auth-code",
            "https://app.example.invalid/cb",
            &pkce,
        )
        .resource("https://api.assinafy.com.br");
        assert_eq!(
            serde_json::to_value(&code).unwrap(),
            serde_json::json!({
                "grant_type": "authorization_code",
                "code": "auth-code",
                "redirect_uri": "https://app.example.invalid/cb",
                "code_verifier": "dBjftJeZ4CVP-mB92K27uhbUJU1p1r_wW1gFWFOEjXk",
                "client_id": "client-id",
                "resource": "https://api.assinafy.com.br"
            })
        );

        let refresh =
            TokenRequest::refresh_token("client-id", "refresh-value").client_secret("shh");
        assert_eq!(
            serde_json::to_value(&refresh).unwrap(),
            serde_json::json!({
                "grant_type": "refresh_token",
                "refresh_token": "refresh-value",
                "client_id": "client-id",
                "client_secret": "shh"
            })
        );
    }

    #[test]
    fn oauth_request_debug_output_redacts_secrets() {
        let pkce =
            PkceChallenge::from_verifier("dBjftJeZ4CVP-mB92K27uhbUJU1p1r_wW1gFWFOEjXk").unwrap();
        let token = TokenRequest::authorization_code(
            "client-id",
            "sentinel-code",
            "https://app.example.invalid/cb",
            &pkce,
        )
        .client_secret("sentinel-secret");
        let rendered = format!("{token:?}");
        for secret in ["sentinel-code", "sentinel-secret", pkce.verifier()] {
            assert!(!rendered.contains(secret), "{rendered} leaked {secret}");
        }

        let revoke = RevokeRequest::refresh_token("client-id", "sentinel-token")
            .client_secret("sentinel-secret");
        let rendered = format!("{revoke:?}");
        assert!(!rendered.contains("sentinel-token"));
        assert!(!rendered.contains("sentinel-secret"));
        assert_eq!(
            serde_json::to_value(&revoke).unwrap(),
            serde_json::json!({
                "token": "sentinel-token",
                "token_type_hint": "refresh_token",
                "client_id": "client-id",
                "client_secret": "sentinel-secret"
            })
        );
    }
}
