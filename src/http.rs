//! Internal HTTP helpers: envelope decoding, error mapping, and the shared
//! [`HttpClient`] used by every resource module.

use std::sync::{Arc, OnceLock};
use std::time::Duration;

use reqwest::header::{ACCEPT, HeaderMap, LOCATION};
use reqwest::{Method, RequestBuilder, Response, StatusCode};
use serde::de::DeserializeOwned;
use serde::{Deserialize, Serialize};
use url::Url;

use crate::auth::Auth;
use crate::config::BaseUrl;
use crate::error::{ApiError, Error, Result};
use crate::pagination::{Page, PaginationMeta};

/// Generic Assinafy response envelope: `{ status, message, data }`.
///
/// Most endpoints wrap their payload in this envelope. A handful (notably
/// document upload) return the data directly; for those we deserialize the
/// raw JSON body instead.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Envelope<T> {
    /// Echo of the HTTP status code.
    pub status: u16,
    /// Optional message (often empty on success).
    #[serde(default)]
    pub message: String,
    /// Payload — present on success; an empty array on errors with no detail.
    pub data: T,
}

/// The SDK-owned transport: TLS 1.2 or later, no redirects, no `Referer`.
pub(crate) fn transport(
    user_agent: &str,
    timeout: Duration,
    connect_timeout: Duration,
) -> reqwest::ClientBuilder {
    let builder = reqwest::Client::builder()
        .user_agent(user_agent)
        .referer(false)
        .redirect(reqwest::redirect::Policy::none())
        .timeout(timeout)
        .connect_timeout(connect_timeout);
    // The API rejects TLS 1.0 and 1.1; never offer them.
    #[cfg(any(feature = "rustls-tls", feature = "native-tls"))]
    let builder = builder.tls_version_min(reqwest::tls::Version::TLS_1_2);
    builder
}

/// The transport for OAuth token and revocation requests, whatever
/// [`ClientBuilder::http_client`](crate::ClientBuilder::http_client)
/// supplied. They carry single-use secrets, so on top of never following a
/// redirect it never resends a request — not even the HTTP/2 refusals reqwest
/// otherwise retries by default.
fn oauth_transport(
    user_agent: &str,
    timeout: Duration,
    connect_timeout: Duration,
) -> reqwest::ClientBuilder {
    transport(user_agent, timeout, connect_timeout).retry(reqwest::retry::never())
}

#[derive(Clone, Debug)]
pub(crate) struct HttpClient {
    inner: reqwest::Client,
    /// [`oauth_transport`], built on first use — a client that never calls
    /// the OAuth token endpoints never builds it — and shared by clones.
    oauth: Arc<OnceLock<reqwest::Client>>,
    timeout: Duration,
    connect_timeout: Duration,
    base: Arc<Url>,
    auth: Arc<Auth>,
    user_agent: Arc<String>,
    restrict_custom_transport_auth: bool,
}

impl HttpClient {
    pub(crate) fn new(
        client: reqwest::Client,
        base: BaseUrl,
        auth: Auth,
        user_agent: String,
        timeout: Duration,
        connect_timeout: Duration,
        restrict_custom_transport_auth: bool,
    ) -> Self {
        HttpClient {
            inner: client,
            oauth: Arc::default(),
            timeout,
            connect_timeout,
            base: Arc::new(base.as_url()),
            auth: Arc::new(auth),
            user_agent: Arc::new(user_agent),
            restrict_custom_transport_auth,
        }
    }

    pub(crate) fn base_url(&self) -> &Url {
        &self.base
    }

    pub(crate) fn auth(&self) -> &Auth {
        &self.auth
    }

    pub(crate) fn with_auth(&self, auth: Auth) -> Self {
        HttpClient {
            inner: self.inner.clone(),
            oauth: self.oauth.clone(),
            timeout: self.timeout,
            connect_timeout: self.connect_timeout,
            base: self.base.clone(),
            auth: Arc::new(auth),
            user_agent: self.user_agent.clone(),
            restrict_custom_transport_auth: self.restrict_custom_transport_auth,
        }
    }

