//! Pure unit tests for the SDK that do not require network access.

use assinafy::models::{
    ArtifactName, AssignmentMethod, DocumentStatus, NotificationMethod, TemplateStatus,
    VerificationMethod,
};
use assinafy::resources::{
    ApiKeyResponse, ConfirmSignerDataBody, CreateAssignmentBody, CreateAssignmentSigner,
    EmailResult, LoginBody, RequestPasswordResetBody, SendTokenBody, SocialLoginBody,
    TemplateDocumentSigner, UpdateTagBody, VerifyCodeBody,
};
use assinafy::{Auth, BaseUrl, Client, Envelope};

#[test]
fn base_url_join_appends_trailing_slash() {
    let u = BaseUrl::Sandbox.as_url();
    assert!(u.as_str().ends_with('/'));
    let joined = u.join("accounts/abc/signers").unwrap();
    assert_eq!(
        joined.as_str(),
        "https://sandbox.assinafy.com.br/v1/accounts/abc/signers"
    );
}

#[test]
fn base_url_custom_normalises_trailing_slash() {
    let u = BaseUrl::custom("https://example.test/api").unwrap();
    let joined = u.as_url().join("docs/statuses").unwrap();
    assert_eq!(joined.as_str(), "https://example.test/api/docs/statuses");
}

#[test]
fn base_url_custom_rejects_unsafe_authority_and_suffixes() {
    for url in [
        "http://example.test/api",
        "https://user:password@example.test/api",
        "https://example.test/api?tenant=1",
        "https://example.test/api#v2",
    ] {
        assert!(BaseUrl::custom(url).is_err(), "accepted unsafe URL {url}");
    }
    assert!(BaseUrl::custom("http://127.0.0.1:8080/v1").is_ok());
    assert!(BaseUrl::custom("http://[::1]:8080/v1").is_ok());
}

#[test]
fn base_url_direct_custom_construction_is_also_normalised() {
    // `BaseUrl::Custom` is a public tuple variant — callers can build one
    // directly with a `Url` that never went through `custom()`.
    let u = BaseUrl::Custom(url::Url::parse("https://example.test/api").unwrap());
    let joined = u.as_url().join("docs").unwrap();
    assert_eq!(joined.path(), "/api/docs");
}

#[test]
fn client_builder_revalidates_direct_custom_urls() {
    let url = url::Url::parse("http://example.test/v1").unwrap();
    assert!(
        Client::builder()
            .base_url(BaseUrl::Custom(url))
            .build()
            .is_err()
    );
}

#[test]
fn document_status_round_trips_known_and_unknown() {
    let s: DocumentStatus = serde_json::from_str("\"certificated\"").unwrap();
    assert_eq!(s, DocumentStatus::Certificated);
    let s: DocumentStatus = serde_json::from_str("\"future_status\"").unwrap();
    assert_eq!(s, DocumentStatus::Unknown("future_status".to_owned()));
    assert_eq!(s.as_str(), "future_status");
}

#[test]
fn template_status_handles_unknown() {
    let s: TemplateStatus = serde_json::from_str("\"new-state\"").unwrap();
    assert_eq!(s, TemplateStatus::Unknown("new-state".into()));
    assert_eq!(s.to_string(), "new-state");
}

#[test]
fn template_status_accepts_capitalized_api_values() {
    let s: TemplateStatus = serde_json::from_str("\"Ready\"").unwrap();
    assert_eq!(s, TemplateStatus::Ready);
}

#[test]
fn assignment_method_serializes_lowercase() {
    let s = serde_json::to_string(&AssignmentMethod::Virtual).unwrap();
    assert_eq!(s, "\"virtual\"");
    let s = serde_json::to_string(&AssignmentMethod::Collect).unwrap();
    assert_eq!(s, "\"collect\"");
}

#[test]
fn verification_and_notification_methods_use_capitalised_strings() {
    assert_eq!(VerificationMethod::Email.as_str(), "Email");
    assert_eq!(
        VerificationMethod::DigitalCertificate.as_str(),
        "DigitalCertificate"
    );
    assert_eq!(NotificationMethod::Whatsapp.as_str(), "Whatsapp");
    let v: VerificationMethod = serde_json::from_str("\"Email\"").unwrap();
    assert_eq!(v, VerificationMethod::Email);
}

