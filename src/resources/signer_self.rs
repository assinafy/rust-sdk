//! Endpoints that operate on the currently authenticated signer.
//!
//! All routes here require an `Auth::AccessCode` credential
//! (`?signer-access-code=...`).

use bytes::Bytes;
use reqwest::Method;
use serde::{Deserialize, Serialize};

use crate::error::Result;
use crate::http::HttpClient;
use crate::models::{ArtifactName, Document, SignDocumentItem, SignerSelf, SignerType};
use crate::pagination::Page;

/// Body for `POST /verify`.
///
/// # Request payload
///
/// ```json
/// {
///   "verification-code": "123456",
///   "signer-access-code": "AbCd1234EfGh5678"
/// }
/// ```
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VerifyCodeBody {
    /// One-time code emailed or sent via WhatsApp to the signer.
    #[serde(rename = "verification-code")]
    pub code: String,
    /// Signer access code. Usually supplied by [`crate::Auth::AccessCode`].
    #[serde(rename = "signer-access-code", skip_serializing_if = "Option::is_none")]
    pub signer_access_code: Option<String>,
}

impl VerifyCodeBody {
    /// Build a verification request from the received code.
    pub fn new<S: Into<String>>(code: S) -> Self {
        Self {
            code: code.into(),
            signer_access_code: None,
        }
    }

    /// Include the signer access code in the JSON body.
    pub fn access_code<S: Into<String>>(mut self, code: S) -> Self {
        self.signer_access_code = Some(code.into());
        self
    }
}

/// Body for `PUT /documents/{document_id}/signers/confirm-data`.
///
/// # Request payload
///
/// All fields are optional; send only the ones being confirmed/updated.
///
/// ```json
/// {
///   "full_name": "Maria Silva",
///   "email": "maria.silva@example.com",
///   "government_id": "123.456.789-09",
///   "whatsapp_phone_number": "+5511998877665",
///   "has_accepted_terms": true,
///   "code": "123456"
/// }
/// ```
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct ConfirmSignerDataBody {
    /// Confirmed/updated full name.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub full_name: Option<String>,
    /// Confirmed/updated email.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub email: Option<String>,
    /// Confirmed/updated government ID (CPF/CNPJ). This is the field the
    /// documented confirm-data body expects.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub government_id: Option<String>,
    /// Confirmed/updated WhatsApp phone number.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub whatsapp_phone_number: Option<String>,
    /// Whether the signer accepts the terms as part of confirmation.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub has_accepted_terms: Option<bool>,
    /// Verification code, when the API requires it inline.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub code: Option<String>,
}

impl ConfirmSignerDataBody {
    /// Create an empty confirmation body.
    pub fn new() -> Self {
        Self::default()
    }

    /// Set the signer email.
    pub fn email<S: Into<String>>(mut self, email: S) -> Self {
        self.email = Some(email.into());
        self
    }

    /// Set the signer full name.
    pub fn full_name<S: Into<String>>(mut self, full_name: S) -> Self {
        self.full_name = Some(full_name.into());
        self
    }

    /// Set the signer government ID (CPF/CNPJ).
    pub fn government_id<S: Into<String>>(mut self, government_id: S) -> Self {
        self.government_id = Some(government_id.into());
        self
    }

    /// Set the signer WhatsApp phone number.
    pub fn whatsapp<S: Into<String>>(mut self, phone: S) -> Self {
        self.whatsapp_phone_number = Some(phone.into());
        self
    }

    /// Accept or decline terms in this request.
    pub fn accepted_terms(mut self, accepted: bool) -> Self {
        self.has_accepted_terms = Some(accepted);
        self
    }

    /// Set an inline verification code when required by the API flow.
    pub fn verification_code<S: Into<String>>(mut self, code: S) -> Self {
        self.code = Some(code.into());
        self
    }
}

/// Body for signer-facing multiple-document signing.
///
/// # Request payload
///
/// ```json
/// {
///   "document_ids": [
///     "103acccd24234c07858ffddf6d84",
///     "103acccd9f0e1d2c3b4a5968778a"
///   ]
/// }
/// ```
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SignMultipleDocumentsBody {
    /// Document IDs to sign.
    pub document_ids: Vec<String>,
}

impl SignMultipleDocumentsBody {
    /// Build a multiple-document signing request.
    pub fn new<I, S>(document_ids: I) -> Self
    where
        I: IntoIterator<Item = S>,
        S: Into<String>,
    {
        Self {
            document_ids: document_ids.into_iter().map(Into::into).collect(),
        }
    }
}

