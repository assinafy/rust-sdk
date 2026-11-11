use std::collections::HashMap;
use std::io::{BufRead, BufReader, Read, Write};
use std::net::TcpListener;
use std::thread::{self, JoinHandle};
use std::time::Duration;

use assinafy::models::{
    ArtifactName, AssignmentMethod, DocumentEnvelope, EmbedSessionExchange, VerificationMethod,
};
use assinafy::resources::{
    AddEnvelopeDocumentBody, AssignmentEntry, AssignmentField, CreateEnvelopeBody, EmbedOriginBody,
    EnvelopeDocumentBody, EnvelopeSignerConfig, ExchangeEmbedSessionBody, UpdateEnvelopeBody,
};
use assinafy::{Auth, BaseUrl, Client, Error};
use serde_json::{Value, json};

struct Request {
    method: String,
    url: url::Url,
    headers: HashMap<String, String>,
    body: Value,
}

fn mock(data: Value) -> (Client, JoinHandle<Request>) {
    mock_response(
        "200 OK",
        "",
        json!({"status":200,"message":"","data":data})
            .to_string()
            .into_bytes(),
    )
}

fn mock_response(
    status: &'static str,
    extra_headers: &str,
    response: Vec<u8>,
) -> (Client, JoinHandle<Request>) {
    let listener = TcpListener::bind("127.0.0.1:0").unwrap();
    let address = listener.local_addr().unwrap();
    let extra_headers = extra_headers.to_owned();
    let server = thread::spawn(move || {
        let (mut stream, _) = listener.accept().unwrap();
        stream
            .set_read_timeout(Some(Duration::from_secs(5)))
            .unwrap();
        let mut reader = BufReader::new(&stream);
        let mut line = String::new();
        reader.read_line(&mut line).unwrap();
        let mut parts = line.split_whitespace();
        let method = parts.next().unwrap().to_owned();
        let url = url::Url::parse(&format!("http://{address}{}", parts.next().unwrap())).unwrap();
        let mut headers = HashMap::new();
        loop {
            let mut line = String::new();
            reader.read_line(&mut line).unwrap();
            if line == "\r\n" {
                break;
            }
            let (name, value) = line.split_once(':').unwrap();
            headers.insert(name.to_ascii_lowercase(), value.trim().to_owned());
        }
        let len = headers
            .get("content-length")
            .map_or(0, |s| s.parse().unwrap());
        let mut bytes = vec![0; len];
        reader.read_exact(&mut bytes).unwrap();
        let body = if bytes.is_empty() {
            Value::Null
        } else {
            serde_json::from_slice(&bytes).unwrap()
        };
        write!(stream, "HTTP/1.1 {status}\r\nContent-Type: application/json\r\n{extra_headers}Content-Length: {}\r\nConnection: close\r\n\r\n", response.len()).unwrap();
        stream.write_all(&response).unwrap();
        Request {
            method,
            url,
            headers,
            body,
        }
    });
    let client = Client::builder()
        .base_url(BaseUrl::custom(format!("http://{address}/v1")).unwrap())
        .api_key("sentinel-workspace-key")
        .timeout(Duration::from_secs(5))
        .build()
        .unwrap();
    (client, server)
}

fn envelope() -> Value {
    json!({"resource":"envelope","id":"envelope","name":"Contract","message":null,"documents":[],"counts":null,"signing_urls":[],"created_at":"2026-11-04T12:00:00Z"})
}

fn create_body() -> CreateEnvelopeBody {
    CreateEnvelopeBody::new(
        "Contract",
        vec![EnvelopeSignerConfig::new("signer")],
        vec![EnvelopeDocumentBody::new("document", ["signer"])],
    )
}

fn assert_request(request: &Request, method: &str, path: &str) {
    assert_eq!(request.method, method);
    assert_eq!(request.url.path(), path);
    assert_eq!(
        request.headers.get("x-api-key").map(String::as_str),
        Some("sentinel-workspace-key")
    );
    assert!(!request.headers.contains_key("authorization"));
}

