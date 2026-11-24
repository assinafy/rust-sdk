mod common;

use assinafy::models::{AssignmentMethod, ScheduledSend, ScheduledSendStatus, SignerCapacity};
use assinafy::resources::{
    AssignmentCreateOptions, CreateAssignmentBody, CreateEnvelopeBody, EnvelopeDocumentBody,
    EnvelopeSignerConfig,
};
use assinafy::{Auth, Error};
use common::{Request, mock, validation_client};
use serde_json::{Value, json};

fn scheduled(kind: &str, status: &str) -> Value {
    json!({"resource":"scheduled_send","id":"scheduled","type":kind,"document_ids":["document"],"status":status,"send_at":"2026-12-01T12:00:00Z","assignment_id":null,"envelope_id":null,"failure_reason":null,"created_at":"2026-11-24T12:00:00Z","processed_at":null})
}

fn check(request: Request, method: &str, path: &str) -> Value {
    assert_eq!(request.method, method);
    assert_eq!(request.url.path(), path);
    assert_eq!(request.headers["x-api-key"], "sentinel-key");
    assert!(!request.headers.contains_key("authorization"));
    request.body
}

fn body() -> CreateAssignmentBody {
    CreateAssignmentBody::new(AssignmentMethod::Virtual, ["signer", "witness"])
}

fn options() -> AssignmentCreateOptions {
    AssignmentCreateOptions::default()
        .sign_as("witness", SignerCapacity::Witness)
        .embedded_signer("signer")
        .max_total_credits(0.0)
}

#[tokio::test]
async fn capacities_are_applied_to_the_correct_signer_without_changing_default_payloads() {
    let (client, server) = mock(json!({"id":"assignment"}));
    client
        .with_auth(Auth::ApiKey("sentinel-key".into()))
        .assignments()
        .create_with_options("document", &body(), &options())
        .await
        .unwrap();
    let payload = check(
        server.join().unwrap(),
        "POST",
        "/v1/documents/document/assignments",
    );
    assert_eq!(
        payload,
        json!({"method":"virtual","signers":[{"id":"signer","embedded":true},{"id":"witness","sign_as":"Witness"}],"max_total_credits":0.0})
    );

    let (client, server) = mock(json!({"total_credits":0,"has_sufficient_resources":true}));
    client
        .with_auth(Auth::ApiKey("sentinel-key".into()))
        .assignments()
        .estimate_cost_with_options("document", &body(), &options())
        .await
        .unwrap();
    assert_eq!(
        check(
            server.join().unwrap(),
            "POST",
            "/v1/documents/document/assignments/estimate-cost"
        ),
        json!({"method":"virtual","signers":[{},{"sign_as":"Witness"}]})
    );
    assert_eq!(
        serde_json::to_value(body()).unwrap(),
        json!({"method":"virtual","signers":[{"id":"signer"},{"id":"witness"}]})
    );
}

#[tokio::test]
async fn scheduling_preserves_assignment_and_envelope_fields_and_time_shapes() {
    let (client, server) = mock(scheduled("assignment", "pending"));
    let result = client
        .with_auth(Auth::ApiKey("sentinel-key".into()))
        .assignments()
        .schedule(
            "document",
            &body().message("Please sign").copy_receivers(["copy"]),
            &options(),
            "2026-12-01T12:00:00Z",
        )
        .await
        .unwrap();
    assert_eq!(result.status, ScheduledSendStatus::Pending);
    let payload = check(
        server.join().unwrap(),
        "POST",
        "/v1/documents/document/scheduled-sends",
    );
    assert_eq!(
        payload,
        json!({"send_at":"2026-12-01T12:00:00Z","method":"virtual","signers":[{"id":"signer","embedded":true},{"id":"witness","sign_as":"Witness"}],"message":"Please sign","copy_receivers":["copy"],"max_total_credits":0.0})
    );

    let (client, server) = mock(scheduled("envelope", "pending"));
    let envelope = CreateEnvelopeBody::new(
        "Contract",
        vec![EnvelopeSignerConfig::new("signer")],
        vec![EnvelopeDocumentBody::new("document", ["signer"])],
    )
    .max_total_credits(1.5);
    client
        .with_auth(Auth::ApiKey("sentinel-key".into()))
        .envelopes("account")
        .schedule(&envelope, 1_800_000_000_i64)
        .await
        .unwrap();
    let request = server.join().unwrap();
    assert!(!request.headers.contains_key("idempotency-key"));
    let payload = check(request, "POST", "/v1/accounts/account/scheduled-sends");
    assert_eq!(payload["send_at"], 1_800_000_000_i64);
    assert_eq!(payload["max_total_credits"], 1.5);
    assert_eq!(
        payload["documents"],
        json!([{"id":"document","signer_ids":["signer"]}])
    );
}

