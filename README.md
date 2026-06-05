# assinafy — Rust SDK

[![Crate](https://img.shields.io/crates/v/assinafy.svg)](https://crates.io/crates/assinafy)
[![Docs](https://docs.rs/assinafy/badge.svg)](https://docs.rs/assinafy)

Async, idiomatic Rust client for the [Assinafy](https://assinafy.com.br)
electronic-signature API.

The SDK is a 1:1 mapping of the public REST surface documented at
<https://api.assinafy.com.br/v1/docs>:

| Surface          | Module                              |
| ---------------- | ----------------------------------- |
| Authentication   | [`Client::auth_api`]                |
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
| Public endpoints | [`Client::public`]                  |

[`Client::auth_api`]: https://docs.rs/assinafy/latest/assinafy/struct.Client.html#method.auth_api
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
[`Client::public`]: https://docs.rs/assinafy/latest/assinafy/struct.Client.html#method.public

## Install

Requires Rust 1.86 or newer.

```toml
[dependencies]
assinafy = "1"
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
    .signers("102d25a489f34a275d31a16045fd")
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
let c1 = Client::builder().api_key("...").build().unwrap();

// User-token flow.
let c2 = Client::builder()
    .bearer("eyJhbGciOi...")
    .build()
    .unwrap();

// Query-parameter access-token flow, when required by an integration.
let c2_query = Client::builder()
    .access_token("eyJhbGciOi...")
    .build()
    .unwrap();

// Signer-facing endpoints use the URL access code.
let c3 = c1.with_auth(Auth::AccessCode("signer-token".into()));
```

## Common flows

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

// Clear expiration by sending the documented JSON null value.
client
    .assignments()
    .reset_expiration("doc_abc", &assignment.id, None)
    .await?;

for url in &assignment.signing_urls {
    println!("signer {} -> {}", url.signer_id, url.url);
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

Document tag operations use tag names, matching the API reference:

```rust
use assinafy::Client;

# async fn run() -> assinafy::Result<()> {
let client = Client::from_api_key("k")?;
let tags = client.tags("acc_123");
tags.add_to_document("doc_abc", ["Contracts", "Urgent"]).await?;
tags.set_on_document("doc_abc", ["Signed"]).await?;
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
            .email("signer@example.com")
            .accepted_terms(true),
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

## Cargo features

* `rustls-tls` *(default)* — TLS via [rustls].
* `native-tls` — TLS via the operating system's native stack.

[rustls]: https://docs.rs/rustls

## Running the integration tests

```bash
export ASSINAFY_API_KEY=<sandbox-key>
export ASSINAFY_ACCOUNT_ID=<sandbox-account>
cargo test --test sandbox -- --ignored --test-threads=1
```

The `--ignored` flag is required because these tests hit the live sandbox, and
`--test-threads=1` keeps the shared workspace state consistent. The
[`Sandbox` workflow](.github/workflows/sandbox.yml) runs them on a daily
schedule when the `ASSINAFY_API_KEY` and `ASSINAFY_ACCOUNT_ID` repository
secrets are configured (they skip themselves when the secrets are absent).

## License

Dual-licensed under MIT or Apache-2.0 at your option.
