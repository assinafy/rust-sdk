//! The top-level [`Client`] and its [`ClientBuilder`].

use std::time::Duration;

use crate::auth::Auth;
use crate::config::BaseUrl;
use crate::error::{Error, Result};
use crate::http::HttpClient;
use crate::resources::{
    AccountApi, AccountsApi, ActivitiesApi, ApiKeysApi, AssignmentsApi, AuthApi, DocumentsApi,
    FieldsApi, PublicApi, SignerSelfApi, SignersApi, TagsApi, TemplatesApi, UsersApi, WebhooksApi,
};

/// Default user-agent used by the SDK.
pub(crate) const DEFAULT_USER_AGENT: &str = concat!("assinafy-rust/", env!("CARGO_PKG_VERSION"));

/// The Assinafy API client.
///
/// `Client` is cheap to clone — internal state is reference-counted — and is
/// safe to share across tasks. Build one once at start-up and reuse it.
///
/// # Example
///
/// ```no_run
/// use assinafy::Client;
///
/// # async fn run() -> assinafy::Result<()> {
/// let client = Client::builder()
///     .api_key("my-api-key")
///     .build()?;
///
/// let statuses = client.documents().statuses().await?;
/// println!("{} document statuses", statuses.len());
/// # Ok(()) }
/// ```
#[derive(Clone)]
pub struct Client {
    http: HttpClient,
}

impl Client {
    /// Start building a client.
    pub fn builder() -> ClientBuilder {
        ClientBuilder::default()
    }

    /// Convenience shortcut: build a client for production with an API key.
    pub fn from_api_key<S: Into<String>>(api_key: S) -> Result<Self> {
        ClientBuilder::default().api_key(api_key).build()
    }

    /// Return a clone of this client that uses a different credential.
    ///
    /// Useful for impersonating a signer with [`Auth::AccessCode`] or for
    /// swapping a freshly obtained bearer token.
    pub fn with_auth(&self, auth: Auth) -> Self {
        Client {
            http: self.http.with_auth(auth),
        }
    }

    /// Returns the credential currently in use.
    pub fn auth(&self) -> &Auth {
        self.http.auth()
    }

    /// Returns the base URL the client is targeting.
    pub fn base_url(&self) -> &url::Url {
        self.http.base_url()
    }

    /// Access the public, unauthenticated endpoints: public document
    /// metadata and the signer access-token send request.
    ///
    /// Document verification lives on
    /// [`documents()`](Self::documents), and password resets on
    /// [`auth_api()`](Self::auth_api).
    pub fn public(&self) -> PublicApi<'_> {
        PublicApi::new(&self.http)
    }

    /// Account-level endpoints: list the accounts the credential can see and
    /// create new accounts.
    pub fn accounts_api(&self) -> AccountsApi<'_> {
        AccountsApi::new(&self.http)
    }

    /// Endpoints scoped to a single account (get, update, delete, theme, logo).
    pub fn account<S: Into<String>>(&self, account_id: S) -> AccountApi<'_> {
        AccountApi::new(&self.http, account_id.into())
    }

    /// Authentication helpers: login, password reset, change password,
    /// social login.
    pub fn auth_api(&self) -> AuthApi<'_> {
        AuthApi::new(&self.http)
    }

    /// API-key management for the authenticated user.
    pub fn api_keys(&self) -> ApiKeysApi<'_> {
        ApiKeysApi::new(&self.http)
    }

    /// Endpoints for the authenticated user (`GET /users/self`).
    pub fn users(&self) -> UsersApi<'_> {
        UsersApi::new(&self.http)
    }

    /// Signer management within an account.
    pub fn signers<S: Into<String>>(&self, account_id: S) -> SignersApi<'_> {
        SignersApi::new(&self.http, account_id.into())
    }

    /// Endpoints that operate on the currently authenticated signer (via
    /// `signer-access-code`).
    pub fn signer_self(&self) -> SignerSelfApi<'_> {
        SignerSelfApi::new(&self.http)
    }

    /// Document endpoints (account-scoped collection + global by-id endpoints).
    pub fn documents(&self) -> DocumentsApi<'_> {
        DocumentsApi::new(&self.http)
    }

    /// Assignment (signature-request) endpoints.
    pub fn assignments(&self) -> AssignmentsApi<'_> {
        AssignmentsApi::new(&self.http)
    }

    /// Tag CRUD and document tag operations.
    pub fn tags<S: Into<String>>(&self, account_id: S) -> TagsApi<'_> {
        TagsApi::new(&self.http, account_id.into())
    }

    /// Field-definition CRUD and validation operations.
    pub fn fields<S: Into<String>>(&self, account_id: S) -> FieldsApi<'_> {
        FieldsApi::new(&self.http, account_id.into())
    }

    /// Template endpoints.
    pub fn templates<S: Into<String>>(&self, account_id: S) -> TemplatesApi<'_> {
        TemplatesApi::new(&self.http, account_id.into())
    }

    /// Webhook subscription and dispatch-history endpoints.
    pub fn webhooks<S: Into<String>>(&self, account_id: S) -> WebhooksApi<'_> {
        WebhooksApi::new(&self.http, account_id.into())
    }

    /// Document activity log endpoints.
    pub fn activities(&self) -> ActivitiesApi<'_> {
        ActivitiesApi::new(&self.http)
    }
}