#[tokio::test]
async fn scheduled_send_list_actions_and_conflicts_preserve_wire_contract() {
    let data = json!({"status":200,"message":"","data":[scheduled("assignment", "pending")]});
    let (client, server) = common::mock_http(
        "200 OK",
        "application/json",
        "X-Pagination-Current-Page: 2\r\nX-Pagination-Page-Count: 3\r\n",
        data.to_string().into_bytes(),
    );
    let result = client
        .with_auth(Auth::ApiKey("sentinel-key".into()))
        .scheduled_sends("account")
        .list()
        .document_id("doc & 1")
        .page(2)
        .per_page(10)
        .send()
        .await
        .unwrap();
    assert_eq!(result.next_page(), Some(3));
    let request = server.join().unwrap();
    assert_eq!(
        request.url.query_pairs().collect::<Vec<_>>(),
        [
            ("document_id".into(), "doc & 1".into()),
            ("page".into(), "2".into()),
            ("per-page".into(), "10".into())
        ]
    );
    check(request, "GET", "/v1/accounts/account/scheduled-sends");

    for action in ["cancel", "send"] {
        let (client, server) = mock(scheduled(
            "assignment",
            if action == "cancel" {
                "cancelled"
            } else {
                "sent"
            },
        ));
        let client = client.with_auth(Auth::ApiKey("sentinel-key".into()));
        let api = client.scheduled_sends("account");
        let result = if action == "cancel" {
            api.cancel("scheduled").await
        } else {
            api.send_now("scheduled").await
        }
        .unwrap();
        assert!(matches!(
            result.status,
            ScheduledSendStatus::Cancelled | ScheduledSendStatus::Sent
        ));
        assert_eq!(
            check(
                server.join().unwrap(),
                "PUT",
                &format!("/v1/accounts/account/scheduled-sends/scheduled/{action}")
            ),
            Value::Null
        );
    }
    let (client, server) = common::mock_http("409 Conflict", "application/json", "", json!({"status":409,"message":"Credit ceiling exceeded","data":null,"code":"CreditCeilingExceeded","total_credits":2,"max_total_credits":0}).to_string().into_bytes());
    let err = client
        .with_auth(Auth::ApiKey("sentinel-key".into()))
        .scheduled_sends("account")
        .send_now("scheduled")
        .await
        .unwrap_err();
    assert_eq!(err.code(), Some("CreditCeilingExceeded"));
    assert_eq!(err.api().unwrap().data["total_credits"], 2);
    server.join().unwrap();
}

#[tokio::test]
async fn document_watchers_capacities_and_notification_timestamps_decode() {
    for update in [false, true] {
        let (client, server) =
            mock(json!([{"id":"signer","full_name":"Maria Silva","email":"user@example.invalid"}]));
        let client = client.with_auth(Auth::ApiKey("sentinel-key".into()));
        let result = if update {
            client
                .documents()
                .set_watchers("document", &["signer".into()])
                .await
        } else {
            client.documents().watchers("document").await
        }
        .unwrap();
        assert_eq!(result[0].id, "signer");
        let payload = check(
            server.join().unwrap(),
            if update { "PUT" } else { "GET" },
            "/v1/documents/document/watchers",
        );
        assert_eq!(
            payload,
            if update {
                json!({"watcher_ids":["signer"]})
            } else {
                Value::Null
            }
        );
    }
    let (client, server) = mock(json!([]));
    client
        .with_auth(Auth::ApiKey("sentinel-key".into()))
        .documents()
        .set_watchers("document", &[])
        .await
        .unwrap();
    assert_eq!(server.join().unwrap().body, json!({"watcher_ids":[]}));

    let (client, server) = mock(json!([{"signer_id":"signer","sign_as":"FutureCapacity"}]));
    let result = client
        .with_auth(Auth::ApiKey("sentinel-key".into()))
        .documents()
        .signer_capacities("document")
        .await
        .unwrap();
    assert_eq!(
        result[0].sign_as,
        SignerCapacity::Other("FutureCapacity".into())
    );
    check(
        server.join().unwrap(),
        "GET",
        "/v1/documents/document/signer-capacities",
    );

    let (client, server) = mock(
        json!([{"signer_id":"signer","notification_history":[{"event":"signature_request","status":"opened","sent_at":"2026-11-24T12:00:00Z","delivered_at":"2026-11-24T12:00:05Z","opened_at":"2026-11-24T12:00:10Z"},{"event":"signature_request","status":"sent","delivered_at":null,"opened_at":null}]}]),
    );
    let result = client
        .with_auth(Auth::ApiKey("sentinel-key".into()))
        .documents()
        .notification_history("document")
        .await
        .unwrap();
    assert_eq!(
        result[0].notification_history[0].delivered_at.as_deref(),
        Some("2026-11-24T12:00:05Z")
    );
    assert_eq!(
        result[0].notification_history[0].opened_at.as_deref(),
        Some("2026-11-24T12:00:10Z")
    );
    assert!(result[0].notification_history[1].opened_at.is_none());
    check(
        server.join().unwrap(),
        "GET",
        "/v1/documents/document/notification-history",
    );
}

