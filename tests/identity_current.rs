use assinafy::resources::DeclineMultipleDocumentsBody;
use assinafy::resources::UpdatePushPreferencesBody;
use assinafy::resources::{UpdateNotificationPolicyBody, UpdateNotificationPolicyEvents};
use assinafy::{Auth, BaseUrl, Client, Error};
use serde_json::{Value, json};
use std::collections::HashMap;
use std::io::{BufRead, BufReader, Read, Write};
use std::net::TcpListener;
use std::thread::{self, JoinHandle};
use std::time::Duration;

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

fn assert_request(r: &Request, method: &str, path: &str, signer: bool) {
    assert_eq!(r.method, method);
    assert_eq!(r.url.path(), path);
    assert!(!r.headers.contains_key("authorization"));
    assert!(!r.headers.contains_key("x-api-key"));
    if signer {
        assert!(
            r.url
                .query_pairs()
                .any(|(k, v)| k == "signer-access-code" && v == "sentinel-code")
        );
    } else {
        assert!(!r.url.query_pairs().any(|(k, _)| k == "signer-access-code"));
    }
}

#[tokio::test]
async fn signer_branding_and_artifact_download_use_only_signer_auth() {
    let (client, server) = mock(json!({"account_name":"Example Workspace","logo":null}));
    let theme = client
        .with_auth(Auth::AccessCode("sentinel-code".into()))
        .signer_self()
        .theme()
        .await
        .unwrap();
    assert_eq!(theme.account_name.as_deref(), Some("Example Workspace"));
    assert_request(&server.join().unwrap(), "GET", "/v1/sign/theme", true);
    let (client, server) = mock_response("image/png", b"png".to_vec());
    client
        .with_auth(Auth::AccessCode("sentinel-code".into()))
        .signer_self()
        .download_logo()
        .await
        .unwrap();
    assert_request(&server.join().unwrap(), "GET", "/v1/sign/logo", true);
    let (client, server) = mock_response("application/pdf", b"pdf".to_vec());
    client
        .with_auth(Auth::AccessCode("sentinel-code".into()))
        .signer_self()
        .download_document("signer", "document", "original")
        .await
        .unwrap();
    assert_request(
        &server.join().unwrap(),
        "GET",
        "/v1/signers/signer/documents/document/download/original",
        true,
    );
}

#[tokio::test]
async fn certified_copy_resends_preserve_request_and_public_credential_boundaries() {
    let data = json!({"available":true,"notification_method":"Email","resends_left":null,"sent_to":"u***@example.invalid"});
    let (client, server) = mock(data.clone());
    let result = client
        .with_auth(Auth::AccessCode("sentinel-code".into()))
        .signer_self()
        .resend_certified_copy(Some("other-document"))
        .await
        .unwrap();
    assert_eq!(result.sent_to.as_deref(), Some("u***@example.invalid"));
    let r = server.join().unwrap();
    assert_request(&r, "POST", "/v1/sign/certified-copy", true);
    assert_eq!(r.body, json!({"document_id":"other-document"}));
    let (client, server) = mock(data);
    client
        .with_auth(Auth::Bearer("sentinel-workspace".into()))
        .public()
        .resend_certified_copy("link-token")
        .await
        .unwrap();
    let r = server.join().unwrap();
    assert_request(&r, "POST", "/v1/signers/downloads/link-token/resend", false);
    assert!(r.body.is_null());
    let (client, server) = mock_response("application/pdf; charset=binary", b"pdf".to_vec());
    assert_eq!(
        &client
            .with_auth(Auth::AccessCode("sentinel-code".into()))
            .public()
            .download_certified_copy_pades("link-token")
            .await
            .unwrap()
            .0[..],
        b"pdf"
    );
    assert_request(
        &server.join().unwrap(),
        "GET",
        "/v1/signers/downloads/link-token/pades",
        false,
    );
    let (client, server) = mock_response("text/html", b"expired sentinel-token".to_vec());
    let error = client
        .public()
        .download_certified_copy("link-token")
        .await
        .unwrap_err();
    assert!(matches!(error, Error::UnexpectedResponse(_)));
    assert!(!format!("{error:?}").contains("sentinel-token"));
    server.join().unwrap();
}

