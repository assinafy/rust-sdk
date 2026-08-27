//! Endpoint configuration.

use std::fmt;

use url::{Host, Url};

use crate::error::{Error, Result};

/// Base URL used for API requests.
///
/// Two presets are provided:
///
/// * [`BaseUrl::Production`] — `https://api.assinafy.com.br/v1`
/// * [`BaseUrl::Sandbox`] — `https://sandbox.assinafy.com.br/v1`
///
/// Use [`BaseUrl::custom`] to point at any other deployment. Custom URLs must
/// use HTTPS; loopback HTTP is accepted for local development and tests.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub enum BaseUrl {
    /// `https://api.assinafy.com.br/v1`
    #[default]
    Production,
    /// `https://sandbox.assinafy.com.br/v1`
    Sandbox,
    /// User-supplied base URL.
    Custom(Url),
}

impl BaseUrl {
    /// Production base URL.
    pub const PRODUCTION: &'static str = "https://api.assinafy.com.br/v1";
    /// Sandbox base URL.
    pub const SANDBOX: &'static str = "https://sandbox.assinafy.com.br/v1";

    /// Parse a custom base URL. Its path is normalized to end with `/` (if
    /// missing) so that relative joins behave consistently.
    ///
    /// HTTPS is required except for loopback hosts. Embedded credentials,
    /// query strings, and fragments are rejected because they are unsafe or
    /// are discarded by relative endpoint joins.
    pub fn custom<S: AsRef<str>>(url: S) -> Result<Self> {
        let parsed = Url::parse(url.as_ref())
            .map_err(|e| Error::Config(format!("invalid custom base URL: {e}")))?;
        validate_custom_url(&parsed)?;
        Ok(BaseUrl::Custom(normalize_path_trailing_slash(parsed)))
    }

    pub(crate) fn validate(&self) -> Result<()> {
        if let Self::Custom(url) = self {
            validate_custom_url(url)?;
        }
        Ok(())
    }

    /// Returns the URL representation used for relative joins. The URL's
    /// path is guaranteed to end with `/`.
    pub fn as_url(&self) -> Url {
        let raw = match self {
            BaseUrl::Production => Self::PRODUCTION,
            BaseUrl::Sandbox => Self::SANDBOX,
            // Normalize here too, not just in `custom()`: `Custom` is a
            // public tuple variant and callers can build one directly with
            // an arbitrary `Url` that skips `custom()` entirely.
            BaseUrl::Custom(u) => return normalize_path_trailing_slash(u.clone()),
        };
        let mut s = raw.to_owned();
        s.push('/');
        Url::parse(&s).expect("static base url parses")
    }
}

/// Ensures `u`'s path ends with `/`, leaving any query string or fragment
/// untouched (unlike string-concatenation, which can splice the slash into
/// whichever of those happens to be present).
fn normalize_path_trailing_slash(mut u: Url) -> Url {
    if !u.path().ends_with('/') {
        let path = format!("{}/", u.path());
        u.set_path(&path);
    }
    u
}

fn validate_custom_url(url: &Url) -> Result<()> {
    let loopback = match url.host() {
        Some(Host::Domain(host)) => host.eq_ignore_ascii_case("localhost"),
        Some(Host::Ipv4(address)) => address.is_loopback(),
        Some(Host::Ipv6(address)) => address.is_loopback(),
        None => false,
    };
    if url.scheme() != "https" && !(url.scheme() == "http" && loopback) {
        return Err(Error::Config(
            "custom base URL must use HTTPS (HTTP is allowed only for loopback hosts)".into(),
        ));
    }
    if url.host_str().is_none() || url.cannot_be_a_base() {
        return Err(Error::Config(
            "custom base URL must be an absolute hierarchical URL with a host".into(),
        ));
    }
    if !url.username().is_empty() || url.password().is_some() {
        return Err(Error::Config(
            "custom base URL must not contain embedded credentials".into(),
        ));
    }
    if url.query().is_some() || url.fragment().is_some() {
        return Err(Error::Config(
            "custom base URL must not contain a query string or fragment".into(),
        ));
    }
    Ok(())
}

impl fmt::Display for BaseUrl {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            BaseUrl::Production => f.write_str(Self::PRODUCTION),
            BaseUrl::Sandbox => f.write_str(Self::SANDBOX),
            BaseUrl::Custom(u) => write!(f, "{}", u.as_str().trim_end_matches('/')),
        }
    }
}