/// Body for signer-facing multiple-document decline.
///
/// # Request payload
///
/// ```json
/// {
///   "document_ids": [
///     "103acccd24234c07858ffddf6d84",
///     "103acccd9f0e1d2c3b4a5968778a"
///   ],
///   "decline_reason": "Incorrect signer information"
/// }
/// ```
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DeclineMultipleDocumentsBody {
    /// Document IDs to decline.
    pub document_ids: Vec<String>,
    /// Decline reason applied to every document.
    pub decline_reason: String,
}

impl DeclineMultipleDocumentsBody {
    /// Build a multiple-document decline request.
    pub fn new<I, S, R>(document_ids: I, reason: R) -> Self
    where
        I: IntoIterator<Item = S>,
        S: Into<String>,
        R: Into<String>,
    {
        Self {
            document_ids: document_ids.into_iter().map(Into::into).collect(),
            decline_reason: reason.into(),
        }
    }
}

/// Builder for `GET /signers/{signer_id}/documents`.
#[derive(Debug)]
pub struct ListSignerDocumentsRequest<'a> {
    http: &'a HttpClient,
    signer_id: String,
    page: Option<u32>,
    per_page: Option<u32>,
    status: Option<String>,
    method: Option<String>,
    search: Option<String>,
    sort: Option<String>,
}

impl<'a> ListSignerDocumentsRequest<'a> {
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

    /// Filter by document status.
    pub fn status<S: Into<String>>(mut self, status: S) -> Self {
        self.status = Some(status.into());
        self
    }

    /// Filter by assignment method.
    pub fn method<S: Into<String>>(mut self, method: S) -> Self {
        self.method = Some(method.into());
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

    /// Execute the request.
    ///
    /// # Response payload
    ///
    /// Paginated via `X-Pagination-*` headers; the body is an enveloped array
    /// of documents.
    ///
    /// ```json
    /// {
    ///   "status": 200,
    ///   "message": "",
    ///   "data": [
    ///     {
    ///       "id": "103acccd24234c07858ffddf6d84",
    ///       "account_id": "102d25a489f34a275d31a16045fd",
    ///       "template_id": null,
    ///       "name": "contract.pdf",
    ///       "status": "metadata_ready",
    ///       "artifacts": { "original": "https://sandbox.assinafy.com.br/v1/documents/103acccd.../download/original" },
    ///       "is_closed": false,
    ///       "signing_url": "https://app-sandbox.assinafy.com.br/sign/103acccd24234c07858ffddf6d84",
    ///       "decline_reason": null,
    ///       "declined_by": null,
    ///       "tags": [],
    ///       "created_at": "2026-07-19T14:56:54Z",
    ///       "updated_at": "2026-07-19T14:56:56Z"
    ///     }
    ///   ]
    /// }
    /// ```
    pub async fn send(self) -> Result<Page<Document>> {
        let path = format!("signers/{}/documents", self.signer_id);
        let mut req = self.http.request(Method::GET, &path)?;
        let mut q: Vec<(&str, String)> = Vec::new();
        if let Some(v) = self.page {
            q.push(("page", v.to_string()));
        }
        if let Some(v) = self.per_page {
            q.push(("per-page", v.to_string()));
        }
        if let Some(v) = self.status {
            q.push(("status", v));
        }
        if let Some(v) = self.method {
            q.push(("method", v));
        }
        if let Some(v) = self.search {
            q.push(("search", v));
        }
        if let Some(v) = self.sort {
            q.push(("sort", v));
        }
        if !q.is_empty() {
            req = req.query(&q);
        }
        self.http.send_paged(req).await
    }
}

/// Signer-facing endpoints.
#[derive(Debug)]
pub struct SignerSelfApi<'a> {
    http: &'a HttpClient,
}

impl<'a> SignerSelfApi<'a> {
    pub(crate) fn new(http: &'a HttpClient) -> Self {
        Self { http }
    }

