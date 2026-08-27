//! Endpoints that operate on the currently authenticated signer.
//!
//! Most routes here require an `Auth::AccessCode` credential
//! (`?signer-access-code={code}`). The signer artifact-download route is the
//! exception: the OpenAPI contract declares it unauthenticated.

use bytes::Bytes;
use reqwest::Method;
use serde::{Deserialize, Serialize};

use crate::error::Result;
use crate::http::HttpClient;
use crate::models::{ArtifactName, Document, SignDocumentItem, Signer, SignerSelf, SignerType};
use crate::pagination::Page;

/// Body for `POST /verify`.
///
/// # Request payload
///
/// ```json
/// { "verification-code": "123456" }
/// ```
#[derive(Clone, Serialize, Deserialize)]
pub struct VerifyCodeBody {
    /// One-time code emailed or sent via WhatsApp to the signer.
    #[serde(rename = "verification-code")]
    pub code: String,
    /// Legacy body copy of the signer access code. Current deployments read
    /// this credential from [`crate::Auth::AccessCode`] in the query string.
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

    /// Include the signer access code in the JSON body for a legacy deployment.
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
///   "email": "user@example.invalid",
///   "government_id": "123.456.789-09"
/// }
/// ```
#[derive(Clone, Default, Serialize, Deserialize)]
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
    /// Legacy sandbox extension for the WhatsApp phone number.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub whatsapp_phone_number: Option<String>,
    /// Legacy extension for accepting terms during confirmation. The current
    /// API exposes [`SignerSelfApi::accept_terms`] separately.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub has_accepted_terms: Option<bool>,
    /// Legacy sandbox extension for an inline verification code.
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

    /// Set the signer WhatsApp phone number for a legacy deployment.
    pub fn whatsapp<S: Into<String>>(mut self, phone: S) -> Self {
        self.whatsapp_phone_number = Some(phone.into());
        self
    }

    /// Accept or decline terms in this request for a legacy deployment.
    pub fn accepted_terms(mut self, accepted: bool) -> Self {
        self.has_accepted_terms = Some(accepted);
        self
    }

    /// Set an inline verification code for a legacy deployment.
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

    /// Compatibility filter by document status. Deployments may ignore it.
    pub fn status<S: Into<String>>(mut self, status: S) -> Self {
        self.status = Some(status.into());
        self
    }

    /// Compatibility filter by assignment method. Deployments may ignore it.
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
    ///       "account_id": "acc_1234567890abcdef12345678",
    ///       "template_id": null,
    ///       "name": "contract.pdf",
    ///       "status": "metadata_ready",
    ///       "artifacts": { "original": "https://api.example.invalid/v1/documents/103acccd24234c07858ffddf6d84/download/original" },
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
        let path = self
            .http
            .path(&["signers", self.signer_id.as_str(), "documents"])?;
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
    ///     "email": "user@example.invalid",
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
    /// The request has no body. [`crate::Auth::AccessCode`] supplies the
    /// required `signer-access-code` query parameter.
    ///
    /// # Response payload
    ///
    /// ```json
    /// {
    ///   "status": 200,
    ///   "message": ""
    /// }
    /// ```
    pub async fn accept_terms(&self) -> Result<()> {
        let req = self.http.request(Method::PUT, "signers/accept-terms")?;
        self.http.send_no_content(req).await
    }

