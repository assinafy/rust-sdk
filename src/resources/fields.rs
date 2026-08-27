//! Field-definition endpoints.

use reqwest::Method;
use serde::{Deserialize, Serialize};

use crate::error::Result;
use crate::http::HttpClient;
use crate::models::{FieldDefinition, FieldType, FieldValidationResult};

/// Body for `POST /accounts/{account_id}/fields`.
///
/// # Request payload
///
/// ```json
/// {
///   "type": "text",
///   "name": "Full name",
///   "regex": "/^.{2,}$/",
///   "is_required": false
/// }
/// ```
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CreateFieldBody {
    /// Field type identifier.
    #[serde(rename = "type")]
    pub kind: String,
    /// Human-readable field name.
    pub name: String,
    /// Optional validation regular expression. Must be a delimited regex
    /// literal (leading/trailing `/`, e.g. `"/^.{2,}$/"`). A bare pattern such
    /// as `"^.{2,}$"` is rejected with `400 Bad Request`.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub regex: Option<String>,
    /// Whether this field is required.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub is_required: Option<bool>,
    /// Compatibility field controlling whether this field is active.
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

    /// Set the validation regular expression. Pass a delimited regex literal
    /// (e.g. `"/^[A-Z]{3}$/"`); a bare pattern is rejected with a 400.
    pub fn regex<S: Into<String>>(mut self, regex: S) -> Self {
        self.regex = Some(regex.into());
        self
    }

    /// Set whether the field is required.
    pub fn required(mut self, required: bool) -> Self {
        self.is_required = Some(required);
        self
    }

    /// Set the compatibility `is_active` create field.
    pub fn active(mut self, active: bool) -> Self {
        self.is_active = Some(active);
        self
    }
}

/// Body for `PUT /accounts/{account_id}/fields/{field_id}`.
///
/// Only the fields that are set are serialized. A `regex` of `null` clears the
/// stored expression.
///
/// # Request payload
///
/// ```json
/// {
///   "name": "Full name (updated)",
///   "regex": null,
///   "is_active": true
/// }
/// ```
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct UpdateFieldBody {
    /// Compatibility field for changing the field type.
    #[serde(skip_serializing_if = "Option::is_none", rename = "type")]
    pub kind: Option<String>,
    /// New field name.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub name: Option<String>,
    /// New validation regular expression. Use `Some(None)` to clear it.
    /// Must be a delimited regex literal (e.g. `"/^[A-Z]{3}$/"`) — see
    /// [`CreateFieldBody::regex`].
    #[serde(skip_serializing_if = "Option::is_none")]
    pub regex: Option<Option<String>>,
    /// Compatibility field for changing the required flag.
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

    /// Set the compatibility field type.
    pub fn kind<S: Into<String>>(mut self, kind: S) -> Self {
        self.kind = Some(kind.into());
        self
    }

    /// Set the field name.
    pub fn name<S: Into<String>>(mut self, name: S) -> Self {
        self.name = Some(name.into());
        self
    }

    /// Set the validation regular expression. Pass a delimited regex literal
    /// (e.g. `"/^[A-Z]{3}$/"`); a bare pattern is rejected with a 400.
    pub fn regex<S: Into<String>>(mut self, regex: S) -> Self {
        self.regex = Some(Some(regex.into()));
        self
    }

    /// Clear the validation regular expression.
    pub fn clear_regex(mut self) -> Self {
        self.regex = Some(None);
        self
    }

    /// Set the compatibility required flag.
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
///
/// The endpoint accepts a JSON array of these entries. A single entry
/// serializes as:
///
/// # Request payload
///
/// ```json
/// {
///   "field_id": "102d25a48bf5816b9029b0ca6043",
///   "value": "400.676.228-36"
/// }
/// ```
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
///
/// This endpoint is **not** paginated — it returns every field definition in a
/// single flat array — so the builder only exposes the documented
/// `include_inactive` / `include_standard` toggles and [`send`](Self::send)
/// returns a plain [`Vec`].
#[derive(Debug)]
pub struct ListFieldsRequest<'a> {
    http: &'a HttpClient,
    account_id: &'a str,
    include_inactive: Option<bool>,
    include_standard: Option<bool>,
}

impl<'a> ListFieldsRequest<'a> {
    /// Include inactive field definitions.
    pub fn include_inactive(mut self, value: bool) -> Self {
        self.include_inactive = Some(value);
        self
    }

    /// Include standard built-in definitions (e.g. `signature`, `initial`,
    /// `signatureDate`).
    pub fn include_standard(mut self, value: bool) -> Self {
        self.include_standard = Some(value);
        self
    }