    /// Retrieve the authenticated signer.
    ///
    /// `GET /signers/self`.
    ///
    /// # Response payload
    ///
    /// ```json
    /// {
    ///   "status": 200,
    ///   "message": "",
    ///   "data": {
    ///     "resource": "signer",
    ///     "id": "102d25a4a1b2c3d4e5f60718293a4b5c",
    ///     "full_name": "Maria Silva",
    ///     "email": "maria.silva@example.com",
    ///     "whatsapp_phone_number": "+5511998877665",
    ///     "has_accepted_terms": true,
    ///     "has_signature": true,
    ///     "has_initial": false,
    ///     "is_signature_reusable": true
    ///   }
    /// }
    /// ```
    pub async fn me(&self) -> Result<SignerSelf> {
        let req = self.http.request(Method::GET, "signers/self")?;
        self.http.send_envelope(req).await
    }

    /// Accept the platform terms.
    ///
    /// `PUT /signers/accept-terms`.
    ///
    /// # Request payload
    ///
    /// A body is sent only when a signer access code is configured on the
    /// client; otherwise the request has no body.
    ///
    /// ```json
    /// {
    ///   "signer-access-code": "AbCd1234EfGh5678"
    /// }
    /// ```
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
    pub async fn accept_terms(&self) -> Result<()> {
        let mut req = self.http.request(Method::PUT, "signers/accept-terms")?;
        if let Some(code) = self.http.auth().signer_access_code() {
            req = req.json(&serde_json::json!({ "signer-access-code": code }));
        }
        self.http.send_no_content(req).await
    }

    /// Verify the signer's identity using a one-time code.
    ///
    /// `POST /verify`.
    ///
    /// # Request payload
    ///
    /// The signer access code is injected automatically when configured on the
    /// client.
    ///
    /// ```json
    /// {
    ///   "verification-code": "123456",
    ///   "signer-access-code": "AbCd1234EfGh5678"
    /// }
    /// ```
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
    pub async fn verify(&self, body: &VerifyCodeBody) -> Result<()> {
        let mut json = serde_json::to_value(body)?;
        if let (Some(code), Some(object)) =
            (self.http.auth().signer_access_code(), json.as_object_mut())
        {
            object
                .entry("signer-access-code")
                .or_insert_with(|| serde_json::Value::String(code.to_owned()));
        }
        let req = self.http.request(Method::POST, "verify")?.json(&json);
        self.http.send_no_content(req).await
    }

    /// Confirm/update signer profile data within a specific document context.
    ///
    /// `PUT /documents/{document_id}/signers/confirm-data`.
    ///
    /// # Request payload
    ///
    /// ```json
    /// {
    ///   "full_name": "Maria Silva",
    ///   "email": "maria.silva@example.com",
    ///   "government_id": "123.456.789-09",
    ///   "whatsapp_phone_number": "+5511998877665",
    ///   "has_accepted_terms": true
    /// }
    /// ```
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
    pub async fn confirm_data<S: AsRef<str>>(
        &self,
        document_id: S,
        body: &ConfirmSignerDataBody,
    ) -> Result<()> {
        let path = format!("documents/{}/signers/confirm-data", document_id.as_ref());
        let req = self.http.request(Method::PUT, &path)?.json(body);
        self.http.send_no_content(req).await
    }

    /// Retrieve the full signable document for the current signer.
    ///
    /// `GET /sign`.
    ///
    /// # Response payload
    ///
    /// ```json
    /// {
    ///   "status": 200,
    ///   "message": "",
    ///   "data": {
    ///     "id": "103acccd24234c07858ffddf6d84",
    ///     "account_id": "102d25a489f34a275d31a16045fd",
    ///     "template_id": null,
    ///     "name": "contract.pdf",
    ///     "status": "metadata_ready",
    ///     "artifacts": {
    ///       "original": "https://sandbox.assinafy.com.br/v1/documents/103acccd.../download/original",
    ///       "thumbnail": "https://sandbox.assinafy.com.br/v1/documents/103acccd.../thumbnail"
    ///     },
    ///     "is_closed": false,
    ///     "signing_url": "https://app-sandbox.assinafy.com.br/sign/103acccd24234c07858ffddf6d84",
    ///     "decline_reason": null,
    ///     "declined_by": null,
    ///     "tags": [],
    ///     "assignment": null,
    ///     "pages": [
    ///       { "id": "103acccd5c73af8009c3644af591", "number": 1, "height": 1651, "width": 1275 }
    ///     ],
    ///     "created_at": "2026-07-19T14:56:54Z",
    ///     "updated_at": "2026-07-19T14:56:56Z"
    ///   }
    /// }
    /// ```
    pub async fn signable_document(&self) -> Result<Document> {
        let req = self.http.request(Method::GET, "sign")?;
        self.http.send_data(req).await
    }

