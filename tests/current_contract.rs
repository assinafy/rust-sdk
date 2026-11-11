use std::collections::HashMap;
use std::io::{BufRead, BufReader, Read, Write};
use std::net::TcpListener;
use std::thread::{self, JoinHandle};
use std::time::Duration;

use assinafy::models::{AssignmentMethod, Document, DocumentVerification, SigningUrl};
use assinafy::resources::{
    AssignmentCreateOptions, CreateAssignmentBody, CreateDocumentFromTemplateBody,
    ListDocumentsRequest, SearchDocumentsRequest, TemplateDocumentOptions, TemplateDocumentSigner,
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
    let client = Client::builder()
        .auth(Auth::ApiKey("example-redacted-key".into()))
        .base_url(BaseUrl::custom("http://127.0.0.1:1/v1").unwrap())
        .build()
        .unwrap();
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
