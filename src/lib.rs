//! # Assinafy Rust SDK
//!
//! Idiomatic, async Rust client for the [Assinafy](https://assinafy.com.br)
//! electronic-signature API (`https://api.assinafy.com.br/v1`).
//!
//! The SDK is organised around a single [`Client`] that exposes a resource
//! module per API surface — accounts, signers, documents, assignments,
//! templates, tags, fields, webhooks, activities, API keys, and authentication
//! helpers. Every call is `async`, returns a strongly-typed [`Result`], and
//! uses the same response envelope the API itself uses
//! (`{ status, message, data }`).
//!
//! Account-level operations are reached via [`Client::accounts_api`] (list and
//! create accounts) and [`Client::account`] (get/update/delete an account plus
//! its theme and logo).
//!
//! ## Quick start
//!
//! ```no_run
//! use assinafy::Client;
//!
//! # async fn run() -> assinafy::Result<()> {
//! let client = Client::builder()
//!     .api_key("your-api-key")
//!     .build()?;
//!
//! // List signers in an account.
//! let signers = client
//!     .signers("acc_1234567890abcdef12345678")
//!     .list()
//!     .send()
//!     .await?;
//!
//! for s in &signers.data {
//!     println!("{} <{:?}>", s.full_name, s.email);
//! }
//! # Ok(()) }
//! ```
//!
//! ## Authentication
//!
//! The SDK supports the three credential schemes the API accepts:
//!
//! * [`Auth::ApiKey`] — sent as the `X-Api-Key` header (recommended for
//!   server-to-server use).
//! * [`Auth::Bearer`] — sent as `Authorization: Bearer <token>` (for tokens
//!   obtained from [`AuthApi::login`](crate::resources::AuthApi::login)).
//! * [`Auth::AccessToken`] — sent as the documented `?access-token=...` query
//!   parameter when that legacy form is required.
//! * [`Auth::AccessCode`] — sent as the `?signer-access-code=...` query
//!   parameter for signer-facing endpoints.
//!
//! Switch credentials at runtime with [`Client::with_auth`].
//!
//! ## Environments
//!
//! Both production and sandbox base URLs are first-class:
//!
//! ```
//! use assinafy::Client;
//!
//! let sandbox = Client::builder()
//!     .api_key("test-key")
//!     .sandbox()
//!     .build()
//!     .unwrap();
//! ```
//!
//! ## Errors
//!
//! All operations return [`Result<T, Error>`](Error). API failures preserve
//! the HTTP status, the server-provided message and the raw `data` payload so
//! callers can branch on specific error codes without losing fidelity. The API
//! rate-limits requests (see the `X-Rate-Limit-*` response headers);
//! [`Error::is_rate_limited`] and [`Error::retry_after`] surface `429`
//! responses and the number of seconds to wait before retrying.
//!
//! ## Cargo features
//!
//! * `rustls-tls` *(default)* — TLS via [rustls].
//! * `native-tls` — TLS via the operating system's native stack.
//!
//! [rustls]: https://docs.rs/rustls

#![deny(missing_docs)]

// Compile-check every code block in the README as a doctest without duplicating
// the crate-level documentation above. `cfg(doctest)` keeps this out of the
// build and the rendered docs.
#[cfg(doctest)]
#[doc = include_str!("../README.md")]
struct ReadmeDoctests;

mod auth;
mod client;
mod config;
mod error;
mod http;
mod pagination;

pub mod models;
pub mod resources;

pub use auth::Auth;
pub use client::{Client, ClientBuilder};
pub use config::BaseUrl;
pub use error::{ApiError, Error, Result};
pub use http::Envelope;
pub use pagination::{Page, PaginationMeta};