    /// Sign one assignment with field values.
    ///
    /// `POST /documents/{document_id}/assignments/{assignment_id}`.
    ///
    /// # Request payload
    ///
    /// A JSON array of filled field items.
    ///
    /// ```json
    /// [
    ///   {
    ///     "itemId": "103acccd7a1b2c3d4e5f60718293",
    ///     "fieldId": "102f88b1c2d3e4f5a6b7c8d9e0f1",
    ///     "pageId": "103acccd5c73af8009c3644af591",
    ///     "value": "Maria Silva"
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
    ///   "data": []
    /// }
    /// ```
    pub async fn sign<D: AsRef<str>, A: AsRef<str>, I>(
        &self,
        document_id: D,
        assignment_id: A,
        items: I,
    ) -> Result<()>
    where
        I: IntoIterator<Item = SignDocumentItem>,
    {
        let path = format!(
            "documents/{}/assignments/{}",
            document_id.as_ref(),
            assignment_id.as_ref()
        );
        let items: Vec<SignDocumentItem> = items.into_iter().collect();
        let req = self.http.request(Method::POST, &path)?.json(&items);
        self.http.send_no_content(req).await
    }

    /// Decline one assignment.
    ///
    /// `PUT /documents/{document_id}/assignments/{assignment_id}/reject`.
    ///
    /// # Request payload
    ///
    /// ```json
    /// {
    ///   "decline_reason": "Incorrect signer information"
    /// }
    /// ```
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
    pub async fn decline<D: AsRef<str>, A: AsRef<str>, R: AsRef<str>>(
        &self,
        document_id: D,
        assignment_id: A,
        reason: R,
    ) -> Result<()> {
        let path = format!(
            "documents/{}/assignments/{}/reject",
            document_id.as_ref(),
            assignment_id.as_ref()
        );
        let req = self
            .http
            .request(Method::PUT, &path)?
            .json(&serde_json::json!({ "decline_reason": reason.as_ref() }));
        self.http.send_no_content(req).await
    }

    /// Retrieve the document associated with a signer before verification.
    ///
    /// `GET /signers/{signer_id}/document`.
    ///
    /// # Response payload
    ///
    /// ```json
    /// {
    ///   "status": 200,
    ///   "message": "",
    ///   "data": {
    ///     "id": "103acccd24234c07858ffddf6d84",
    ///     "account_id": "102d25a489f34a275d31a16045fd",
    ///     "template_id": null,
    ///     "name": "contract.pdf",
    ///     "status": "metadata_ready",
    ///     "artifacts": {
    ///       "original": "https://sandbox.assinafy.com.br/v1/documents/103acccd.../download/original",
    ///       "thumbnail": "https://sandbox.assinafy.com.br/v1/documents/103acccd.../thumbnail"
    ///     },
    ///     "is_closed": false,
    ///     "signing_url": "https://app-sandbox.assinafy.com.br/sign/103acccd24234c07858ffddf6d84",
    ///     "decline_reason": null,
    ///     "declined_by": null,
    ///     "tags": [],
    ///     "assignment": null,
    ///     "pages": [
    ///       { "id": "103acccd5c73af8009c3644af591", "number": 1, "height": 1651, "width": 1275 }
    ///     ],
    ///     "created_at": "2026-07-19T14:56:54Z",
    ///     "updated_at": "2026-07-19T14:56:56Z"
    ///   }
    /// }
    /// ```
    pub async fn current_document<S: AsRef<str>>(&self, signer_id: S) -> Result<Document> {
        let path = format!("signers/{}/document", signer_id.as_ref());
        let req = self.http.request(Method::GET, &path)?;
        self.http.send_envelope(req).await
    }

