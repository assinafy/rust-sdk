//! Contract tracking endpoints.

use std::collections::BTreeMap;

use reqwest::Method;
use serde::Serialize;

use crate::error::{Error, Result};
use crate::http::HttpClient;
use crate::models::{
    ContractExternalReference, ContractRelationType, ContractStatus, DocumentContract,
    DocumentContractDetails,
};
use crate::pagination::Page;

/// Partial body for `PUT /accounts/{account_id}/documents/{document_id}/contract`.
///
/// Only configured fields are sent. Nullable fields use `Some(None)` to clear
/// their stored value; the fluent `clear_*` methods provide that wire shape.
///
/// # Request payload
///
/// ```json
/// {
///   "status": "active",
///   "counterparty_name": "Example Counterparty",
///   "value": 120000.5,
///   "currency": "BRL",
///   "start_date": "2026-11-01",
///   "end_date": "2027-10-31",
///   "auto_renew": true,
///   "renewal_term_months": 12,
///   "notice_period_days": 30,
///   "metadata": { "crm_stage": "won" },
///   "external_references": [{ "system": "hubspot", "id": "98765" }]
/// }
/// ```
#[derive(Debug, Clone, Default, Serialize)]
pub struct UpdateContractBody {
    /// New lifecycle state.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub status: Option<ContractStatus>,
    /// New counterparty name; `Some(None)` clears it.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub counterparty_name: Option<Option<String>>,
    /// New counterparty CPF/CNPJ; `Some(None)` clears it.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub counterparty_taxpayer_id: Option<Option<String>>,
    /// New contract value; `Some(None)` clears it.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub value: Option<Option<f64>>,
    /// New ISO 4217 currency; `Some(None)` resets it to the API default while
    /// a value is set.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub currency: Option<Option<String>>,
    /// New start date; `Some(None)` clears it.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub start_date: Option<Option<String>>,
    /// New end date; `Some(None)` clears it.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub end_date: Option<Option<String>>,
    /// Whether the lifecycle job should auto-renew the contract.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub auto_renew: Option<bool>,
    /// New renewal length; `Some(None)` restores the API's 12-month default.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub renewal_term_months: Option<Option<u32>>,
    /// New notice period; `Some(None)` disables notice-period alerts.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub notice_period_days: Option<Option<u32>>,
    /// New owner; `Some(None)` clears the owner.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub owner_id: Option<Option<String>>,
    /// Replacement metadata; `Some(None)` clears it.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub metadata: Option<Option<BTreeMap<String, String>>>,
    /// Replacement external-reference list; `Some(None)` clears it.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub external_references: Option<Option<Vec<ContractExternalReference>>>,
    /// Parent document for an amendment link; `Some(None)` unlinks a pending
    /// amendment.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub parent_document_id: Option<Option<String>>,
    /// Relationship to the parent document.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub relation_type: Option<ContractRelationType>,
    /// Previously read `updated_at`, used as an optimistic write precondition.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub updated_at: Option<String>,
}

impl UpdateContractBody {
    /// Create an empty partial update. Set at least one field before sending it.
    pub fn new() -> Self {
        Self::default()
    }

    /// Set the lifecycle state.
    pub fn status(mut self, status: ContractStatus) -> Self {
        self.status = Some(status);
        self
    }

    /// Set the counterparty name.
    pub fn counterparty_name<S: Into<String>>(mut self, value: S) -> Self {
        self.counterparty_name = Some(Some(value.into()));
        self
    }

    /// Clear the counterparty name.
    pub fn clear_counterparty_name(mut self) -> Self {
        self.counterparty_name = Some(None);
        self
    }

    /// Set the counterparty CPF/CNPJ.
    pub fn counterparty_taxpayer_id<S: Into<String>>(mut self, value: S) -> Self {
        self.counterparty_taxpayer_id = Some(Some(value.into()));
        self
    }

    /// Clear the counterparty CPF/CNPJ.
    pub fn clear_counterparty_taxpayer_id(mut self) -> Self {
        self.counterparty_taxpayer_id = Some(None);
        self
    }

    /// Set the contract value.
    pub fn value(mut self, value: f64) -> Self {
        self.value = Some(Some(value));
        self
    }

