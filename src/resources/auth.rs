//! Authentication helpers: login, password management, social login.

use reqwest::Method;
use serde::{Deserialize, Serialize};

use crate::error::Result;
use crate::http::HttpClient;
use crate::models::LoginResult;

/// `POST /login` request body.
#[derive(Clone, Serialize, Deserialize)]
pub struct LoginBody {
    /// User email.
    pub email: String,
    /// User password.
    pub password: String,
}

impl LoginBody {
    /// Build a login request.
    pub fn new<E, P>(email: E, password: P) -> Self
    where
        E: Into<String>,
        P: Into<String>,
    {
        Self {
            email: email.into(),
            password: password.into(),
        }
    }
}

/// `POST /authentication/social-login` request body.
#[derive(Clone, Serialize, Deserialize)]
pub struct SocialLoginBody {
    /// Provider identifier. Currently only `"google"` is supported.
    pub provider: String,
    /// The access token or ID token obtained from the social-login provider.
    pub token: String,
    /// Whether the user accepted Assinafy's terms during the provider flow.
    pub has_accepted_terms: bool,
}

impl SocialLoginBody {
    /// Build a social-login request.
    pub fn new<P, T>(provider: P, token: T, accepted_terms: bool) -> Self
    where
        P: Into<String>,
        T: Into<String>,
    {
        Self {
            provider: provider.into(),
            token: token.into(),
            has_accepted_terms: accepted_terms,
        }
    }

    /// Build a Google social-login request.
    pub fn google<T: Into<String>>(token: T, accepted_terms: bool) -> Self {
        Self::new("google", token, accepted_terms)
    }
}

/// `PUT /authentication/change-password` request body.
#[derive(Clone, Serialize, Deserialize)]
pub struct ChangePasswordBody {
    /// User email.
    pub email: String,
    /// Current password (for verification).
    #[serde(rename = "password")]
    pub current_password: String,
    /// New password.
    pub new_password: String,
}

impl ChangePasswordBody {
    /// Build a password-change request.
    pub fn new<E, P, N>(email: E, current_password: P, new_password: N) -> Self
    where
        E: Into<String>,
        P: Into<String>,
        N: Into<String>,
    {
        Self {
            email: email.into(),
            current_password: current_password.into(),
            new_password: new_password.into(),
        }
    }
}

/// `PUT /authentication/request-password-reset` request body.
#[derive(Clone, Serialize, Deserialize)]
pub struct RequestPasswordResetBody {
    /// Email of the user requesting a password reset.
    pub email: String,
}

impl RequestPasswordResetBody {
    /// Build a password-reset request.
    pub fn new<E: Into<String>>(email: E) -> Self {
        Self {
            email: email.into(),
        }
    }
}

/// `PUT /authentication/reset-password` request body.
#[derive(Clone, Serialize, Deserialize)]
pub struct ResetPasswordBody {
    /// User email.
    pub email: String,
    /// Single-use reset token emailed to the user.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub token: Option<String>,
    /// New password.
    pub new_password: String,
}

impl ResetPasswordBody {
    /// Build a reset-password request.
    pub fn new<E, N>(email: E, new_password: N) -> Self
    where
        E: Into<String>,
        N: Into<String>,
    {
        Self {
            email: email.into(),
            token: None,
            new_password: new_password.into(),
        }
    }

    /// Set the reset token received by email.
    pub fn token<S: Into<String>>(mut self, token: S) -> Self {
        self.token = Some(token.into());
        self
    }
}

/// `POST /auth/link-social-login` request body.
#[derive(Clone, Serialize, Deserialize)]
pub struct LinkSocialLoginBody {
    /// Provider identifier. Currently only `"google"` is supported.
    pub provider: String,
    /// The token obtained from the social-login provider.
    pub token: String,
}

impl LinkSocialLoginBody {
    /// Build a link-social-login request.
    pub fn new<P, T>(provider: P, token: T) -> Self
    where
        P: Into<String>,
        T: Into<String>,
    {
        Self {
            provider: provider.into(),
            token: token.into(),
        }
    }