#[tokio::test]
async fn envelope_creation_estimate_and_nullable_update_have_exact_payloads() {
    let (client, server) = mock(envelope());
    client
        .envelopes("account")
        .create(create_body().max_total_credits(1.5))
        .idempotency_key("retry-the-same-envelope")
        .send()
        .await
        .unwrap();
    let request = server.join().unwrap();
    assert_request(&request, "POST", "/v1/accounts/account/envelopes");
    assert_eq!(
        request.headers["idempotency-key"],
        "retry-the-same-envelope"
    );
    assert_eq!(
        request.body,
        json!({"name":"Contract","signers":[{"id":"signer"}],"documents":[{"id":"document","signer_ids":["signer"]}],"max_total_credits":1.5})
    );

    let (client, server) = mock(
        json!({"documents":1,"credits":0,"needs_extra_document":false,"extra_document_cost":1,"total_credits":0,"breakdown":[],"document_balance":null,"credit_balance":null,"has_sufficient_resources":true,"blocking_reason":null,"message":null}),
    );
    client
        .envelopes("account")
        .estimate_cost(&create_body().max_total_credits(f64::NAN))
        .await
        .unwrap();
    let request = server.join().unwrap();
    assert_request(
        &request,
        "POST",
        "/v1/accounts/account/envelopes/estimate-cost",
    );
    assert!(request.body.get("max_total_credits").is_none());
    assert_eq!(request.body["signers"], json!([{"id":"signer"}]));

    let (client, server) = mock(envelope());
    client
        .envelopes("account")
        .update(
            "envelope",
            &UpdateEnvelopeBody::default()
                .clear_message()
                .document_ids(["document"]),
        )
        .await
        .unwrap();
    let request = server.join().unwrap();
    assert_request(&request, "PATCH", "/v1/accounts/account/envelopes/envelope");
    assert_eq!(
        request.body,
        json!({"message":null,"document_ids":["document"]})
    );
    let clear: UpdateEnvelopeBody = serde_json::from_value(json!({"message":null})).unwrap();
    assert_eq!(
        serde_json::to_value(clear).unwrap(),
        json!({"message":null})
    );
    assert_eq!(
        serde_json::to_value(UpdateEnvelopeBody::default()).unwrap(),
        json!({})
    );
    assert_eq!(
        serde_json::to_value(UpdateEnvelopeBody::default().message("Please sign")).unwrap(),
        json!({"message":"Please sign"})
    );
}

#[tokio::test]
async fn envelope_listing_and_activity_queries_preserve_pagination() {
    let (client, server) = mock_response(
        "200 OK",
        "X-Pagination-Current-Page: 2\r\nX-Pagination-Page-Count: 3\r\n",
        json!({"status":200,"message":"","data":[envelope()]})
            .to_string()
            .into_bytes(),
    );
    let page = client
        .envelopes("account")
        .list()
        .search("Contract & annex")
        .status("open")
        .page(2)
        .per_page(7)
        .send()
        .await
        .unwrap();
    assert_eq!(page.next_page(), Some(3));
    let request = server.join().unwrap();
    assert_request(&request, "GET", "/v1/accounts/account/envelopes");
    assert_eq!(
        request
            .url
            .query_pairs()
            .into_owned()
            .collect::<HashMap<_, _>>(),
        HashMap::from([
            ("search".into(), "Contract & annex".into()),
            ("status".into(), "open".into()),
            ("page".into(), "2".into()),
            ("per-page".into(), "7".into())
        ])
    );

    let (client, server) = mock(
        json!([{"id":1,"event":"document_uploaded","message":"Uploaded","payload":[],"origin":null,"created_at":"2026-11-04T12:00:00Z","document_id":"document","document_name":"Contract.pdf"}]),
    );
    let page = client
        .envelopes("account")
        .activities("envelope")
        .sort("id")
        .page(3)
        .per_page(4)
        .send()
        .await
        .unwrap();
    assert_eq!(page.data[0].activity.event, "document_uploaded");
    assert_eq!(page.data[0].document_id, "document");
    let request = server.join().unwrap();
    assert_request(
        &request,
        "GET",
        "/v1/accounts/account/envelopes/envelope/activities",
    );
    assert_eq!(
        request
            .url
            .query_pairs()
            .into_owned()
            .collect::<HashMap<_, _>>(),
        HashMap::from([
            ("sort".into(), "id".into()),
            ("page".into(), "3".into()),
            ("per-page".into(), "4".into())
        ])
    );
}

