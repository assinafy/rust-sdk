use std::collections::HashMap;
use std::io::{BufRead, BufReader, Read, Write};
use std::net::TcpListener;
use std::thread::{self, JoinHandle};
use std::time::Duration;

use assinafy::models::{
    AssignmentMethod, IdentityVerificationStatus, NotificationMethod, VerificationMethod,
};
use assinafy::resources::{
    CreateAssignmentBody, CreateAssignmentSigner, CreateDocumentFromTemplateBody,
    IdentityReviewBody, IdentityReviewDecision, TemplateDocumentSigner, WebhookEndpointBody,
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
        "application/json",
        json!({"status":200, "message":"", "data":data})
            .to_string()
            .into_bytes(),
    )
}

fn mock_response(content_type: &'static str, response: Vec<u8>) -> (Client, JoinHandle<Request>) {
    let listener = TcpListener::bind("127.0.0.1:0").unwrap();
    let address = listener.local_addr().unwrap();
    let server = thread::spawn(move || {
        let (mut stream, _) = listener.accept().unwrap();
        stream
            .set_read_timeout(Some(Duration::from_secs(5)))
            .unwrap();
        let mut reader = BufReader::new(&stream);
        let mut line = String::new();
        reader.read_line(&mut line).unwrap();
        let parts: Vec<_> = line.split_whitespace().collect();
        let method = parts[0].to_owned();
        let url = url::Url::parse(&format!("http://{address}{}", parts[1])).unwrap();
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
        write!(stream, "HTTP/1.1 200 OK\r\nContent-Type: {content_type}\r\nContent-Length: {}\r\nConnection: close\r\n\r\n", response.len()).unwrap();
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
        .timeout(Duration::from_secs(5))
        .build()
        .unwrap();
    (client, server)
}

fn assert_workspace(request: &Request, method: &str, path: &str) {
    assert_eq!(request.method, method);
    assert_eq!(request.url.path(), path);
    assert_eq!(
        request.headers.get("authorization").map(String::as_str),
        Some("Bearer sentinel-workspace-token")
    );
    assert!(!request.headers.contains_key("x-api-key"));
    assert!(request.url.query().is_none());
}

#[tokio::test]
async fn signer_identity_status_preserves_states_and_redacts_capture_links() {
    for status in [
        None,
        Some("Pending"),
        Some("InReview"),
        Some("Approved"),
        Some("Declined"),
        Some("Expired"),
        Some("Abandoned"),
        Some("FutureState"),
    ] {
        let capture = "https://verify.example.invalid/c/session#t=sentinel-capture-token";
        let (client, server) = mock(
            json!({"can_start":status.is_none(), "status":status, "url":capture, "expires_at":null, "completed_at":null}),
        );
        let result = client
            .with_auth(Auth::AccessCode("sentinel-signer-code".into()))
            .signer_self()
            .identity_verification()
            .await
            .unwrap();
        assert_eq!(
            serde_json::to_value(&result).unwrap()["status"],
            json!(status)
        );
        assert_eq!(result.url.as_deref(), Some(capture));
        assert!(!format!("{result:?}").contains("sentinel-capture-token"));
        if status == Some("FutureState") {
            assert_eq!(
                result.status,
                Some(IdentityVerificationStatus::Other("FutureState".into()))
            );
        }
        let request = server.join().unwrap();
        assert_eq!(request.method, "GET");
        assert_eq!(request.url.path(), "/v1/signers/identity-verification");
        assert_eq!(
            request.url.query_pairs().collect::<Vec<_>>(),
            [("signer-access-code".into(), "sentinel-signer-code".into())]
        );
        assert!(!request.headers.contains_key("authorization"));
        assert!(!request.headers.contains_key("x-api-key"));
    }
}

#[tokio::test]
async fn identity_start_requires_consent_and_only_a_signer_credential() {
    let client = Client::from_api_key("sentinel-api-key").unwrap();
    assert!(matches!(
        client.signer_self().identity_verification().await,
        Err(Error::Config(_))
    ));
    assert!(matches!(
        client.signer_self().start_identity_verification(true).await,
        Err(Error::Config(_))
    ));
    let empty = client.with_auth(Auth::AccessCode(" ".into()));
    assert!(matches!(
        empty.signer_self().identity_verification().await,
        Err(Error::Validation(_))
    ));
    let signer = client.with_auth(Auth::AccessCode("sentinel-code".into()));
    assert!(matches!(
        signer
            .signer_self()
            .start_identity_verification(false)
            .await,
        Err(Error::Validation(_))
    ));

    let capture = "https://verify.example.invalid/c/session#t=sentinel-capture-token";
    let (client, server) = mock(json!({"id":"session", "url":capture, "expires_at":null}));
    let session = client
        .with_auth(Auth::AccessCode("sentinel-code".into()))
        .signer_self()
        .start_identity_verification(true)
        .await
        .unwrap();
    assert_eq!(session.url, capture);
    assert_eq!(serde_json::to_value(&session).unwrap()["url"], capture);
    assert!(!format!("{session:?}").contains("sentinel-capture-token"));
    let request = server.join().unwrap();
    assert_eq!(request.method, "POST");
    assert_eq!(request.url.path(), "/v1/signers/identity-verification");
    assert_eq!(request.body, json!({"consent":true}));
    assert!(!request.headers.contains_key("x-api-key"));
    assert!(!request.headers.contains_key("authorization"));
}

#[tokio::test]
async fn account_identity_review_reads_evidence_and_sends_explicit_decisions() {
    let evidence = json!({
        "id":"verification", "status":"InReview", "provider":"AssinafyVerify",
        "verification_method":"IdDocumentFaceMatchBasic",
        "reason_codes":[{"code":"FUTURE_REASON", "description":"FUTURE_REASON"}],
        "evidence_available":true,
        "expected":{"name":"Maria Silva", "cpf":null},
        "document":{"type":"CNH", "name":"MARIA SILVA", "cpf":null},
        "matches":{"cpf":null, "name":0.52, "birth_date":null},
        "checks":[{"name":"name", "result":"review", "score":0.52, "reasons":["FUTURE_REASON"]}],
        "images":["document_front", "document_back", "document_pdf", "face"],
        "ruleset_version":"rules", "evidence_sha256":null,
        "reviewed_by":null, "reviewed_at":null, "review_note":null, "completed_at":null
    });
    let path = "/v1/documents/document/assignments/assignment/signers/signer/identity-verification";
    let (client, server) = mock(evidence.clone());
    let review = client
        .with_auth(Auth::Bearer("sentinel-workspace-token".into()))
        .assignments()
        .identity_verification("document", "assignment", "signer")
        .await
        .unwrap();
    assert_eq!(review.status, IdentityVerificationStatus::InReview);
    assert_eq!(serde_json::to_value(review).unwrap(), evidence);
    assert_workspace(&server.join().unwrap(), "GET", path);

    for decision in [
        IdentityReviewDecision::Approved,
        IdentityReviewDecision::Rejected,
    ] {
        let (client, server) = mock(json!({"id":"verification", "status":"Approved"}));
        let body = IdentityReviewBody::new(decision).note("Identity checked");
        client
            .with_auth(Auth::Bearer("sentinel-workspace-token".into()))
            .assignments()
            .review_identity_verification("document", "assignment", "signer", &body)
            .await
            .unwrap();
        let request = server.join().unwrap();
        assert_workspace(&request, "POST", &format!("{path}/review"));
        assert_eq!(
            request.body,
            json!({"decision":decision, "note":"Identity checked"})
        );
    }
    assert_eq!(
        serde_json::to_value(IdentityReviewBody::new(IdentityReviewDecision::Rejected)).unwrap(),
        json!({"decision":"Rejected"})
    );
}

#[tokio::test]
async fn identity_images_use_safe_paths_and_workspace_credentials() {
    for name in ["document_front", "document_back", "document_pdf", "face"] {
        let (expected_mime, expected_bytes) = if name == "document_pdf" {
            ("application/pdf", b"%PDF-1.4\n".to_vec())
        } else {
            ("image/png", b"\x89PNG\r\n\x1a\n".to_vec())
        };
        let (client, server) = mock_response(expected_mime, expected_bytes.clone());
        let (bytes, mime) = client
            .with_auth(Auth::Bearer("sentinel-workspace-token".into()))
            .assignments()
            .download_identity_image("document", "assignment", "signer", name)
            .await
            .unwrap();
        assert_eq!(&bytes[..], expected_bytes);
        assert_eq!(mime, expected_mime);
        assert_workspace(
            &server.join().unwrap(),
            "GET",
            &format!(
                "/v1/documents/document/assignments/assignment/signers/signer/identity-verification/images/{name}"
            ),
        );
    }
    let client = Client::builder().build().unwrap();
    for (d, a, s, name) in [
        ("../escape", "a", "s", "face"),
        ("d", "a?query", "s", "face"),
        ("d", "a", "s/escape", "face"),
        ("d", "a", "s", "../face"),
    ] {
        assert!(matches!(
            client
                .assignments()
                .download_identity_image(d, a, s, name)
                .await,
            Err(Error::Config(_))
        ));
    }
    let client = client.with_auth(Auth::AccessCode("sentinel-code".into()));
    assert!(matches!(
        client
            .assignments()
            .identity_verification("d", "a", "s")
            .await,
        Err(Error::Config(_))
    ));
    assert!(matches!(
        client
            .assignments()
            .review_identity_verification(
                "d",
                "a",
                "s",
                &IdentityReviewBody::new(IdentityReviewDecision::Approved)
            )
            .await,
        Err(Error::Config(_))
    ));
}

#[tokio::test]
async fn verification_methods_work_on_assignment_and_template_create_and_estimate() {
    for (verification, notification) in [
        (VerificationMethod::Email, NotificationMethod::Email),
        (VerificationMethod::Whatsapp, NotificationMethod::Whatsapp),
        (
            VerificationMethod::DigitalCertificate,
            NotificationMethod::Email,
        ),
        (
            VerificationMethod::DigitalCertificate,
            NotificationMethod::Whatsapp,
        ),
        (
            VerificationMethod::id_document_face_match_basic(),
            NotificationMethod::Email,
        ),
        (
            VerificationMethod::id_document_face_match_basic(),
            NotificationMethod::Whatsapp,
        ),
    ] {
        let body = CreateAssignmentBody::from_signers(
            AssignmentMethod::Virtual,
            [CreateAssignmentSigner::new("signer")
                .step(1)
                .verification_method(verification.clone())
                .notification_methods(vec![notification.clone()])],
        );
        for estimate in [false, true] {
            let response = if estimate {
                json!({"total_credits":1.5, "has_sufficient_resources":true})
            } else {
                json!({"id":"assignment"})
            };
            let (client, server) = mock(response);
            let client = client.with_auth(Auth::Bearer("sentinel-workspace-token".into()));
            if estimate {
                client
                    .assignments()
                    .estimate_cost("document", &body)
                    .await
                    .unwrap();
            } else {
                client
                    .assignments()
                    .create("document", &body)
                    .await
                    .unwrap();
            }
            let request = server.join().unwrap();
            let suffix = if estimate { "/estimate-cost" } else { "" };
            assert_workspace(
                &request,
                "POST",
                &format!("/v1/documents/document/assignments{suffix}"),
            );
            let mut signer =
                json!({"verification_method":verification, "notification_methods":[notification]});
            if !estimate {
                signer["id"] = json!("signer");
                signer["step"] = json!(1);
            }
            assert_eq!(
                request.body,
                json!({"method":"virtual", "signers":[signer]})
            );
        }
        let body =
            CreateDocumentFromTemplateBody::default().signers([TemplateDocumentSigner::existing(
                "role", "signer",
            )
            .step(1)
            .verification_method(verification.clone())
            .notification_methods(vec![notification.clone()])]);
        for estimate in [false, true] {
            let response = if estimate {
                json!({"total_credits":1.5, "has_sufficient_resources":true})
            } else {
                json!({"id":"document", "name":"Example", "status":"pending_signature"})
            };
            let (client, server) = mock(response);
            let client = client.with_auth(Auth::Bearer("sentinel-workspace-token".into()));
            if estimate {
                client
                    .templates("account")
                    .estimate_cost("template", &body)
                    .await
                    .unwrap();
            } else {
                client
                    .templates("account")
                    .create_document("template", &body)
                    .await
                    .unwrap();
            }
            let request = server.join().unwrap();
            let suffix = if estimate { "/estimate-cost" } else { "" };
            assert_workspace(
                &request,
                "POST",
                &format!("/v1/accounts/account/templates/template/documents{suffix}"),
            );
            let mut signer = json!({"role_id":"role", "verification_method":verification, "notification_methods":[notification]});
            if !estimate {
                signer["id"] = json!("signer");
                signer["step"] = json!(1);
            }
            assert_eq!(request.body, json!({"signers":[signer]}));
        }
    }
}

#[tokio::test]
async fn webhook_creation_preserves_the_one_time_secret_for_oauth_apps() {
    let (client, server) =
        mock(json!({"id":"endpoint", "signing_enabled":true, "secret":"whsec_sentinel-secret"}));
    let body = WebhookEndpointBody::new(
        "https://hooks.example.invalid/assinafy",
        "user@example.invalid",
    )
    .events(["document_ready"])
    .signing_enabled(true);
    let endpoint = client
        .with_auth(Auth::Bearer("sentinel-workspace-token".into()))
        .webhooks("account")
        .create_endpoint(&body)
        .await
        .unwrap();
    assert_eq!(endpoint.secret.as_deref(), Some("whsec_sentinel-secret"));
    assert_eq!(
        serde_json::to_value(&endpoint).unwrap()["secret"],
        "whsec_sentinel-secret"
    );
    assert!(!format!("{endpoint:?}").contains("sentinel-secret"));
    let request = server.join().unwrap();
    assert_workspace(&request, "POST", "/v1/accounts/account/webhooks/endpoints");
    assert_eq!(request.body, serde_json::to_value(body).unwrap());
}

#[tokio::test]
async fn webhook_url_lookup_and_delete_encode_the_exact_registered_url() {
    let registered = "https://hooks.example.invalid/assinafy?a=1&b=two+words#fragment";
    for delete in [false, true] {
        let (client, server) = mock(json!([]));
        let client = client.with_auth(Auth::ApiKey("sentinel-api-key".into()));
        if delete {
            client
                .webhooks("account")
                .delete_endpoint_by_url(registered)
                .await
                .unwrap();
        } else {
            assert!(
                client
                    .webhooks("account")
                    .list_endpoints_by_url(registered)
                    .await
                    .unwrap()
                    .is_empty()
            );
        }
        let request = server.join().unwrap();
        assert_eq!(request.method, if delete { "DELETE" } else { "GET" });
        assert_eq!(
            request.url.path(),
            "/v1/accounts/account/webhooks/endpoints"
        );
        assert_eq!(
            request.url.query_pairs().collect::<Vec<_>>(),
            [("url".into(), registered.into())]
        );
        assert_eq!(
            request.headers.get("x-api-key").map(String::as_str),
            Some("sentinel-api-key")
        );
        assert_eq!(request.body, Value::Null);
    }
    assert!(matches!(
        Client::builder()
            .build()
            .unwrap()
            .webhooks("account")
            .delete_endpoint_by_url(" ")
            .await,
        Err(Error::Validation(_))
    ));
}
