//! Field-definition endpoints.

use reqwest::Method;
use serde::{Deserialize, Serialize};

use crate::error::Result;
use crate::http::HttpClient;
use crate::models::{FieldDefinition, FieldType, FieldValidationResult};
use crate::pagination::Page;

/// Body for `POST /accounts/{account_id}/fields`.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CreateFieldBody {
    /// Field type identifier.
    #[serde(rename = "type")]
    pub kind: String,
    /// Human-readable field name.
    pub name: String,
    /// Optional validation regular expression.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub regex: Option<String>,
    /// Whether this field is required.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub is_required: Option<bool>,
    /// Whether this field is active.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub is_active: Option<bool>,
}

impl CreateFieldBody {
    /// Build a field definition request.
    pub fn new<K, N>(kind: K, name: N) -> Self
    where
        K: Into<String>,
        N: Into<String>,
    {
        Self {
            kind: kind.into(),
            name: name.into(),
            regex: None,
            is_required: None,
            is_active: None,
        }
    }

    /// Set the validation regular expression.
    pub fn regex<S: Into<String>>(mut self, regex: S) -> Self {
        self.regex = Some(regex.into());
        self
    }

    /// Set whether the field is required.
    pub fn required(mut self, required: bool) -> Self {
        self.is_required = Some(required);
        self
    }

    /// Set whether the field is active.
    pub fn active(mut self, active: bool) -> Self {
        self.is_active = Some(active);
        self
    }
}

/// Body for `PUT /accounts/{account_id}/fields/{field_id}`.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct UpdateFieldBody {
    /// New field type identifier.
    #[serde(skip_serializing_if = "Option::is_none", rename = "type")]
    pub kind: Option<String>,
    /// New field name.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub name: Option<String>,
    /// New validation regular expression. Use `Some(None)` to clear it.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub regex: Option<Option<String>>,
    /// New required flag.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub is_required: Option<bool>,
    /// New active flag.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub is_active: Option<bool>,
}

impl UpdateFieldBody {
    /// New empty update body.
    pub fn new() -> Self {
        Self::default()
    }

    /// Set the field type.
    pub fn kind<S: Into<String>>(mut self, kind: S) -> Self {
        self.kind = Some(kind.into());
        self
    }

    /// Set the field name.
    pub fn name<S: Into<String>>(mut self, name: S) -> Self {
        self.name = Some(name.into());
        self
    }

    /// Set the validation regular expression.
    pub fn regex<S: Into<String>>(mut self, regex: S) -> Self {
        self.regex = Some(Some(regex.into()));
        self
    }

    /// Clear the validation regular expression.
    pub fn clear_regex(mut self) -> Self {
        self.regex = Some(None);
        self
    }

    /// Set the required flag.
    pub fn required(mut self, required: bool) -> Self {
        self.is_required = Some(required);
        self
    }

    /// Set the active flag.
    pub fn active(mut self, active: bool) -> Self {
        self.is_active = Some(active);
        self
    }
}

/// Entry for `POST /accounts/{account_id}/fields/validate-multiple`.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ValidateFieldEntry {
    /// Field definition identifier.
    pub field_id: String,
    /// Value to validate.
    pub value: serde_json::Value,
}

impl ValidateFieldEntry {
    /// Build a validation entry.
    pub fn new<S: Into<String>>(field_id: S, value: impl Into<serde_json::Value>) -> Self {
        Self {
            field_id: field_id.into(),
            value: value.into(),
        }
    }
}

/// Builder for `GET /accounts/{account_id}/fields`.
#[derive(Debug)]
pub struct ListFieldsRequest<'a> {
    http: &'a HttpClient,
    account_id: &'a str,
    page: Option<u32>,
    per_page: Option<u32>,
    search: Option<String>,
    sort: Option<String>,
    include_inactive: Option<bool>,
    include_standard: Option<bool>,
}

impl<'a> ListFieldsRequest<'a> {
    /// 1-based page number.
    pub fn page(mut self, page: u32) -> Self {
        self.page = Some(page);
        self
    }

    /// Results per page.
    pub fn per_page(mut self, per_page: u32) -> Self {
        self.per_page = Some(per_page);
        self
    }

    /// Free-text search term.
    pub fn search<S: Into<String>>(mut self, search: S) -> Self {
        self.search = Some(search.into());
        self
    }

    /// Sort expression.
    pub fn sort<S: Into<String>>(mut self, sort: S) -> Self {
        self.sort = Some(sort.into());
        self
    }

    /// Include inactive field definitions.
    pub fn include_inactive(mut self, value: bool) -> Self {
        self.include_inactive = Some(value);
        self
    }

    /// Include standard built-in definitions.
    pub fn include_standard(mut self, value: bool) -> Self {
        self.include_standard = Some(value);
        self
    }