    /// Build a relative URL by joining `path` to the base URL.
    ///
    /// Reject URL syntax and dot segments so caller-supplied resource IDs
    /// cannot replace the intended endpoint path, query, or fragment.
    pub(crate) fn url(&self, path: &str) -> Result<Url> {
        let has_unsafe_byte = path.bytes().any(|byte| {
            !byte.is_ascii_alphanumeric() && !matches!(byte, b'/' | b'-' | b'_' | b'.' | b'~')
        });
        let has_dot_segment = path.split('/').any(|segment| matches!(segment, "." | ".."));
        if path.is_empty()
            || path.starts_with('/')
            || path.contains("//")
            || has_unsafe_byte
            || has_dot_segment
        {
            return Err(Error::Config(
                "request path contains unsafe URL syntax or path segments".into(),
            ));
        }
        let url = self.base.join(path)?;
        if !same_origin(&url, &self.base) || !url.path().starts_with(self.base.path()) {
            return Err(Error::Config(
                "request path escaped the configured API base URL".into(),
            ));
        }
        Ok(url)
    }

    /// Build a route from validated static and opaque-ID path segments.
    pub(crate) fn path(&self, segments: &[&str]) -> Result<String> {
        let valid = !segments.is_empty()
            && segments.iter().all(|segment| {
                !segment.is_empty()
                    && segment.bytes().all(|byte| {
                        byte.is_ascii_alphanumeric() || matches!(byte, b'-' | b'_' | b'.' | b'~')
                    })
                    && !matches!(*segment, "." | "..")
            });
        if !valid {
            return Err(Error::Config(
                "request path segment is empty, unsafe, or contains reserved URL syntax".into(),
            ));
        }
        Ok(segments.join("/"))
    }

    /// Build the URL of an RFC 8615 `.well-known` document at the API
    /// origin's root.
    ///
    /// These documents are defined at the origin, not under the versioned API
    /// base path, so [`url`](Self::url) — which joins relative to the base and
    /// refuses to leave it — cannot reach them.
    pub(crate) fn well_known_url(&self, name: &str) -> Result<Url> {
        let valid = !name.is_empty()
            && name
                .bytes()
                .all(|byte| byte.is_ascii_alphanumeric() || matches!(byte, b'-' | b'_' | b'.'))
            && !matches!(name, "." | "..");
        if !valid {
            return Err(Error::Config(
                "`.well-known` document name is empty or contains unsafe URL syntax".into(),
            ));
        }
        let url = self.base.join(&format!("/.well-known/{name}"))?;
        if !same_origin(&url, &self.base) {
            return Err(Error::Config(
                "`.well-known` document escaped the configured API origin".into(),
            ));
        }
        Ok(url)
    }

    /// Build an unauthenticated request against an absolute URL.
    ///
    /// OAuth discovery spans two hosts — this API and the authorization
    /// server that issues tokens for it — so the target is not always
    /// relative to the configured base URL. No credential is ever applied:
    /// the authorization server is a separate origin and must not receive
    /// this client's API key.
    pub(crate) fn request_absolute_public(
        &self,
        method: Method,
        url: Url,
    ) -> Result<RequestBuilder> {
        crate::config::validate_https_url(&url)?;
        Ok(self.unauthenticated(&self.inner, method, url))
    }

    pub(crate) fn request(&self, method: Method, path: &str) -> Result<RequestBuilder> {
        if self.restrict_custom_transport_auth && !self.auth.is_none() {
            return Err(Error::Config(
                "custom HTTP clients cannot be used with credentials because their redirect policy cannot be verified"
                    .into(),
            ));
        }
        Ok(self.auth.apply(self.request_public(method, path)?))
    }

    /// Build a request without applying the client's configured credential.
    ///
    /// This is reserved for operations whose OpenAPI definition explicitly
    /// declares `security: []`.
    pub(crate) fn request_public(&self, method: Method, path: &str) -> Result<RequestBuilder> {
        Ok(self.unauthenticated(&self.inner, method, self.url(path)?))
    }