#[test]
fn assignment_item_accepts_compact_api_objects() {
    let item: assinafy::models::AssignmentItem = serde_json::from_value(serde_json::json!({
        "id": "item_1",
        "page": null,
        "signer": { "id": "signer_1" },
        "field": { "id": "field_1", "type": "signature" },
        "display_settings": [],
        "value": null,
        "completed": false
    }))
    .unwrap();

    assert_eq!(item.signer.as_ref().unwrap().id, "signer_1");
    assert_eq!(
        item.field.as_ref().unwrap().kind.as_deref(),
        Some("signature")
    );
}

#[test]
fn artifact_name_round_trip() {
    assert_eq!(ArtifactName::from("original"), ArtifactName::Original);
    assert_eq!(
        ArtifactName::from("custom-thing"),
        ArtifactName::Other("custom-thing".into())
    );
    assert_eq!(ArtifactName::Certificated.as_str(), "certificated");
    assert_eq!(ArtifactName::from("pades"), ArtifactName::Pades);
    // `From<String>` (not just `From<&str>`) is supported too.
    assert_eq!(
        ArtifactName::from(String::from("bundle")),
        ArtifactName::Bundle
    );
}

#[test]
fn envelope_decodes_signer_payload() {
    let body = r#"{"status":200,"message":"","data":{"resource":"signer","id":"abc","full_name":"Bill","email":"user@example.invalid","whatsapp_phone_number":null,"has_accepted_terms":false}}"#;
    let env: Envelope<assinafy::models::Signer> = serde_json::from_str(body).unwrap();
    assert_eq!(env.status, 200);
    assert_eq!(env.data.full_name, "Bill");
    assert_eq!(env.data.email.as_deref(), Some("user@example.invalid"));
}

#[test]
fn client_builder_carries_auth_and_base_url() {
    let client = Client::builder()
        .api_key("k")
        .sandbox()
        .user_agent("ci/1.0")
        .build()
        .unwrap();
    assert_eq!(
        client.base_url().as_str(),
        "https://sandbox.assinafy.com.br/v1/"
    );
    assert!(matches!(client.auth(), Auth::ApiKey(k) if k == "k"));
}

#[test]
fn client_from_api_key_defaults_to_production() {
    let client = Client::from_api_key("k").unwrap();
    assert_eq!(
        client.base_url().as_str(),
        "https://api.assinafy.com.br/v1/"
    );
    assert!(matches!(client.auth(), Auth::ApiKey(k) if k == "k"));
}

#[test]
fn custom_http_clients_are_limited_to_unauthenticated_requests() {
    use std::time::Duration;
    let client = Client::builder()
        .connect_timeout(Duration::from_secs(1))
        .http_client(reqwest::Client::new())
        .build()
        .unwrap();
    assert!(matches!(client.auth(), Auth::None));

    assert!(
        Client::builder()
            .api_key("k")
            .http_client(reqwest::Client::new())
            .build()
            .is_err()
    );
}

#[test]
fn authenticated_custom_http_clients_require_explicit_opt_in() {
    let transport = reqwest::Client::builder()
        .referer(false)
        .redirect(reqwest::redirect::Policy::none())
        .build()
        .unwrap();
    let client = Client::builder()
        .api_key("k")
        .authenticated_http_client(transport)
        .build()
        .unwrap();
    assert!(matches!(client.auth(), Auth::ApiKey(key) if key == "k"));
}

#[tokio::test]
async fn adding_auth_later_cannot_bypass_custom_client_redirect_safety() {
    let client = Client::builder()
        .http_client(reqwest::Client::new())
        .build()
        .unwrap()
        .with_auth(Auth::ApiKey("k".into()));
    assert!(matches!(
        client.documents().statuses().await,
        Err(assinafy::Error::Config(_))
    ));
}

#[tokio::test]
async fn resource_ids_cannot_change_the_request_route() {
    let client = Client::builder().build().unwrap();
    assert!(matches!(
        client.account("victim/logo").delete().await,
        Err(assinafy::Error::Config(_))
    ));
}