#[tokio::test]
async fn envelope_reminder_estimate_accepts_its_distinct_response_shape() {
    let (client, server) = mock(
        json!({"total":0.45,"breakdown":[{"code":"NotificationWhatsappResend","name":"Reminder","cost":0.45}],"credit_balance":null,"has_sufficient_credits":true}),
    );
    let result = client
        .with_auth(Auth::ApiKey("sentinel-key".into()))
        .envelopes("account")
        .estimate_resend_cost("envelope", "signer")
        .await
        .unwrap();
    assert_eq!(result.total_credits, 0.45);
    assert!(result.has_sufficient_resources);
    assert_eq!(result.credit_balance, f64::INFINITY);
    assert_eq!(
        check(
            server.join().unwrap(),
            "POST",
            "/v1/accounts/account/envelopes/envelope/signers/signer/estimate-resend-cost"
        ),
        Value::Null
    );
}

#[tokio::test]
async fn new_routes_reject_unsafe_ids_wrong_credentials_and_invalid_options_before_sending() {
    let client = validation_client().with_auth(Auth::ApiKey("sentinel-key".into()));
    for id in ["../other", "doc/other", "doc?token=secret", ""] {
        assert!(matches!(
            client.documents().watchers(id).await,
            Err(Error::Config(_))
        ));
        assert!(matches!(
            client.scheduled_sends("account").cancel(id).await,
            Err(Error::Config(_))
        ));
        assert!(matches!(
            client
                .assignments()
                .schedule(id, &body(), &options(), 1_800_000_000_i64)
                .await,
            Err(Error::Config(_))
        ));
    }
    for auth in [Auth::None, Auth::AccessCode("signer-code".into())] {
        let client = client.with_auth(auth);
        assert!(matches!(
            client.documents().notification_history("doc").await,
            Err(Error::Config(_))
        ));
        assert!(matches!(
            client.documents().signer_capacities("doc").await,
            Err(Error::Config(_))
        ));
        assert!(matches!(
            client.documents().watchers("doc").await,
            Err(Error::Config(_))
        ));
        assert!(matches!(
            client.documents().set_watchers("doc", &[]).await,
            Err(Error::Config(_))
        ));
        assert!(matches!(
            client.scheduled_sends("account").list().send().await,
            Err(Error::Config(_))
        ));
        assert!(matches!(
            client
                .scheduled_sends("account")
                .send_now("scheduled")
                .await,
            Err(Error::Config(_))
        ));
        assert!(matches!(
            client
                .envelopes("account")
                .estimate_resend_cost("env", "signer")
                .await,
            Err(Error::Config(_))
        ));
    }
    for ids in [vec![String::new()], vec!["signer".into(); 11]] {
        assert!(matches!(
            client.documents().set_watchers("doc", &ids).await,
            Err(Error::Validation(_))
        ));
    }
    let invalid = AssignmentCreateOptions::default().sign_as("absent", SignerCapacity::Witness);
    assert!(matches!(
        client
            .assignments()
            .create_with_options("doc", &body(), &invalid)
            .await,
        Err(Error::Validation(_))
    ));
    assert!(matches!(
        client
            .assignments()
            .estimate_cost_with_options("doc", &body(), &invalid)
            .await,
        Err(Error::Validation(_))
    ));
    assert!(matches!(
        client
            .assignments()
            .schedule("doc", &body(), &invalid, "2026-12-01T12:00:00Z")
            .await,
        Err(Error::Validation(_))
    ));
    assert!(matches!(
        client
            .assignments()
            .schedule(
                "doc",
                &body(),
                &options().max_total_credits(f64::NAN),
                1_800_000_000_i64
            )
            .await,
        Err(Error::Validation(_))
    ));
}

#[test]
fn scheduled_send_models_preserve_new_states_and_nullable_fields() {
    let data = scheduled("future", "paused");
    let decoded: ScheduledSend = serde_json::from_value(data.clone()).unwrap();
    assert_eq!(
        decoded.status,
        ScheduledSendStatus::Unknown("paused".into())
    );
    assert_eq!(serde_json::to_value(decoded).unwrap(), data);
}