    /// Build an OAuth token or revocation `POST`, which carries single-use
    /// secrets in its body, on the SDK-owned OAuth transport: never the
    /// caller's client, never a redirect, never a second send. No credential
    /// is applied; the OAuth client identifies itself in the body.
    pub(crate) fn request_oauth(&self, path: &str) -> Result<RequestBuilder> {
        let url = self.url(path)?;
        let oauth = match self.oauth.get() {
            Some(oauth) => oauth,
            None => {
                let oauth = oauth_transport(&self.user_agent, self.timeout, self.connect_timeout)
                    .build()
                    .map_err(|e| Error::Config(format!("failed to build http client: {e}")))?;
                self.oauth.get_or_init(|| oauth)
            }
        };
        Ok(self.unauthenticated(oauth, Method::POST, url))
    }

    fn unauthenticated(
        &self,
        client: &reqwest::Client,
        method: Method,
        url: Url,
    ) -> RequestBuilder {
        client
            .request(method, url)
            .header(ACCEPT, "application/json")
            .header(reqwest::header::USER_AGENT, self.user_agent.as_str())
    }

    /// Perform a request, decode the JSON envelope, and return the `data`.
    pub(crate) async fn send_envelope<T: DeserializeOwned>(
        &self,
        req: RequestBuilder,
    ) -> Result<T> {
        let res = req.send().await?;
        let (status, headers, body) = take_response(res).await?;
        decode_envelope::<T>(status, &headers, &body)
    }

    /// Perform a request and decode either the standard Assinafy envelope or a
    /// direct JSON payload. A few endpoints, including document upload and some
    /// signer-facing routes, have historically returned the resource directly.
    pub(crate) async fn send_data<T: DeserializeOwned>(&self, req: RequestBuilder) -> Result<T> {
        let res = req.send().await?;
        let (status, headers, body) = take_response(res).await?;
        decode_data::<T>(status, &headers, &body)
    }

    /// Perform a request that returns an envelope wrapping a list, and combine
    /// the decoded items with paging metadata pulled from response headers.
    pub(crate) async fn send_paged<T: DeserializeOwned>(
        &self,
        req: RequestBuilder,
    ) -> Result<Page<T>> {
        let res = req.send().await?;
        let (status, headers, body) = take_response(res).await?;
        let data = decode_envelope::<Vec<T>>(status, &headers, &body)?;
        let meta = PaginationMeta::from_headers(&headers);
        Ok(Page { data, meta })
    }

    /// Perform a request and return the raw response body bytes plus the
    /// response headers (e.g. for downloading PDF artifacts). Errors are still
    /// decoded from any JSON envelope the server returns on failure.
    async fn send_bytes(
        &self,
        req: RequestBuilder,
        authenticate_same_origin_redirects: bool,
    ) -> Result<(bytes::Bytes, HeaderMap)> {
        let mut res = req.send().await?;
        for _ in 0..10 {
            if !res.status().is_redirection() {
                break;
            }
            let Some(location) = res.headers().get(LOCATION).and_then(|v| v.to_str().ok()) else {
                break;
            };
            let target = res.url().join(location)?;
            let mut redirect = self
                .inner
                .get(target.clone())
                .header(ACCEPT, "application/octet-stream")
                .header(reqwest::header::USER_AGENT, self.user_agent.as_str());
            if authenticate_same_origin_redirects && same_origin(&target, &self.base) {
                redirect = self.auth.apply(redirect);
            }
            res = redirect.send().await?;
        }
        if res.status().is_redirection() {
            return Err(Error::UnexpectedResponse(
                "download exceeded 10 redirects or returned an invalid redirect".into(),
            ));
        }
        let (status, headers, body) = take_response(res).await?;
        ensure_success(status, Some(&headers), &body)?;
        Ok((body, headers))
    }

