//! Pure unit tests for the SDK that do not require network access.

use assinafy::models::{
    ArtifactName, AssignmentMethod, DocumentStatus, NotificationMethod, TemplateStatus,
    VerificationMethod,
};
use assinafy::resources::{
    ApiKeyResponse, ConfirmSignerDataBody, CreateAssignmentBody, CreateAssignmentSigner, LoginBody,
    RequestPasswordResetBody, SendTokenBody, SocialLoginBody, TemplateDocumentSigner,
    UpdateTagBody, VerifyCodeBody,
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
    assert_eq!(NotificationMethod::Whatsapp.as_str(), "Whatsapp");
    let v: VerificationMethod = serde_json::from_str("\"Email\"").unwrap();
    assert_eq!(v, VerificationMethod::Email);
}

#[test]
fn artifact_name_round_trip() {
    assert_eq!(ArtifactName::from("original"), ArtifactName::Original);
    assert_eq!(
        ArtifactName::from("custom-thing"),
        ArtifactName::Other("custom-thing".into())
    );
    assert_eq!(ArtifactName::Certificated.as_str(), "certificated");
}

#[test]
fn envelope_decodes_signer_payload() {
    let body = r#"{"status":200,"message":"","data":{"resource":"signer","id":"abc","full_name":"Bill","email":"b@e.com","whatsapp_phone_number":null,"has_accepted_terms":false}}"#;
    let env: Envelope<assinafy::models::Signer> = serde_json::from_str(body).unwrap();
    assert_eq!(env.status, 200);
    assert_eq!(env.data.full_name, "Bill");
    assert_eq!(env.data.email.as_deref(), Some("b@e.com"));
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
fn api_key_response_accepts_missing_key() {
    let response: ApiKeyResponse = serde_json::from_str(r#"{"api_key":null}"#).unwrap();
    assert_eq!(response.api_key, None);
}

#[test]
fn documented_public_send_token_body_shape() {
    let body = serde_json::to_value(SendTokenBody::email("bill@example.com")).unwrap();
    assert_eq!(
        body,
        serde_json::json!({
            "recipient": "bill@example.com",
            "channel": "email"
        })
    );
}

#[test]
fn verify_code_body_uses_documented_field_names() {
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
        serde_json::to_value(LoginBody::new("bill@example.com", "secret")).unwrap(),
        serde_json::json!({ "email": "bill@example.com", "password": "secret" })
    );
    assert_eq!(
        serde_json::to_value(RequestPasswordResetBody::new("bill@example.com")).unwrap(),
        serde_json::json!({ "email": "bill@example.com" })
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
fn signer_confirmation_builder_can_set_all_documented_fields() {
    let json = serde_json::to_value(
        ConfirmSignerDataBody::new()
            .full_name("Bill Murray")
            .email("bill@example.com")
            .whatsapp("+5548999990000")
            .accepted_terms(true)
            .verification_code("123456"),
    )
    .unwrap();
    assert_eq!(
        json,
        serde_json::json!({
            "full_name": "Bill Murray",
            "email": "bill@example.com",
            "whatsapp_phone_number": "+5548999990000",
            "has_accepted_terms": true,
            "code": "123456"
        })
    );
}

#[test]
fn public_send_token_supports_whatsapp_channel() {
    let body = serde_json::to_value(SendTokenBody::whatsapp("+5548999990000")).unwrap();
    assert_eq!(
        body,
        serde_json::json!({
            "recipient": "+5548999990000",
            "channel": "whatsapp"
        })
    );
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
fn api_error_decodes_route_not_found_shape() {
    // The API returns two distinct 404 bodies: a resource-404 with a `data`
    // field, and this route-not-found shape (extra `name`/`code`, no `data`).
    // `ApiError::data` relies on `#[serde(default)]` to decode the latter.
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