#[tokio::test]
async fn envelope_reads_lifecycle_downloads_and_document_mutations_use_exact_routes() {
    let (client, server) = mock(envelope());
    client.envelopes("account").get("envelope").await.unwrap();
    assert_request(
        &server.join().unwrap(),
        "GET",
        "/v1/accounts/account/envelopes/envelope",
    );

    let (client, server) = mock(json!([]));
    client
        .envelopes("account")
        .delete("envelope")
        .await
        .unwrap();
    assert_request(
        &server.join().unwrap(),
        "DELETE",
        "/v1/accounts/account/envelopes/envelope",
    );

    let (client, server) =
        mock(json!([{"id":"document","status":"future_status","cancelled":false}]));
    let result = client
        .envelopes("account")
        .cancel("envelope")
        .await
        .unwrap();
    assert_eq!(result[0].status.as_str(), "future_status");
    assert!(!result[0].cancelled);
    assert_request(
        &server.join().unwrap(),
        "PUT",
        "/v1/accounts/account/envelopes/envelope/cancel",
    );

    let (client, server) =
        mock(json!({"is_sent":false,"envelope_id":"envelope","signer_id":"signer"}));
    assert!(
        !client
            .envelopes("account")
            .resend("envelope", "signer")
            .await
            .unwrap()
            .is_sent
    );
    assert_request(
        &server.join().unwrap(),
        "PUT",
        "/v1/accounts/account/envelopes/envelope/signers/signer/resend",
    );

    let (client, server) = mock(envelope());
    client
        .envelopes("account")
        .add_document(
            "envelope",
            &AddEnvelopeDocumentBody::new(
                EnvelopeDocumentBody::new("document", ["signer"])
                    .expires_at(1800000000)
                    .copy_receivers(["copy"]),
            )
            .max_total_credits(0.0),
        )
        .await
        .unwrap();
    let request = server.join().unwrap();
    assert_request(
        &request,
        "POST",
        "/v1/accounts/account/envelopes/envelope/documents",
    );
    assert_eq!(
        request.body,
        json!({"id":"document","signer_ids":["signer"],"expires_at":1800000000,"copy_receiver_ids":["copy"],"max_total_credits":0.0})
    );

    let (client, server) = mock(envelope());
    client
        .envelopes("account")
        .remove_document("envelope", "document")
        .await
        .unwrap();
    assert_request(
        &server.join().unwrap(),
        "DELETE",
        "/v1/accounts/account/envelopes/envelope/documents/document",
    );

    for artifact in [
        ArtifactName::Original,
        ArtifactName::Certificated,
        ArtifactName::CertificatePage,
        ArtifactName::Pades,
        ArtifactName::Bundle,
    ] {
        let (client, server) = mock_response("200 OK", "", b"ZIP".to_vec());
        let (bytes, _) = client
            .envelopes("account")
            .download_artifact("envelope", artifact.clone())
            .await
            .unwrap();
        assert_eq!(&bytes[..], b"ZIP");
        assert_request(
            &server.join().unwrap(),
            "GET",
            &format!(
                "/v1/accounts/account/envelopes/envelope/download/{}",
                artifact.as_str()
            ),
        );
    }
}

