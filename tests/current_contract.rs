mod common;

use common::{mock, validation_client};

use assinafy::models::{AssignmentMethod, Document, DocumentVerification, SigningUrl};
use assinafy::resources::{
    AssignmentCreateOptions, CreateAssignmentBody, CreateDocumentFromTemplateBody,
    ListDocumentsRequest, SearchDocumentsRequest, TemplateDocumentOptions, TemplateDocumentSigner,
};
use assinafy::{Auth, Error};
use serde_json::json;

#[test]
fn closed_signer_and_embedded_url_responses_decode() {
    let document: Document = serde_json::from_value(json!({
        "id":"document", "name":"Contract.pdf", "status":"certificated",
        "current_signer":{"id":"signer","verification_method":"Email","notification_methods":null,"completed":true},
        "is_signer_access_closed":true,
        "certified_copy":{"available":true,"notification_method":"Email","resends_left":null},
        "envelope_id":"envelope", "envelope":{"id":"envelope","name":"Contract"}
    })).unwrap();
    assert!(document.current_signer.unwrap().full_name.is_empty());
    assert_eq!(document.is_signer_access_closed, Some(true));
    assert!(document.certified_copy.unwrap().available);
    assert_eq!(document.envelope.unwrap().id, "envelope");
    let url: SigningUrl =
        serde_json::from_value(json!({"signer_id":"signer","url":null,"embedded":true})).unwrap();
    assert!(url.url.is_empty());
    assert!(url.embedded);
    let verification: DocumentVerification =
        serde_json::from_value(json!({"is_valid":true,"certificated_at":"2026-11-01T00:00:00Z"}))
            .unwrap();
    assert_eq!(
        verification.certificated_at.as_deref(),
        Some("2026-11-01T00:00:00Z")
    );
}

#[tokio::test]
async fn document_filters_and_expansions_reach_the_wire() {
    let (client, server) = mock(json!([]));
    client
        .documents()
        .list(
            "account",
            ListDocumentsRequest::default().envelope_id("env & 1"),
        )
        .await
        .unwrap();
    assert_eq!(
        server.join().unwrap().url.query_pairs().collect::<Vec<_>>(),
        [("envelope_id".into(), "env & 1".into())]
    );
    let (client, server) = mock(json!([]));
    client
        .documents()
        .search(
            "account",
            SearchDocumentsRequest::new("contract").envelope_id("envelope"),
        )
        .await
        .unwrap();
    assert!(
        server
            .join()
            .unwrap()
            .url
            .query_pairs()
            .any(|(key, value)| key == "envelope_id" && value == "envelope")
    );
    let (client, server) =
        mock(json!({"id":"document","name":"Contract.pdf","status":"metadata_ready"}));
    client
        .documents()
        .get_with_expand("document", "envelope_id,envelope")
        .await
        .unwrap();
    assert_eq!(
        server.join().unwrap().url.query_pairs().collect::<Vec<_>>(),
        [("expand".into(), "envelope_id,envelope".into())]
    );
    let (client, server) = mock(json!({"document_id":"root","status":"active"}));
    client
        .contracts("account")
        .related_with_document("amendment")
        .await
        .unwrap();
    assert_eq!(
        server.join().unwrap().url.query_pairs().collect::<Vec<_>>(),
        [("expand".into(), "document".into())]
    );
}

#[tokio::test]
async fn assignment_options_preserve_default_body_and_add_spending_and_embed_controls() {
    let body = CreateAssignmentBody::new(AssignmentMethod::Virtual, ["signer"]);
    let original = serde_json::to_value(&body).unwrap();
    let (client, server) = mock(
        json!({"id":"assignment","signing_urls":[{"signer_id":"signer","url":null,"embedded":true}]}),
    );
    let result = client
        .assignments()
        .create_with_options(
            "document",
            &body,
            &AssignmentCreateOptions::default()
                .max_total_credits(1.5)
                .embedded_signer("signer"),
        )
        .await
        .unwrap();
    assert!(result.signing_urls[0].embedded);
    let request = server.join().unwrap();
    assert_eq!(request.method, "POST");
    assert_eq!(request.url.path(), "/v1/documents/document/assignments");
    assert_eq!(request.body["signers"][0]["embedded"], true);
    assert_eq!(request.body["max_total_credits"], 1.5);
    assert_eq!(serde_json::to_value(&body).unwrap(), original);
}