    /// Perform a request that returns a binary artifact, and return its bytes
    /// alongside the response `Content-Type` (defaulting to
    /// `application/octet-stream` when the header is absent).
    pub(crate) async fn send_download(
        &self,
        req: RequestBuilder,
    ) -> Result<(bytes::Bytes, String)> {
        let (bytes, headers) = self.send_bytes(req, true).await?;
        Ok((bytes, content_type_of(&headers)))
    }

    /// Perform an unauthenticated binary request without adding credentials on
    /// redirects, including redirects back to the configured API origin.
    pub(crate) async fn send_public_download(
        &self,
        req: RequestBuilder,
    ) -> Result<(bytes::Bytes, String)> {
        let (bytes, headers) = self.send_bytes(req, false).await?;
        Ok((bytes, content_type_of(&headers)))
    }

    /// Perform a request expected to return no meaningful body. Tolerates both
    /// an empty `204` response and a `200` envelope (`{ status, message, data }`)
    /// whose payload is discarded.
    pub(crate) async fn send_no_content(&self, req: RequestBuilder) -> Result<()> {
        let res = req.send().await?;
        let (status, headers, body) = take_response(res).await?;
        ensure_success(status, Some(&headers), &body)
    }
}

fn same_origin(left: &Url, right: &Url) -> bool {
    left.scheme() == right.scheme()
        && left.host_str() == right.host_str()
        && left.port_or_known_default() == right.port_or_known_default()
}

/// Returns `Ok(())` for 2xx responses, otherwise maps the body into an
/// [`Error::Api`]. Centralises the success check shared by every send path.
fn ensure_success(status: StatusCode, headers: Option<&HeaderMap>, body: &[u8]) -> Result<()> {
    if status.is_success() {
        Ok(())
    } else {
        Err(map_error(status, headers, body))
    }
}

/// Parses the number of seconds to wait before retrying from the `Retry-After`
/// or `X-Rate-Limit-Reset` response header, when present.
fn retry_after_from(headers: Option<&HeaderMap>) -> Option<u64> {
    let headers = headers?;
    ["retry-after", "x-rate-limit-reset"]
        .iter()
        .find_map(|name| {
            headers
                .get(*name)
                .and_then(|v| v.to_str().ok())
                .and_then(|s| s.trim().parse::<u64>().ok())
        })
}

/// Reads the scopes named by a `WWW-Authenticate: Bearer
/// error="insufficient_scope", scope="…"` challenge (RFC 6750 §3), which the
/// API sends when an OAuth token lacks the scope an endpoint requires.
fn insufficient_scope_from(headers: Option<&HeaderMap>) -> Option<String> {
    let challenge = headers?
        .get(reqwest::header::WWW_AUTHENTICATE)?
        .to_str()
        .ok()?;
    let (mut error, mut scope) = (None, None);
    // Split on the commas between parameters, not those inside quoted values.
    let mut quoted = false;
    let params = challenge.split(|c| {
        quoted ^= c == '"';
        c == ',' && !quoted
    });
    for param in params {
        let Some((name, value)) = param.split_once('=') else {
            continue;
        };
        // The first parameter follows the `Bearer` scheme name.
        let name = name.trim().rsplit(' ').next().unwrap_or_default();
        let value = value.trim().trim_matches('"');
        match name {
            "error" => error = Some(value),
            "scope" => scope = Some(value),
            _ => {}
        }
    }
    (error == Some("insufficient_scope")).then(|| scope.unwrap_or_default().to_owned())
}

/// Extracts the response `Content-Type`, falling back to
/// `application/octet-stream` when absent or non-UTF-8.
fn content_type_of(headers: &HeaderMap) -> String {
    headers
        .get(reqwest::header::CONTENT_TYPE)
        .and_then(|v| v.to_str().ok())
        .unwrap_or("application/octet-stream")
        .to_owned()
}

async fn take_response(res: Response) -> Result<(StatusCode, HeaderMap, bytes::Bytes)> {
    let status = res.status();
    let headers = res.headers().clone();
    let body = res.bytes().await?;
    Ok((status, headers, body))
}