    /// List documents visible to a signer.
    ///
    /// `GET /signers/{signer_id}/documents`.
    pub fn list_documents<S: Into<String>>(&self, signer_id: S) -> ListSignerDocumentsRequest<'_> {
        ListSignerDocumentsRequest {
            http: self.http,
            signer_id: signer_id.into(),
            page: None,
            per_page: None,
            status: None,
            method: None,
            search: None,
            sort: None,
        }
    }

    /// Search a signer's documents (lightweight).
    ///
    /// `GET /signers/{signer_id}/documents/search?search={term}`. Paginated via
    /// `X-Pagination-*` headers.
    ///
    /// # Response payload
    ///
    /// ```json
    /// {
    ///   "status": 200,
    ///   "message": "",
    ///   "data": [
    ///     {
    ///       "id": "103acccd24234c07858ffddf6d84",
    ///       "account_id": "102d25a489f34a275d31a16045fd",
    ///       "template_id": null,
    ///       "name": "contract.pdf",
    ///       "status": "metadata_ready",
    ///       "artifacts": { "original": "https://sandbox.assinafy.com.br/v1/documents/103acccd.../download/original" },
    ///       "is_closed": false,
    ///       "signing_url": "https://app-sandbox.assinafy.com.br/sign/103acccd24234c07858ffddf6d84",
    ///       "decline_reason": null,
    ///       "declined_by": null,
    ///       "tags": [],
    ///       "created_at": "2026-07-19T14:56:54Z",
    ///       "updated_at": "2026-07-19T14:56:56Z"
    ///     }
    ///   ]
    /// }
    /// ```
    pub async fn search_documents<S: AsRef<str>, T: AsRef<str>>(
        &self,
        signer_id: S,
        term: T,
    ) -> Result<Page<Document>> {
        let path = format!("signers/{}/documents/search", signer_id.as_ref());
        let req = self
            .http
            .request(Method::GET, &path)?
            .query(&[("search", term.as_ref())]);
        self.http.send_paged(req).await
    }

    /// Sign multiple virtual documents at once.
    ///
    /// `PUT /signers/documents/sign-multiple`.
    ///
    /// # Request payload
    ///
    /// ```json
    /// {
    ///   "document_ids": [
    ///     "103acccd24234c07858ffddf6d84",
    ///     "103acccd9f0e1d2c3b4a5968778a"
    ///   ]
    /// }
    /// ```
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
    pub async fn sign_multiple(&self, body: &SignMultipleDocumentsBody) -> Result<()> {
        let req = self
            .http
            .request(Method::PUT, "signers/documents/sign-multiple")?
            .json(body);
        self.http.send_no_content(req).await
    }

    /// Decline multiple documents at once.
    ///
    /// `PUT /signers/documents/decline-multiple`.
    ///
    /// # Request payload
    ///
    /// ```json
    /// {
    ///   "document_ids": [
    ///     "103acccd24234c07858ffddf6d84",
    ///     "103acccd9f0e1d2c3b4a5968778a"
    ///   ],
    ///   "decline_reason": "Incorrect signer information"
    /// }
    /// ```
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
    pub async fn decline_multiple(&self, body: &DeclineMultipleDocumentsBody) -> Result<()> {
        let req = self
            .http
            .request(Method::PUT, "signers/documents/decline-multiple")?
            .json(body);
        self.http.send_no_content(req).await
    }

    /// Download a signer-visible document artifact.
    ///
    /// `GET /signers/{signer_id}/documents/{document_id}/download/{artifact_name}`.
    pub async fn download_document<S: AsRef<str>, D: AsRef<str>>(
        &self,
        signer_id: S,
        document_id: D,
        artifact: impl Into<ArtifactName>,
    ) -> Result<(Bytes, String)> {
        let artifact: ArtifactName = artifact.into();
        let path = format!(
            "signers/{}/documents/{}/download/{}",
            signer_id.as_ref(),
            document_id.as_ref(),
            artifact.as_str()
        );
        let req = self.http.request(Method::GET, &path)?;
        self.http.send_download(req).await
    }

    /// Upload a signature or initial image.
    ///
    /// The server stores the bytes as-is; supply a PNG or JPEG.
    /// `POST /signature`.
    ///
    /// The request body is the raw image bytes (not JSON); the image kind is
    /// selected via the `?type=signature|initial` query parameter.
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
    pub async fn upload_signature(
        &self,
        kind: SignerType,
        content_type: &str,
        bytes: impl Into<Bytes>,
    ) -> Result<()> {
        let bytes = bytes.into();
        let req = self
            .http
            .request(Method::POST, "signature")?
            .query(&[("type", kind.as_str())])
            .header(reqwest::header::CONTENT_TYPE, content_type)
            .body(bytes);
        self.http.send_no_content(req).await
    }

    /// Download the previously uploaded signature or initial image.
    ///
    /// `GET /signature/{type}`.
    pub async fn download_signature(&self, kind: SignerType) -> Result<(Bytes, String)> {
        let path = format!("signature/{}", kind.as_str());
        let req = self.http.request(Method::GET, &path)?;
        self.http.send_download(req).await
    }
}
