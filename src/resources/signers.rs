//! Signer management endpoints.

use reqwest::Method;
use serde::{Deserialize, Serialize};

use crate::error::Result;
use crate::http::HttpClient;
use crate::models::Signer;
use crate::pagination::Page;

/// Body for `POST /accounts/{account_id}/signers`.
///
/// At least one of `email` or `whatsapp_phone_number` is required by the API
/// for signers that will receive notifications, though either may be omitted
/// for `Collect` (in-person) flows.
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct CreateSignerBody {
    /// Full legal name.
    pub full_name: String,
    /// Email address.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub email: Option<String>,
    /// WhatsApp phone number in E.164 format.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub whatsapp_phone_number: Option<String>,
}

impl CreateSignerBody {
    /// Create a body with only the required `full_name` field.
    pub fn new<S: Into<String>>(full_name: S) -> Self {
        CreateSignerBody {
            full_name: full_name.into(),
            email: None,
            whatsapp_phone_number: None,
        }
    }

    /// Set the email.
    pub fn email<S: Into<String>>(mut self, email: S) -> Self {
        self.email = Some(email.into());
        self
    }

    /// Set the WhatsApp phone number.
    pub fn whatsapp<S: Into<String>>(mut self, phone: S) -> Self {
        self.whatsapp_phone_number = Some(phone.into());
        self
    }
}

/// Body for `PUT /accounts/{account_id}/signers/{signer_id}`. Any omitted
/// field is left unchanged server-side.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct UpdateSignerBody {
    /// New full name.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub full_name: Option<String>,
    /// New email address. Pass `Some(String::new())` is **not** the way to
    /// clear it — most fields cannot be nulled via this endpoint; check the
    /// API reference if you need to remove a value.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub email: Option<String>,
    /// New WhatsApp phone number.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub whatsapp_phone_number: Option<String>,
}

impl UpdateSignerBody {
    /// New empty update body.
    pub fn new() -> Self {
        Self::default()
    }

    /// Set the full name.
    pub fn full_name<S: Into<String>>(mut self, name: S) -> Self {
        self.full_name = Some(name.into());
        self
    }

    /// Set the email address.
    pub fn email<S: Into<String>>(mut self, email: S) -> Self {
        self.email = Some(email.into());
        self
    }

    /// Set the WhatsApp phone number.
    pub fn whatsapp<S: Into<String>>(mut self, phone: S) -> Self {
        self.whatsapp_phone_number = Some(phone.into());
        self
    }
}

/// Builder for `GET /accounts/{account_id}/signers`.
#[derive(Debug)]
pub struct ListSignersRequest<'a> {
    http: &'a HttpClient,
    account_id: &'a str,
    page: Option<u32>,
    per_page: Option<u32>,
    search: Option<String>,
    sort: Option<String>,
}

impl<'a> ListSignersRequest<'a> {
    /// 1-based page number.
    pub fn page(mut self, page: u32) -> Self {
        self.page = Some(page);
        self
    }

    /// Results per page (server caps at 100).
    pub fn per_page(mut self, per_page: u32) -> Self {
        self.per_page = Some(per_page);
        self
    }

    /// Free-text search term.
    pub fn search<S: Into<String>>(mut self, search: S) -> Self {
        self.search = Some(search.into());
        self
    }

    /// Sort expression. Prefix with `-` for descending order
    /// (e.g. `"-created_at"`).
    pub fn sort<S: Into<String>>(mut self, sort: S) -> Self {
        self.sort = Some(sort.into());
        self
    }

    /// Execute the request.
    pub async fn send(self) -> Result<Page<Signer>> {
        let path = format!("accounts/{}/signers", self.account_id);
        let mut req = self.http.request(Method::GET, &path)?;
        let mut query: Vec<(&str, String)> = Vec::with_capacity(4);
        if let Some(p) = self.page {
            query.push(("page", p.to_string()));
        }
        if let Some(p) = self.per_page {
            query.push(("per-page", p.to_string()));
        }
        if let Some(s) = self.search {
            query.push(("search", s));
        }
        if let Some(s) = self.sort {
            query.push(("sort", s));
        }
        if !query.is_empty() {
            req = req.query(&query);
        }
        self.http.send_paged(req).await
    }
}

/// Signer endpoints for a specific account.
#[derive(Debug)]
pub struct SignersApi<'a> {
    http: &'a HttpClient,
    account_id: String,
}

impl<'a> SignersApi<'a> {
    pub(crate) fn new(http: &'a HttpClient, account_id: String) -> Self {
        Self { http, account_id }
    }

    /// Create a signer.
    ///
    /// `POST /accounts/{account_id}/signers`.
    pub async fn create(&self, body: &CreateSignerBody) -> Result<Signer> {
        let path = format!("accounts/{}/signers", self.account_id);
        let req = self.http.request(Method::POST, &path)?.json(body);
        self.http.send_envelope(req).await
    }

    /// List signers. Returns a builder to apply pagination/search/sort.
    ///
    /// `GET /accounts/{account_id}/signers`.
    pub fn list(&self) -> ListSignersRequest<'_> {
        ListSignersRequest {
            http: self.http,
            account_id: &self.account_id,
            page: None,
            per_page: None,
            search: None,
            sort: None,
        }
    }

    /// Retrieve a single signer.
    ///
    /// `GET /accounts/{account_id}/signers/{signer_id}`.
    pub async fn get<S: AsRef<str>>(&self, signer_id: S) -> Result<Signer> {
        let path = format!(
            "accounts/{}/signers/{}",
            self.account_id,
            signer_id.as_ref()
        );
        let req = self.http.request(Method::GET, &path)?;
        self.http.send_envelope(req).await
    }

    /// Update a signer.
    ///
    /// `PUT /accounts/{account_id}/signers/{signer_id}`.
    pub async fn update<S: AsRef<str>>(
        &self,
        signer_id: S,
        body: &UpdateSignerBody,
    ) -> Result<Signer> {
        let path = format!(
            "accounts/{}/signers/{}",
            self.account_id,
            signer_id.as_ref()
        );
        let req = self.http.request(Method::PUT, &path)?.json(body);
        self.http.send_envelope(req).await
    }

    /// Delete a signer.
    ///
    /// `DELETE /accounts/{account_id}/signers/{signer_id}`.
    pub async fn delete<S: AsRef<str>>(&self, signer_id: S) -> Result<()> {
        let path = format!(
            "accounts/{}/signers/{}",
            self.account_id,
            signer_id.as_ref()
        );
        let req = self.http.request(Method::DELETE, &path)?;
        self.http.send_no_content(req).await
    }
}