#[test]
fn client_with_auth_swaps_credential() {
    let client = Client::builder().api_key("k").build().unwrap();
    let bumped = client.with_auth(Auth::Bearer("token".into()));
    assert!(matches!(bumped.auth(), Auth::Bearer(t) if t == "token"));
    // Original is unchanged.
    assert!(matches!(client.auth(), Auth::ApiKey(_)));
}

#[test]
fn auth_debug_redacts_credentials() {
    let rendered = format!(
        "{:?} {:?} {:?} {:?}",
        Auth::ApiKey("secret".into()),
        Auth::Bearer("token".into()),
        Auth::AccessToken("query-token".into()),
        Auth::AccessCode("access".into())
    );
    assert!(!rendered.contains("secret"));
    assert!(!rendered.contains("token"));
    assert!(!rendered.contains("query-token"));
    assert!(!rendered.contains("access"));
    assert!(rendered.contains("redacted"));
}

#[test]
fn response_debug_redacts_bearer_and_signing_urls() {
    let login: assinafy::models::LoginResult = serde_json::from_value(serde_json::json!({
        "access_token": "bearer-secret",
        "user": {
            "id": "user-1",
            "name": "Example User",
            "email": "user@example.invalid",
            "created_at": "2026-08-20T00:00:00Z"
        },
        "accounts": []
    }))
    .unwrap();
    let signing_url: assinafy::models::SigningUrl = serde_json::from_value(serde_json::json!({
        "signer_id": "signer-1",
        "url": "https://sign.example.invalid/private-token"
    }))
    .unwrap();

    let rendered = format!("{login:?} {signing_url:?}");
    assert!(!rendered.contains("bearer-secret"));
    assert!(!rendered.contains("private-token"));
    assert!(rendered.contains("redacted"));
}