#[tokio::test]
async fn template_inline_signers_idempotency_and_ceiling_reach_the_wire() {
    let body =
        CreateDocumentFromTemplateBody::default().signers([TemplateDocumentSigner::from_email(
            "role",
            "user@example.invalid",
            "Maria Silva",
        )
        .whatsapp("+5500000000000")]);
    let (client, server) = mock(json!({"id":"document","name":"Contract.pdf","status":"uploaded"}));
    client
        .templates("account")
        .create_document_with_options(
            "template",
            &body,
            &TemplateDocumentOptions::default()
                .max_total_credits(2.0)
                .idempotency_key("retry-key")
                .embedded_role("role"),
        )
        .await
        .unwrap();
    let request = server.join().unwrap();
    assert_eq!(
        request.headers.get("idempotency-key").map(String::as_str),
        Some("retry-key")
    );
    assert_eq!(request.body["signers"][0]["name"], "Maria Silva");
    assert!(request.body["signers"][0].get("full_name").is_none());
    assert_eq!(request.body["signers"][0]["embedded"], true);
    assert_eq!(request.body["max_total_credits"], 2.0);
}

#[tokio::test]
async fn creation_controls_reject_invalid_ceilings_and_unknown_signers_before_sending() {
    let client = validation_client().with_auth(Auth::ApiKey("example-redacted-key".into()));
    let assignment = CreateAssignmentBody::new(AssignmentMethod::Virtual, ["signer"]);
    let template = CreateDocumentFromTemplateBody::default()
        .signers([TemplateDocumentSigner::existing("role", "signer")]);
    for ceiling in [f64::NAN, f64::INFINITY, -0.1] {
        assert!(matches!(
            client
                .assignments()
                .create_with_options(
                    "document",
                    &assignment,
                    &AssignmentCreateOptions::default().max_total_credits(ceiling)
                )
                .await,
            Err(Error::Validation(_))
        ));
        assert!(matches!(
            client
                .templates("account")
                .create_document_with_options(
                    "template",
                    &template,
                    &TemplateDocumentOptions::default().max_total_credits(ceiling)
                )
                .await,
            Err(Error::Validation(_))
        ));
    }
    assert!(matches!(
        client
            .assignments()
            .create_with_options(
                "document",
                &assignment,
                &AssignmentCreateOptions::default().embedded_signer("missing")
            )
            .await,
        Err(Error::Validation(_))
    ));
    assert!(matches!(
        client
            .templates("account")
            .create_document_with_options(
                "template",
                &template,
                &TemplateDocumentOptions::default().embedded_role("missing")
            )
            .await,
        Err(Error::Validation(_))
    ));
    assert!(matches!(
        client
            .assignments()
            .reject("document", "assignment", "x".repeat(2001))
            .await,
        Err(Error::Validation(_))
    ));
}

#[tokio::test]
async fn default_creation_options_preserve_existing_requests() {
    let assignment = CreateAssignmentBody::new(AssignmentMethod::Virtual, ["signer"]);
    let template = CreateDocumentFromTemplateBody::default()
        .signers([TemplateDocumentSigner::existing("role", "signer")]);
    for with_options in [false, true] {
        let (client, server) = mock(json!({"id":"assignment"}));
        let api = client.assignments();
        if with_options {
            api.create_with_options("document", &assignment, &AssignmentCreateOptions::default())
                .await
                .unwrap();
        } else {
            api.create("document", &assignment).await.unwrap();
        }
        let request = server.join().unwrap();
        assert_eq!(request.method, "POST");
        assert_eq!(request.url.path(), "/v1/documents/document/assignments");
        assert!(request.url.query().is_none());
        assert_eq!(request.body, serde_json::to_value(&assignment).unwrap());
        assert!(!request.headers.contains_key("idempotency-key"));

        let (client, server) =
            mock(json!({"id":"document","name":"Contract.pdf","status":"uploaded"}));
        let api = client.templates("account");
        if with_options {
            api.create_document_with_options(
                "template",
                &template,
                &TemplateDocumentOptions::default(),
            )
            .await
            .unwrap();
        } else {
            api.create_document("template", &template).await.unwrap();
        }
        let request = server.join().unwrap();
        assert_eq!(request.method, "POST");
        assert_eq!(
            request.url.path(),
            "/v1/accounts/account/templates/template/documents"
        );
        assert!(request.url.query().is_none());
        assert_eq!(request.body, serde_json::to_value(&template).unwrap());
        assert!(!request.headers.contains_key("idempotency-key"));
    }
}