    /// Build a Google link request.
    pub fn google<T: Into<String>>(token: T) -> Self {
        Self::new("google", token)
    }
}

/// Email echoed by the password-management endpoints after a successful
/// request.
#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
#[non_exhaustive]
pub struct EmailResult {
    /// Email address associated with the completed operation.
    pub email: String,
}

/// Authentication endpoints.
///
/// # Browser-only OAuth endpoints
///
/// The API also exposes `GET /auth/authenticate` (start social login) and
/// `GET /login-callback` (the provider redirect target). These are
/// browser-redirect (`302`) flows that a JSON HTTP client cannot meaningfully
/// invoke — the user's browser must follow the redirects. They are therefore
/// intentionally not wrapped as request-executing methods; use
/// [`social_login_url`](AuthApi::social_login_url) to build the start URL you
/// redirect the user to, then exchange the returned provider token with
/// [`social_login`](AuthApi::social_login) or
/// [`link_social_login`](AuthApi::link_social_login).
#[derive(Debug)]
pub struct AuthApi<'a> {
    http: &'a HttpClient,
}

impl<'a> AuthApi<'a> {
    pub(crate) fn new(http: &'a HttpClient) -> Self {
        Self { http }
    }

    /// Authenticate with email + password and obtain a bearer token.
    ///
    /// `POST /login`.
    ///
    /// # Request payload
    ///
    /// ```json
    /// { "email": "user@example.invalid", "password": "s3cr3t-p4ss" }
    /// ```
    ///
    /// # Response payload
    ///
    /// ```json
    /// {
    ///   "status": 200,
    ///   "message": "",
    ///   "data": {
    ///     "access_token": "example-redacted-access-token",
    ///     "user": {
    ///       "id": "acc_1234567890abcdef12345678",
    ///       "name": "Bill Madeira",
    ///       "email": "user@example.invalid",
    ///       "telephone": null,
    ///       "government_id": null,
    ///       "is_email_verified": true,
    ///       "has_accepted_terms": true,
    ///       "created_at": "2026-01-14T12:03:41Z",
    ///       "to_be_deleted_at": null
    ///     },
    ///     "accounts": [
    ///       {
    ///         "id": "acc_1234567890abcdef12345678",
    ///         "name": "Feba Capital",
    ///         "roles": ["owner"],
    ///         "is_delete_allowed": false,
    ///         "created_at": "2026-01-14T12:03:41Z"
    ///       }
    ///     ]
    ///   }
    /// }
    /// ```
    pub async fn login(&self, body: &LoginBody) -> Result<LoginResult> {
        let req = self.http.request_public(Method::POST, "login")?.json(body);
        self.http.send_envelope(req).await
    }

    /// Authenticate via a social provider.
    ///
    /// `POST /authentication/social-login`.
    ///
    /// # Request payload
    ///
    /// ```json
    /// { "provider": "google", "token": "example-redacted-provider-token", "has_accepted_terms": true }
    /// ```
    ///
    /// # Response payload
    ///
    /// ```json
    /// {
    ///   "status": 200,
    ///   "message": "",
    ///   "data": {
    ///     "access_token": "example-redacted-access-token",
    ///     "user": {
    ///       "id": "acc_1234567890abcdef12345678",
    ///       "name": "Bill Madeira",
    ///       "email": "user@example.invalid",
    ///       "telephone": null,
    ///       "government_id": null,
    ///       "is_email_verified": true,
    ///       "has_accepted_terms": true,
    ///       "created_at": "2026-01-14T12:03:41Z",
    ///       "to_be_deleted_at": null
    ///     },
    ///     "accounts": [
    ///       {
    ///         "id": "acc_1234567890abcdef12345678",
    ///         "name": "Feba Capital",
    ///         "roles": ["owner"],
    ///         "is_delete_allowed": false,
    ///         "created_at": "2026-01-14T12:03:41Z"
    ///       }
    ///     ]
    ///   }
    /// }
    /// ```
    pub async fn social_login(&self, body: &SocialLoginBody) -> Result<LoginResult> {
        let req = self
            .http
            .request_public(Method::POST, "authentication/social-login")?
            .json(body);
        self.http.send_envelope(req).await
    }