#[test]
fn api_key_response_accepts_missing_key() {
    let response: ApiKeyResponse = serde_json::from_str(r#"{"api_key":null}"#).unwrap();
    assert_eq!(response.api_key, None);
}

#[test]
fn api_key_response_accepts_null_envelope_data() {
    let envelope: Envelope<Option<ApiKeyResponse>> =
        serde_json::from_str(r#"{"status":200,"message":"","data":null}"#).unwrap();
    let response = envelope.data.unwrap_or_default();
    assert_eq!(response.api_key, None);
}

#[test]
fn documented_public_send_token_body_shape() {
    let body = serde_json::to_value(SendTokenBody::email("user@example.invalid")).unwrap();
    assert_eq!(
        body,
        serde_json::json!({
            "email": "user@example.invalid"
        })
    );
}

#[test]
fn verify_code_body_uses_documented_field_names() {
    let body = serde_json::to_value(VerifyCodeBody::new("123456")).unwrap();
    assert_eq!(body, serde_json::json!({ "verification-code": "123456" }));
}

#[test]
fn verify_code_body_can_emit_legacy_access_code() {
    let body =
        serde_json::to_value(VerifyCodeBody::new("123456").access_code("signer-code")).unwrap();
    assert_eq!(
        body,
        serde_json::json!({
            "verification-code": "123456",
            "signer-access-code": "signer-code"
        })
    );
}

#[test]
fn assignment_create_body_uses_current_signers_shape() {
    let body =
        CreateAssignmentBody::new(AssignmentMethod::Virtual, ["s1", "s2"]).copy_receivers(["c1"]);
    let json = serde_json::to_value(body).unwrap();
    assert_eq!(
        json["signers"],
        serde_json::json!([{ "id": "s1" }, { "id": "s2" }])
    );
    assert_eq!(json["copy_receivers"], serde_json::json!(["c1"]));
    assert!(json.get("signer_ids").is_none());
    assert!(json.get("signerIds").is_none());
}

#[test]
fn assignment_body_can_still_emit_legacy_signer_ids() {
    let body = CreateAssignmentBody::from_signers(AssignmentMethod::Virtual, [])
        .legacy_signer_ids(["s1", "s2"]);
    let json = serde_json::to_value(body).unwrap();
    assert_eq!(json["signer_ids"], serde_json::json!(["s1", "s2"]));
    assert!(json.get("signers").is_none());
}

#[test]
fn assignment_signer_builder_sets_method_channels_and_step() {
    let signer = CreateAssignmentSigner::new("s1")
        .verification_method(VerificationMethod::Whatsapp)
        .notification_methods(vec![NotificationMethod::Whatsapp])
        .step(2);
    let json = serde_json::to_value(signer).unwrap();
    assert_eq!(
        json,
        serde_json::json!({
            "id": "s1",
            "verification_method": "Whatsapp",
            "notification_methods": ["Whatsapp"],
            "step": 2
        })
    );
}

#[test]
fn assignment_signer_default_omits_empty_id_for_estimates() {
    let json = serde_json::to_value(CreateAssignmentSigner::default()).unwrap();
    assert_eq!(json, serde_json::json!({}));
}

#[test]
fn template_document_signer_role_can_be_role_only_for_estimates() {
    let json = serde_json::to_value(TemplateDocumentSigner::role("role_123")).unwrap();
    assert_eq!(json, serde_json::json!({ "role_id": "role_123" }));
}

#[test]
fn update_tag_body_can_clear_color() {
    let json = serde_json::to_value(UpdateTagBody::new().clear_color()).unwrap();
    assert_eq!(json, serde_json::json!({ "color": null }));
}

#[test]
fn auth_request_builders_emit_documented_fields() {
    assert_eq!(
        serde_json::to_value(LoginBody::new("user@example.invalid", "secret")).unwrap(),
        serde_json::json!({ "email": "user@example.invalid", "password": "secret" })
    );
    assert_eq!(
        serde_json::to_value(RequestPasswordResetBody::new("user@example.invalid")).unwrap(),
        serde_json::json!({ "email": "user@example.invalid" })
    );
    assert_eq!(
        serde_json::to_value(SocialLoginBody::google("token", true)).unwrap(),
        serde_json::json!({
            "provider": "google",
            "token": "token",
            "has_accepted_terms": true
        })
    );
}

#[test]
fn signer_confirmation_builder_emits_documented_fields() {
    let json = serde_json::to_value(
        ConfirmSignerDataBody::new()
            .full_name("Bill Murray")
            .email("user@example.invalid")
            .government_id("12345678909"),
    )
    .unwrap();
    assert_eq!(
        json,
        serde_json::json!({
            "full_name": "Bill Murray",
            "email": "user@example.invalid",
            "government_id": "12345678909"
        })
    );
}

#[test]
fn signer_confirmation_builder_can_emit_legacy_extensions() {
    let json = serde_json::to_value(
        ConfirmSignerDataBody::new()
            .whatsapp("+5548999990000")
            .accepted_terms(true)
            .verification_code("123456"),
    )
    .unwrap();
    assert_eq!(
        json,
        serde_json::json!({
            "whatsapp_phone_number": "+5548999990000",
            "has_accepted_terms": true,
            "code": "123456"
        })
    );
}

#[test]
fn password_endpoint_email_result_decodes_documented_shape() {
    let envelope: Envelope<EmailResult> = serde_json::from_value(serde_json::json!({
        "status": 200,
        "message": "",
        "data": { "email": "user@example.invalid" }
    }))
    .unwrap();
    assert_eq!(envelope.data.email, "user@example.invalid");
}

#[test]
fn webhook_dispatch_preserves_resource_discriminator() {
    let dispatch: assinafy::models::WebhookDispatch = serde_json::from_value(serde_json::json!({
        "resource": "activity_dispatching_history",
        "id": "dispatch_1",
        "event": "document_ready"
    }))
    .unwrap();
    assert_eq!(
        dispatch.resource.as_deref(),
        Some("activity_dispatching_history")
    );
}

#[test]
fn template_default_document_tag_can_include_color() {
    let tag: assinafy::models::TemplateTagRef = serde_json::from_value(serde_json::json!({
        "id": "tag_1",
        "name": "Contracts",
        "color": "ff8800"
    }))
    .unwrap();
    assert_eq!(tag.color.as_deref(), Some("ff8800"));
}

#[test]
fn api_error_derived_deserialize_tolerates_route_not_found_shape() {
    // This exercises `ApiError`'s plain `#[derive(Deserialize)]` on a raw
    // route-not-found body (extra `name`/`code`, no `data` key) — relevant if
    // a caller deserializes an error body themselves. It does NOT exercise
    // the SDK's actual HTTP-error path: that path goes through the private
    // `map_error`, which instead preserves the *whole* body in `data` so
    // `name`/`code` survive (see `map_error_keeps_whole_body_for_route_miss_shape`
    // in src/http.rs's own inline tests). The two intentionally decode
    // differently for the same input.
    let body = r#"{"name":"Not Found","message":"Página não encontrada.","code":0,"status":404}"#;
    let err: assinafy::ApiError = serde_json::from_str(body).unwrap();
    assert_eq!(err.status, 404);
    assert_eq!(err.message, "Página não encontrada.");
    assert!(err.data.is_null());
}

#[test]
fn document_verification_decodes_string_counts_for_valid_and_invalid() {
    use assinafy::models::DocumentVerification;

    let invalid = r#"{"hash":"INVALID","id":null,"status":null,"page_count":null,
        "signer_count":null,"completed_count":null,"completed_at":null,
        "verified_at":"2026-06-05T20:53:51Z","is_valid":false,
        "message":"Document not signed or not found."}"#;
    let v: DocumentVerification = serde_json::from_str(invalid).unwrap();
    assert!(!v.is_valid);
    assert_eq!(v.hash.as_deref(), Some("INVALID"));
    assert_eq!(v.page_count, None);

    // The API reports page_count / signer_count as STRINGS.
    let valid = r#"{"hash":"FE32","id":"63ddb172","status":"certificated",
        "page_count":"1","signer_count":"1","completed_count":1,
        "completed_at":"2023-01-27T19:27:44Z","verified_at":"2023-01-27T19:27:46Z",
        "is_valid":true,"message":""}"#;
    let v: DocumentVerification = serde_json::from_str(valid).unwrap();
    assert!(v.is_valid);
    assert_eq!(v.page_count.as_deref(), Some("1"));
    assert_eq!(v.signer_count.as_deref(), Some("1"));
    assert_eq!(v.completed_count, Some(1));
}