    /// Clear the contract value.
    pub fn clear_value(mut self) -> Self {
        self.value = Some(None);
        self
    }

    /// Set the ISO 4217 currency code.
    pub fn currency<S: Into<String>>(mut self, value: S) -> Self {
        self.currency = Some(Some(value.into()));
        self
    }

    /// Reset the currency to the API default while a value is set.
    pub fn clear_currency(mut self) -> Self {
        self.currency = Some(None);
        self
    }

    /// Set the start date in `YYYY-MM-DD` form.
    pub fn start_date<S: Into<String>>(mut self, value: S) -> Self {
        self.start_date = Some(Some(value.into()));
        self
    }

    /// Clear the start date.
    pub fn clear_start_date(mut self) -> Self {
        self.start_date = Some(None);
        self
    }

    /// Set the end date in `YYYY-MM-DD` form.
    pub fn end_date<S: Into<String>>(mut self, value: S) -> Self {
        self.end_date = Some(Some(value.into()));
        self
    }

    /// Clear the end date.
    pub fn clear_end_date(mut self) -> Self {
        self.end_date = Some(None);
        self
    }

    /// Enable or disable automatic renewal.
    pub fn auto_renew(mut self, value: bool) -> Self {
        self.auto_renew = Some(value);
        self
    }

    /// Set the renewal length in months.
    pub fn renewal_term_months(mut self, value: u32) -> Self {
        self.renewal_term_months = Some(Some(value));
        self
    }

    /// Restore the API's default renewal length.
    pub fn clear_renewal_term_months(mut self) -> Self {
        self.renewal_term_months = Some(None);
        self
    }

    /// Set the notice period in days.
    pub fn notice_period_days(mut self, value: u32) -> Self {
        self.notice_period_days = Some(Some(value));
        self
    }

    /// Disable the notice period.
    pub fn clear_notice_period_days(mut self) -> Self {
        self.notice_period_days = Some(None);
        self
    }

    /// Assign the contract to a workspace member.
    pub fn owner_id<S: Into<String>>(mut self, value: S) -> Self {
        self.owner_id = Some(Some(value.into()));
        self
    }

    /// Clear the contract owner.
    pub fn clear_owner_id(mut self) -> Self {
        self.owner_id = Some(None);
        self
    }

    /// Replace the caller-defined metadata.
    pub fn metadata(mut self, value: BTreeMap<String, String>) -> Self {
        self.metadata = Some(Some(value));
        self
    }

    /// Clear caller-defined metadata.
    pub fn clear_metadata(mut self) -> Self {
        self.metadata = Some(None);
        self
    }

    /// Replace all external references.
    pub fn external_references<I>(mut self, values: I) -> Self
    where
        I: IntoIterator<Item = ContractExternalReference>,
    {
        self.external_references = Some(Some(values.into_iter().collect()));
        self
    }

    /// Clear all external references.
    pub fn clear_external_references(mut self) -> Self {
        self.external_references = Some(None);
        self
    }

    /// Link this document to a parent contract document.
    pub fn parent_document_id<S: Into<String>>(mut self, value: S) -> Self {
        self.parent_document_id = Some(Some(value.into()));
        self
    }

    /// Unlink this document from its parent while the amendment is pending.
    pub fn clear_parent_document_id(mut self) -> Self {
        self.parent_document_id = Some(None);
        self
    }

    /// Set how this document relates to its parent contract.
    pub fn relation_type(mut self, value: ContractRelationType) -> Self {
        self.relation_type = Some(value);
        self
    }

    /// Refuse the write if the resource has changed since this timestamp.
    pub fn updated_at<S: Into<String>>(mut self, value: S) -> Self {
        self.updated_at = Some(value.into());
        self
    }
}

/// Body for renewing a tracked contract.
///
/// # Request payload
///
/// Renew from a known term:
///
/// ```json
/// { "term": 1, "value": 126000 }
/// ```
///
/// Or renew to an explicit date:
///
/// ```json
/// { "end_date": "2029-06-30", "value": 130000 }
/// ```
#[derive(Debug, Clone, Serialize)]
pub struct RenewContractBody {
    /// Explicit new end date; absent uses the configured renewal term.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub end_date: Option<String>,
    /// Replacement value; `Some(None)` clears it.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub value: Option<Option<f64>>,
    /// Current term, required when no explicit end date is sent.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub term: Option<u32>,
    /// Previously read `updated_at`, used as an optimistic write precondition.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub updated_at: Option<String>,
}

