//! Endpoint configuration.

use std::fmt;

use url::Url;

use crate::error::{Error, Result};

/// Base URL used for API requests.
///
/// Two presets are provided:
///
/// * [`BaseUrl::Production`] — `https://api.assinafy.com.br/v1`
/// * [`BaseUrl::Sandbox`] — `https://sandbox.assinafy.com.br/v1`
///
/// Use [`BaseUrl::custom`] to point at any other deployment.
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

    /// Parse a custom base URL. A trailing slash is appended if missing so
    /// that relative joins behave consistently.
    pub fn custom<S: AsRef<str>>(url: S) -> Result<Self> {
        let mut s = url.as_ref().to_owned();
        if !s.ends_with('/') {
            s.push('/');
        }
        let parsed = Url::parse(&s)
            .map_err(|e| Error::Config(format!("invalid base url `{}`: {e}", url.as_ref())))?;
        Ok(BaseUrl::Custom(parsed))
    }

    /// Returns the URL representation used for relative joins. The URL is
    /// guaranteed to end with `/`.
    pub fn as_url(&self) -> Url {
        let raw = match self {
            BaseUrl::Production => Self::PRODUCTION,
            BaseUrl::Sandbox => Self::SANDBOX,
            BaseUrl::Custom(u) => return u.clone(),
        };
        let mut s = raw.to_owned();
        s.push('/');
        Url::parse(&s).expect("static base url parses")
    }
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