impl std::fmt::Debug for Client {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Client")
            .field("base_url", &self.base_url().as_str())
            .finish()
    }
}

/// Fluent builder for [`Client`].
///
/// # Example
///
/// ```
/// use assinafy::{Client, Auth};
/// use std::time::Duration;
///
/// let client = Client::builder()
///     .auth(Auth::ApiKey("my-key".into()))
///     .sandbox()
///     .timeout(Duration::from_secs(30))
///     .user_agent("my-app/1.0")
///     .build()
///     .unwrap();
/// ```
#[derive(Debug, Default, Clone)]
pub struct ClientBuilder {
    base_url: Option<BaseUrl>,
    auth: Option<Auth>,
    timeout: Option<Duration>,
    connect_timeout: Option<Duration>,
    user_agent: Option<String>,
    http_client: Option<reqwest::Client>,
    allow_authenticated_http_client: bool,
}

impl ClientBuilder {
    /// Set the base URL.
    pub fn base_url(mut self, base: BaseUrl) -> Self {
        self.base_url = Some(base);
        self
    }

    /// Target the production environment (the default).
    pub fn production(self) -> Self {
        self.base_url(BaseUrl::Production)
    }

    /// Target the sandbox environment.
    pub fn sandbox(self) -> Self {
        self.base_url(BaseUrl::Sandbox)
    }

    /// Set the credential.
    pub fn auth(mut self, auth: Auth) -> Self {
        self.auth = Some(auth);
        self
    }

    /// Convenience: use an `X-Api-Key` credential.
    pub fn api_key<S: Into<String>>(self, key: S) -> Self {
        self.auth(Auth::ApiKey(key.into()))
    }

    /// Convenience: use a bearer token credential.
    pub fn bearer<S: Into<String>>(self, token: S) -> Self {
        self.auth(Auth::Bearer(token.into()))
    }

    /// Convenience: use an access token as `?access-token=...`.
    ///
    /// Prefer [`ClientBuilder::bearer`] unless the API flow you are integrating
    /// with specifically requires query-parameter authentication.
    pub fn access_token<S: Into<String>>(self, token: S) -> Self {
        self.auth(Auth::AccessToken(token.into()))
    }

    /// Convenience: use a signer access code credential.
    pub fn access_code<S: Into<String>>(self, code: S) -> Self {
        self.auth(Auth::AccessCode(code.into()))
    }

    /// Total request timeout (default: 60 seconds).
    pub fn timeout(mut self, timeout: Duration) -> Self {
        self.timeout = Some(timeout);
        self
    }

    /// Connect timeout (default: 10 seconds).
    pub fn connect_timeout(mut self, timeout: Duration) -> Self {
        self.connect_timeout = Some(timeout);
        self
    }