impl RenewContractBody {
    /// Renew from the current term using the configured renewal length.
    pub fn from_term(term: u32) -> Self {
        Self {
            end_date: None,
            value: None,
            term: Some(term),
            updated_at: None,
        }
    }

    /// Renew to an explicit end date in `YYYY-MM-DD` form.
    pub fn to_date<S: Into<String>>(end_date: S) -> Self {
        Self {
            end_date: Some(end_date.into()),
            value: None,
            term: None,
            updated_at: None,
        }
    }

    /// Set the next term's value.
    pub fn value(mut self, value: f64) -> Self {
        self.value = Some(Some(value));
        self
    }

    /// Clear the contract value on renewal.
    pub fn clear_value(mut self) -> Self {
        self.value = Some(None);
        self
    }

    /// Refuse the renewal if the contract changed since this timestamp.
    pub fn updated_at<S: Into<String>>(mut self, value: S) -> Self {
        self.updated_at = Some(value.into());
        self
    }
}

/// Optional body for terminating a tracked contract.
///
/// # Request payload
///
/// ```json
/// { "reason": "Supplier replaced", "updated_at": "2026-10-23T12:00:00Z" }
/// ```
#[derive(Debug, Clone, Default, Serialize)]
pub struct TerminateContractBody {
    /// Reason recorded in the termination activity.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub reason: Option<String>,
    /// Previously read `updated_at`, used as an optimistic write precondition.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub updated_at: Option<String>,
}

impl TerminateContractBody {
    /// Create a termination request without a reason or precondition.
    pub fn new() -> Self {
        Self::default()
    }

    /// Record a reason for the termination.
    pub fn reason<S: Into<String>>(mut self, value: S) -> Self {
        self.reason = Some(value.into());
        self
    }

    /// Refuse the termination if the contract changed since this timestamp.
    pub fn updated_at<S: Into<String>>(mut self, value: S) -> Self {
        self.updated_at = Some(value.into());
        self
    }
}

/// Builder for `GET /accounts/{account_id}/contracts`.
#[derive(Debug)]
pub struct ListContractsRequest<'a> {
    http: &'a HttpClient,
    account_id: &'a str,
    statuses: Vec<ContractStatus>,
    counterparty: Option<String>,
    search: Option<String>,
    end_date_from: Option<String>,
    end_date_to: Option<String>,
    ending_within_days: Option<u16>,
    external_system: Option<String>,
    external_id: Option<String>,
    sort: Option<String>,
    expand_document: bool,
    page: Option<u32>,
    per_page: Option<u32>,
}

impl<'a> ListContractsRequest<'a> {
    /// Match any of the supplied lifecycle states.
    pub fn statuses<I>(mut self, values: I) -> Self
    where
        I: IntoIterator<Item = ContractStatus>,
    {
        self.statuses = values.into_iter().collect();
        self
    }

    /// Match part of a counterparty name or an exact CPF/CNPJ.
    pub fn counterparty<S: Into<String>>(mut self, value: S) -> Self {
        self.counterparty = Some(value.into());
        self
    }

    /// Match part of the document name.
    pub fn search<S: Into<String>>(mut self, value: S) -> Self {
        self.search = Some(value.into());
        self
    }

    /// Match contracts ending on or after this `YYYY-MM-DD` date.
    pub fn end_date_from<S: Into<String>>(mut self, value: S) -> Self {
        self.end_date_from = Some(value.into());
        self
    }

    /// Match contracts ending on or before this `YYYY-MM-DD` date.
    pub fn end_date_to<S: Into<String>>(mut self, value: S) -> Self {
        self.end_date_to = Some(value.into());
        self
    }

    /// Match contracts ending between today and this many days later.
    pub fn ending_within_days(mut self, value: u16) -> Self {
        self.ending_within_days = Some(value);
        self
    }

    /// Match an external-reference system.
    pub fn external_system<S: Into<String>>(mut self, value: S) -> Self {
        self.external_system = Some(value.into());
        self
    }

