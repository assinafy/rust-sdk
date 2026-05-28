# Assinafy Rust SDK Audit

Audit date: 2026-05-28

This crate was audited against the Assinafy REST API surface at
`https://api.assinafy.com.br/v1/docs` and verified against the public sandbox.

## Endpoint Coverage

| API surface | Official endpoints | Rust SDK entry points |
| --- | --- | --- |
| Authentication | `POST /login`, `POST /authentication/social-login`, `PUT /authentication/change-password`, `PUT /authentication/request-password-reset`, `PUT /authentication/reset-password` | `Client::auth_api()` |
| API keys | `POST /users/api-keys`, `GET /users/api-keys`, `DELETE /users/api-keys` | `Client::api_keys()` |
| Signers | `POST/GET /accounts/{account_id}/signers`, `GET/PUT/DELETE /accounts/{account_id}/signers/{signer_id}` | `Client::signers(account_id)` |
| Signer self | `GET /signers/self`, `PUT /signers/accept-terms`, `POST /verify`, `PUT /documents/{document_id}/signers/confirm-data`, `POST /signature`, `GET /signature/{type}` | `Client::signer_self()` |
| Documents | `GET /documents/statuses`, `GET/POST /accounts/{account_id}/documents`, `GET/DELETE /documents/{document_id}`, artifact, thumbnail, page download, and verify endpoints | `Client::documents()` |
| Public documents | `GET /public/documents/{document_id}`, `PUT /public/documents/{document_id}/send-token` | `Client::public()` |
| Document tags | `GET/POST/PUT /accounts/{account_id}/documents/{document_id}/tags`, `DELETE /accounts/{account_id}/documents/{document_id}/tags/{tag_id}` | `Client::tags(account_id)` |
| Templates | `GET /accounts/{account_id}/templates`, `GET/POST/PUT /accounts/{account_id}/templates/{template_id}`, create-document and estimate-cost endpoints | `Client::templates(account_id)` |
| Tags | `GET/POST /accounts/{account_id}/tags`, `PUT/DELETE /accounts/{account_id}/tags/{tag_id}` | `Client::tags(account_id)` |
| Assignments | Create, estimate cost, signer resend, resend estimate, reset expiration, signer sign/decline, signer view, WhatsApp notifications | `Client::assignments()` and `Client::signer_self()` |
| Signer documents | Current document, list, sign multiple, decline multiple, download artifact | `Client::signer_self()` |
| Fields | CRUD, validate, validate-multiple, `GET /field-types` | `Client::fields(account_id)` |
| Webhooks | Subscription get/update/delete/inactivate, event types, dispatch list, retry | `Client::webhooks(account_id)` |
| Activities | `GET /documents/{document_id}/activities` | `Client::activities()` |

## File Audit

| File | Audit result |
| --- | --- |
| `Cargo.toml` | Uses Rust 2024 edition, MSRV 1.85, current `reqwest` 0.13, rustls default TLS, native TLS opt-in, and no unused direct runtime dependencies. |
| `src/lib.rs` | Public crate surface is documented with `#![deny(missing_docs)]`; exports are limited to client, auth, error, pagination, envelope, models, and resources. |
| `src/client.rs` | Client is cloneable, reusable, timeout-configurable, and exposes one resource accessor per API surface. |
| `src/auth.rs` | Supports API key, bearer token, query access token, signer access code, and unauthenticated public calls with redacted debug output. |
| `src/config.rs` | Production, sandbox, and custom base URLs normalize trailing slashes for safe URL joining. |
| `src/http.rs` | Centralizes request creation, envelope decoding, direct JSON fallback, bytes downloads, empty-success handling, and API error mapping. |
| `src/error.rs` | Preserves HTTP status, server message, and structured `data` on API errors. |
| `src/pagination.rs` | Maps `X-Pagination-*` headers into typed pagination metadata. |
| `src/resources/*.rs` | Resource modules map the documented endpoint paths and payloads; assignment creation now defaults to the current `signers: [{ id }]` shape while keeping `signer_ids` legacy support. |
| `src/models/*.rs` | API response models tolerate optional/missing fields, preserve unknown enum values, and include documented resource discriminators where returned. |
| `tests/unit.rs` | Covers serialization shapes, unknown enum handling, redaction, builders, pagination/base URL behavior, and documented request field names. |
| `tests/sandbox.rs` | Exercises the live sandbox for statuses, signer/tag/field lifecycles, document upload/delete, reference lists, pagination, and API-error mapping. |
| `examples/*.rs` | Compile-tested examples cover listing/creating signers, uploading documents, creating assignments, and tag workflows. |
| `README.md` | Documents installation, authentication, common flows, sandbox testing, pagination, errors, and Cargo features. |

## Verification

All commands were run from `/Users/billm/Dev/assinafy-sdk/rust`.

```bash
/Users/billm/.cargo/bin/cargo fmt --check
/Users/billm/.cargo/bin/cargo test
/Users/billm/.cargo/bin/cargo clippy --all-targets --all-features -- -D warnings
/Users/billm/.cargo/bin/cargo doc --no-deps --all-features
/Users/billm/.cargo/bin/cargo package --allow-dirty --offline
ASSINAFY_API_KEY=<sandbox-key> ASSINAFY_ACCOUNT_ID=<sandbox-account> \
  /Users/billm/.cargo/bin/cargo test --test sandbox -- --ignored --test-threads=1
```

Result: all local tests, doctests, clippy, docs, package verification, and live
sandbox tests passed.