    /// Override the user agent.
    pub fn user_agent<S: Into<String>>(mut self, ua: S) -> Self {
        self.user_agent = Some(ua.into());
        self
    }

    /// Supply your own pre-configured [`reqwest::Client`] for unauthenticated
    /// requests.
    ///
    /// Only [`ClientBuilder::timeout`]/[`ClientBuilder::connect_timeout`] are
    /// bypassed when this is set (they configure `reqwest::Client` itself, so
    /// your own client's settings win). The SDK still sets its own
    /// `User-Agent` header (the default, or your [`ClientBuilder::user_agent`]
    /// override) explicitly on every request — via `RequestBuilder::header`,
    /// which always overrides a client-level default — so a `User-Agent`
    /// configured on the supplied `reqwest::Client` itself has no effect. Use
    /// [`ClientBuilder::user_agent`] if you need a custom value.
    ///
    /// Reqwest does not expose a supplied client's redirect policy, and its
    /// default cross-origin redirect handling does not strip Assinafy's
    /// `X-Api-Key` header. This method therefore rejects custom-client requests
    /// whenever a credential is configured. Prefer the SDK-owned client for
    /// authenticated calls; use [`ClientBuilder::authenticated_http_client`]
    /// only when the caller has disabled redirects explicitly.
    pub fn http_client(mut self, client: reqwest::Client) -> Self {
        self.http_client = Some(client);
        self.allow_authenticated_http_client = false;
        self
    }

    /// Supply a custom [`reqwest::Client`] for authenticated requests.
    ///
    /// # Security
    ///
    /// The supplied client must disable automatic redirects with
    /// [`reqwest::redirect::Policy::none`] and disable the `Referer` header.
    /// Reqwest does not expose those settings after construction, so the SDK
    /// cannot verify them. A client that follows redirects can forward an
    /// `X-Api-Key` header to another origin.
    ///
    /// ```
    /// use assinafy::Client;
    ///
    /// let transport = reqwest::Client::builder()
    ///     .redirect(reqwest::redirect::Policy::none())
    ///     .referer(false)
    ///     .build()
    ///     .unwrap();
    /// let client = Client::builder()
    ///     .api_key("redacted-api-key")
    ///     .authenticated_http_client(transport)
    ///     .build()
    ///     .unwrap();
    /// ```
    pub fn authenticated_http_client(mut self, client: reqwest::Client) -> Self {
        self.http_client = Some(client);
        self.allow_authenticated_http_client = true;
        self
    }

    /// Finalise the builder.
    pub fn build(self) -> Result<Client> {
        let base = self.base_url.unwrap_or_default();
        base.validate()?;
        let auth = self.auth.unwrap_or_default();
        let user_agent = self
            .user_agent
            .unwrap_or_else(|| DEFAULT_USER_AGENT.to_string());

        let restrict_custom_transport_auth =
            self.http_client.is_some() && !self.allow_authenticated_http_client;
        let http = match self.http_client {
            Some(c) => {
                if restrict_custom_transport_auth && !auth.is_none() {
                    return Err(Error::Config(
                        "custom HTTP clients cannot be used with credentials because their redirect policy cannot be verified"
                            .into(),
                    ));
                }
                c
            }
            None => {
                let builder = reqwest::Client::builder()
                    .user_agent(&user_agent)
                    .referer(false)
                    .redirect(reqwest::redirect::Policy::none())
                    .timeout(self.timeout.unwrap_or_else(|| Duration::from_secs(60)))
                    .connect_timeout(
                        self.connect_timeout
                            .unwrap_or_else(|| Duration::from_secs(10)),
                    );
                builder
                    .build()
                    .map_err(|e| Error::Config(format!("failed to build http client: {e}")))?
            }
        };

        Ok(Client {
            http: HttpClient::new(http, base, auth, user_agent, restrict_custom_transport_auth),
        })
    }
}

#[cfg(test)]
mod tests {
    use std::io::{Read, Write};
    use std::net::TcpListener;
    use std::thread;

    use super::*;