    /// Match an external-reference identifier.
    pub fn external_id<S: Into<String>>(mut self, value: S) -> Self {
        self.external_id = Some(value.into());
        self
    }

    /// Sort by `end_date` or `updated_at`, optionally prefixed with `-`.
    pub fn sort<S: Into<String>>(mut self, value: S) -> Self {
        self.sort = Some(value.into());
        self
    }

    /// Embed each contract's document.
    pub fn expand_document(mut self) -> Self {
        self.expand_document = true;
        self
    }

    /// Set the 1-based result page.
    pub fn page(mut self, value: u32) -> Self {
        self.page = Some(value);
        self
    }

    /// Set the number of rows per page.
    pub fn per_page(mut self, value: u32) -> Self {
        self.per_page = Some(value);
        self
    }

    /// Execute the request.
    ///
    /// `GET /accounts/{account_id}/contracts`.
    ///
    /// # Request payload
    ///
    /// No request body. Query parameters are URL-encoded; only configured
    /// filters are sent.
    ///
    /// ```json
    /// {
    ///   "status": "active,expired",
    ///   "counterparty": "Example Counterparty",
    ///   "search": "Services",
    ///   "end_date_from": "2027-01-01",
    ///   "end_date_to": "2027-12-31",
    ///   "ending_within_days": 30,
    ///   "external_system": "hubspot",
    ///   "external_id": "98765",
    ///   "sort": "end_date",
    ///   "expand": "document",
    ///   "page": 1,
    ///   "per-page": 20
    /// }
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
    ///       "document_id": "66b2f0e1c3a4d5e6f7a8b9c0",
    ///       "status": "active",
    ///       "counterparty_name": "Example Counterparty",
    ///       "value": 120000.5,
    ///       "currency": "BRL",
    ///       "end_date": "2027-10-31",
    ///       "auto_renew": true,
    ///       "term": 1,
    ///       "external_references": [],
    ///       "created_at": "2026-10-23T12:00:00Z",
    ///       "updated_at": "2026-10-23T12:00:00Z"
    ///     }
    ///   ]
    /// }
    /// ```
    pub async fn send(self) -> Result<Page<DocumentContract>> {
        let path = self
            .http
            .path(&["accounts", self.account_id, "contracts"])?;
        let mut query: Vec<(&str, String)> = Vec::new();
        if !self.statuses.is_empty() {
            query.push((
                "status",
                self.statuses
                    .iter()
                    .map(ContractStatus::as_str)
                    .collect::<Vec<_>>()
                    .join(","),
            ));
        }
        if let Some(value) = self.counterparty {
            query.push(("counterparty", value));
        }
        if let Some(value) = self.search {
            query.push(("search", value));
        }
        if let Some(value) = self.end_date_from {
            query.push(("end_date_from", value));
        }
        if let Some(value) = self.end_date_to {
            query.push(("end_date_to", value));
        }
        if let Some(value) = self.ending_within_days {
            query.push(("ending_within_days", value.to_string()));
        }
        if let Some(value) = self.external_system {
            query.push(("external_system", value));
        }
        if let Some(value) = self.external_id {
            query.push(("external_id", value));
        }
        if let Some(value) = self.sort {
            query.push(("sort", value));
        }
        if self.expand_document {
            query.push(("expand", "document".into()));
        }
        if let Some(value) = self.page {
            query.push(("page", value.to_string()));
        }
        if let Some(value) = self.per_page {
            query.push(("per-page", value.to_string()));
        }

        let mut request = self.http.request(Method::GET, &path)?;
        if !query.is_empty() {
            request = request.query(&query);
        }
        self.http.send_paged(request).await
    }
}

/// Contract tracking endpoints for one workspace.
///
/// Obtain one with [`Client::contracts`](crate::Client::contracts).
#[derive(Debug)]
pub struct ContractsApi<'a> {
    http: &'a HttpClient,
    account_id: String,
}

impl<'a> ContractsApi<'a> {
    pub(crate) fn new(http: &'a HttpClient, account_id: String) -> Self {
        Self { http, account_id }
    }

