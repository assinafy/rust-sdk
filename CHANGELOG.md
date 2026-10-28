# Changelog

All notable changes to the `assinafy` crate are documented here. The format follows
[Keep a Changelog](https://keepachangelog.com/en/1.1.0/) and the crate adheres to
[Semantic Versioning](https://semver.org/).

## [3.8.0] - 2026-10-28

### Added

- Contract tracking for listing, reading, updating, relating, renewing and terminating document
  contracts, including typed terms, amendments, external references and optimistic write
  preconditions.
- Contract lifecycle notification preferences and `Document::has_contract`.
- Typed webhook event bodies, delivery attempt metadata, endpoint hold state and workspace webhook
  endpoint limits.
- Document verification codes and public verification URLs on document models.

## [3.7.1] - 2026-10-19

### Fixed

- Assignment creation and cost estimation now reject digital-certificate signers on collect
  assignments before they can be invited or charged for an assignment they cannot complete.

## [3.7.0] - 2026-10-16

### Added

- Hosted biometric identity verification with explicit consent, capture-session status and
  account review, including evidence downloads and approval or rejection decisions.
- `VerificationMethod::id_document_face_match_basic()` configures biometric verification
  for assignment and template signers. Signer profiles expose the verification method and
  assignment signers expose the identity verification status.
- Webhook endpoint lookup and deletion by exact registered URL.
- Public `ListActivitiesRequest` builder for document activity queries.
- `WebhookEndpoint::secret` preserves the signing secret returned at creation and redacts
  it from `Debug`, so OAuth applications can store it without a separate secret request.

### Fixed

- Webhook signature verification rejects an empty signing key.
- Updated locked TLS and QUIC dependencies with security fixes.

## [3.6.1] - 2026-10-12

### Added

- Two-factor authentication: `AuthApi::verify_mfa` completes a login that answered with
  `LoginResult::mfa_token`, and `UsersApi` gains `mfa_methods`, `start_totp_enrollment`,
  `confirm_totp_enrollment`, `regenerate_recovery_codes` and `remove_mfa_method`, with the
  `MfaStatus`, `MfaMethod`, `TotpEnrollment` and `RecoveryCodes` models (secrets redacted in
  `Debug`).
- `PublicApi::send_token_with_result` returns the send-token response
  `{ document, channel, recipient }` as `SendTokenResult`.
- `SignerSelfApi::list_documents_as_workspace` and `search_documents_as_workspace` read a
  signer's documents with the workspace API key or user token.
- `Error::rate_limit` and `ApiError::rate_limit` expose the `X-Rate-Limit-Limit`,
  `X-Rate-Limit-Remaining` and `X-Rate-Limit-Reset` window as `RateLimit`;
  `PaginationMeta::rate_limit_reset` carries the reset on successful list responses.
- `government_id` on `Signer`, `SignerSelf`, `AssignmentSigner` and `DeclinedBy`, and
  `resource` on `AssignmentSigner`.
- `CreateSignerBody::government_id` sets the signer's CPF/CNPJ at creation.

### Changed

- Signer-facing methods (`SignerSelfApi`, `AssignmentsApi::sign` and `AssignmentsApi::reject`)
  now require a non-empty `Auth::AccessCode` and send only that code. An API key or user token
  is refused with `Error::Config` and an empty code with `Error::Validation`, before any request.
  Use the new `*_as_workspace` methods for workspace access to a signer's documents.
- `PublicApi::send_token` accepts only the `email` channel, as the API does, and refuses another
  channel or an empty recipient with `Error::Validation`. `SendTokenBody::whatsapp` is
  deprecated.
- `SignerSelfApi::verify` sends only `{ "verification-code" }`; `VerifyCodeBody::access_code`
  is deprecated and no longer sent.
- `Error::retry_after` reads `Retry-After`; `X-Rate-Limit-Reset` is used only for a `429` that
  omits it, so other errors no longer report a retry delay.
- `PublicDocument::page_count` accepts the integer the send-token response sends as well as a
  numeric string.
- `CreateSignerBody` has a new public field, `government_id`. Code that builds it with an
  exhaustive struct literal must add `government_id: None` or use `CreateSignerBody::new`.