fn decode_envelope<T: DeserializeOwned>(
    status: StatusCode,
    headers: &HeaderMap,
    body: &[u8],
) -> Result<T> {
    ensure_success(status, Some(headers), body)?;
    if body.is_empty() {
        // Some PUT endpoints respond 200 with no body; only types that
        // accept `()` survive this branch.
        return serde_json::from_str("null").map_err(Error::from);
    }
    let envelope: Envelope<T> = serde_json::from_slice(body)
        .map_err(|e| unexpected_decode_error("envelope", e, body.len()))?;
    Ok(envelope.data)
}

fn decode_data<T: DeserializeOwned>(
    status: StatusCode,
    headers: &HeaderMap,
    body: &[u8],
) -> Result<T> {
    match decode_envelope(status, headers, body) {
        Ok(data) => Ok(data),
        Err(Error::UnexpectedResponse(_)) if status.is_success() => serde_json::from_slice(body)
            .map_err(|e| unexpected_decode_error("response body", e, body.len())),
        Err(err) => Err(err),
    }
}

fn unexpected_decode_error(context: &str, error: serde_json::Error, body_len: usize) -> Error {
    Error::UnexpectedResponse(format!(
        "failed to decode {context}: {error}; body length: {body_len} bytes"
    ))
}

fn map_error(status: StatusCode, headers: Option<&HeaderMap>, body: &[u8]) -> Error {
    let retry_after = retry_after_from(headers);
    let insufficient_scope = insufficient_scope_from(headers);
    let api = match serde_json::from_slice::<serde_json::Value>(body) {
        Ok(serde_json::Value::Object(mut map)) => {
            let code = map
                .get("status")
                .and_then(|v| v.as_u64())
                .map(|n| n as u16)
                .unwrap_or_else(|| status.as_u16());
            // The OAuth endpoints answer with RFC 6749 §5.2's flat
            // `{error, error_description}` instead of this API's envelope, so
            // there is no `message` to read; use the description, falling
            // back to the bare error code.
            let message = map
                .get("message")
                .or_else(|| map.get("error_description"))
                .or_else(|| map.get("error"))
                .and_then(|v| v.as_str())
                .unwrap_or("")
                .to_owned();
            // Standard error envelopes carry a `data` field. Bodies without one
            // (e.g. a route-miss `{ name, message, code, status }`) are kept
            // whole so their `name`/`code` survive.
            let data = match map.remove("data") {
                Some(data) => data,
                None => serde_json::Value::Object(map),
            };
            ApiError {
                status: code,
                message,
                data,
                retry_after,
                insufficient_scope,
            }
        }
        _ => ApiError {
            status: status.as_u16(),
            message: String::from_utf8_lossy(body).into_owned(),
            data: serde_json::Value::Null,
            retry_after,
            insufficient_scope,
        },
    };
    Error::Api(api)
}

#[cfg(test)]
mod tests {
    use super::*;

    // `map_error` is private, so its route-miss vs resource-miss merge logic
    // (the behavior `ApiError::data`'s doc comment promises) can only be
    // exercised from an inline test, not from tests/unit.rs.

    #[test]
    fn map_error_keeps_whole_body_for_route_miss_shape() {
        let body = "{\"name\":\"Not Found\",\"message\":\"P\u{e1}gina n\u{e3}o encontrada.\",\"code\":0,\"status\":404}"
            .as_bytes();
        let Error::Api(err) = map_error(StatusCode::NOT_FOUND, None, body) else {
            panic!("expected Error::Api");
        };
        assert_eq!(err.status, 404);
        assert_eq!(err.data["name"], "Not Found");
        assert_eq!(err.data["code"], 0);
    }

    #[test]
    fn map_error_extracts_data_for_resource_miss_shape() {
        let body = br#"{"status":404,"message":"Signer not found","data":{"field":"id"}}"#;
        let Error::Api(err) = map_error(StatusCode::NOT_FOUND, None, body) else {
            panic!("expected Error::Api");
        };
        assert_eq!(err.status, 404);
        assert_eq!(err.message, "Signer not found");
        assert_eq!(err.data, serde_json::json!({"field": "id"}));
    }