#[tokio::test]
async fn preferences_merge_only_supplied_fields_and_reject_empty_updates() {
    let (client, server) = mock(json!({"SignerViewed":false,"SignerSigned":true,"Future":true}));
    let prefs = client
        .users()
        .update_push_preferences(&UpdatePushPreferencesBody::new().signer_viewed(false))
        .await
        .unwrap();
    assert_eq!(prefs.signer_viewed, Some(false));
    let r = server.join().unwrap();
    assert_request(&r, "PUT", "/v1/users/self/push-preferences", false);
    assert_eq!(r.body, json!({"SignerViewed":false}));
    let (client, server) =
        mock(json!({"editable":true,"owners_receive_all":true,"events":{"SignerViewed":false}}));
    let policy = client
        .account("account")
        .update_notification_policy(
            &UpdateNotificationPolicyBody::new()
                .events(UpdateNotificationPolicyEvents::new().signer_viewed(false))
                .owners_receive_all(true),
        )
        .await
        .unwrap();
    assert!(policy.editable);
    assert_eq!(policy.events.signer_viewed, Some(false));
    let r = server.join().unwrap();
    assert_request(&r, "PUT", "/v1/accounts/account/notification-policy", false);
    assert_eq!(
        r.body,
        json!({"events":{"SignerViewed":false},"owners_receive_all":true})
    );
    let client = Client::builder().build().unwrap();
    assert!(matches!(
        client
            .users()
            .update_push_preferences(&UpdatePushPreferencesBody::new())
            .await,
        Err(Error::Validation(_))
    ));
    assert!(matches!(
        client
            .account("account")
            .update_notification_policy(
                &UpdateNotificationPolicyBody::new().events(UpdateNotificationPolicyEvents::new())
            )
            .await,
        Err(Error::Validation(_))
    ));
    assert!(matches!(
        client
            .signer_self()
            .decline_multiple(&DeclineMultipleDocumentsBody::new(
                ["doc"],
                "é".repeat(2001)
            ))
            .await,
        Err(Error::Validation(_))
    ));
}

#[tokio::test]
async fn account_capabilities_and_webhook_test_and_overlap_rotation_match_contract() {
    let (client, server) = mock(
        json!({"id":"account","name":"Example Workspace","signing_features":{"DigitalCertificate":true,"Embed":true}}),
    );
    assert!(
        client
            .account("account")
            .get_with_signing_features()
            .await
            .unwrap()
            .signing_features
            .unwrap()
            .embed
    );
    let r = server.join().unwrap();
    assert_eq!(r.url.query(), Some("expand=signing_features"));
    let (client, server) =
        mock(json!({"id":"dispatch","event":"webhook_test","kind":"manual","delivered":true}));
    assert!(
        client
            .webhooks("account")
            .test_endpoint("endpoint")
            .await
            .unwrap()
            .delivered
    );
    let r = server.join().unwrap();
    assert_request(
        &r,
        "POST",
        "/v1/accounts/account/webhooks/endpoints/endpoint/test",
        false,
    );
    assert!(r.body.is_null());
    let (client, server) = mock(
        json!({"secret":"sentinel-secret","previous_secret_expires_at":"2026-11-12T12:00:00Z"}),
    );
    let result = client
        .webhooks("account")
        .rotate_endpoint_secret_with_overlap("endpoint", true)
        .await
        .unwrap();
    assert_eq!(
        result.previous_secret_expires_at.as_deref(),
        Some("2026-11-12T12:00:00Z")
    );
    assert!(!format!("{result:?}").contains("sentinel-secret"));
    let r = server.join().unwrap();
    assert_request(
        &r,
        "POST",
        "/v1/accounts/account/webhooks/endpoints/endpoint/secret/rotate",
        false,
    );
    assert_eq!(r.body, json!({"keep_previous":true}));
}

#[tokio::test]
async fn preference_reads_and_signing_options_preserve_reduced_payloads() {
    let (client, server) = mock(json!({"SignerViewed": true}));
    assert_eq!(
        client
            .users()
            .push_preferences()
            .await
            .unwrap()
            .signer_viewed,
        Some(true)
    );
    assert_request(
        &server.join().unwrap(),
        "GET",
        "/v1/users/self/push-preferences",
        false,
    );
    let (client, server) =
        mock(json!({"editable": false, "owners_receive_all": false, "events": {}}));
    assert!(
        !client
            .account("account")
            .notification_policy()
            .await
            .unwrap()
            .editable
    );
    assert_request(
        &server.join().unwrap(),
        "GET",
        "/v1/accounts/account/notification-policy",
        false,
    );
    let (client, server) = mock(
        json!({"id":"doc","name":"example.pdf","status":"certificated","is_signer_access_closed":true,"current_signer":{"id":"signer","completed":true},"certified_copy":{"available":true,"notification_method":"Email","resends_left":null},"envelope":{"id":"env","name":"Example envelope"}}),
    );
    let document = client
        .with_auth(Auth::AccessCode("sentinel-code".into()))
        .signer_self()
        .signable_document_with_options(Some(true), true, Some("doc"))
        .await
        .unwrap();
    assert_eq!(document.id, "doc");
    let r = server.join().unwrap();
    assert_request(&r, "GET", "/v1/sign", true);
    let query: HashMap<_, _> = r.url.query_pairs().collect();
    assert_eq!(query.get("expand").map(|v| v.as_ref()), Some("envelope"));
    assert_eq!(query.get("document_id").map(|v| v.as_ref()), Some("doc"));
    assert_eq!(
        query.get("has_accepted_terms").map(|v| v.as_ref()),
        Some("true")
    );
}