#[test]
fn public_document_decodes_with_string_page_count() {
    use assinafy::models::PublicDocument;
    let body = r#"{"resource":"document","id":"doc1","name":"1.pdf","page_count":"1","created_by":"John Smith"}"#;
    let d: PublicDocument = serde_json::from_str(body).unwrap();
    assert_eq!(d.id, "doc1");
    assert_eq!(d.name, "1.pdf");
    assert_eq!(d.page_count.as_deref(), Some("1"));
    assert_eq!(d.created_by.as_deref(), Some("John Smith"));
}

#[test]
fn public_document_preserves_current_document_shape() {
    use assinafy::models::PublicDocument;

    let body = serde_json::json!({
        "resource": "document",
        "id": "document_1",
        "account_id": "account_1",
        "template_id": null,
        "name": "agreement.pdf",
        "status": "metadata_ready",
        "artifacts": { "original": "https://files.example.invalid/original.pdf" },
        "pages": [{
            "id": "page_1",
            "number": 1,
            "height": 1651,
            "width": 1275
        }],
        "assignment": null,
        "is_closed": false,
        "signing_url": "https://sign.example.invalid/document_1",
        "decline_reason": null,
        "declined_by": null,
        "tags": [{ "id": "tag_1", "name": "Agreements" }],
        "created_at": "2026-08-20T12:00:00Z",
        "updated_at": "2026-08-20T12:01:00Z"
    });
    let document: PublicDocument = serde_json::from_value(body).unwrap();

    assert_eq!(document.account_id.as_deref(), Some("account_1"));
    assert_eq!(
        document.status.as_ref().map(DocumentStatus::as_str),
        Some("metadata_ready")
    );
    assert_eq!(document.artifacts.len(), 1);
    assert_eq!(document.pages.len(), 1);
    assert_eq!(document.is_closed, Some(false));
    assert_eq!(document.tags.len(), 1);
    assert!(document.created_at.is_some());
    assert!(document.page_count.is_none());
}