    /// Execute the request.
    ///
    /// `GET /accounts/{account_id}/fields`. Returns a bare (non-paginated)
    /// array of field definitions.
    ///
    /// # Response payload
    ///
    /// ```json
    /// {
    ///   "status": 200,
    ///   "message": "",
    ///   "data": [
    ///     {
    ///       "id": "102d25a48bec03ebcf3b5f651998",
    ///       "name": "Nome",
    ///       "type": "personName",
    ///       "regex": null,
    ///       "is_pre_defined": true,
    ///       "is_active": true,
    ///       "is_required": false,
    ///       "is_standard": false,
    ///       "is_read_only": false,
    ///       "is_visible": true
    ///     }
    ///   ]
    /// }
    /// ```
    pub async fn send(self) -> Result<Vec<FieldDefinition>> {
        let path = self.http.path(&["accounts", self.account_id, "fields"])?;
        let mut req = self.http.request(Method::GET, &path)?;
        let mut q: Vec<(&str, String)> = Vec::new();
        if let Some(v) = self.include_inactive {
            q.push(("include_inactive", v.to_string()));
        }
        if let Some(v) = self.include_standard {
            q.push(("include_standard", v.to_string()));
        }
        if !q.is_empty() {
            req = req.query(&q);
        }
        self.http.send_envelope(req).await
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
    ///
    /// # Request payload
    ///
    /// ```json
    /// {
    ///   "type": "text",
    ///   "name": "Full name",
    ///   "is_required": false
    /// }
    /// ```
    ///
    /// # Response payload
    ///
    /// ```json
    /// {
    ///   "status": 200,
    ///   "message": "",
    ///   "data": {
    ///     "resource": "field",
    ///     "id": "103b03a56d52a4bea540f9af20a8",
    ///     "name": "Full name",
    ///     "type": "text",
    ///     "regex": null,
    ///     "is_pre_defined": false,
    ///     "is_active": true,
    ///     "is_required": false,
    ///     "is_standard": false,
    ///     "is_read_only": false,
    ///     "is_visible": true
    ///   }
    /// }
    /// ```
    pub async fn create(&self, body: &CreateFieldBody) -> Result<FieldDefinition> {
        let path = self
            .http
            .path(&["accounts", self.account_id.as_str(), "fields"])?;
        let req = self.http.request(Method::POST, &path)?.json(body);
        self.http.send_envelope(req).await
    }

    /// List field definitions.
    ///
    /// `GET /accounts/{account_id}/fields`. Returns every definition in one
    /// response (the endpoint is not paginated).
    pub fn list(&self) -> ListFieldsRequest<'_> {
        ListFieldsRequest {
            http: self.http,
            account_id: &self.account_id,
            include_inactive: None,
            include_standard: None,
        }
    }

    /// Retrieve a field definition.
    ///
    /// `GET /accounts/{account_id}/fields/{field_id}`.
    ///
    /// # Response payload
    ///
    /// ```json
    /// {
    ///   "status": 200,
    ///   "message": "",
    ///   "data": {
    ///     "resource": "field",
    ///     "id": "103b03a56d52a4bea540f9af20a8",
    ///     "name": "Example Field",
    ///     "type": "text",
    ///     "regex": null,
    ///     "is_pre_defined": false,
    ///     "is_active": true,
    ///     "is_required": false,
    ///     "is_standard": false,
    ///     "is_read_only": false,
    ///     "is_visible": true
    ///   }
    /// }
    /// ```
    pub async fn get<S: AsRef<str>>(&self, field_id: S) -> Result<FieldDefinition> {
        let path = self.http.path(&[
            "accounts",
            self.account_id.as_str(),
            "fields",
            field_id.as_ref(),
        ])?;
        let req = self.http.request(Method::GET, &path)?;
        self.http.send_envelope(req).await
    }

    /// Update a field definition.
    ///
    /// `PUT /accounts/{account_id}/fields/{field_id}`.
    ///
    /// # Request payload
    ///
    /// ```json
    /// {
    ///   "name": "Full name (updated)",
    ///   "regex": null,
    ///   "is_active": true
    /// }
    /// ```
    ///
    /// # Response payload
    ///
    /// ```json
    /// {
    ///   "status": 200,
    ///   "message": "",
    ///   "data": {
    ///     "resource": "field",
    ///     "id": "103b03a56d52a4bea540f9af20a8",
    ///     "name": "Full name (updated)",
    ///     "type": "text",
    ///     "regex": null,
    ///     "is_pre_defined": false,
    ///     "is_active": true,
    ///     "is_required": false,
    ///     "is_standard": false,
    ///     "is_read_only": false,
    ///     "is_visible": true
    ///   }
    /// }
    /// ```
    pub async fn update<S: AsRef<str>>(
        &self,
        field_id: S,
        body: &UpdateFieldBody,
    ) -> Result<FieldDefinition> {
        let path = self.http.path(&[
            "accounts",
            self.account_id.as_str(),
            "fields",
            field_id.as_ref(),
        ])?;
        let req = self.http.request(Method::PUT, &path)?.json(body);
        self.http.send_envelope(req).await
    }