#[tokio::test]
async fn embed_origin_crud_and_session_creation_use_workspace_credentials() {
    let origin = json!({"id":"origin","origin":"https://app.example.invalid","created_at":"2026-11-04T12:00:00Z"});
    let (client, server) = mock(json!([origin.clone()]));
    assert_eq!(
        client
            .embedded_signing()
            .list_origins("account")
            .await
            .unwrap()
            .len(),
        1
    );
    assert_request(
        &server.join().unwrap(),
        "GET",
        "/v1/accounts/account/embed-origins",
    );

    let (client, server) = mock(origin.clone());
    client
        .embedded_signing()
        .create_origin(
            "account",
            &EmbedOriginBody::new("https://app.example.invalid"),
        )
        .await
        .unwrap();
    let request = server.join().unwrap();
    assert_request(&request, "POST", "/v1/accounts/account/embed-origins");
    assert_eq!(
        request.body,
        json!({"origin":"https://app.example.invalid"})
    );

    let (client, server) = mock(origin);
    client
        .embedded_signing()
        .update_origin(
            "account",
            "origin",
            &EmbedOriginBody::new("https://new.example.invalid"),
        )
        .await
        .unwrap();
    let request = server.join().unwrap();
    assert_request(&request, "PUT", "/v1/accounts/account/embed-origins/origin");
    assert_eq!(
        request.body,
        json!({"origin":"https://new.example.invalid"})
    );

    let (client, server) = mock(json!([]));
    client
        .embedded_signing()
        .delete_origin("account", "origin")
        .await
        .unwrap();
    assert_request(
        &server.join().unwrap(),
        "DELETE",
        "/v1/accounts/account/embed-origins/origin",
    );

    let secret_url = "https://app.example.invalid/embed/sentinel-embed-token";
    let (client, server) = mock(json!({"url":secret_url,"expires_at":"2026-11-04T12:05:00Z"}));
    let session = client
        .embedded_signing()
        .create_session("document", "assignment", "signer")
        .await
        .unwrap();
    assert_eq!(session.url, secret_url);
    assert!(!format!("{session:?}").contains("sentinel-embed-token"));
    assert_eq!(serde_json::to_value(&session).unwrap()["url"], secret_url);
    assert_request(
        &server.join().unwrap(),
        "POST",
        "/v1/documents/document/assignments/assignment/signers/signer/embed-sessions",
    );
}

#[tokio::test]
async fn embed_exchange_is_public_redacted_and_preserves_unknown_verification_methods() {
    for auth in [
        Auth::ApiKey("sentinel-key".into()),
        Auth::Bearer("sentinel-bearer".into()),
        Auth::AccessCode("sentinel-code".into()),
        Auth::AccessToken("sentinel-token".into()),
    ] {
        let (client, server) = mock(
            json!({"signer_access_code":"sentinel-exchanged-code","signer_id":"signer","document_id":"document","verification_method":"FutureMethod","embed_origins":["https://app.example.invalid"]}),
        );
        let exchanged = client
            .with_auth(auth)
            .embedded_signing()
            .exchange_session(
                "token",
                Some(
                    &ExchangeEmbedSessionBody::default()
                        .parent_origin("https://app.example.invalid"),
                ),
            )
            .await
            .unwrap();
        assert_eq!(exchanged.signer_access_code, "sentinel-exchanged-code");
        assert_eq!(exchanged.verification_method.as_str(), "FutureMethod");
        assert!(!format!("{exchanged:?}").contains("sentinel-exchanged-code"));
        let request = server.join().unwrap();
        assert_eq!(request.method, "POST");
        assert_eq!(request.url.path(), "/v1/embed/sessions/token/exchange");
        assert!(request.url.query().is_none());
        assert!(!request.headers.contains_key("x-api-key"));
        assert!(!request.headers.contains_key("authorization"));
        assert_eq!(
            request.body,
            json!({"parent_origin":"https://app.example.invalid"})
        );
    }
}

