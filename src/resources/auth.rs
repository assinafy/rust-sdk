//! Authentication helpers: login, password management, social login.

use reqwest::Method;
use serde::{Deserialize, Serialize};

use crate::error::Result;
use crate::http::HttpClient;
use crate::models::LoginResult;

/// `POST /login` request body.
#[derive(Debug, Clone, Serialize, Deserialize)]
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
///
/// The exact shape of `token` depends on the social provider. Provide it as a
/// JSON value so the SDK does not have to track every provider's contract.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SocialLoginBody {
    /// Provider identifier (e.g. `"google"`, `"apple"`).
    pub provider: String,
    /// Provider-specific token payload.
    pub token: serde_json::Value,
    /// Whether the user accepted Assinafy's terms during the provider flow.
    pub has_accepted_terms: bool,
}

impl SocialLoginBody {
    /// Build a social-login request.
    pub fn new<P>(provider: P, token: impl Into<serde_json::Value>, accepted_terms: bool) -> Self
    where
        P: Into<String>,
    {
        Self {
            provider: provider.into(),
            token: token.into(),
            has_accepted_terms: accepted_terms,
        }
    }

    /// Build a Google social-login request.
    pub fn google(token: impl Into<serde_json::Value>, accepted_terms: bool) -> Self {
        Self::new("google", token, accepted_terms)
    }
}

/// `PUT /authentication/change-password` request body.
#[derive(Debug, Clone, Serialize, Deserialize)]
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
#[derive(Debug, Clone, Serialize, Deserialize)]
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
#[derive(Debug, Clone, Serialize, Deserialize)]
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

/// Authentication endpoints.
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
    pub async fn login(&self, body: &LoginBody) -> Result<LoginResult> {
        let req = self.http.request(Method::POST, "login")?.json(body);
        self.http.send_envelope(req).await
    }

    /// Authenticate via a social provider.
    ///
    /// `POST /authentication/social-login`.
    pub async fn social_login(&self, body: &SocialLoginBody) -> Result<LoginResult> {
        let req = self
            .http
            .request(Method::POST, "authentication/social-login")?
            .json(body);
        self.http.send_envelope(req).await
    }

    /// Change the authenticated user's password.
    ///
    /// `PUT /authentication/change-password`.
    pub async fn change_password(&self, body: &ChangePasswordBody) -> Result<()> {
        let req = self
            .http
            .request(Method::PUT, "authentication/change-password")?
            .json(body);
        self.http.send_no_content(req).await
    }

    /// Request a password-reset email.
    ///
    /// `PUT /authentication/request-password-reset`.
    pub async fn request_password_reset(&self, body: &RequestPasswordResetBody) -> Result<()> {
        let req = self
            .http
            .request(Method::PUT, "authentication/request-password-reset")?
            .json(body);
        self.http.send_no_content(req).await
    }

    /// Complete a password reset using the emailed token.
    ///
    /// `PUT /authentication/reset-password`.
    pub async fn reset_password(&self, body: &ResetPasswordBody) -> Result<()> {
        let req = self
            .http
            .request(Method::PUT, "authentication/reset-password")?
            .json(body);
        self.http.send_no_content(req).await
    }
}