#[test]
fn editor_field_serializes_documented_shape() {
    use assinafy::resources::EditorField;
    let json = serde_json::to_value(EditorField::new("field_1", "hello")).unwrap();
    assert_eq!(
        json,
        serde_json::json!({ "field_id": "field_1", "value": "hello" })
    );
}

#[test]
fn assignment_body_omits_undocumented_top_level_methods() {
    // Top-level verification_method/notification_methods were removed because
    // the server silently ignores them (only the per-signer fields work).
    let json =
        serde_json::to_value(CreateAssignmentBody::new(AssignmentMethod::Virtual, ["s1"])).unwrap();
    assert!(json.get("verification_method").is_none());
    assert!(json.get("notification_methods").is_none());
}

#[test]
fn account_decodes_list_and_by_id_shapes() {
    use assinafy::models::Account;
    // List shape: roles + is_delete_allowed, no colors.
    let list = r#"{"resource":"account","id":"acc1","name":"MT","notification_sender_type":"User","roles":["owner"],"is_delete_allowed":true,"created_at":"2026-05-12T18:05:11Z"}"#;
    let a: Account = serde_json::from_str(list).unwrap();
    assert_eq!(a.id, "acc1");
    assert_eq!(a.roles, vec!["owner".to_string()]);
    assert!(a.is_delete_allowed);
    assert_eq!(a.primary_color, None);
    assert_eq!(
        a.notification_sender_type,
        Some(assinafy::models::NotificationSenderType::User)
    );

    // By-id shape: colors present (possibly null), no roles.
    let by_id = r#"{"id":"acc1","name":"MT","primary_color":"2072b9","secondary_color":null,"created_at":"2026-05-12T18:05:11Z"}"#;
    let a: Account = serde_json::from_str(by_id).unwrap();
    assert_eq!(a.primary_color.as_deref(), Some("2072b9"));
    assert_eq!(a.secondary_color, None);
    assert!(a.roles.is_empty());
}

#[test]
fn account_theme_decodes_live_shape() {
    use assinafy::models::AccountTheme;
    let body =
        r#"{"account_name":"MT","primary_color":"2072b9","secondary_color":"ffffff","logo":null}"#;
    let t: AccountTheme = serde_json::from_str(body).unwrap();
    assert_eq!(t.account_name.as_deref(), Some("MT"));
    assert_eq!(t.primary_color.as_deref(), Some("2072b9"));
    assert_eq!(t.logo, None);
}

#[test]
fn create_account_body_and_sender_type_serialize() {
    use assinafy::resources::{CreateAccountBody, NotificationSenderType};
    let json = serde_json::to_value(
        CreateAccountBody::new("Acme").notification_sender_type(NotificationSenderType::Account),
    )
    .unwrap();
    assert_eq!(
        json,
        serde_json::json!({ "name": "Acme", "notification_sender_type": "Account" })
    );
    // Omitted when unset.
    let json = serde_json::to_value(CreateAccountBody::new("Acme")).unwrap();
    assert_eq!(json, serde_json::json!({ "name": "Acme" }));
}

#[test]
fn signer_self_decodes_is_signature_reusable() {
    use assinafy::models::SignerSelf;
    let body = r#"{"resource":"signer","id":"s1","full_name":"Bill","email":"user@example.invalid",
        "whatsapp_phone_number":null,"has_accepted_terms":true,
        "has_signature":true,"has_initial":false,"is_signature_reusable":true}"#;
    let s: SignerSelf = serde_json::from_str(body).unwrap();
    assert!(s.has_signature);
    assert!(s.is_signature_reusable);
    // Older payloads without the flag default to false.
    let older = r#"{"id":"s1","full_name":"Bill","has_signature":false,"has_initial":false}"#;
    let s: SignerSelf = serde_json::from_str(older).unwrap();
    assert!(!s.is_signature_reusable);
}

#[test]
fn assignment_summary_signer_decodes_whatsapp() {
    use assinafy::models::AssignmentSummarySigner;
    let body = r#"{"id":"s1","full_name":"Ada","email":"user@example.invalid",
        "whatsapp_phone_number":"+5548999990000","has_accepted_terms":false,"completed":false}"#;
    let s: AssignmentSummarySigner = serde_json::from_str(body).unwrap();
    assert_eq!(s.whatsapp_phone_number.as_deref(), Some("+5548999990000"));
    assert!(!s.completed);
}