    fn read_request(stream: &mut std::net::TcpStream) -> String {
        let mut request = [0_u8; 2048];
        let read = stream.read(&mut request).unwrap();
        String::from_utf8_lossy(&request[..read]).into_owned()
    }

    #[tokio::test]
    async fn binary_redirects_follow_without_forwarding_credentials() {
        let target_listener = TcpListener::bind("127.0.0.1:0").unwrap();
        let target_address = target_listener.local_addr().unwrap();
        let target = thread::spawn(move || {
            let (mut stream, _) = target_listener.accept().unwrap();
            let request = read_request(&mut stream);
            stream
                .write_all(
                    b"HTTP/1.1 200 OK\r\nContent-Type: image/png\r\nContent-Length: 3\r\nConnection: close\r\n\r\nPNG",
                )
                .unwrap();
            request
        });

        let api_listener = TcpListener::bind("127.0.0.1:0").unwrap();
        let api_address = api_listener.local_addr().unwrap();
        let api = thread::spawn(move || {
            let (mut stream, _) = api_listener.accept().unwrap();
            let request = read_request(&mut stream);
            stream
                .write_all(
                    format!(
                        "HTTP/1.1 302 Found\r\nLocation: http://{target_address}/artifact\r\nContent-Length: 0\r\nConnection: close\r\n\r\n"
                    )
                    .as_bytes(),
                )
                .unwrap();
            request
        });

        let transport = reqwest::Client::builder()
            .referer(false)
            .redirect(reqwest::redirect::Policy::none())
            .timeout(Duration::from_secs(2))
            .build()
            .unwrap();
        let base = BaseUrl::custom(format!("http://{api_address}/v1")).unwrap();
        let client = Client::builder()
            .base_url(base)
            .api_key("sentinel-api-key")
            .authenticated_http_client(transport)
            .build()
            .unwrap();

        let (body, content_type) = client
            .documents()
            .download_thumbnail("document-id")
            .await
            .unwrap();
        assert_eq!(&body[..], b"PNG");
        assert_eq!(content_type, "image/png");

        let api_request = api.join().unwrap().to_ascii_lowercase();
        assert!(api_request.contains("x-api-key: sentinel-api-key"));
        let target_request = target.join().unwrap().to_ascii_lowercase();
        assert!(!target_request.contains("sentinel-api-key"));
        assert!(!target_request.contains("referer:"));
    }

    #[tokio::test]
    async fn public_binary_redirects_never_gain_credentials() {
        let listener = TcpListener::bind("127.0.0.1:0").unwrap();
        let address = listener.local_addr().unwrap();
        let server = thread::spawn(move || {
            let (mut first, _) = listener.accept().unwrap();
            let first_request = read_request(&mut first);
            first
                .write_all(
                    b"HTTP/1.1 302 Found\r\nLocation: /v1/artifact\r\nContent-Length: 0\r\nConnection: close\r\n\r\n",
                )
                .unwrap();

            let (mut second, _) = listener.accept().unwrap();
            let second_request = read_request(&mut second);
            second
                .write_all(
                    b"HTTP/1.1 200 OK\r\nContent-Type: application/pdf\r\nContent-Length: 3\r\nConnection: close\r\n\r\nPDF",
                )
                .unwrap();
            [first_request, second_request]
        });

        let transport = reqwest::Client::builder()
            .redirect(reqwest::redirect::Policy::none())
            .build()
            .unwrap();
        let base = BaseUrl::custom(format!("http://{address}/v1")).unwrap();
        let client = Client::builder()
            .base_url(base)
            .http_client(transport)
            .build()
            .unwrap()
            .with_auth(Auth::ApiKey("sentinel-api-key".into()));

        let (body, content_type) = client
            .signer_self()
            .download_document("signer-id", "document-id", "original")
            .await
            .unwrap();
        assert_eq!(&body[..], b"PDF");
        assert_eq!(content_type, "application/pdf");

        for request in server.join().unwrap() {
            assert!(!request.to_ascii_lowercase().contains("sentinel-api-key"));
        }
    }
}
