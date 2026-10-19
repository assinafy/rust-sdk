# assinafy — Rust SDK

*[Leia em português](README.md) · English*

[![Crate](https://img.shields.io/crates/v/assinafy.svg)](https://crates.io/crates/assinafy)
[![Docs](https://docs.rs/assinafy/badge.svg)](https://docs.rs/assinafy)

Async, idiomatic Rust client for the [Assinafy](https://assinafy.com.br) electronic-signature
API — a Brazilian document-signing platform.

It covers the entire public REST surface documented at <https://api.assinafy.com.br/v1/docs>:
accounts, signers, documents, assignments, templates, tags, fields, webhooks, activities, API
keys, OAuth 2.1, and the signer-facing endpoints.

## Contents

- [Install](#install)
- [Quick start](#quick-start)
- [How the SDK is organised](#how-the-sdk-is-organised)
- [Authentication](#authentication)
  - [API key](#api-key)
  - [User token](#user-token)
  - [OAuth 2.1 with PKCE](#oauth-21-with-pkce)
  - [Signer access code](#signer-access-code)
- [Environments](#environments)
- [End-to-end document flow](#end-to-end-document-flow)
- [Signer verification methods](#signer-verification-methods)
- [Biometric identity verification](#biometric-identity-verification)
- [ICP-Brasil digital certificates (A1/A3)](#icp-brasil-digital-certificates-a1a3)
- [Signer-facing flow](#signer-facing-flow)
- [Templates](#templates)
- [Tags](#tags)
- [Custom fields](#custom-fields)
- [Webhooks](#webhooks)
- [Activities and artifacts](#activities-and-artifacts)
- [Pagination](#pagination)
- [Errors and rate limits](#errors-and-rate-limits)
- [Operation index](#operation-index)
- [Cargo features](#cargo-features)
- [Tests](#tests)
- [License](#license)

## Install

Requires Rust 1.86 or newer and uses the Rust 2024 edition. CI tests both the declared 1.86
minimum and the current stable release; Rust has no LTS channel. The default HTTP client requires
TLS 1.2 or newer.

```toml
[dependencies]
assinafy = "3"
tokio    = { version = "1", features = ["macros", "rt-multi-thread", "time"] }
```

## Quick start

```rust,no_run
use assinafy::Client;

#[tokio::main]
async fn main() -> assinafy::Result<()> {
    let client = Client::builder()
        .api_key(std::env::var("ASSINAFY_API_KEY").unwrap())
        .build()?;

    let signers = client
        .signers("acc_1234567890abcdef12345678")
        .list()
        .per_page(50)
        .send()
        .await?;

    for s in &signers.data {
        println!("{} <{:?}>", s.full_name, s.email);
    }
    Ok(())
}
```

## How the SDK is organised

A single `Client` exposes one handle per API surface. Handles are cheap to create per call and
borrow from the client; `Client` itself is `Clone` (reference-counted internals), so build one
at start-up and reuse it.

| Surface | Accessor |
| ------- | -------- |
| Authentication (login, password, social) | `Client::auth_api` |
| OAuth 2.1 / OpenID Connect | `Client::oauth` |
| Accounts | `Client::accounts_api` / `Client::account` |
| API keys | `Client::api_keys` |
| Authenticated user | `Client::users` |
| Signers | `Client::signers` |
| Signer (self) | `Client::signer_self` |
| Documents | `Client::documents` |
| Assignments | `Client::assignments` |
| Tags | `Client::tags` |
| Fields | `Client::fields` |
| Templates | `Client::templates` |
| Webhooks | `Client::webhooks` |
| Activities | `Client::activities` |
| Public endpoints | `Client::public` |

Every call is `async` and returns `Result<T, Error>`. Responses use the API's own envelope
(`{ status, message, data }`), of which the SDK hands back the typed `data` — except the OAuth
endpoints, which by specification answer with a flat object.

## Authentication

The SDK supports all four credential schemes the API accepts. Swap credentials at runtime with
`Client::with_auth`.

### API key

Sent as the `X-Api-Key` header. This is the default for server-to-server integrations.

```rust
use assinafy::Client;

fn build_client() -> assinafy::Result<Client> {
    Client::builder().api_key("redacted-api-key").build()
}
```

### User token

Obtained through `login` and sent as `Authorization: Bearer <token>`.

```rust,no_run
use assinafy::resources::LoginBody;
use assinafy::{Auth, Client};

async fn sign_in() -> assinafy::Result<Client> {
    let anonymous = Client::builder().build()?;
    let session = anonymous
        .auth_api()
        .login(&LoginBody::new("user@example.invalid", "password"))
        .await?;

    // `LoginResult` also carries the user and the accounts they can see.
    Ok(anonymous.with_auth(Auth::Bearer(session.access_token)))
}
```

A legacy query-string form (`?access-token=...`) is available through
`ClientBuilder::access_token` for integrations that require it.

### OAuth 2.1 with PKCE

Use OAuth when an application acts **in a user's workspace with that user's permission** — as
opposed to an API key or user token, which authenticate the workspace or the user directly. The
resulting token carries only the scopes the user approved, works for one workspace, and can
never reach billing, account lifecycle, credential management or admin surfaces, whatever its
scopes.

Register the app at [Assinafy](https://app.assinafy.com.br), under **Settings → OAuth applications →
New application**. The owning workspace requires an eligible plan and its owner must register the
app. Choose only the permissions the app needs. Register an HTTPS callback URI for each environment;
matching is exact, including a trailing slash. Local development needs an HTTPS tunnel. Choose
**Confidential** for a server that can protect a secret, or **Public** for a client that cannot.
Confidential clients add `.client_secret(secret)` to every token and revocation request.

Store each connection by your application user and Assinafy workspace, with the granted scopes,
expiry and encrypted tokens. Keep PKCE and random state in the initiating browser session, expire
them after the attempt, and reject duplicate callback parameters. Codes expire after 60 seconds and
can be exchanged once. The SDK returns the `id_token` without verifying it: an OpenID Connect
library must validate its signature using the discovery `jwks_uri`, issuer, audience, expiry and any
nonce supplied through `.nonce(value)`.

The flow is the authorization-code grant with **mandatory** PKCE (S256). The SDK covers every
step except the browser redirect:

```text
1. discovery       GET /.well-known/oauth-protected-resource
                   GET {issuer}/.well-known/oauth-authorization-server
2. authorization   browser → {authorization_endpoint}?code_challenge=...
3. consent         user approves → redirect_uri?code=...&state=...&iss=...
4. exchange        POST /v1/oauth/token   (code + code_verifier)
5. use             Authorization: Bearer <access_token>
                   POST /v1/oauth/token   (refresh_token)
                   POST /v1/oauth/revoke
```

Create a new PKCE pair and a new `state` for every attempt. When the browser returns to your
`redirect_uri`, before anything else — `error=...` returns included — check that `state` equals
the stored value and that `iss` equals `https://auth.assinafy.com.br`; if either differs, stop:
the response is not yours.

```rust,no_run
use assinafy::resources::{AuthorizationRequest, PkceChallenge, TokenRequest, scope};
use assinafy::{Auth, Client};

async fn authorize(client_id: &str, redirect_uri: &str) -> assinafy::Result<()> {
    let client = Client::builder().build()?;

    // 1. Discovery: the API names its authorization server, which in turn
    //    publishes the browser-facing authorization endpoint.
    let resource = client.oauth().protected_resource_metadata().await?;
    let server = client
        .oauth()
        .authorization_server_metadata(&resource.authorization_servers[0])
        .await?;

    // 2. A new PKCE pair and `state` for every attempt; keep both until the callback.
    let pkce = PkceChallenge::generate()?;
    let state = PkceChallenge::generate()?.verifier().to_owned();
    let url = AuthorizationRequest::new(client_id, redirect_uri, &pkce)
        .scopes([scope::DOCUMENTS_READ, scope::ACCOUNT_READ, scope::OPENID, scope::OFFLINE_ACCESS])
        .state(&state)
        .resource(&resource.resource)
        .url(&server.authorization_endpoint)?;
    println!("open {url}");

    // 4. Once `state` and `iss` check out, exchange the `code` the redirect came back
    //    with, repeating the `resource` sent in step 2.
    let token = client
        .oauth()
        .token(
            &TokenRequest::authorization_code(
                client_id,
                "code-from-the-redirect",
                redirect_uri,
                &pkce,
            )
            .resource(&resource.resource),
        )
        .await?;

    // 5. Start acting for the user. The token reaches one workspace, the only one
    //    `GET /v1/accounts` returns: store its id with the tokens.
    let as_user = client.with_auth(Auth::Bearer(token.access_token.clone()));
    let workspace = &as_user.accounts_api().list().await?[0].id;
    let who = as_user.oauth().userinfo().await?;
    println!("acting for {} in workspace {workspace}", who.sub);
    Ok(())
}
```

For internal Assinafy services, `TokenRequest::token_exchange` supports the RFC 8693 grant,
with optional `.requested_token_type("urn:ietf:params:oauth:token-type:access_token")`.
Its response exposes `issued_token_type` and has no refresh token. Ordinary applications
receive `invalid_client` for this grant and use the authorization-code flow above.

Available scopes (constants in `assinafy::resources::scope`):

| Scope | Grants |
| --- | --- |
| `documents:read` | Read documents, pages, tags, signers, assignments and activity |
| `documents:write` | Create, update and delete documents and manage their signers and assignments |
| `templates:read` | Read templates, pages, roles, fields and tags |
| `templates:write` | Create, update and delete templates and their components |
| `account:read` | Read the workspace's profile, theme and logo |
| `webhooks:write` | Create, update and delete the workspace webhook endpoints |
| `openid` | Identify the authenticated user and enable `/oauth/userinfo` |
| `profile` | Include the user's name in the `id_token`/userinfo claims |
| `email` | Include the email and its verification status in the `id_token`/userinfo claims |
| `offline_access` | Issue a refresh token — only for clients that ask explicitly |

Always request the narrowest set. A missing scope answers `403` with
`WWW-Authenticate: Bearer error="insufficient_scope"` naming what is missing; the SDK hands those
scopes back from `Error::insufficient_scope`. Reconnect requesting them — retrying the call cannot
succeed. A `403` without that challenge means another workspace, the user's own role, or an area
OAuth tokens never reach.

Refresh: an access token lasts 1 hour and, with `offline_access`, is renewed without the user.
Every refresh returns a **new** refresh token and retires the old one, and reusing a retired
refresh token ends the whole connection. So save the new refresh token before anything else, and
refresh one at a time per connection. The SDK sends each token request exactly once — it never
retries one or follows a redirect — and rejects a refresh that succeeds without a new refresh
token.

When a refresh fails without a definitive answer — a timeout, a dropped connection, a `5xx`, a
success without a new refresh token — assume the token you sent is retired. Re-read the stored
token: if another worker saved a different one, carry on with that one; if it is unchanged,
**never send it again** — mark the connection disconnected and ask the user to connect again.
Only failures that provably happened before the request was sent — DNS resolution, a refused
connection, the TLS handshake: an `Error::Http` whose `is_connect()` is true — leave the token
unused and are safe to retry.

A refresh token is valid for 30 days, and every refresh returns a new one with a fresh 30 days: a
connection only expires after 30 days without a refresh.

Token-endpoint failures follow RFC 6749 §5.2's flat `{ error, error_description }` object. The
SDK preserves both: the description becomes the error's message and the code is available from
`ApiError::oauth_error`.

```rust,no_run
use assinafy::Client;
use assinafy::models::TokenResponse;
use assinafy::resources::TokenRequest;

/// Refreshes the access token. `save` durably stores the response's new refresh token;
/// if it fails, its error is returned and the access token goes unused.
async fn refresh(
    client: &Client,
    client_id: &str,
    refresh_token: &str,
    save: impl FnOnce(&TokenResponse) -> std::io::Result<()>,
) -> assinafy::Result<Option<String>> {
    match client
        .oauth()
        .token(&TokenRequest::refresh_token(client_id, refresh_token))
        .await
    {
        Ok(fresh) => {
            // The refresh token just sent is now retired, and a successful refresh always
            // carries a new one: save it before using the access token.
            save(&fresh)?;
            Ok(Some(fresh.access_token))
        }
        // The connection is over (refresh token already used, 30 days without a refresh,
        // revoked, or the user approved again with different permissions): mark it
        // disconnected and ask the user to connect again, without retrying the refresh.
        Err(e) if e.oauth_error() == Some("invalid_grant") => Ok(None),
        // Anything else may have retired the token just sent: unless `is_connect()` shows
        // the request never left, re-read the stored token and never resend it unchanged.
        Err(other) => Err(other),
    }
}
```

On an API `401`, refresh once; if the refresh fails, ask the user to connect again.

To disconnect, revoke the current refresh token, then delete the stored tokens:

```rust,no_run
use assinafy::Client;
use assinafy::resources::RevokeRequest;

async fn disconnect(
    client: &Client,
    client_id: &str,
    refresh_token: &str,
) -> assinafy::Result<()> {
    // Revocation answers 200 for every token outcome — including a token that
    // does not exist — so it can never be used to probe whether one exists.
    client
        .oauth()
        .revoke(&RevokeRequest::refresh_token(client_id, refresh_token))
        .await
}
```

> The OAuth endpoints exist in **production only**. Other deployments answer
> `404 Página não encontrada.` and do not publish `/.well-known/oauth-protected-resource`.

### Signer access code

Signer-facing endpoints use the access code delivered out of band (email or WhatsApp), sent as
`?signer-access-code=...`.

```rust
use assinafy::{Auth, Client};

fn as_signer(client: &Client) -> Client {
    client.with_auth(Auth::AccessCode("signer-access-code".into()))
}
```

## Environments

| Environment | Base URL | Builder |
| --- | --- | --- |
| Production | `https://api.assinafy.com.br/v1` | default, or `.production()` |
| Other deployment | any HTTPS URL | `.base_url(BaseUrl::custom(...)?)` |


`BaseUrl::custom` points at any other deployment. Custom URLs must use HTTPS (HTTP only for
loopback), with no embedded credentials, query string or fragment.

```rust
use assinafy::{BaseUrl, Client};

fn local_client() -> assinafy::Result<Client> {
    Client::builder()
        .base_url(BaseUrl::custom("http://127.0.0.1:8080/v1")?)
        .api_key("test-key")
        .build()
}
```

## End-to-end document flow

The whole path of a signature, from upload to certificated PDF:

```text
 1. create a signer       POST /accounts/{account}/signers
 2. upload the PDF        POST /accounts/{account}/documents      → status: uploaded
 3. wait for metadata     GET  /documents/{id}                    → status: metadata_ready
 4. estimate the cost     POST /documents/{id}/assignments/estimate-cost
 5. request signatures    POST /documents/{id}/assignments        → status: pending_signature
                          (the API notifies each signer and returns signing_urls)
 6. the signer signs      accept terms → confirm data → OTP, biometric capture or A1/A3
                          (biometric InReview: the account decides before certification)
 7. track                 GET  /documents/{id}                    → status: certificated
                          or the document_ready webhook
 8. download the result   GET  /documents/{id}/download/certificated
```

```rust,no_run
use assinafy::Client;
use assinafy::models::{ArtifactName, AssignmentMethod, DocumentStatus};
use assinafy::resources::{CreateAssignmentBody, CreateSignerBody, UploadDocumentRequest};

async fn sign_a_contract() -> assinafy::Result<()> {
    let account = std::env::var("ASSINAFY_ACCOUNT_ID").unwrap();
    let client = Client::builder()
        .api_key(std::env::var("ASSINAFY_API_KEY").unwrap())
        .build()?;

    // 1. A signer needs an email address or a WhatsApp number to be notified.
    let signer = client
        .signers(&account)
        .create(&CreateSignerBody::new("Maria Silva").email("maria@example.invalid"))
        .await?;

    // 2. Upload is multipart; the document starts as `uploaded` and metadata
    //    extraction happens asynchronously.
    let file = UploadDocumentRequest::from_path("./contract.pdf").await?;
    let document = client.documents().upload(&account, file).await?;

    // 3. Signatures can only be requested from `metadata_ready` onward.
    let mut ready = false;
    for _ in 0..30 {
        let current = client.documents().get(&document.id).await?;
        if matches!(current.status, DocumentStatus::MetadataReady) {
            ready = true;
            break;
        }
        if !matches!(current.status, DocumentStatus::Uploading | DocumentStatus::Uploaded | DocumentStatus::MetadataProcessing) {
            return Err(assinafy::Error::UnexpectedResponse(format!(
                "document processing stopped: {}", current.status
            )));
        }
        tokio::time::sleep(std::time::Duration::from_secs(2)).await;
    }
    if !ready {
        return Err(assinafy::Error::UnexpectedResponse("document metadata timed out".into()));
    }

    // 4–5. Request the signatures. `Virtual` notifies remotely; `Collect`
    //      collects in person on the sender's device.
    let request = CreateAssignmentBody::new(
        AssignmentMethod::Virtual,
        [signer.id.as_str()],
    )
    .message("Please review and sign this contract.");

    let cost = client
        .assignments()
        .estimate_cost(&document.id, &request)
        .await?;
    println!("estimated cost: {} credits", cost.total_credits);

    if !cost.has_sufficient_resources {
        return Err(assinafy::Error::Validation("insufficient resources for signing".into()));
    }

    let assignment = client.assignments().create(&document.id, &request).await?;

    // Signing links grant access to the document: deliver them privately and
    // never log them.
    for link in &assignment.signing_urls {
        println!("link issued for signer {}", link.signer_id);
    }

    // 7–8. Once everyone has signed, the document becomes `certificated`.
    let current = client.documents().get(&document.id).await?;
    if matches!(current.status, DocumentStatus::Certificated) {
        let (pdf, content_type) = client
            .documents()
            .download_artifact(&document.id, ArtifactName::Certificated)
            .await?;
        assert_eq!(content_type, "application/pdf");
        tokio::fs::write("./contract-certificated.pdf", pdf).await?;
    }

    Ok(())
}
```

The sender receives signing links; the signer completes the [signer flow](#signer-facing-flow)
separately. Handle completion through `document_ready` webhooks or bounded polling, then download
the final artifact. Keep document and signer IDs so a timeout resumes the existing operation instead
of uploading or sending the same document again.

Document states, in lifecycle order:

| Status | Meaning |
| --- | --- |
| `uploading` / `uploaded` | Upload in progress / complete |
| `metadata_processing` / `metadata_ready` | Extracting metadata / ready for an assignment |
| `pending_signature` | Waiting on one or more signers |
| `pending_identity_review` | All signers finished; waiting for account identity review (`status.as_str()`) |
| `certificating` / `certificated` | Generating the certificate / signed and certificated |
| `rejected_by_signer` / `rejected_by_user` | Declined by a signer / cancelled by the sender |
| `expired` | Signing deadline passed |
| `failed` | Processing failed |

Future values the SDK does not model yet arrive as `DocumentStatus::Unknown(String)` rather
than failing deserialization.

## Signer verification methods

Set per signer when creating the assignment. Verification and notification are **coupled**:
send one, both, or neither — the missing side is inferred. With neither, both default to
`Email`.

| Method | How it works | Cost per signer |
| --- | --- | --- |
| `Email` *(default)* | One-time code (OTP) by email, required before signing | Free |
| `Whatsapp` | One-time code (OTP) over WhatsApp | Verification free; notification 0.45 credits, paid plans only |
| `DigitalCertificate` | The signer signs with their **own ICP-Brasil certificate (A1/A3)** through the Web PKI browser extension, producing a **qualified PAdES signature** | 0.5 credits, on top of the notification cost |
| `IdDocumentFaceMatchBasic` | Hosted identity-document and face capture, with explicit biometric consent and account review when needed | Use `estimate_cost` for the account’s current price |

Allowed pairings: `Email` → notified by `Email`; `Whatsapp` → notified by `Whatsapp`;
`DigitalCertificate` and `IdDocumentFaceMatchBasic` → notified by `Email` **or** `Whatsapp`. Only one notification method per
signer.

`DigitalCertificate` requires `AssignmentMethod::Virtual`. The certificate flow completes the
signer's virtual item and cannot complete placed fields in a `Collect` assignment. Therefore,
`assignments().estimate_cost` and `assignments().create` reject that combination before sending a
request, with `Error::Validation` and the stable
`DIGITAL_CERTIFICATE_REQUIRES_VIRTUAL_CODE`. The same restriction applies to templates: do not
assign `DigitalCertificate` to a role with placed signer fields; the generated assignment method is
determined by the template layout.

```rust,no_run
use assinafy::Client;
use assinafy::models::{AssignmentMethod, NotificationMethod, VerificationMethod};
use assinafy::resources::{CreateAssignmentBody, CreateAssignmentSigner};

async fn request_with_methods(document: &str) -> assinafy::Result<()> {
    let client = Client::from_api_key("redacted-api-key")?;

    let request = CreateAssignmentBody::from_signers(
        AssignmentMethod::Virtual,
        [
            // Email OTP (the default, stated explicitly); signs first.
            CreateAssignmentSigner::new("sig_email")
                .step(1)
                .verification_method(VerificationMethod::Email),
            // WhatsApp OTP: verification and notification travel together.
            CreateAssignmentSigner::new("sig_whatsapp")
                .step(2)
                .verification_method(VerificationMethod::Whatsapp)
                .notification_methods(vec![NotificationMethod::Whatsapp]),
            // ICP-Brasil certificate, notified by email.
            CreateAssignmentSigner::new("sig_certificate")
                .step(3)
                .verification_method(VerificationMethod::DigitalCertificate)
                .notification_methods(vec![NotificationMethod::Email]),
            // Biometric capture, notified by email.
            CreateAssignmentSigner::new("sig_biometric")
                .step(4)
                .verification_method(VerificationMethod::id_document_face_match_basic())
                .notification_methods(vec![NotificationMethod::Email]),
        ],
    )
    .expires_at("2026-12-31T23:59:59Z");

    client.assignments().create(document, &request).await?;
    Ok(())
}
```

`step` makes signing sequential: the step-2 signer is only notified once step 1 completes.
Without `step`, everyone signs in parallel.

Estimate before sending — the estimate reports the balance, the total cost, and the blocking
reason when credits fall short:

```rust,no_run
use assinafy::Client;
use assinafy::models::AssignmentMethod;
use assinafy::resources::EstimateAssignmentCostBody;

async fn estimate(document: &str) -> assinafy::Result<()> {
    let client = Client::from_api_key("redacted-api-key")?;
    let estimate = client
        .assignments()
        .estimate_cost(
            document,
            &EstimateAssignmentCostBody::new(AssignmentMethod::Virtual, ["sig_1", "sig_2"]),
        )
        .await?;

    println!("total: {} credits", estimate.total_credits);
    println!("balance: {}", estimate.credit_balance);
    for item in &estimate.breakdown {
        println!("  {} × {} = {}", item.quantity, item.name, item.cost);
    }
    if let Some(reason) = &estimate.blocking_reason {
        let detail = estimate.message.as_deref().unwrap_or("");
        println!("blocked: {reason:?} — {detail}");
    }
    Ok(())
}
```

## Biometric identity verification

Use `VerificationMethod::id_document_face_match_basic()` on assignment or template signers.
It sends `IdDocumentFaceMatchBasic` and is represented by `VerificationMethod::Other` when read.
`SignerSelf::verification_method` identifies this flow; the signer uses hosted document and face
capture instead of the OTP endpoint. Start only after explicit consent:

```rust,no_run
use assinafy::Client;
use assinafy::models::IdentityVerificationSession;

async fn start_capture(
    signer: &Client,
    has_consented: bool,
) -> assinafy::Result<IdentityVerificationSession> {
    signer.signer_self().start_identity_verification(has_consented).await
}
```

The client above must use `Auth::AccessCode`. Open `session.url` unchanged, including any `#t=`
fragment. It is a bearer link: never log it or persist it in browser storage. Poll
`signer_self().identity_verification()` with a deadline; start only when `can_start` is true
(no session or `Expired`). `Pending` keeps its existing session. `Declined` and `Abandoned`
require a new document and assignment. A resend never replaces a live session; an expired
capture session can be restarted without another verification charge.

With `InReview`, the signer can confirm their data and sign while the account reviews.
`AssignmentSigner::identity_verification_status` exposes this state. The document waits at
`pending_identity_review` after everyone signs; read it with `DocumentStatus::as_str()`.
The account calls `assignments().identity_verification(document, assignment, signer)` to read
the reasons, checks and evidence summary. Its `images` names can be passed to
`download_identity_image`; evidence images become unavailable after a decision.

After a human has reviewed the evidence, record their explicit decision with the workspace client:

```rust,no_run
use assinafy::Client;
use assinafy::resources::{IdentityReviewBody, IdentityReviewDecision};

async fn record_review(
    workspace: &Client,
    document: &str,
    assignment: &str,
    signer: &str,
    decision: IdentityReviewDecision,
) -> assinafy::Result<()> {
    workspace.assignments().review_identity_verification(
        document, assignment, signer, &IdentityReviewBody::new(decision),
    ).await?;
    Ok(())
}
```

`Approved` releases certification once every signature and review finishes. `Rejected` cancels
the document and notifies every signer and copy receiver. An optional `.note(...)` stays private.

## ICP-Brasil digital certificates (A1/A3)

Requires the **Digital Certificate** feature on the account (Standard and Pro plans), a CPF or
CNPJ in the signer's `government_id`, and exactly **one certificate signer per step**. A CPF
requires that person's certificate (an e-CPF, or an e-CNPJ naming them as legal representative);
a CNPJ requires the company's e-CNPJ.

Before the assignment opens, the signer must confirm their identity data and accept the terms.
The ordinary signing endpoint **rejects** certificate signers — their signature is produced by a
two-step handshake with the Web PKI extension:

```text
POST /v1/signers/certificate/start     → data.token   (Web PKI operation token)
        ↓  the browser signs the token with the signer's certificate
POST /v1/signers/certificate/complete  → data.signerName
```

> These two routes are extensions deployed in **production only**: other deployments do not expose
> them and they are absent from the published OpenAPI document, so the SDK does not wrap them.

Once the flow completes, downloading the `pades` artifact returns the qualified PAdES signature.

## Signer-facing flow

With `Auth::AccessCode`, the client speaks through the signer endpoints. The code reaches the
signer by email or WhatsApp and cannot be obtained through the API. These methods refuse an empty
code and any other credential, and send only the code; to read a signer's documents with the API
key, use `list_documents_as_workspace` and `search_documents_as_workspace`.

```rust,no_run
use assinafy::models::{SignDocumentItem, SignerType};
use assinafy::resources::{ConfirmSignerDataBody, VerifyCodeBody};
use assinafy::{Auth, Client};

async fn sign_as_signer(document: &str, assignment: &str) -> assinafy::Result<()> {
    let client = Client::builder()
        .auth(Auth::AccessCode("signer-access-code".into()))
        .build()?;
    let signer = client.signer_self();

    // 1. Who am I, and what am I signing.
    let me = signer.me().await?;
    let doc = signer.signable_document().await?;
    println!("{} is signing {}", me.full_name, doc.name);

    // 2. Accept the terms and confirm the identity data.
    signer.accept_terms().await?;
    signer
        .confirm_data(
            document,
            &ConfirmSignerDataBody::new()
                .full_name("Maria Silva")
                .government_id("123.456.789-09"),
        )
        .await?;

    // 3. Validate the one-time code received by email or WhatsApp.
    signer.verify(&VerifyCodeBody::new("123456")).await?;

    // 4. Upload the signature image and fill the assignment's fields.
    let png = tokio::fs::read("./signature.png").await?;
    signer
        .upload_signature(SignerType::Signature, "image/png", png)
        .await?;

    let items: Vec<SignDocumentItem> = doc
        .assignment
        .iter()
        .flat_map(|a| a.items.iter())
        .filter(|item| !item.completed && item.signer.as_ref().is_some_and(|s| s.id == me.id))
        .filter_map(|item| {
            let field = item.field.as_ref()?;
            let page = item.page.as_ref()?;
            Some(SignDocumentItem::new(
                &item.id,
                &field.id,
                &page.id,
                "Maria Silva",
            ))
        })
        .collect();
    signer.sign(document, assignment, items).await?;

    Ok(())
}
```

Declining is equally explicit, and the reason is recorded on the document:

```rust,no_run
use assinafy::{Auth, Client};

async fn decline(document: &str, assignment: &str) -> assinafy::Result<()> {
    let client = Client::from_api_key("redacted-api-key")?
        .with_auth(Auth::AccessCode("signer-access-code".into()));
    client
        .signer_self()
        .decline(document, assignment, "Incorrect details in the contract")
        .await?;
    Ok(())
}
```

A signer with several pending documents can sign or decline in bulk with `sign_multiple` and
`decline_multiple`.

## Templates

A template is a reusable document with roles and fields already positioned. Creating a document
from one skips both the upload and the placement.

```rust,no_run
use assinafy::Client;
use assinafy::models::VerificationMethod;
use assinafy::resources::{CreateDocumentFromTemplateBody, TemplateDocumentSigner};

async fn create_from_template(account: &str, template: &str) -> assinafy::Result<()> {
    let client = Client::from_api_key("redacted-api-key")?;

    // Each template role is bound to a concrete signer.
    let body = CreateDocumentFromTemplateBody::default()
        .name("Contract — Maria Silva")
        .message("Please sign.")
        .signers(vec![
            // A signer already stored in the account…
            TemplateDocumentSigner::existing("role_client", "sig_1")
                .verification_method(VerificationMethod::Email),
            // …or one created inline from a name and a contact.
            TemplateDocumentSigner::inline("role_supplier", "João Souza")
                .whatsapp("+5511999999999")
                .verification_method(VerificationMethod::Whatsapp),
        ]);

    let estimate = client.templates(account).estimate_cost(template, &body).await?;
    println!("cost: {} credits", estimate.total_credits);

    let document = client.templates(account).create_document(template, &body).await?;
    println!("document {} created and already out for signature", document.id);
    Ok(())
}
```

Listing, fetching, creating (multipart), updating and deleting templates are available too, as
is downloading each rendered page.

## Tags

```rust,no_run
use assinafy::Client;
use assinafy::resources::CreateTagBody;

async fn tag_a_document(account: &str, document: &str) -> assinafy::Result<()> {
    let client = Client::from_api_key("redacted-api-key")?;
    let tags = client.tags(account);

    let contracts = tags.create(&CreateTagBody::new("Contracts").color("3399ff")).await?;
    println!("tag {} created", contracts.id);

    // Attach and replace take **names**; remove takes the **id**.
    tags.add_to_document(document, ["Contracts", "Urgent"]).await?;
    tags.set_on_document(document, ["Signed"]).await?;
    tags.remove_from_document(document, &contracts.id).await?;
    Ok(())
}
```

`add_to_document` and `set_on_document` upsert **by name**: the API matches each entry
case-insensitively against the account's existing tags and creates whichever is missing. Passing
a tag id here creates a new tag named after that id. `remove_from_document` is the one
document-tag operation that takes a real id, as returned by `create` or `list_for_document`.

## Custom fields

Fields define the data a signer fills in. Each account has the platform's standard fields plus
the ones you create; regular-expression validation runs server-side.

```rust,no_run
use assinafy::Client;
use assinafy::resources::{CreateFieldBody, ValidateFieldEntry};

async fn fields(account: &str) -> assinafy::Result<()> {
    let client = Client::from_api_key("redacted-api-key")?;
    let fields = client.fields(account);

    for kind in fields.list_types().await? {
        println!("available type: {} ({})", kind.name, kind.kind);
    }

    // The regex must be delimited by slashes — a bare `^.{2,}$` is rejected
    // with `400 Padrão RegEx inválido.`
    let field = fields
        .create(
            &CreateFieldBody::new("text", "Employee number")
                .regex("/^[0-9]{6}$/")
                .required(true),
        )
        .await?;

    let result = fields.validate(&field.id, "123456").await?;
    println!("valid: {}", result.success);

    // Or validate several at once.
    let batch = fields
        .validate_multiple([ValidateFieldEntry::new(&field.id, "123456")])
        .await?;
    println!("{} results", batch.len());
    Ok(())
}
```

## Webhooks

Each account has one webhook endpoint, or up to three on paid plans. Every active endpoint
subscribed to an event receives it; the API keeps a delivery history that can be retried. With
`signing_enabled`, each delivery carries a [Standard Webhooks](https://www.standardwebhooks.com)
signature in the `webhook-id`, `webhook-timestamp` and `webhook-signature` headers.

```rust,no_run
use assinafy::Client;
use assinafy::resources::WebhookEndpointBody;

async fn webhooks(account: &str) -> assinafy::Result<()> {
    let client = Client::from_api_key("redacted-api-key")?;
    let hooks = client.webhooks(account);

    for kind in hooks.event_types().await? {
        println!("{}: {}", kind.id, kind.description);
    }

    let endpoint = hooks
        .create_endpoint(
            &WebhookEndpointBody::new("https://app.example.invalid/hooks", "ops@example.invalid")
                .name("ERP")
                .events(["document_ready", "signer_rejected_document"])
                .signing_enabled(true),
        )
        .await?;
    // Store the `whsec_…` secret in the receiver's secret storage.
    let secret = endpoint.secret.as_deref().ok_or_else(||
        assinafy::Error::UnexpectedResponse("missing webhook signing secret".into())
    )?;
    let _ = secret;

    // Updates send only the fields that are set.
    hooks
        .update_endpoint(&endpoint.id, &WebhookEndpointBody::default().active(false))
        .await?;

    // The endpoint's delivery history, retrying the ones that failed.
    let dispatches = hooks
        .list_dispatches()
        .endpoint_id(&endpoint.id)
        .delivered(false)
        .per_page(50)
        .send()
        .await?;
    for dispatch in &dispatches.data {
        println!("{} → HTTP {:?}", dispatch.event, dispatch.http_status);
        hooks.retry_dispatch(&dispatch.id).await?;
    }

    hooks.delete_endpoint(&endpoint.id).await?;
    Ok(())
}
```

`rotate_endpoint_secret` replaces the secret and invalidates the old one immediately. The secret
routes are not available to OAuth applications; store `WebhookEndpoint::secret` from creation.
If an OAuth app loses it, delete and recreate that endpoint. Use `list_endpoints_by_url` to find
an exact registered URL and `delete_endpoint_by_url` to remove it. `register`, `get_subscription`
and `inactivate` keep operating on the account's oldest endpoint.

In the receiver, verify the **raw** body, exactly as received, before parsing it:

```rust
use assinafy::resources::verify_webhook_signature;

fn receive(
    secret: &str,
    id: &str,
    timestamp: &str,
    signature: &str,
    body: &[u8],
) -> assinafy::Result<serde_json::Value> {
    verify_webhook_signature(secret, id, timestamp, signature, body)?;
    Ok(serde_json::from_slice(body)?)
}
```

Verification compares in constant time and rejects timestamps more than
`WEBHOOK_TIMESTAMP_TOLERANCE` (five minutes) from the local clock. Use the `webhook-id` header to
drop repeated deliveries.

## Activities and artifacts

`Client::activities` returns every recorded event for a document, each with a snapshot of the
event `payload` and the request `origin` (`ip`, `user-agent`).

```rust,no_run
use assinafy::Client;

async fn audit_trail(document: &str) -> assinafy::Result<()> {
    let client = Client::from_api_key("redacted-api-key")?;
    let events = client.activities().list(document).per_page(100).send().await?;
    for event in &events.data {
        let ip = event.origin.as_ref().and_then(|o| o.ip.as_deref()).unwrap_or("-");
        let message = event.message.as_deref().unwrap_or("");
        println!("{} — {message} (from {ip})", event.event);
    }
    Ok(())
}
```

Downloadable artifacts:

| Artifact | Contents |
| --- | --- |
| `original` | The uploaded PDF, as received |
| `certificated` | The signed document with the platform's certification |
| `certificate-page` | The certification page alone |
| `pades` | The signers' ICP-Brasil signatures + certification box — present only on documents that had digital-certificate signers |
| `bundle` | A zip of `original`, `certificated` and `certificate-page`, plus `pades` when present |

`thumbnail` is not a valid value on the `download/{artifact}` route; the SDK transparently
redirects `ArtifactName::Thumbnail` to `GET /documents/{id}/thumbnail`.

Public verification checks a signed document by its signature hash, unauthenticated:

```rust,no_run
use assinafy::Client;

async fn verify(hash: &str) -> assinafy::Result<()> {
    let client = Client::builder().build()?;
    let result = client.documents().verify(hash).await?;
    println!("valid: {} — {}", result.is_valid, result.message);
    Ok(())
}
```

## Pagination

Every paged endpoint returns `Page<T>` with `data` and `meta`, the latter read from the
`X-Pagination-*` headers. `meta` also carries the rate-limit state (`X-Rate-Limit-*`), useful
for self-throttling before hitting a `429`.

```rust,no_run
use assinafy::Client;

async fn walk_pages(account: &str) -> assinafy::Result<()> {
    let client = Client::from_api_key("redacted-api-key")?;
    let mut page = Some(1);
    while let Some(n) = page {
        let result = client.signers(account).list().page(n).per_page(100).send().await?;
        println!("page {n}: {} items", result.data.len());
        if let Some(remaining) = result.meta.rate_limit_remaining {
            println!("  {remaining} requests left in this window");
        }
        page = result.next_page();
    }
    Ok(())
}
```

## Errors and rate limits

Every operation returns `Result<T, Error>`. API errors preserve the HTTP status, the server's
message and the raw `data` payload, so you can branch on a specific code without losing detail.

```rust,no_run
use assinafy::{Client, Error};

async fn handle(account: &str) -> assinafy::Result<()> {
    let client = Client::from_api_key("redacted-api-key")?;
    match client.signers(account).get("missing").await {
        Ok(signer) => println!("{}", signer.full_name),
        Err(e) if e.is_rate_limited() => {
            println!("wait {:?}s before retrying", e.retry_after());
        }
        Err(Error::Api(e)) if e.status == 404 => println!("not found: {}", e.message),
        Err(Error::Api(e)) => println!("error {}: {} — {:?}", e.status, e.message, e.data),
        Err(other) => return Err(other),
    }
    Ok(())
}
```

`e.rate_limit()` returns the `X-Rate-Limit-Limit`/`-Remaining`/`-Reset` window reported with
the error; `retry_after()` reads the `Retry-After` sent with a `429`.

`Error` variants: `Config` (invalid configuration), `Http` (transport), `Serde`, `Io`, `Url`,
`Api` (non-2xx response), `UnexpectedResponse` (undecodable payload) and `Validation` (request
refused locally, before sending). Credentials never
appear in `Display`/`Debug` — the URL is stripped from transport errors, and tokens render as
`**redacted**`.

## Operation index

| Resource | Operations |
| --- | --- |
| Authentication | `login`, `verify_mfa`, `social_login`, `change_password`, `request_password_reset`, `reset_password`, `link_social_login`, `social_login_url` |
| OAuth 2.1 | `protected_resource_metadata`, `authorization_server_metadata`, `token`, `revoke`, `userinfo` |
| Accounts | `list`, `create`, `get`, `update`, `delete`, `delete_forcing`, `theme`, `stats`, `download_logo`, `upload_logo`, `delete_logo` |
| API keys | `create`, `get`, `delete` |
| Users | `me`, `stats`, `notification_preferences`, `update_notification_preferences`, `mfa_methods`, `start_totp_enrollment`, `confirm_totp_enrollment`, `regenerate_recovery_codes`, `remove_mfa_method` |
| Signers | `create`, `list`, `get`, `update`, `delete` |
| Documents | `statuses`, `list`, `upload`, `search`, `get`, `rename`, `delete`, `download_artifact`, `download_thumbnail`, `download_page`, `verify` |
| Assignments | `list`, `list_current`, `create`, `estimate_cost`, `reset_expiration`, `reset_expiration_at`, `resend_to_signer`, `estimate_resend_cost`, `whatsapp_notifications`, `identity_verification`, `download_identity_image`, `review_identity_verification`, `sign`, `reject` |
| Tags | `list`, `create`, `update`, `delete`, `delete_with_force`, `list_for_document`, `add_to_document`, `set_on_document`, `remove_from_document` |
| Fields | `create`, `list`, `get`, `update`, `delete`, `validate`, `validate_multiple`, `list_types` |
| Templates | `list`, `get`, `create`, `update`, `delete`, `download_page`, `create_document`, `estimate_cost` |
| Webhooks | `list_endpoints`, `list_endpoints_by_url`, `delete_endpoint_by_url`, `create_endpoint`, `get_endpoint`, `update_endpoint`, `delete_endpoint`, `endpoint_secret`, `rotate_endpoint_secret`, `register`, `get_subscription`, `inactivate`, `event_types`, `list_dispatches`, `retry_dispatch`; `verify_webhook_signature` |
| Activities | `list` |
| Public | `document`, `send_token`, `send_token_with_result` |
| Signer (self) | `me`, `identity_verification`, `start_identity_verification`, `accept_terms`, `verify`, `confirm_data`, `signable_document`, `signable_document_with_accepted_terms`, `current_document`, `list_documents`, `list_documents_as_workspace`, `search_documents`, `search_documents_as_workspace`, `sign`, `decline`, `sign_multiple`, `decline_multiple`, `download_document`, `upload_signature`, `upload_signature_with_reuse`, `download_signature` |

Each method's Rustdoc states the HTTP route it calls and shows the request and response wire
shapes. Runnable examples live in [`examples/`](examples).

## Cargo features

* `rustls-tls` *(default)* — TLS via [rustls](https://docs.rs/rustls).
* `native-tls` — TLS via the operating system's native stack.

## Tests

```bash
cargo test --all-targets --all-features --locked
cargo test --doc --all-features --locked
```

The tests run offline: request shapes, validation, unknown values, pagination, credential
redaction, safe download routing and webhook signature verification.

### Interactive OAuth connection

The interactive production connection test checks authorization, workspace access, userinfo,
refresh-token rotation and revocation. Register a temporary app with `documents:read`,
`account:read`, `openid` and `offline_access`, and start a local HTTPS tunnel:

```bash
cloudflared tunnel --url http://127.0.0.1:62477 --no-autoupdate
```

Register the tunnel’s HTTPS URL plus `/callback` as the exact redirect URI. In another terminal, set
these runtime values, then open the authorization URL printed by the test within five minutes:

```bash
export ASSINAFY_OAUTH_CLIENT_ID='registered-client-id'
export ASSINAFY_OAUTH_REDIRECT_URI='https://your-tunnel.example.invalid/callback'
export ASSINAFY_OAUTH_CALLBACK_ADDR='127.0.0.1:62477'
# Confidential apps also set ASSINAFY_OAUTH_CLIENT_SECRET.
cargo test --test oauth --locked -- --ignored --nocapture
```

The test revokes its tokens before asserting the result. Stop the tunnel afterward. Email OTP
signing needs the recipient to supply their delivered access code; A1/A3 signing needs the signer’s
own certificate and Web PKI extension. These interactive steps are separate from API-key tests.
WhatsApp notification requires an eligible plan.

## License

Released under the [MIT License](LICENSE).