#[test]
fn resend_cost_estimate_decodes_live_shape() {
    use assinafy::models::ResendCostEstimate;
    let body = r#"{"total":0,"breakdown":[{"code":"NotificationEmailResend",
        "name":"Email Notification Resend","cost":0}],
        "credit_balance":0,"has_sufficient_credits":true}"#;
    let e: ResendCostEstimate = serde_json::from_str(body).unwrap();
    assert_eq!(e.total, 0.0);
    assert_eq!(e.breakdown.len(), 1);
    assert_eq!(e.breakdown[0].code, "NotificationEmailResend");
    assert!(e.has_sufficient_credits);
}

#[test]
fn confirm_data_body_emits_government_id() {
    let json = serde_json::to_value(
        ConfirmSignerDataBody::new()
            .full_name("Maria")
            .government_id("123.456.789-09"),
    )
    .unwrap();
    assert_eq!(
        json,
        serde_json::json!({ "full_name": "Maria", "government_id": "123.456.789-09" })
    );
}

#[test]
fn api_error_exposes_retry_after_field() {
    // The rate-limit path populates `retry_after`; it round-trips via serde and
    // is omitted when absent. `ApiError` is `#[non_exhaustive]`, so build it
    // via deserialization rather than a struct literal.
    let with: assinafy::ApiError = serde_json::from_str(
        r#"{"status":429,"message":"Too Many Requests","data":null,"retry_after":50}"#,
    )
    .unwrap();
    let json = serde_json::to_value(&with).unwrap();
    assert_eq!(json["retry_after"], 50);

    let without = r#"{"status":404,"message":"x","data":null}"#;
    let e: assinafy::ApiError = serde_json::from_str(without).unwrap();
    assert_eq!(e.retry_after, None);
}

#[test]
fn template_document_body_serializes_only_documented_fields() {
    use assinafy::resources::{
        CreateDocumentFromTemplateBody, EditorField, TemplateDocumentSigner,
    };
    let json = serde_json::to_value(
        CreateDocumentFromTemplateBody::default()
            .expires_at("2024-07-30T23:59:00Z")
            .signers(vec![TemplateDocumentSigner::existing("role_1", "signer_1")])
            .editor_fields(vec![EditorField::new("f1", "v1")])
            .tags(["Contracts"]),
    )
    .unwrap();
    assert_eq!(json["expires_at"], "2024-07-30T23:59:00Z");
    assert_eq!(
        json["editor_fields"],
        serde_json::json!([{ "field_id": "f1", "value": "v1" }])
    );
    assert_eq!(json["tags"], serde_json::json!(["Contracts"]));
    // Undocumented fields must not be emitted.
    assert!(json.get("expiration").is_none());
    assert!(json.get("tag_ids").is_none());
}

#[test]
fn cost_estimate_decodes_live_shape_and_breakdown() {
    use assinafy::models::{BlockingReason, CostEstimate};

    // Exact body returned live by POST /documents/{id}/assignments/estimate-cost.
    let live = r#"{"documents":1,"credits":0,"needs_extra_document":false,
        "extra_document_cost":0,"total_credits":0,"breakdown":[],
        "document_balance":42,"credit_balance":0,"has_sufficient_resources":true,
        "blocking_reason":null,"message":null}"#;
    let est: CostEstimate = serde_json::from_str(live).unwrap();
    assert_eq!(est.documents, 1.0);
    assert_eq!(est.document_balance, 42.0);
    assert!(est.has_sufficient_resources);
    assert!(est.blocking_reason.is_none());
    assert!(est.breakdown.is_empty());

    // A blocked estimate with a populated breakdown.
    let blocked = r#"{"documents":1,"credits":2,"needs_extra_document":true,
        "extra_document_cost":1.5,"total_credits":3.5,
        "breakdown":[{"code":"NotificationWhatsapp","name":"WhatsApp","cost":2.0,
                      "quantity":2.0,"unit_cost":1.0}],
        "document_balance":0,"credit_balance":0,"has_sufficient_resources":false,
        "blocking_reason":"InsufficientCredits","message":"Sem créditos"}"#;
    let est: CostEstimate = serde_json::from_str(blocked).unwrap();
    assert_eq!(est.breakdown.len(), 1);
    assert_eq!(est.breakdown[0].code, "NotificationWhatsapp");
    assert_eq!(est.breakdown[0].unit_cost, 1.0);
    assert_eq!(
        est.blocking_reason,
        Some(BlockingReason::InsufficientCredits)
    );

    // An unmodelled blocking reason degrades to `Other` instead of failing.
    let unknown = r#"{"blocking_reason":"SomethingNew"}"#;
    let est: CostEstimate = serde_json::from_str(unknown).unwrap();
    assert_eq!(
        est.blocking_reason,
        Some(BlockingReason::Other("SomethingNew".into()))
    );
}