#[tokio::test]
async fn exchange_ignores_custom_transport_credentials_and_redirect_policy() {
    let (base_client, server) = mock_response(
        "307 Temporary Redirect",
        "Location: http://127.0.0.1:1/leak\r\n",
        Vec::new(),
    );
    let mut defaults = reqwest::header::HeaderMap::new();
    defaults.insert("x-api-key", "sentinel-custom-default".parse().unwrap());
    let custom = reqwest::Client::builder()
        .default_headers(defaults)
        .build()
        .unwrap();
    let client = Client::builder()
        .base_url(BaseUrl::custom(base_client.base_url().as_str()).unwrap())
        .http_client(custom)
        .build()
        .unwrap();
    let error = client
        .embedded_signing()
        .exchange_session("token", None)
        .await
        .unwrap_err();
    assert_eq!(error.status(), Some(307));
    let request = server.join().unwrap();
    assert!(!request.headers.contains_key("x-api-key"));
    assert_eq!(request.body, Value::Null);
}

#[tokio::test]
async fn envelope_validation_rejects_invalid_work_before_network() {
    let client = Client::from_api_key("example-redacted-key").unwrap();
    for ceiling in [-1.0, f64::NAN, f64::INFINITY] {
        assert!(matches!(
            client
                .envelopes("account")
                .create(create_body().max_total_credits(ceiling))
                .send()
                .await,
            Err(Error::Validation(_))
        ));
    }
    let mut invalid = create_body();
    invalid.signers.push(EnvelopeSignerConfig::new("unused"));
    assert!(matches!(
        client.envelopes("account").create(invalid).send().await,
        Err(Error::Validation(_))
    ));
    let certificate = EnvelopeSignerConfig::new("signer")
        .verification_method(VerificationMethod::DigitalCertificate);
    let mut invalid = create_body();
    invalid.signers = vec![certificate];
    invalid.documents[0].method = Some(AssignmentMethod::Collect);
    invalid.documents[0].entries = Some(vec![AssignmentEntry::new(
        "page",
        vec![
            AssignmentField::new("signer", "field")
                .display_settings(json!({"left":10,"top":10,"width":100,"height":30})),
        ],
    )]);
    assert!(matches!(
        client.envelopes("account").create(invalid).send().await,
        Err(Error::Validation(_))
    ));
    let mut invalid = create_body();
    invalid.documents[0].method = Some(AssignmentMethod::Collect);
    assert!(matches!(
        client.envelopes("account").create(invalid).send().await,
        Err(Error::Validation(_))
    ));
    let mut invalid = create_body();
    invalid.signers[0] = EnvelopeSignerConfig::new("signer")
        .verification_method(VerificationMethod::id_document_face_match_basic())
        .embedded(true);
    assert!(matches!(
        client.envelopes("account").create(invalid).send().await,
        Err(Error::Validation(_))
    ));
    assert!(matches!(
        client
            .envelopes("account")
            .download_artifact("envelope", ArtifactName::Thumbnail)
            .await,
        Err(Error::Validation(_))
    ));
    assert!(matches!(
        client.envelopes("../account").get("envelope").await,
        Err(Error::Config(_))
    ));
    assert!(matches!(
        client
            .embedded_signing()
            .exchange_session("../token", None)
            .await,
        Err(Error::Config(_))
    ));
    assert!(matches!(
        client
            .with_auth(Auth::AccessCode("code".into()))
            .embedded_signing()
            .create_session("document", "assignment", "signer")
            .await,
        Err(Error::Config(_))
    ));
    assert!(matches!(
        client
            .with_auth(Auth::AccessCode("code".into()))
            .envelopes("account")
            .get("envelope")
            .await,
        Err(Error::Config(_))
    ));
}