#[tokio::test]
async fn expired_link_redirect_html_is_not_a_pdf_and_never_gains_credentials() {
    let listener = TcpListener::bind("127.0.0.1:0").unwrap();
    let address = listener.local_addr().unwrap();
    let server = thread::spawn(move || {
        for index in 0..2 {
            let (mut stream, _) = listener.accept().unwrap();
            stream
                .set_read_timeout(Some(Duration::from_secs(5)))
                .unwrap();
            let mut reader = BufReader::new(&stream);
            let mut request = String::new();
            loop {
                let mut line = String::new();
                reader.read_line(&mut line).unwrap();
                if line == "\r\n" {
                    break;
                }
                request.push_str(&line);
            }
            assert!(!request.to_lowercase().contains("authorization:"));
            assert!(!request.contains("sentinel-code"));
            let response = if index == 0 {
                "HTTP/1.1 302 Found\r\nLocation: /expired?token=link-token\r\nContent-Length: 0\r\nConnection: close\r\n\r\n"
            } else {
                "HTTP/1.1 200 OK\r\nContent-Type: text/html\r\nContent-Length: 7\r\nConnection: close\r\n\r\nexpired"
            };
            stream.write_all(response.as_bytes()).unwrap();
        }
    });
    let client = Client::builder()
        .base_url(BaseUrl::custom(format!("http://{address}/v1")).unwrap())
        .timeout(Duration::from_secs(5))
        .build()
        .unwrap()
        .with_auth(Auth::AccessCode("sentinel-code".into()));
    let error = client
        .public()
        .download_certified_copy("link-token")
        .await
        .unwrap_err();
    assert!(matches!(error, Error::UnexpectedResponse(_)));
    server.join().unwrap();
}

#[tokio::test]
async fn account_usage_preserves_unlimited_and_fractional_balances() {
    let (client, server) = mock(
        json!({"plan":null,"documents_renewal_date":null,"balances":{"documents":{"balance":null,"limit":null,"spent":123},"credits":{"balance":0.5,"limit":10,"spent":9.5}}}),
    );
    let usage = client
        .with_auth(Auth::Bearer("workspace-token".into()))
        .account("account")
        .usage()
        .await
        .unwrap();
    assert!(usage.plan.is_none());
    assert!(usage.balances.documents.balance.is_none());
    assert_eq!(usage.balances.credits.balance, Some(0.5));
    assert_eq!(usage.balances.credits.spent, 9.5);
    let request = server.join().unwrap();
    assert_eq!(request.method, "GET");
    assert_eq!(request.url.path(), "/v1/accounts/account/usage");
    assert_eq!(
        request.headers.get("authorization").map(String::as_str),
        Some("Bearer workspace-token")
    );
}

#[tokio::test]
async fn signer_envelope_filters_keep_workspace_and_signer_auth_separate() {
    let (client, server) = mock(json!([]));
    client
        .with_auth(Auth::AccessCode("sentinel-code".into()))
        .signer_self()
        .list_documents("signer")
        .envelope_id("envelope")
        .send()
        .await
        .unwrap();
    let request = server.join().unwrap();
    assert_request(&request, "GET", "/v1/signers/signer/documents", true);
    assert!(
        request
            .url
            .query_pairs()
            .any(|(k, v)| k == "envelope_id" && v == "envelope")
    );
    let (client, server) = mock(json!([]));
    client
        .with_auth(Auth::AccessCode("sentinel-code".into()))
        .signer_self()
        .search_documents_in_envelope("signer", "contract", "envelope")
        .await
        .unwrap();
    let request = server.join().unwrap();
    assert_request(&request, "GET", "/v1/signers/signer/documents/search", true);
    assert!(
        request
            .url
            .query_pairs()
            .any(|(k, v)| k == "envelope_id" && v == "envelope")
    );
    let (client, server) = mock(json!([]));
    client
        .with_auth(Auth::Bearer("workspace-token".into()))
        .signer_self()
        .search_documents_in_envelope_as_workspace("signer", "contract", "envelope")
        .await
        .unwrap();
    let request = server.join().unwrap();
    assert_eq!(
        request.headers.get("authorization").map(String::as_str),
        Some("Bearer workspace-token")
    );
    assert!(
        !request
            .url
            .query_pairs()
            .any(|(k, _)| k == "signer-access-code")
    );
    let (client, server) = mock(
        json!({"id":"doc","name":"example.pdf","status":"pending_signature","envelope":{"id":"envelope","name":"Example envelope"}}),
    );
    client
        .with_auth(Auth::AccessCode("sentinel-code".into()))
        .signer_self()
        .current_document_with_envelope("signer")
        .await
        .unwrap();
    let request = server.join().unwrap();
    assert_request(&request, "GET", "/v1/signers/signer/document", true);
    assert!(
        request
            .url
            .query_pairs()
            .any(|(k, v)| k == "expand" && v == "envelope")
    );
}

#[test]
fn webhook_duration_decodes_new_and_legacy_dispatches() {
    for duration in [Some(184u64), None] {
        let dispatch: assinafy::models::WebhookDispatch = serde_json::from_value(
            json!({"id":"dispatch", "event":"webhook_test", "duration_ms":duration}),
        )
        .unwrap();
        assert_eq!(dispatch.duration_ms, duration);
    }
    let legacy: assinafy::models::WebhookDispatch =
        serde_json::from_value(json!({"id":"dispatch", "event":"webhook_test"})).unwrap();
    assert_eq!(legacy.duration_ms, None);
}