    /// Delete a field definition.
    ///
    /// `DELETE /accounts/{account_id}/fields/{field_id}`.
    ///
    /// # Response payload
    ///
    /// ```json
    /// {
    ///   "status": 200,
    ///   "message": "",
    ///   "data": []
    /// }
    /// ```
    pub async fn delete<S: AsRef<str>>(&self, field_id: S) -> Result<()> {
        let path = self.http.path(&[
            "accounts",
            self.account_id.as_str(),
            "fields",
            field_id.as_ref(),
        ])?;
        let req = self.http.request(Method::DELETE, &path)?;
        self.http.send_no_content(req).await
    }

    /// Validate a value against one field definition.
    ///
    /// `POST /accounts/{account_id}/fields/{field_id}/validate`. Works in both
    /// the authenticated-user context (API key / bearer) and the signer
    /// context — for the latter, build the client with
    /// [`Auth::AccessCode`](crate::Auth::AccessCode) and the
    /// `signer-access-code` query parameter is sent automatically.
    ///
    /// # Request payload
    ///
    /// ```json
    /// {
    ///   "value": "400.676.228-36"
    /// }
    /// ```
    ///
    /// # Response payload
    ///
    /// ```json
    /// {
    ///   "status": 200,
    ///   "message": "",
    ///   "data": {
    ///     "type": "text",
    ///     "success": true,
    ///     "error_message": ""
    ///   }
    /// }
    /// ```
    pub async fn validate<S: AsRef<str>>(
        &self,
        field_id: S,
        value: impl Into<serde_json::Value>,
    ) -> Result<FieldValidationResult> {
        let path = self.http.path(&[
            "accounts",
            self.account_id.as_str(),
            "fields",
            field_id.as_ref(),
            "validate",
        ])?;
        let req = self
            .http
            .request(Method::POST, &path)?
            .json(&serde_json::json!({ "value": value.into() }));
        self.http.send_envelope(req).await
    }

    /// Validate multiple field values.
    ///
    /// `POST /accounts/{account_id}/fields/validate-multiple`.
    ///
    /// # Request payload
    ///
    /// ```json
    /// [
    ///   {
    ///     "field_id": "102d25a48bf5816b9029b0ca6043",
    ///     "value": "400.676.228-36"
    ///   },
    ///   {
    ///     "field_id": "102d25a48c0e2d4e79477d673896",
    ///     "value": "user@example.invalid"
    ///   }
    /// ]
    /// ```
    ///
    /// # Response payload
    ///
    /// ```json
    /// {
    ///   "status": 200,
    ///   "message": "",
    ///   "data": [
    ///     {
    ///       "type": "cpf",
    ///       "success": true,
    ///       "error_message": ""
    ///     },
    ///     {
    ///       "type": "email",
    ///       "success": true,
    ///       "error_message": ""
    ///     }
    ///   ]
    /// }
    /// ```
    pub async fn validate_multiple<I>(&self, entries: I) -> Result<Vec<FieldValidationResult>>
    where
        I: IntoIterator<Item = ValidateFieldEntry>,
    {
        let entries: Vec<ValidateFieldEntry> = entries.into_iter().collect();
        let path = self.http.path(&[
            "accounts",
            self.account_id.as_str(),
            "fields",
            "validate-multiple",
        ])?;
        let req = self.http.request(Method::POST, &path)?.json(&entries);
        self.http.send_envelope(req).await
    }

    /// List supported field types.
    ///
    /// `GET /field-types`.
    ///
    /// # Response payload
    ///
    /// ```json
    /// {
    ///   "status": 200,
    ///   "message": "",
    ///   "data": [
    ///     { "type": "personName", "name": "Nome" },
    ///     { "type": "text", "name": "Texto" }
    ///   ]
    /// }
    /// ```
    pub async fn list_types(&self) -> Result<Vec<FieldType>> {
        let req = self.http.request(Method::GET, "field-types")?;
        self.http.send_envelope(req).await
    }
}