    #[test]
    fn map_error_reads_the_scopes_an_insufficient_scope_challenge_names() {
        let body = br#"{"status":403,"message":"You are not allowed to perform this action.","data":null}"#;
        let with = |challenge: &str| {
            let mut headers = HeaderMap::new();
            headers.insert(
                reqwest::header::WWW_AUTHENTICATE,
                challenge.parse().unwrap(),
            );
            map_error(StatusCode::FORBIDDEN, Some(&headers), body)
        };

        let error = with(
            r#"Bearer error="insufficient_scope", scope="documents:write", resource_metadata="https://api.assinafy.com.br/.well-known/oauth-protected-resource""#,
        );
        assert_eq!(error.status(), Some(403));
        assert_eq!(error.insufficient_scope(), Some("documents:write"));
        // A comma inside a quoted value does not split the parameters.
        let error = with(
            r#"Bearer error="insufficient_scope", error_description="needs a, b", scope="documents:write templates:read""#,
        );
        assert_eq!(
            error.insufficient_scope(),
            Some("documents:write templates:read")
        );
        // Any other 403 carries no scope to reconnect with.
        assert_eq!(
            with(r#"Bearer error="invalid_token""#).insufficient_scope(),
            None
        );
        assert_eq!(
            map_error(StatusCode::FORBIDDEN, None, body).insufficient_scope(),
            None
        );
    }

    #[test]
    fn public_request_omits_every_configured_credential() {
        let base = BaseUrl::custom("https://api.example.invalid/v1").unwrap();
        let credentials = [
            Auth::ApiKey("placeholder-api-key".into()),
            Auth::Bearer("placeholder-bearer".into()),
            Auth::AccessToken("placeholder-access-token".into()),
            Auth::AccessCode("placeholder-access-code".into()),
        ];

        for auth in credentials {
            let http = HttpClient::new(
                reqwest::Client::new(),
                base.clone(),
                auth,
                "assinafy-test".into(),
                Duration::ZERO,
                Duration::ZERO,
                false,
            );
            let request = http
                .request_public(Method::GET, "public/documents/document-id")
                .unwrap()
                .build()
                .unwrap();

            assert!(!request.headers().contains_key("x-api-key"));
            assert!(
                !request
                    .headers()
                    .contains_key(reqwest::header::AUTHORIZATION)
            );
            assert!(request.url().query().is_none());
        }
    }

    #[test]
    fn request_paths_cannot_retarget_the_api_url() {
        let http = HttpClient::new(
            reqwest::Client::new(),
            BaseUrl::custom("https://api.example.invalid/v1").unwrap(),
            Auth::None,
            "assinafy-test".into(),
            Duration::ZERO,
            Duration::ZERO,
            false,
        );

        for path in [
            "documents/../users/self",
            "documents/%2e%2e/users/self",
            "documents/id?access-token=secret",
            "documents/id#fragment",
            "documents\\..\\users\\self",
            "documents/\t../users/self",
            "documents/\n../users/self",
            "/users/self",
            "documents//users/self",
        ] {
            assert!(
                matches!(http.url(path), Err(Error::Config(_))),
                "accepted unsafe path {path:?}"
            );
        }
        assert_eq!(
            http.url("accounts/account-id/documents/document-id")
                .unwrap()
                .as_str(),
            "https://api.example.invalid/v1/accounts/account-id/documents/document-id"
        );
    }

    #[test]
    fn resource_path_segments_cannot_change_route_shape() {
        let http = HttpClient::new(
            reqwest::Client::new(),
            BaseUrl::custom("https://api.example.invalid/v1").unwrap(),
            Auth::None,
            "assinafy-test".into(),
            Duration::ZERO,
            Duration::ZERO,
            false,
        );

        assert_eq!(
            http.path(&["accounts", "account-id.v2~draft", "logo"])
                .unwrap(),
            "accounts/account-id.v2~draft/logo"
        );
        for account_id in [
            "",
            "victim/logo",
            "..",
            "victim?force=true",
            "victim#fragment",
            "victim%2flogo",
            "victim account",
        ] {
            assert!(
                matches!(http.path(&["accounts", account_id]), Err(Error::Config(_))),
                "accepted unsafe account id {account_id:?}"
            );
        }
    }

    #[test]
    fn malformed_success_responses_do_not_expose_response_bodies() {
        const SECRET: &str = "sentinel-response-secret";
        let headers = HeaderMap::new();
        let envelope = format!(
            r#"{{"status":200,"message":"","data":{{"access_token":"{SECRET}","user":null,"accounts":[]}}}}"#
        );
        let direct = format!(r#"{{"access_token":"{SECRET}","user":null,"accounts":[]}}"#);

        let errors = [
            decode_envelope::<crate::models::LoginResult>(
                StatusCode::OK,
                &headers,
                envelope.as_bytes(),
            )
            .unwrap_err(),
            decode_data::<crate::models::LoginResult>(StatusCode::OK, &headers, direct.as_bytes())
                .unwrap_err(),
        ];

        for error in errors {
            let rendered = format!("{error} {error:?}");
            assert!(!rendered.contains(SECRET));
            assert!(rendered.contains("body length:"));
        }
    }

    #[tokio::test]
    async fn a_refused_http2_token_request_is_not_resent() {
        use std::io::{Read, Write};
        use std::sync::atomic::{AtomicUsize, Ordering};

        // A bare HTTP/2 server that answers every request stream with
        // RST_STREAM(REFUSED_STREAM), a refusal reqwest retries by default.
        let listener = std::net::TcpListener::bind("127.0.0.1:0").unwrap();
        let address = listener.local_addr().unwrap();
        let requests = Arc::new(AtomicUsize::new(0));
        let seen = requests.clone();
        std::thread::spawn(move || -> std::io::Result<()> {
            let (mut stream, _) = listener.accept()?;
            stream.read_exact(&mut [0; 24])?; // client connection preface
            stream.write_all(&[0, 0, 0, 4, 0, 0, 0, 0, 0])?; // empty SETTINGS
            let mut header = [0; 9];
            loop {
                stream.read_exact(&mut header)?;
                let length = u32::from_be_bytes([0, header[0], header[1], header[2]]);
                std::io::copy(&mut (&mut stream).take(length.into()), &mut std::io::sink())?;
                match header[3] {
                    // SETTINGS: acknowledge the client's.
                    4 if header[4] & 1 == 0 => stream.write_all(&[0, 0, 0, 4, 1, 0, 0, 0, 0])?,
                    // HEADERS: a request. Refuse its stream.
                    1 => {
                        seen.fetch_add(1, Ordering::SeqCst);
                        stream
                            .write_all(&[&[0, 0, 4, 3, 0], &header[5..], &[0, 0, 0, 7]].concat())?;
                    }
                    _ => {}
                }
            }
        });

        let timeout = Duration::from_secs(5);
        let http = HttpClient::new(
            reqwest::Client::new(),
            BaseUrl::custom(format!("http://{address}/v1")).unwrap(),
            Auth::None,
            "assinafy-test".into(),
            timeout,
            timeout,
            false,
        );
        // The production OAuth transport, speaking HTTP/2 without TLS.
        let oauth = oauth_transport("assinafy-test", timeout, timeout)
            .http2_prior_knowledge()
            .build()
            .unwrap();
        http.oauth.set(oauth).unwrap();
        let refresh =
            crate::resources::TokenRequest::refresh_token("client-id", "sentinel-refresh");
        assert!(matches!(
            crate::resources::OAuthApi::new(&http).token(&refresh).await,
            Err(Error::Http(_))
        ));
        // Each refusal was sent before `token` could return: the count is final.
        assert_eq!(requests.load(Ordering::SeqCst), 1);
    }
}
