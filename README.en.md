# assinafy — Rust SDK

*[Leia em português](README.md) · English*

[![Crate](https://img.shields.io/crates/v/assinafy.svg)](https://crates.io/crates/assinafy)
[![Docs](https://docs.rs/assinafy/badge.svg)](https://docs.rs/assinafy)

Async Rust client for every operation in the [Assinafy API](https://api.assinafy.com.br/v1/docs), plus supported template compatibility routes. Each public endpoint method documents its route, request, response, authentication, and typed models on [docs.rs](https://docs.rs/assinafy).

## Install

Rust 1.86+ and edition 2024 are supported. Rust has no LTS channel; CI tests the MSRV and current stable. HTTPS uses TLS 1.2 or newer.

```toml
[dependencies]
assinafy = "3.9"
tokio = { version = "1", features = ["macros", "rt-multi-thread", "time"] }
```

The default feature is `rustls-tls`; use `default-features = false, features = ["native-tls"]` for the operating-system TLS stack.

## Quick start

```rust,no_run
use assinafy::Client;

#[tokio::main]
async fn main() -> assinafy::Result<()> {
    let client = Client::builder()
        .api_key(std::env::var("ASSINAFY_API_KEY").expect("ASSINAFY_API_KEY"))
        .build()?;
    let signers = client
        .signers("acc_1234567890abcdef12345678")
        .list()
        .per_page(50)
        .send()
        .await?;
    for signer in signers.data {
        println!("{}", signer.full_name);
    }
    Ok(())
}
```

Build one cloneable `Client` and use its resource handles: `accounts_api`, `account`, `api_keys`, `users`, `signers`, `signer_self`, `documents`, `assignments`, `contracts`, `tags`, `fields`, `templates`, `webhooks`, `activities`, `public`, `auth_api`, and `oauth`. Calls return `assinafy::Result<T>` and unwrap the API's `{status,message,data}` envelope; OAuth metadata and tokens remain flat.

## Authentication and environments

- `Auth::ApiKey` / `ClientBuilder::api_key`: `X-Api-Key`, for server integrations.
- `Auth::Bearer`: user or OAuth token in `Authorization`.
- `Auth::AccessToken`: legacy `?access-token=` form.
- `Auth::AccessCode`: only signer routes, as `?signer-access-code=`. It cannot be obtained programmatically.

Production is the default. For sandbox, use `BaseUrl::custom("https://sandbox.assinafy.com.br/v1")`. Custom non-loopback URLs must be HTTPS and cannot contain credentials, a query, or a fragment. Secrets and private download/capture URLs are redacted from `Debug`.

## OAuth 2.1 with PKCE

Register an exact HTTPS callback under Settings → OAuth applications. Public clients have no secret; protected servers use Confidential and add `.client_secret(...)`. Request only needed scopes; include `account:read` to identify the workspace, `openid` for userinfo, and `offline_access` only for refresh.

1. Read `oauth().protected_resource_metadata()`, then `authorization_server_metadata(...)`.
2. Generate fresh `PkceChallenge`, state, and optional OIDC nonce; keep them in the initiating session for one attempt.
3. Build the browser URL with `AuthorizationRequest`; validate the callback origin/path, duplicate parameters, `state`, and `iss` before handling its code or error.
4. Exchange the single-use, 60-second code once with `TokenRequest::authorization_code`. Confidential clients add the secret.
5. Validate any ID token with an OIDC library: signature/JWKS, issuer, audience, expiry, and nonce. The SDK does not validate it.
6. Use `Auth::Bearer`, call `accounts_api().list()`, and persist the one workspace ID, encrypted tokens, expiry, and returned scopes.
7. Serialize refreshes per connection. Every refresh rotates the token for another 30 days; atomically save the new refresh token before using the access token. Never retry an uncertain refresh or reuse a retired token.
8. Revoke the current refresh token once with `RevokeRequest` before deleting a connection.

Token and revocation requests are form-encoded, never redirected, and sent once. `offline_access` is not returned in the granted `scope`. OAuth is available in production only.

## Complete document and signing flow

1. Create or reuse signers with `signers(account).create(...)`; provide the contact required by the selected channel and `government_id` when needed.
2. Upload a PDF (≤25 MB, ≤2000 pages) with `documents().upload(...)`, or instantiate a template with existing signer IDs.
3. Poll `documents().get(...)` with a deadline until `metadata_ready`; persist IDs so retries resume existing resources.
4. Add tags by **name**, place fields, and call `assignments().estimate_cost(...)`.
5. Create a `Virtual` or `Collect` assignment. Do not repeat document or assignment creation after an uncertain response.
6. Deliver each private signing URL. The signer accepts terms, confirms data, completes verification, fills their own items, and signs.
7. Await a verified webhook or bounded polling; then download `ArtifactName::Certificated`, `Pades`, `CertificatePage`, or `Bundle`.

Allowed verification/notification pairs:

| Verification | Notification | Notes |
| --- | --- | --- |
| `Email` | `Email` | Free OTP; default. |
| `Whatsapp` | `Whatsapp` | OTP; notification is plan-gated and billed. |
| `DigitalCertificate` | `Email` or `Whatsapp` | Signer's ICP-Brasil A1/A3 certificate; virtual only; produces PAdES. |
| `VerificationMethod::id_document_face_match_basic()` | `Email` or `Whatsapp` | Hosted document/face capture with explicit consent. |

Use exactly one notification channel. Missing verification/notification defaults or is inferred by the API. `DigitalCertificate` is rejected locally on `Collect`; template certificate roles cannot have placed signer fields. Estimate costs before creation because plan gates and pricing vary.

### Biometric and certificate completion

The signer client must carry only `Auth::AccessCode`. For biometric verification, inspect `signer_self().identity_verification()`, call `start_identity_verification(true)` only after explicit consent and when `can_start`, and open the returned bearer URL unchanged—including its fragment. Never log or store that URL in browser storage. Poll with a deadline: `Expired` may restart; `Declined` or `Abandoned` requires a new document and assignment.

`InReview` permits signing but holds certification. The workspace reads `assignments().identity_verification(...)`, downloads only named evidence images, and submits an explicit `IdentityReviewDecision`. Approval releases certification after all signatures; rejection cancels the document. Evidence images disappear after review.

A1/A3 completion requires the signer's own certificate and the Web PKI browser extension. Its production-only start/complete handshake is outside the published OpenAPI contract and this server SDK; the SDK fully configures, estimates, creates, and observes certificate assignments and downloads their PAdES artifact.

## Templates, contracts, and webhooks

Template document creation requires existing signer IDs. `estimate_cost` projects only pricing fields. Template CRUD/page download compatibility routes remain available because they work live even though they are absent from OpenAPI.

`contracts(account)` covers list, get/update, related family, renew, and terminate. Nullable update fields distinguish omitted/value/JSON `null`; pass the last `updated_at` for optimistic concurrency. Never retry writes automatically. Contract writes may be plan-gated while reads remain available.

For signed webhooks, create an endpoint with `signing_enabled(true)`, encrypt the one-time returned `secret`, and verify the **raw** body before parsing with `verify_webhook_signature`. Reject replayed webhook IDs in application storage. OAuth apps cannot retrieve/rotate a lost secret; delete and recreate only their own endpoint. Never modify an existing endpoint for a test.

## Queries, errors, and compatibility

List builders expose supported `search`, `sort`, filter, `page`, and `per_page` options. Assignment listing and lightweight document search both support `sort`. Results are `Page<T>` with pagination and rate-limit metadata.

`Error::Api` preserves HTTP status, server message, safe structured data, `Retry-After`, rate-limit headers, and OAuth error codes. Decode errors report category/location/body length, never the response body. Caller-supplied path IDs cannot retarget a route; authenticated binary redirects keep credentials only on safe same-origin HTTPS URLs.

Unknown documented response values are retained where the public model supports an `Other`/`Unknown` variant. The SDK preserves live compatibility behavior such as assignment `accountId`, field update extras, old webhook subscription routes, and template CRUD; it does not remove working operations merely because a deployment or plan rejects them.

## Validation

All committed tests are offline; the OAuth connection test is ignored and interactive.

```bash
cargo fmt --all --check
cargo clippy --all-targets --all-features --locked -- -D warnings
cargo test --all-targets --all-features --locked
cargo test --doc --all-features --locked
RUSTDOCFLAGS="-D warnings" cargo doc --no-deps --all-features --locked
cargo build --all-targets --no-default-features --features native-tls --locked
```

Run the all-target/all-feature tests on Rust 1.86 as well. Interactive OTP, biometric capture, and certificate completion require a person and cannot be proven by the offline suite.

## License

[MIT](LICENSE)