    /// List tracked contracts in this workspace.
    ///
    /// `GET /accounts/{account_id}/contracts`.
    ///
    /// # Request payload
    ///
    /// See [`ListContractsRequest::send`] for every query field.
    ///
    /// # Response payload
    ///
    /// See [`ListContractsRequest::send`] for the JSON response example.
    pub fn list(&self) -> ListContractsRequest<'_> {
        ListContractsRequest {
            http: self.http,
            account_id: &self.account_id,
            statuses: Vec::new(),
            counterparty: None,
            search: None,
            end_date_from: None,
            end_date_to: None,
            ending_within_days: None,
            external_system: None,
            external_id: None,
            sort: None,
            expand_document: false,
            page: None,
            per_page: None,
        }
    }

    /// Read a document's contract terms or amendment link.
    ///
    /// `GET /accounts/{account_id}/documents/{document_id}/contract`.
    ///
    /// # Request payload
    ///
    /// No request body. Resource identifiers are supplied in the route.
    ///
    /// # Response payload
    ///
    /// ```json
    /// {
    ///   "status": 200,
    ///   "message": "",
    ///   "data": {
    ///     "resource": "document_contract",
    ///     "document_id": "66b2f0e1c3a4d5e6f7a8b9c0",
    ///     "status": "active",
    ///     "counterparty_name": "Example Counterparty",
    ///     "value": 120000.5,
    ///     "currency": "BRL",
    ///     "start_date": "2026-11-01",
    ///     "end_date": "2027-10-31",
    ///     "auto_renew": true,
    ///     "term": 1,
    ///     "external_references": [],
    ///     "amendments": [],
    ///     "created_at": "2026-10-23T12:00:00Z",
    ///     "updated_at": "2026-10-23T12:00:00Z"
    ///   }
    /// }
    /// ```
    pub async fn get<S: AsRef<str>>(&self, document_id: S) -> Result<DocumentContractDetails> {
        let path = self.contract_path(document_id.as_ref(), &[])?;
        let request = self.http.request(Method::GET, &path)?;
        self.http.send_envelope(request).await
    }

    /// Create or partially update contract terms, or link an amendment.
    ///
    /// `PUT /accounts/{account_id}/documents/{document_id}/contract`.
    ///
    /// # Request payload
    ///
    /// ```json
    /// {
    ///   "status": "active",
    ///   "counterparty_name": "Example Counterparty",
    ///   "value": 120000.5,
    ///   "currency": "BRL",
    ///   "end_date": "2027-10-31",
    ///   "updated_at": "2026-10-23T12:00:00Z"
    /// }
    /// ```
    ///
    /// To link an amendment, send `parent_document_id`, `relation_type`, and
    /// only the terms changed by the instrument.
    ///
    /// # Response payload
    ///
    /// ```json
    /// {
    ///   "status": 200,
    ///   "message": "",
    ///   "data": {
    ///     "resource": "document_contract_amendment",
    ///     "document_id": "7c3d1e2f4a5b6c7d8e9f0a1b",
    ///     "document_name": "Amendment 1.pdf",
    ///     "parent_document_id": "66b2f0e1c3a4d5e6f7a8b9c0",
    ///     "relation_type": "amendment",
    ///     "number": null,
    ///     "status": "pending",
    ///     "document_status": "pending_signature",
    ///     "apply_error": null,
    ///     "currency": "BRL",
    ///     "terms": { "value": 130000 },
    ///     "changes": { "value": { "from": 120000.5, "to": 130000 } },
    ///     "applied_at": null,
    ///     "created_at": "2027-05-10T15:00:00Z",
    ///     "updated_at": "2027-05-10T15:00:00Z"
    ///   }
    /// }
    /// ```
    pub async fn update<S: AsRef<str>>(
        &self,
        document_id: S,
        body: &UpdateContractBody,
    ) -> Result<DocumentContractDetails> {
        let path = self.contract_path(document_id.as_ref(), &[])?;
        let request = self.http.request(Method::PUT, &path)?.json(body);
        self.http.send_envelope(request).await
    }

    /// Read the root contract and every linked instrument in its family.
    ///
    /// `GET /accounts/{account_id}/documents/{document_id}/contract/related`.
    ///
    /// # Request payload
    ///
    /// No request body. Resource identifiers are supplied in the route.
    ///
    /// # Response payload
    ///
    /// ```json
    /// {
    ///   "status": 200,
    ///   "message": "",
    ///   "data": {
    ///     "resource": "document_contract",
    ///     "document_id": "66b2f0e1c3a4d5e6f7a8b9c0",
    ///     "status": "active",
    ///     "auto_renew": true,
    ///     "term": 2,
    ///     "external_references": [],
    ///     "amendments": [
    ///       {
    ///         "resource": "document_contract_amendment",
    ///         "document_id": "7c3d1e2f4a5b6c7d8e9f0a1b",
    ///         "document_name": "Amendment 1.pdf",
    ///         "parent_document_id": "66b2f0e1c3a4d5e6f7a8b9c0",
    ///         "relation_type": "amendment",
    ///         "number": 1,
    ///         "status": "applied",
    ///         "document_status": "certificated"
    ///       }
    ///     ]
    ///   }
    /// }
    /// ```
    pub async fn related<S: AsRef<str>>(&self, document_id: S) -> Result<DocumentContract> {
        let path = self.contract_path(document_id.as_ref(), &["related"])?;
        let request = self.http.request(Method::GET, &path)?;
        self.http.send_envelope(request).await
    }

    /// Start the contract's next term.
    ///
    /// `POST /accounts/{account_id}/documents/{document_id}/contract/renew`.
    ///
    /// # Request payload
    ///
    /// ```json
    /// { "term": 1, "value": 126000, "updated_at": "2026-10-23T12:00:00Z" }
    /// ```
    ///
    /// # Response payload
    ///
    /// ```json
    /// {
    ///   "status": 200,
    ///   "message": "",
    ///   "data": {
    ///     "resource": "document_contract",
    ///     "document_id": "66b2f0e1c3a4d5e6f7a8b9c0",
    ///     "status": "active",
    ///     "value": 126000,
    ///     "currency": "BRL",
    ///     "end_date": "2028-10-31",
    ///     "auto_renew": true,
    ///     "term": 2,
    ///     "external_references": [],
    ///     "updated_at": "2027-10-15T09:30:00Z"
    ///   }
    /// }
    /// ```
    pub async fn renew<S: AsRef<str>>(
        &self,
        document_id: S,
        body: &RenewContractBody,
    ) -> Result<DocumentContract> {
        if body.end_date.is_none() && body.term.is_none() {
            return Err(Error::Validation(
                "a contract renewal requires the current term or a new end date".into(),
            ));
        }
        if body.term == Some(0) {
            return Err(Error::Validation(
                "a contract renewal term must be at least 1".into(),
            ));
        }
        let path = self.contract_path(document_id.as_ref(), &["renew"])?;
        let request = self.http.request(Method::POST, &path)?.json(body);
        self.http.send_envelope(request).await
    }

    /// Terminate a tracked contract.
    ///
    /// `POST /accounts/{account_id}/documents/{document_id}/contract/terminate`.
    ///
    /// # Request payload
    ///
    /// ```json
    /// { "reason": "Supplier replaced", "updated_at": "2026-10-23T12:00:00Z" }
    /// ```
    ///
    /// # Response payload
    ///
    /// ```json
    /// {
    ///   "status": 200,
    ///   "message": "",
    ///   "data": {
    ///     "resource": "document_contract",
    ///     "document_id": "66b2f0e1c3a4d5e6f7a8b9c0",
    ///     "status": "terminated",
    ///     "counterparty_name": "Example Counterparty",
    ///     "value": 120000.5,
    ///     "currency": "BRL",
    ///     "end_date": "2027-10-31",
    ///     "auto_renew": true,
    ///     "term": 1,
    ///     "external_references": [],
    ///     "updated_at": "2027-03-02T14:00:00Z"
    ///   }
    /// }
    /// ```
    pub async fn terminate<S: AsRef<str>>(
        &self,
        document_id: S,
        body: &TerminateContractBody,
    ) -> Result<DocumentContract> {
        let path = self.contract_path(document_id.as_ref(), &["terminate"])?;
        let request = self.http.request(Method::POST, &path)?.json(body);
        self.http.send_envelope(request).await
    }

    fn contract_path(&self, document_id: &str, suffix: &[&str]) -> Result<String> {
        let mut segments = vec![
            "accounts",
            self.account_id.as_str(),
            "documents",
            document_id,
            "contract",
        ];
        segments.extend_from_slice(suffix);
        self.http.path(&segments)
    }
}
