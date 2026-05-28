//! Internal HTTP helpers: envelope decoding, error mapping, and the shared
//! [`HttpClient`] used by every resource module.

use std::sync::Arc;

use reqwest::header::{ACCEPT, HeaderMap};
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

#[derive(Clone, Debug)]
pub(crate) struct HttpClient {
    inner: reqwest::Client,
    base: Arc<Url>,
    auth: Arc<Auth>,
    user_agent: Arc<String>,
}

impl HttpClient {
    pub(crate) fn new(
        client: reqwest::Client,
        base: BaseUrl,
        auth: Auth,
        user_agent: String,
    ) -> Self {
        HttpClient {
            inner: client,
            base: Arc::new(base.as_url()),
            auth: Arc::new(auth),
            user_agent: Arc::new(user_agent),
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
            base: self.base.clone(),
            auth: Arc::new(auth),
            user_agent: self.user_agent.clone(),
        }
    }

    /// Build a relative URL by joining `path` to the base URL.
    ///
    /// `path` must not start with `/` (the base URL ends in `/`).
    pub(crate) fn url(&self, path: &str) -> Result<Url> {
        let path = path.trim_start_matches('/');
        self.base.join(path).map_err(Error::from)
    }

    pub(crate) fn request(&self, method: Method, path: &str) -> Result<RequestBuilder> {
        let url = self.url(path)?;
        let req = self
            .inner
            .request(method, url)
            .header(ACCEPT, "application/json")
            .header(reqwest::header::USER_AGENT, self.user_agent.as_str());
        Ok(self.auth.apply(req))
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

    /// Perform a request and return the raw response body bytes (e.g. for
    /// downloading PDF artifacts). Errors are still decoded from any JSON
    /// envelope the server returns on failure.
    pub(crate) async fn send_bytes(
        &self,
        req: RequestBuilder,
    ) -> Result<(bytes::Bytes, HeaderMap)> {
        let res = req.send().await?;
        let status = res.status();
        let headers = res.headers().clone();
        let body = res.bytes().await?;
        if !status.is_success() {
            return Err(map_error(status, &body));
        }
        Ok((body, headers))
    }

    /// Perform a request expected to return no body (e.g. DELETE/PUT 204).
    pub(crate) async fn send_no_content(&self, req: RequestBuilder) -> Result<()> {
        let res = req.send().await?;
        let status = res.status();
        let body = res.bytes().await?;
        if !status.is_success() {
            return Err(map_error(status, &body));
        }
        Ok(())
    }
}

async fn take_response(res: Response) -> Result<(StatusCode, HeaderMap, bytes::Bytes)> {
    let status = res.status();
    let headers = res.headers().clone();
    let body = res.bytes().await?;
    Ok((status, headers, body))
}

fn decode_envelope<T: DeserializeOwned>(
    status: StatusCode,
    _headers: &HeaderMap,
    body: &[u8],
) -> Result<T> {
    if !status.is_success() {
        return Err(map_error(status, body));
    }
    if body.is_empty() {
        // Some PUT endpoints respond 200 with no body; only types that
        // accept `()` survive this branch.
        return serde_json::from_str("null").map_err(Error::from);
    }
    let envelope: Envelope<T> = serde_json::from_slice(body).map_err(|e| {
        Error::UnexpectedResponse(format!(
            "failed to decode envelope: {e}; body: {}",
            String::from_utf8_lossy(body)
        ))
    })?;
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
            .map_err(|e| {
                Error::UnexpectedResponse(format!(
                    "failed to decode response body: {e}; body: {}",
                    String::from_utf8_lossy(body)
                ))
            }),
        Err(err) => Err(err),
    }
}

fn map_error(status: StatusCode, body: &[u8]) -> Error {
    let api = serde_json::from_slice::<ApiError>(body)
        .or_else(|_| {
            serde_json::from_slice::<Envelope<serde_json::Value>>(body).map(|e| ApiError {
                status: e.status,
                message: e.message,
                data: e.data,
            })
        })
        .unwrap_or_else(|_| ApiError {
            status: status.as_u16(),
            message: String::from_utf8_lossy(body).into_owned(),
            data: serde_json::Value::Null,
        });
    Error::Api(api)
}