    /// Change the authenticated user's password.
    ///
    /// `PUT /authentication/change-password`.
    ///
    /// # Request payload
    ///
    /// The current password is sent as `password`; the replacement as `new_password`.
    ///
    /// ```json
    /// {
    ///   "email": "user@example.invalid",
    ///   "password": "0ld_p4ssw0rd",
    ///   "new_password": "N3w_p4ssw0rd"
    /// }
    /// ```
    ///
    /// # Response payload
    ///
    /// ```json
    /// { "status": 200, "message": "", "data": { "email": "user@example.invalid" } }
    /// ```
    pub async fn change_password(&self, body: &ChangePasswordBody) -> Result<EmailResult> {
        let req = self
            .http
            .request(Method::PUT, "authentication/change-password")?
            .json(body);
        self.http.send_envelope(req).await
    }

    /// Request a password-reset email.
    ///
    /// `PUT /authentication/request-password-reset`.
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
    /// { "status": 200, "message": "", "data": { "email": "user@example.invalid" } }
    /// ```
    pub async fn request_password_reset(
        &self,
        body: &RequestPasswordResetBody,
    ) -> Result<EmailResult> {
        let req = self
            .http
            .request_public(Method::PUT, "authentication/request-password-reset")?
            .json(body);
        self.http.send_envelope(req).await
    }

    /// Complete a password reset using the emailed token.
    ///
    /// `PUT /authentication/reset-password`.
    ///
    /// # Request payload
    ///
    /// `token` is the single-use value emailed to the user; it is omitted when unset.
    ///
    /// ```json
    /// {
    ///   "email": "user@example.invalid",
    ///   "token": "b3ac64d6c55b3ac64d6c55",
    ///   "new_password": "N3w_p4ssw0rd"
    /// }
    /// ```
    ///
    /// # Response payload
    ///
    /// ```json
    /// { "status": 200, "message": "", "data": { "email": "user@example.invalid" } }
    /// ```
    pub async fn reset_password(&self, body: &ResetPasswordBody) -> Result<EmailResult> {
        let req = self
            .http
            .request_public(Method::PUT, "authentication/reset-password")?
            .json(body);
        self.http.send_envelope(req).await
    }

    /// Link a social-login provider to the authenticated user's account.
    ///
    /// `POST /auth/link-social-login`. Requires an authenticated credential.
    /// The API responds with a bare success envelope (no data payload).
    ///
    /// # Request payload
    ///
    /// ```json
    /// { "provider": "google", "token": "example-redacted-provider-token" }
    /// ```
    ///
    /// # Response payload
    ///
    /// ```json
    /// { "status": 200, "message": "" }
    /// ```
    pub async fn link_social_login(&self, body: &LinkSocialLoginBody) -> Result<()> {
        let req = self
            .http
            .request(Method::POST, "auth/link-social-login")?
            .json(body);
        self.http.send_no_content(req).await
    }

    /// Build the browser URL that starts the social-login (OAuth) compatibility
    /// flow available in the sandbox.
    ///
    /// Redirect the user's browser to this URL (`GET /auth/authenticate?authclient={provider}`);
    /// the provider then redirects back to `GET /login-callback`. This method
    /// only constructs the URL; it performs no request.
    ///
    /// ```
    /// # use assinafy::Client;
    /// let client = Client::from_api_key("k").unwrap();
    /// let url = client.auth_api().social_login_url("google").unwrap();
    /// assert!(url.ends_with("/auth/authenticate?authclient=google"));
    /// ```
    pub fn social_login_url(&self, provider: &str) -> Result<String> {
        let mut url = self.http.url("auth/authenticate")?;
        url.query_pairs_mut().append_pair("authclient", provider);
        Ok(url.into())
    }
}