    /// Verify the signer's identity using a one-time code.
    ///
    /// `POST /verify`.
    ///
    /// # Request payload
    ///
    /// [`crate::Auth::AccessCode`] supplies the signer access code as the
    /// documented query parameter.
    ///
    /// ```json
    /// { "verification-code": "123456" }
    /// ```
    ///
    /// # Response payload
    ///
    /// ```json
    /// {
    ///   "status": 200,
    ///   "message": ""
    /// }
    /// ```
    pub async fn verify(&self, body: &VerifyCodeBody) -> Result<()> {
        let req = self.http.request(Method::POST, "verify")?.json(body);
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
    ///   "email": "user@example.invalid",
    ///   "government_id": "123.456.789-09"
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
    ///     "resource": "signer",
    ///     "id": "102d25a4a1b2c3d4e5f60718293a4b5c",
    ///     "full_name": "Maria Silva",
    ///     "email": "user@example.invalid",
    ///     "whatsapp_phone_number": null,
    ///     "has_accepted_terms": false
    ///   }
    /// }
    /// ```
    pub async fn confirm_data<S: AsRef<str>>(
        &self,
        document_id: S,
        body: &ConfirmSignerDataBody,
    ) -> Result<Signer> {
        let path =
            self.http
                .path(&["documents", document_id.as_ref(), "signers", "confirm-data"])?;
        let req = self.http.request(Method::PUT, &path)?.json(body);
        self.http.send_envelope(req).await
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
    ///     "account_id": "acc_1234567890abcdef12345678",
    ///     "template_id": null,
    ///     "name": "contract.pdf",
    ///     "status": "metadata_ready",
    ///     "artifacts": {
    ///       "original": "https://api.example.invalid/v1/documents/103acccd24234c07858ffddf6d84/download/original",
    ///       "thumbnail": "https://api.example.invalid/v1/documents/103acccd24234c07858ffddf6d84/thumbnail"
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

    /// Retrieve the signable document and record the terms-acceptance flag.
    ///
    /// `GET /sign?has_accepted_terms={has_accepted_terms}`. The response is
    /// the same full [`Document`] payload shown by [`Self::signable_document`].
    /// For digital-certificate signers, confirm their data and accept the terms
    /// through [`Self::confirm_data`] before this request; the server checks
    /// that prerequisite before processing this query parameter.
    pub async fn signable_document_with_accepted_terms(
        &self,
        has_accepted_terms: bool,
    ) -> Result<Document> {
        let req = self
            .http
            .request(Method::GET, "sign")?
            .query(&[("has_accepted_terms", has_accepted_terms)]);
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
    ///   "data": {}
    /// }
    /// ```
    pub async fn sign<D: AsRef<str>, A: AsRef<str>, I>(
        &self,
        document_id: D,
        assignment_id: A,
        items: I,
    ) -> Result<serde_json::Value>
    where
        I: IntoIterator<Item = SignDocumentItem>,
    {
        crate::resources::AssignmentsApi::new(self.http)
            .sign(document_id, assignment_id, items)
            .await
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
    ///   "message": ""
    /// }
    /// ```
    pub async fn decline<D: AsRef<str>, A: AsRef<str>, R: AsRef<str>>(
        &self,
        document_id: D,
        assignment_id: A,
        reason: R,
    ) -> Result<()> {
        crate::resources::AssignmentsApi::new(self.http)
            .reject(document_id, assignment_id, reason)
            .await
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
    ///     "account_id": "acc_1234567890abcdef12345678",
    ///     "template_id": null,
    ///     "name": "contract.pdf",
    ///     "status": "metadata_ready",
    ///     "artifacts": {
    ///       "original": "https://api.example.invalid/v1/documents/103acccd24234c07858ffddf6d84/download/original",
    ///       "thumbnail": "https://api.example.invalid/v1/documents/103acccd24234c07858ffddf6d84/thumbnail"
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
        let path = self
            .http
            .path(&["signers", signer_id.as_ref(), "document"])?;
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
    ///       "account_id": "acc_1234567890abcdef12345678",
    ///       "template_id": null,
    ///       "name": "contract.pdf",
    ///       "status": "metadata_ready",
    ///       "artifacts": { "original": "https://api.example.invalid/v1/documents/103acccd24234c07858ffddf6d84/download/original" },
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
        let path = self
            .http
            .path(&["signers", signer_id.as_ref(), "documents", "search"])?;
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
    ///   "message": ""
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
    ///
    /// Only `original`, `certificated`, `certificate-page`, `pades`, and `bundle` are
    /// valid here — unlike [`DocumentsApi::download_artifact`](crate::resources::DocumentsApi::download_artifact),
    /// [`ArtifactName::Thumbnail`] is **not** redirected on this signer-facing
    /// route: no equivalent signer-facing thumbnail route exists to redirect
    /// to, so passing it will 404.
    ///
    /// # Response
    ///
    /// The response body is the raw artifact bytes, not a JSON envelope. The
    /// returned tuple carries the bytes and the response `Content-Type`.
    pub async fn download_document<S: AsRef<str>, D: AsRef<str>>(
        &self,
        signer_id: S,
        document_id: D,
        artifact: impl Into<ArtifactName>,
    ) -> Result<(Bytes, String)> {
        let artifact: ArtifactName = artifact.into();
        let path = self.http.path(&[
            "signers",
            signer_id.as_ref(),
            "documents",
            document_id.as_ref(),
            "download",
            artifact.as_str(),
        ])?;
        let req = self.http.request_public(Method::GET, &path)?;
        self.http.send_public_download(req).await
    }

    /// Upload a signature or initial image.
    ///
    /// `POST /signature`. The production contract accepts PNG bytes with
    /// `Content-Type: image/png`. The explicit `content_type` argument is
    /// retained for older deployments that also accept JPEG.
    ///
    /// The request body is the raw image bytes (not JSON); the image kind is
    /// selected via the `?type=signature|initial` query parameter.
    ///
    /// # Response payload
    ///
    /// ```json
    /// {
    ///   "status": 200,
    ///   "message": ""
    /// }
    /// ```
    pub async fn upload_signature(
        &self,
        kind: SignerType,
        content_type: &str,
        bytes: impl Into<Bytes>,
    ) -> Result<()> {
        self.upload_signature_with_reuse(kind, content_type, bytes, None)
            .await
    }

    /// Upload a signature or initial image, optionally setting the signer's
    /// `is_signature_reusable` flag.
    ///
    /// `POST /signature?type=signature|initial&reuse=true|false`.
    /// Production callers should pass `image/png`; other media types are a
    /// legacy compatibility extension.
    ///
    /// `reuse = Some(v)` records whether the signer opted to reuse this
    /// signature in future processes; `None` omits the parameter and leaves
    /// the flag unchanged. This is the only endpoint that can set
    /// [`SignerSelf::is_signature_reusable`](crate::models::SignerSelf).
    ///
    /// # Response payload
    ///
    /// ```json
    /// {
    ///   "status": 200,
    ///   "message": ""
    /// }
    /// ```
    pub async fn upload_signature_with_reuse(
        &self,
        kind: SignerType,
        content_type: &str,
        bytes: impl Into<Bytes>,
        reuse: Option<bool>,
    ) -> Result<()> {
        let bytes = bytes.into();
        let mut req = self
            .http
            .request(Method::POST, "signature")?
            .query(&[("type", kind.as_str())]);
        if let Some(v) = reuse {
            req = req.query(&[("reuse", v)]);
        }
        let req = req
            .header(reqwest::header::CONTENT_TYPE, content_type)
            .body(bytes);
        self.http.send_no_content(req).await
    }

    /// Download the previously uploaded signature or initial image.
    ///
    /// `GET /signature/{type}`.
    ///
    /// # Response
    ///
    /// The response body is the raw image bytes, not a JSON envelope. The
    /// returned tuple carries the bytes and the response `Content-Type`.
    pub async fn download_signature(&self, kind: SignerType) -> Result<(Bytes, String)> {
        let path = self.http.path(&["signature", kind.as_str()])?;
        let req = self.http.request(Method::GET, &path)?;
        self.http.send_download(req).await
    }
}