#[test]
fn envelope_urls_redact_in_nested_debug_and_nullable_signing_links_decode() {
    let mut value = envelope();
    value["documents"] = json!([{"id":"document","position":1,"name":"Contract.pdf","status":"future_status","artifacts":{"original":"https://example.invalid/sentinel-download"},"signers":[{"id":"signer","full_name":"Maria Silva","email":null,"whatsapp_phone_number":null,"completed":false,"signed_at":null}]}]);
    value["signing_urls"] = json!([{"signer_id":"signer","url":null,"embedded":true}]);
    let decoded: DocumentEnvelope = serde_json::from_value(value).unwrap();
    assert_eq!(decoded.documents[0].status.as_str(), "future_status");
    assert!(!format!("{decoded:?}").contains("sentinel-download"));
    assert_eq!(
        serde_json::to_value(decoded).unwrap()["documents"][0]["artifacts"]["original"],
        "https://example.invalid/sentinel-download"
    );
    let result: EmbedSessionExchange = serde_json::from_value(json!({"signer_access_code":"sentinel-code","signer_id":"signer","document_id":"document","verification_method":"Email","embed_origins":[]})).unwrap();
    assert_eq!(
        serde_json::to_value(result).unwrap()["signer_access_code"],
        "sentinel-code"
    );
}

#[tokio::test]
async fn envelope_collect_payload_reuses_complete_field_placements() {
    let field = AssignmentField::new("signer", "field")
        .display_settings(json!({"left":10,"top":20,"width":100,"height":30,"fontSize":12}));
    let document = EnvelopeDocumentBody::new("document", ["signer"])
        .method(AssignmentMethod::Collect)
        .entries(vec![AssignmentEntry::new("page", vec![field])])
        .expires_at("2026-12-31T23:59:59Z");
    let (client, server) = mock(envelope());
    client
        .envelopes("account")
        .create(CreateEnvelopeBody::new(
            "Contract",
            vec![EnvelopeSignerConfig::new("signer").embedded(true)],
            vec![document.clone()],
        ))
        .send()
        .await
        .unwrap();
    let request = server.join().unwrap();
    assert_eq!(
        request.body["documents"][0],
        json!({"id":"document","signer_ids":["signer"],"method":"collect","entries":[{"page_id":"page","fields":[{"signer_id":"signer","field_id":"field","display_settings":{"left":10,"top":20,"width":100,"height":30,"fontSize":12}}]}],"expires_at":"2026-12-31T23:59:59Z"})
    );
    assert_eq!(
        request.body["signers"][0],
        json!({"id":"signer","embedded":true})
    );
    let (client, server) = mock(envelope());
    client
        .envelopes("account")
        .add_document("envelope", &AddEnvelopeDocumentBody::new(document))
        .await
        .unwrap();
    let added = server.join().unwrap();
    assert_eq!(added.body, request.body["documents"][0]);
}

#[tokio::test]
async fn envelope_signer_notification_pairings_match_assignment_rules() {
    use assinafy::models::NotificationMethod::{Email, Whatsapp};
    for (verification, notification) in [
        (VerificationMethod::Email, Email),
        (VerificationMethod::Whatsapp, Whatsapp),
        (VerificationMethod::DigitalCertificate, Email),
        (VerificationMethod::DigitalCertificate, Whatsapp),
        (VerificationMethod::id_document_face_match_basic(), Email),
        (VerificationMethod::id_document_face_match_basic(), Whatsapp),
    ] {
        let (client, server) = mock(envelope());
        let mut body = create_body();
        body.signers[0] = EnvelopeSignerConfig::new("signer")
            .verification_method(verification.clone())
            .notification_methods(vec![notification.clone()]);
        client
            .envelopes("account")
            .create(body)
            .send()
            .await
            .unwrap();
        assert_eq!(
            server.join().unwrap().body["signers"][0],
            json!({"id":"signer","verification_method":verification,"notification_methods":[notification]})
        );
    }
    let client = Client::from_api_key("example-redacted-key").unwrap();
    for notifications in [vec![], vec![Email, Whatsapp], vec![Whatsapp]] {
        let mut body = create_body();
        body.signers[0] = EnvelopeSignerConfig::new("signer")
            .verification_method(VerificationMethod::Email)
            .notification_methods(notifications);
        assert!(matches!(
            client.envelopes("account").create(body).send().await,
            Err(Error::Validation(_))
        ));
    }
}
