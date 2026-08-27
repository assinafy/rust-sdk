# assinafy — Rust SDK

[![Crate](https://img.shields.io/crates/v/assinafy.svg)](https://crates.io/crates/assinafy)
[![Docs](https://docs.rs/assinafy/badge.svg)](https://docs.rs/assinafy)

Async, idiomatic Rust client for the [Assinafy](https://assinafy.com.br)
electronic-signature API.

The SDK covers the public REST surface documented at
<https://api.assinafy.com.br/v1/docs>:

| Surface          | Module                              |
| ---------------- | ----------------------------------- |
| Authentication   | [`Client::auth_api`]                |
| Accounts         | [`Client::account`] / [`Client::accounts_api`] |
| API keys         | [`Client::api_keys`]                |
| Signers          | [`Client::signers`]                 |
| Signer (self)    | [`Client::signer_self`]             |
| Documents        | [`Client::documents`]               |
| Assignments      | [`Client::assignments`]             |
| Tags             | [`Client::tags`]                    |
| Fields           | [`Client::fields`]                  |
| Templates        | [`Client::templates`]               |
| Webhooks         | [`Client::webhooks`]                |
| Activities       | [`Client::activities`]              |
| Users            | [`Client::users`]                   |
| Public endpoints | [`Client::public`]                  |

[`Client::auth_api`]: https://docs.rs/assinafy/latest/assinafy/struct.Client.html#method.auth_api
[`Client::account`]: https://docs.rs/assinafy/latest/assinafy/struct.Client.html#method.account
[`Client::accounts_api`]: https://docs.rs/assinafy/latest/assinafy/struct.Client.html#method.accounts_api
[`Client::api_keys`]: https://docs.rs/assinafy/latest/assinafy/struct.Client.html#method.api_keys
[`Client::signers`]: https://docs.rs/assinafy/latest/assinafy/struct.Client.html#method.signers
[`Client::signer_self`]: https://docs.rs/assinafy/latest/assinafy/struct.Client.html#method.signer_self
[`Client::documents`]: https://docs.rs/assinafy/latest/assinafy/struct.Client.html#method.documents
[`Client::assignments`]: https://docs.rs/assinafy/latest/assinafy/struct.Client.html#method.assignments
[`Client::tags`]: https://docs.rs/assinafy/latest/assinafy/struct.Client.html#method.tags
[`Client::fields`]: https://docs.rs/assinafy/latest/assinafy/struct.Client.html#method.fields
[`Client::templates`]: https://docs.rs/assinafy/latest/assinafy/struct.Client.html#method.templates
[`Client::webhooks`]: https://docs.rs/assinafy/latest/assinafy/struct.Client.html#method.webhooks
[`Client::activities`]: https://docs.rs/assinafy/latest/assinafy/struct.Client.html#method.activities
[`Client::users`]: https://docs.rs/assinafy/latest/assinafy/struct.Client.html#method.users
[`Client::public`]: https://docs.rs/assinafy/latest/assinafy/struct.Client.html#method.public

## Install

Requires Rust 1.86 or newer and uses the Rust 2024 edition. CI tests both the
declared 1.86 minimum and the current stable Rust release; Rust does not have
an LTS channel.

```toml
[dependencies]
assinafy = "2"
tokio    = { version = "1", features = ["macros", "rt-multi-thread"] }
```

## Quick start

```rust
use assinafy::Client;

# async fn run() -> assinafy::Result<()> {
let client = Client::builder()
    .api_key(std::env::var("ASSINAFY_API_KEY").unwrap())
    .sandbox() // omit for production
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
# Ok(()) }
```

## Authentication

```rust
use assinafy::{Auth, Client};

// Server-to-server via API key (default for most users).
let c1 = Client::builder().api_key("redacted-api-key").build().unwrap();

// User-token flow.
let c2 = Client::builder()
    .bearer("redacted-access-token")
    .build()
    .unwrap();

// Query-parameter access-token flow, when required by an integration.
let c2_query = Client::builder()
    .access_token("redacted-access-token")
    .build()
    .unwrap();

// Signer-facing endpoints use the URL access code.
let c3 = c1.with_auth(Auth::AccessCode("signer-token".into()));
```

## End-to-end document flow

This flow creates a signer, uploads a PDF, requests a virtual signature,
checks the resulting document, downloads the certificated PDF when it is
ready, and optionally removes sandbox fixtures. Signing links grant access to
the document: send them through a private channel and never log or persist
them in plaintext.

```rust,no_run
use assinafy::Client;
use assinafy::models::{ArtifactName, AssignmentMethod, DocumentStatus};
use assinafy::resources::{
    CreateAssignmentBody, CreateSignerBody, UploadDocumentRequest,
};

# async fn run() -> assinafy::Result<()> {
let account_id = std::env::var("ASSINAFY_ACCOUNT_ID").unwrap();
let client = Client::builder()
    .api_key(std::env::var("ASSINAFY_API_KEY").unwrap())
    .sandbox() // remove this call for production
    .build()?;

let signer = client
    .signers(&account_id)
    .create(
        &CreateSignerBody::new("Integration Signer")
            .email(std::env::var("ASSINAFY_SIGNER_EMAIL").unwrap()),
    )
    .await?;

let upload = UploadDocumentRequest::from_path("./contract.pdf").await?;
let document = client.documents().upload(&account_id, upload).await?;

let request = CreateAssignmentBody::new(
    AssignmentMethod::Virtual,
    [&signer.id],
)
.message("Please review and sign this contract.");
let assignment = client.assignments().create(&document.id, &request).await?;
println!("assignment {} created for signer {}", assignment.id, signer.id);

// Run this status check again after the recipient completes the signing flow.
let current = client.documents().get(&document.id).await?;
if matches!(current.status, DocumentStatus::Certificated) {
    let (pdf, content_type) = client
        .documents()
        .download_artifact(&document.id, ArtifactName::Certificated)
        .await?;
    assert_eq!(content_type, "application/pdf");
    tokio::fs::write("./contract-certificated.pdf", pdf).await?;
}

// Explicit opt-in cleanup for disposable sandbox fixtures.
if std::env::var_os("ASSINAFY_CLEANUP").is_some() {
    let _ = client.documents().delete(&document.id).await;
    let _ = client.signers(&account_id).delete(&signer.id).await;
}
# Ok(()) }
```

The complete runnable examples live in [`examples/`](examples). Each API
method’s Rustdoc states its HTTP route and shows the request and response wire
shape.

## Operation index

| Resource | SDK operations |
| --- | --- |
| Authentication | `login`, `social_login`, `change_password`, `request_password_reset`, `reset_password`, `link_social_login`, `social_login_url` |
| Accounts | `list`, `create`, `get`, `update`, `delete`, `delete_forcing`, `theme`, `stats`, `download_logo`, `upload_logo`, `delete_logo` |
| API keys | `create`, `get`, `delete` |
| Users | `me`, `stats`, `notification_preferences`, `update_notification_preferences` |
| Signers | `create`, `list`, `get`, `update`, `delete` |
| Documents | `statuses`, `list`, `upload`, `search`, `get`, `rename`, `delete`, `download_artifact`, `download_thumbnail`, `download_page`, `verify` |
| Assignments | `list`, `list_current`, `create`, `estimate_cost`, `reset_expiration`, `reset_expiration_at`, `resend_to_signer`, `estimate_resend_cost`, `whatsapp_notifications`, `sign`, `reject` |
| Tags | `list`, `create`, `update`, `delete`, `delete_with_force`, `list_for_document`, `add_to_document`, `set_on_document`, `remove_from_document` |
| Fields | `create`, `list`, `get`, `update`, `delete`, `validate`, `validate_multiple`, `list_types` |
| Templates | `list`, `get`, `create`, `update`, `delete`, `download_page`, `create_document`, `estimate_cost` |
| Webhooks | `register`, `get_subscription`, `inactivate`, `event_types`, `list_dispatches`, `retry_dispatch` |
| Activities | `list` |
| Public | `document`, `send_token`, `send_token_legacy` |
| Signer self | `me`, `accept_terms`, `verify`, `confirm_data`, `signable_document`, `signable_document_with_accepted_terms`, `current_document`, `list_documents`, `search_documents`, `sign`, `decline`, `sign_multiple`, `decline_multiple`, `download_document`, `upload_signature`, `upload_signature_with_reuse`, `download_signature` |

## Focused examples

### Upload a document

```rust
use assinafy::Client;
use assinafy::resources::UploadDocumentRequest;

# async fn run() -> assinafy::Result<()> {
let client = Client::from_api_key("k")?;
let upload = UploadDocumentRequest::from_path("./contract.pdf").await?;
let doc = client.documents().upload("acc_123", upload).await?;
println!("uploaded {} ({})", doc.name, doc.id);
# Ok(()) }
```

### Request signatures

```rust
use assinafy::Client;
use assinafy::models::AssignmentMethod;
use assinafy::resources::CreateAssignmentBody;

# async fn run() -> assinafy::Result<()> {
let client = Client::from_api_key("k")?;

let body = CreateAssignmentBody::new(
        AssignmentMethod::Virtual,
        ["sig_1", "sig_2"],
    )
    .message("Please sign by Friday.");

let assignment = client.assignments().create("doc_abc", &body).await?;

// Extend the assignment using the documented ISO-8601 timestamp.
client
    .assignments()
    .reset_expiration_at("doc_abc", &assignment.id, "2026-12-31T23:59:59Z")
    .await?;

// Signing URLs grant document access: deliver them privately; do not log them.
for signing_url in &assignment.signing_urls {
    println!("signing link issued for {}", signing_url.signer_id);
}
# Ok(()) }
```

### Templates

```rust
use assinafy::Client;
use assinafy::models::VerificationMethod;
use assinafy::resources::{CreateDocumentFromTemplateBody, TemplateDocumentSigner};

# async fn run() -> assinafy::Result<()> {
let client = Client::from_api_key("k")?;

let estimate_body = CreateDocumentFromTemplateBody::default()
    .signers(vec![
        TemplateDocumentSigner::role("role_123")
            .verification_method(VerificationMethod::Whatsapp),
    ]);
let _cost = client
    .templates("acc_123")
    .estimate_cost("tmpl_abc", &estimate_body)
    .await?;
# Ok(()) }
```

### Tags

```rust
use assinafy::Client;
use assinafy::resources::CreateTagBody;

# async fn run() -> assinafy::Result<()> {
let client = Client::from_api_key("k")?;
let tag = client
    .tags("acc_123")
    .create(&CreateTagBody::new("Contracts").color("3399ff"))
    .await?;
println!("tag id: {}", tag.id);
# Ok(()) }
```

`add_to_document`/`set_on_document` take tag **names**, not IDs: the API
upserts each entry by a case-insensitive name match against the account's
existing tags, auto-creating a new tag if none matches. Passing a real tag
ID here creates a junk tag named after that literal ID string.
`remove_from_document` is the one document-tag operation that takes a real
tag ID (as returned by `tags.create(...)` / `tags.list()`):

```rust
use assinafy::Client;

# async fn run() -> assinafy::Result<()> {
let client = Client::from_api_key("k")?;
let tags = client.tags("acc_123");
tags.add_to_document("doc_abc", ["Contracts", "Urgent"]).await?;
tags.set_on_document("doc_abc", ["Signed"]).await?;
tags.remove_from_document("doc_abc", "tag_id_returned_by_list_for_document").await?;
# Ok(()) }
```

### Accounts

```rust
use assinafy::Client;
use assinafy::resources::UpdateAccountBody;

# async fn run() -> assinafy::Result<()> {
let client = Client::from_api_key("k")?;

// List the accounts this credential can see.
for account in client.accounts_api().list().await? {
    println!("{} ({})", account.name, account.id);
}

// Fetch and update one account, plus its branding theme.
let account = client.account("acc_123").get().await?;
client
    .account("acc_123")
    .update(&UpdateAccountBody::new().name("Renamed workspace"))
    .await?;
let theme = client.account("acc_123").theme().await?;
println!("{} theme: {:?}", account.name, theme.primary_color);
# Ok(()) }
```

### The authenticated user

```rust
use assinafy::Client;

# async fn run() -> assinafy::Result<()> {
let client = Client::from_api_key("k")?;
let me = client.users().me().await?;
println!("{} <{}>", me.name, me.email);
# Ok(()) }
```

### Signer-facing flows

Signer-facing endpoints use `Auth::AccessCode`, which automatically adds the
`signer-access-code` query parameter to every request:

```rust
use assinafy::{Auth, Client};
use assinafy::resources::{ConfirmSignerDataBody, VerifyCodeBody};

# async fn run() -> assinafy::Result<()> {
let client = Client::from_api_key("k")?
    .with_auth(Auth::AccessCode("signer-access-code".into()));

let signer = client.signer_self().me().await?;
client.signer_self().verify(&VerifyCodeBody::new("123456")).await?;
client
    .signer_self()
    .confirm_data(
        "doc_abc",
        &ConfirmSignerDataBody::new()
            .email("user@example.invalid")
            .government_id("12345678909"),
    )
    .await?;

println!("ready signer {}", signer.id);
# Ok(()) }
```

### Pagination

Every paged endpoint returns a [`Page<T>`](https://docs.rs/assinafy/latest/assinafy/struct.Page.html)
containing `data` and `meta` (extracted from the `X-Pagination-*` response
headers):

```rust
use assinafy::Client;

# async fn run() -> assinafy::Result<()> {
let client = Client::from_api_key("k")?;
let mut next = Some(1);
while let Some(page) = next {
    let res = client.signers("acc_123").list().page(page).per_page(100).send().await?;
    println!("page {page}: {} items", res.data.len());
    next = res.next_page();
}
# Ok(()) }
```

### Errors

All operations return [`Result<T, Error>`](https://docs.rs/assinafy/latest/assinafy/enum.Error.html).
API errors retain the HTTP status, server message and raw error payload:

```rust
use assinafy::{Client, Error};

# async fn run() -> assinafy::Result<()> {
let client = Client::from_api_key("k")?;
match client.signers("acc_123").get("missing").await {
    Ok(_) => {}
    Err(Error::Api(e)) if e.status == 404 => eprintln!("not found"),
    Err(other) => return Err(other),
}
# Ok(()) }
```

## Sandbox

Use [`ClientBuilder::sandbox`] to target the public sandbox at
`https://sandbox.assinafy.com.br/v1`.

[`ClientBuilder::sandbox`]: https://docs.rs/assinafy/latest/assinafy/struct.ClientBuilder.html#method.sandbox

## Cargo features

* `rustls-tls` *(default)* — TLS via [rustls].
* `native-tls` — TLS via the operating system's native stack.

[rustls]: https://docs.rs/rustls

## Running the integration tests

```bash
export ASSINAFY_API_KEY=<sandbox-key>
export ASSINAFY_ACCOUNT_ID=<sandbox-account>
export ASSINAFY_TEST_EMAIL_PRIMARY=<notification-test-inbox>
export ASSINAFY_TEST_EMAIL_SECONDARY=<secondary-test-inbox>
cargo test --test sandbox -- --ignored --test-threads=1
```

The `--ignored` flag is required because these tests call the sandbox API, and
`--test-threads=1` keeps the shared workspace state consistent. The email
variables are read only at runtime and are required for notification-delivery
coverage; they are never compiled into the SDK. The
[`Sandbox` workflow](.github/workflows/sandbox.yml) runs them on a daily
schedule and reads all four secrets from the GitHub `sandbox` environment.
Explicit ignored-test runs fail immediately when any value is absent.

## License

Dual-licensed under MIT or Apache-2.0 at your option.