    /// Execute the request.
    pub async fn send(self) -> Result<Page<FieldDefinition>> {
        let path = format!("accounts/{}/fields", self.account_id);
        let mut req = self.http.request(Method::GET, &path)?;
        let mut q: Vec<(&str, String)> = Vec::new();
        if let Some(v) = self.page {
            q.push(("page", v.to_string()));
        }
        if let Some(v) = self.per_page {
            q.push(("per-page", v.to_string()));
        }
        if let Some(v) = self.search {
            q.push(("search", v));
        }
        if let Some(v) = self.sort {
            q.push(("sort", v));
        }
        if let Some(v) = self.include_inactive {
            q.push(("include_inactive", v.to_string()));
        }
        if let Some(v) = self.include_standard {
            q.push(("include_standard", v.to_string()));
        }
        if !q.is_empty() {
            req = req.query(&q);
        }
        self.http.send_paged(req).await
    }
}

/// Field-definition endpoints for a specific account.
#[derive(Debug)]
pub struct FieldsApi<'a> {
    http: &'a HttpClient,
    account_id: String,
}

impl<'a> FieldsApi<'a> {
    pub(crate) fn new(http: &'a HttpClient, account_id: String) -> Self {
        Self { http, account_id }
    }

    /// Create a field definition.
    ///
    /// `POST /accounts/{account_id}/fields`.
    pub async fn create(&self, body: &CreateFieldBody) -> Result<FieldDefinition> {
        let path = format!("accounts/{}/fields", self.account_id);
        let req = self.http.request(Method::POST, &path)?.json(body);
        self.http.send_envelope(req).await
    }

    /// List field definitions.
    ///
    /// `GET /accounts/{account_id}/fields`.
    pub fn list(&self) -> ListFieldsRequest<'_> {
        ListFieldsRequest {
            http: self.http,
            account_id: &self.account_id,
            page: None,
            per_page: None,
            search: None,
            sort: None,
            include_inactive: None,
            include_standard: None,
        }
    }

    /// Retrieve a field definition.
    ///
    /// `GET /accounts/{account_id}/fields/{field_id}`.
    pub async fn get<S: AsRef<str>>(&self, field_id: S) -> Result<FieldDefinition> {
        let path = format!("accounts/{}/fields/{}", self.account_id, field_id.as_ref());
        let req = self.http.request(Method::GET, &path)?;
        self.http.send_envelope(req).await
    }

    /// Update a field definition.
    ///
    /// `PUT /accounts/{account_id}/fields/{field_id}`.
    pub async fn update<S: AsRef<str>>(
        &self,
        field_id: S,
        body: &UpdateFieldBody,
    ) -> Result<FieldDefinition> {
        let path = format!("accounts/{}/fields/{}", self.account_id, field_id.as_ref());
        let req = self.http.request(Method::PUT, &path)?.json(body);
        self.http.send_envelope(req).await
    }

    /// Delete a field definition.
    ///
    /// `DELETE /accounts/{account_id}/fields/{field_id}`.
    pub async fn delete<S: AsRef<str>>(&self, field_id: S) -> Result<()> {
        let path = format!("accounts/{}/fields/{}", self.account_id, field_id.as_ref());
        let req = self.http.request(Method::DELETE, &path)?;
        self.http.send_no_content(req).await
    }

    /// Validate a value against one field definition.
    ///
    /// `POST /accounts/{account_id}/fields/{field_id}/validate`.
    pub async fn validate<S: AsRef<str>>(
        &self,
        field_id: S,
        value: impl Into<serde_json::Value>,
    ) -> Result<FieldValidationResult> {
        let path = format!(
            "accounts/{}/fields/{}/validate",
            self.account_id,
            field_id.as_ref()
        );
        let req = self
            .http
            .request(Method::POST, &path)?
            .json(&serde_json::json!({ "value": value.into() }));
        self.http.send_envelope(req).await
    }

    /// Validate multiple field values.
    ///
    /// `POST /accounts/{account_id}/fields/validate-multiple`.
    pub async fn validate_multiple<I>(&self, entries: I) -> Result<Vec<FieldValidationResult>>
    where
        I: IntoIterator<Item = ValidateFieldEntry>,
    {
        let entries: Vec<ValidateFieldEntry> = entries.into_iter().collect();
        let path = format!("accounts/{}/fields/validate-multiple", self.account_id);
        let req = self.http.request(Method::POST, &path)?.json(&entries);
        self.http.send_envelope(req).await
    }

    /// List supported field types.
    ///
    /// `GET /field-types`.
    pub async fn list_types(&self) -> Result<Vec<FieldType>> {
        let req = self.http.request(Method::GET, "field-types")?;
        self.http.send_envelope(req).await
    }
}