#[test]
fn activity_decodes_hyphenated_and_underscored_user_agent() {
    use assinafy::models::Activity;

    // The API sends `user-agent`; the alias also accepts `user_agent`.
    let hyphenated = r#"{"id":19178,"event":"signature_requested","message":"Assinatura solicitada",
        "payload":{"id":19178},"origin":{"ip":"203.0.113.7","user-agent":"Mozilla/5.0"},
        "created_at":"2026-08-10T18:00:00Z"}"#;
    let activity: Activity = serde_json::from_str(hyphenated).unwrap();
    assert_eq!(activity.id, 19178);
    assert_eq!(activity.event, "signature_requested");
    assert_eq!(
        activity.origin.as_ref().unwrap().user_agent.as_deref(),
        Some("Mozilla/5.0")
    );

    let underscored = r#"{"id":1,"event":"e","origin":{"user_agent":"curl/8"},
        "created_at":"2026-08-10T18:00:00Z"}"#;
    let activity: Activity = serde_json::from_str(underscored).unwrap();
    assert_eq!(
        activity.origin.unwrap().user_agent.as_deref(),
        Some("curl/8")
    );
}

#[test]
fn self_user_decodes_live_users_self_shape() {
    use assinafy::models::SelfUser;

    // Exact body returned live by GET /users/self.
    let live = r#"{"user":{"id":"md3j6p9w8b7y6qvqaoy5er42","name":"Multica Test",
        "email":"user@example.invalid","telephone":null,"government_id":"",
        "is_email_verified":true,"has_accepted_terms":true,"is_password_set":true,
        "created_at":"2026-05-12T18:05:11Z","to_be_deleted_at":null},
        "accounts":[{"id":"acc_1234567890abcdef12345678","name":"MT","roles":["owner"],
        "is_delete_allowed":true,"created_at":"2026-05-12T18:05:11Z"}]}"#;
    let me: SelfUser = serde_json::from_str(live).unwrap();
    assert_eq!(me.user.email, "user@example.invalid");
    assert!(me.user.is_password_set);
    assert_eq!(me.accounts.len(), 1);
    assert_eq!(me.accounts[0].roles, vec!["owner"]);
}

#[test]
fn auth_change_reset_and_link_bodies_emit_documented_fields() {
    use assinafy::resources::{ChangePasswordBody, LinkSocialLoginBody, ResetPasswordBody};

    let change = serde_json::to_value(ChangePasswordBody::new(
        "user@example.invalid",
        "old-pw",
        "new-pw",
    ))
    .unwrap();
    // `current_password` is sent on the wire as `password`.
    assert_eq!(
        change,
        serde_json::json!({
            "email": "user@example.invalid",
            "password": "old-pw",
            "new_password": "new-pw"
        })
    );

    // `token` is omitted entirely until set.
    let reset =
        serde_json::to_value(ResetPasswordBody::new("user@example.invalid", "new-pw")).unwrap();
    assert!(reset.get("token").is_none());
    let reset = serde_json::to_value(
        ResetPasswordBody::new("user@example.invalid", "new-pw").token("tok-123"),
    )
    .unwrap();
    assert_eq!(reset["token"], "tok-123");
    assert_eq!(reset["new_password"], "new-pw");

    let link = serde_json::to_value(LinkSocialLoginBody::new("google", "id-token")).unwrap();
    assert_eq!(link["provider"], "google");
}
