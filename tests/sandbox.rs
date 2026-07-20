//! End-to-end integration tests against the Assinafy sandbox.
//!
//! These tests are `#[ignore]` by default. Run them with:
//!
//! ```bash
//! ASSINAFY_API_KEY=<sandbox key> ASSINAFY_ACCOUNT_ID=<sandbox account> \
//!   cargo test --test sandbox -- --ignored --test-threads=1
//! ```

mod common;

use assinafy::resources::{
    CreateFieldBody, CreateSignerBody, CreateTagBody, CreateTemplateRequest,
    SearchDocumentsRequest, UpdateFieldBody, UpdateSignerBody, UpdateTagBody,
    UploadDocumentRequest,
};
use uuid::Uuid;

fn unique<S: AsRef<str>>(prefix: S) -> String {
    format!(
        "{}-{}",
        prefix.as_ref(),
        Uuid::new_v4().simple().to_string().split_at(8).0
    )
}

#[tokio::test]
#[ignore = "hits live sandbox"]
async fn document_statuses_endpoint_returns_known_codes() {
    let (client, _) = sandbox_or_skip!();
    let statuses = client
        .documents()
        .statuses()
        .await
        .expect("documents/statuses");
    let codes: Vec<String> = statuses.iter().map(|s| s.code.to_string()).collect();
    for expected in [
        "uploaded",
        "metadata_processing",
        "metadata_ready",
        "certificated",
        "pending_signature",
    ] {
        assert!(
            codes.iter().any(|c| c == expected),
            "expected status `{expected}` in {codes:?}"
        );
    }
}

#[tokio::test]
#[ignore = "hits live sandbox"]
async fn signers_full_lifecycle() {
    let (client, account_id) = sandbox_or_skip!();
    let signers = client.signers(&account_id);

    let full_name = unique("Rust SDK Signer");
    let email = format!("bill+{}@febacapital.com", unique("rust-sdk"));
    let created = signers
        .create(&CreateSignerBody::new(&full_name).email(&email))
        .await
        .expect("create signer");
    assert_eq!(created.full_name, full_name);
    assert_eq!(created.email.as_deref(), Some(email.as_str()));

    let fetched = signers.get(&created.id).await.expect("get signer");
    assert_eq!(fetched.id, created.id);

    let new_name = format!("{full_name} (updated)");
    let updated_email = format!("billm+{}@billm.org", unique("rust-sdk"));
    let updated = signers
        .update(
            &created.id,
            &UpdateSignerBody::new()
                .full_name(&new_name)
                .email(&updated_email),
        )
        .await
        .expect("update signer");
    assert_eq!(updated.full_name, new_name);
    assert_eq!(updated.email.as_deref(), Some(updated_email.as_str()));

    let page = signers
        .list()
        .per_page(100)
        .search(&new_name)
        .send()
        .await
        .expect("list signers");
    assert!(
        page.data.iter().any(|s| s.id == created.id),
        "newly created signer should appear in search results"
    );

    signers.delete(&created.id).await.expect("delete signer");
}

#[tokio::test]
#[ignore = "hits live sandbox"]
async fn tags_full_lifecycle() {
    let (client, account_id) = sandbox_or_skip!();
    let tags = client.tags(&account_id);

    let name = unique("rust-sdk-tag");
    let created = tags
        .create(&CreateTagBody::new(&name).color("3366ff"))
        .await
        .expect("create tag");
    assert_eq!(created.name, name);

    let updated = tags
        .update(&created.id, &UpdateTagBody::new().color("ff6633"))
        .await
        .expect("update tag");
    assert_eq!(updated.color.as_deref(), Some("ff6633"));

    let page = tags.list().search(&name).send().await.expect("list tags");
    assert!(page.data.iter().any(|t| t.id == created.id));

    tags.delete(&created.id).await.expect("delete tag");
}

#[tokio::test]
#[ignore = "hits live sandbox"]
async fn read_only_reference_endpoints_are_available() {
    let (client, account_id) = sandbox_or_skip!();

    let field_types = client
        .fields(&account_id)
        .list_types()
        .await
        .expect("field types");
    assert!(
        field_types
            .iter()
            .any(|field_type| field_type.kind == "email"),
        "expected email field type in {field_types:?}"
    );

    let _fields = client
        .fields(&account_id)
        .list()
        .include_standard(true)
        .send()
        .await
        .expect("list fields");

    let _templates_page = client
        .templates(&account_id)
        .list()
        .per_page(5)
        .send()
        .await
        .expect("list templates");

    let _subscription = client
        .webhooks(&account_id)
        .get_subscription()
        .await
        .expect("get webhook subscription");

    let event_types = client
        .webhooks(&account_id)
        .event_types()
        .await
        .expect("webhook event types");
    assert!(
        event_types.iter().any(|event| event.id == "document_ready"),
        "expected document_ready event type in {event_types:?}"
    );

    let _dispatches = client
        .webhooks(&account_id)
        .list_dispatches()
        .per_page(5)
        .send()
        .await
        .expect("list webhook dispatches");
}

#[tokio::test]
#[ignore = "hits live sandbox"]
async fn fields_full_lifecycle() {
    let (client, account_id) = sandbox_or_skip!();
    let fields = client.fields(&account_id);

    let name = unique("Rust SDK Field");
    let created = fields
        .create(
            &CreateFieldBody::new("text", &name)
                .regex("/^[A-Z]{3}$/")
                .required(true),
        )
        .await
        .expect("create field");
    assert_eq!(created.name, name);
    assert_eq!(created.kind, "text");

    let fetched = fields.get(&created.id).await.expect("get field");
    assert_eq!(fetched.id, created.id);

    let valid = fields
        .validate(&created.id, "ABC")
        .await
        .expect("validate field");
    assert!(
        valid.success,
        "expected ABC to satisfy field regex: {valid:?}"
    );

    let new_name = format!("{name} Updated");
    let updated = fields
        .update(
            &created.id,
            &UpdateFieldBody::new()
                .name(&new_name)
                .clear_regex()
                .required(false),
        )
        .await
        .expect("update field");
    assert_eq!(updated.name, new_name);

    let all = fields.list().send().await.expect("list fields");
    assert!(all.iter().any(|field| field.id == created.id));

    fields.delete(&created.id).await.expect("delete field");
}

#[tokio::test]
#[ignore = "hits live sandbox"]
async fn list_signers_returns_pagination_metadata() {
    let (client, account_id) = sandbox_or_skip!();
    let page = client
        .signers(&account_id)
        .list()
        .per_page(1)
        .send()
        .await
        .expect("list signers paginated");
    assert!(
        page.meta.current_page.is_some(),
        "current_page header parsed"
    );
    assert!(page.meta.per_page.is_some(), "per_page header parsed");
    assert!(page.meta.total_count.is_some(), "total_count header parsed");
}

#[tokio::test]
#[ignore = "hits live sandbox"]
async fn list_documents_does_not_error_on_empty_account() {
    let (client, account_id) = sandbox_or_skip!();
    use assinafy::resources::ListDocumentsRequest;
    let _page = client
        .documents()
        .list(&account_id, ListDocumentsRequest::default().per_page(5))
        .await
        .expect("list documents");
}

#[tokio::test]
#[ignore = "hits live sandbox"]
async fn missing_signer_returns_404_api_error() {
    let (client, account_id) = sandbox_or_skip!();
    let err = client
        .signers(&account_id)
        .get("definitely-not-a-real-id")
        .await
        .expect_err("expected 404");
    assert_eq!(err.status(), Some(404), "got error: {err}");
}

#[tokio::test]
#[ignore = "hits live sandbox"]
async fn upload_document_then_delete() {
    use assinafy::models::DocumentStatus;
    use std::time::Duration;

    let (client, account_id) = sandbox_or_skip!();
    let pdf = minimal_pdf();
    let upload = assinafy::resources::UploadDocumentRequest::from_bytes(
        format!("{}.pdf", unique("rust-sdk-doc")),
        pdf,
    );
    let doc = client
        .documents()
        .upload(&account_id, upload)
        .await
        .expect("upload document");
    assert!(!doc.id.is_empty(), "uploaded doc has id");
    assert!(
        doc.artifacts.contains_key("original"),
        "upload response should expose the original artifact url"
    );

    // Poll until the document leaves the non-deletable processing states.
    let deletable_statuses = client
        .documents()
        .statuses()
        .await
        .expect("documents/statuses");
    let is_deletable = |s: &DocumentStatus| {
        deletable_statuses
            .iter()
            .any(|info| info.code == *s && info.deletable)
    };

    let mut status = doc.status.clone();
    let mut tries = 0;
    while !is_deletable(&status) {
        if tries >= 30 {
            panic!(
                "document {} stuck in non-deletable status {} after 30 tries",
                doc.id, status
            );
        }
        tokio::time::sleep(Duration::from_secs(1)).await;
        status = client
            .documents()
            .get(&doc.id)
            .await
            .expect("get doc")
            .status;
        tries += 1;
    }

    client
        .documents()
        .delete(&doc.id)
        .await
        .expect("delete uploaded document");
}

#[tokio::test]
#[ignore = "hits live sandbox"]
async fn verify_unknown_hash_returns_invalid_typed_result() {
    let (client, _) = sandbox_or_skip!();
    let result = client
        .documents()
        .verify("INVALIDHASHEXAMPLE")
        .await
        .expect("verify endpoint");
    assert!(
        !result.is_valid,
        "an unknown hash must not verify: {result:?}"
    );
    assert!(
        result.id.is_none(),
        "unknown hash should have no document id"
    );
    assert!(
        result.verified_at.is_some(),
        "verified_at is always returned"
    );
}

#[tokio::test]
#[ignore = "hits live sandbox"]
async fn public_document_info_is_typed() {
    use assinafy::resources::ListDocumentsRequest;
    let (client, account_id) = sandbox_or_skip!();
    let page = client
        .documents()
        .list(&account_id, ListDocumentsRequest::default().per_page(1))
        .await
        .expect("list documents");
    let Some(doc) = page.data.first() else {
        eprintln!("skipping: no documents in sandbox account to query publicly");
        return;
    };
    let public = client
        .public()
        .document(&doc.id)
        .await
        .expect("public document info");
    assert_eq!(public.id, doc.id);
    assert!(!public.name.is_empty(), "public document has a name");
}

#[tokio::test]
#[ignore = "hits live sandbox"]
async fn accounts_list_get_and_theme() {
    let (client, account_id) = sandbox_or_skip!();

    let accounts = client.accounts_api().list().await.expect("list accounts");
    assert!(
        accounts.iter().any(|a| a.id == account_id),
        "the configured account should appear in the list"
    );

    let account = client
        .account(&account_id)
        .get()
        .await
        .expect("get account");
    assert_eq!(account.id, account_id);
    assert!(!account.name.is_empty(), "account has a name");

    // Theme is always available (colors/logo may be null).
    let _theme = client
        .account(&account_id)
        .theme()
        .await
        .expect("get theme");
}

#[tokio::test]
#[ignore = "hits live sandbox"]
async fn assignments_list_requires_account_context() {
    let (client, account_id) = sandbox_or_skip!();
    // The SDK always supplies the required `accountId` query param, so this
    // must not 400 with "account context required".
    let _page = client
        .assignments()
        .list(&account_id)
        .per_page(5)
        .send()
        .await
        .expect("list assignments");
}

#[tokio::test]
#[ignore = "hits live sandbox"]
async fn documents_rename_and_search() {
    use assinafy::models::DocumentStatus;
    use std::time::Duration;

    let (client, account_id) = sandbox_or_skip!();
    let docs = client.documents();

    let upload =
        UploadDocumentRequest::from_bytes(format!("{}.pdf", unique("rust-rename")), minimal_pdf());
    let doc = docs.upload(&account_id, upload).await.expect("upload");

    // Wait until the document is deletable (metadata ready) before mutating it.
    let deletable = docs.statuses().await.expect("statuses");
    let is_deletable = |s: &DocumentStatus| {
        deletable
            .iter()
            .any(|info| info.code == *s && info.deletable)
    };
    let mut status = doc.status.clone();
    let mut tries = 0;
    while !is_deletable(&status) {
        assert!(tries < 30, "document stuck in {status}");
        tokio::time::sleep(Duration::from_secs(1)).await;
        status = docs.get(&doc.id).await.expect("get doc").status;
        tries += 1;
    }

    let new_name = format!("{}.pdf", unique("rust-renamed"));
    let renamed = docs.rename(&doc.id, &new_name).await.expect("rename");
    assert_eq!(renamed.name, new_name);

    let page = docs
        .search(
            &account_id,
            SearchDocumentsRequest::new("rust-renamed").per_page(5),
        )
        .await
        .expect("search documents");
    assert!(page.meta.current_page.is_some(), "search is paginated");

    docs.delete(&doc.id).await.expect("delete document");
}

#[tokio::test]
#[ignore = "hits live sandbox"]
async fn templates_create_get_delete() {
    use assinafy::models::TemplateStatus;
    use std::time::Duration;

    let (client, account_id) = sandbox_or_skip!();
    let templates = client.templates(&account_id);

    let file =
        CreateTemplateRequest::from_bytes(format!("{}.pdf", unique("rust-tpl")), minimal_pdf());
    let created = templates.create(file).await.expect("create template");
    assert!(!created.id.is_empty(), "template has an id");

    // A template cannot be deleted until it finishes processing.
    let mut fetched = templates.get(&created.id).await.expect("get template");
    assert_eq!(fetched.id, created.id);
    let mut tries = 0;
    while matches!(
        fetched.status,
        TemplateStatus::Processing | TemplateStatus::Uploading | TemplateStatus::Uploaded
    ) {
        assert!(tries < 30, "template stuck in {}", fetched.status);
        tokio::time::sleep(Duration::from_secs(1)).await;
        fetched = templates.get(&created.id).await.expect("get template");
        tries += 1;
    }

    templates
        .delete(&created.id)
        .await
        .expect("delete template");
}

/// A 1-page valid PDF used to exercise the upload endpoint without bundling a
/// binary fixture.
fn minimal_pdf() -> Vec<u8> {
    // Tiny valid PDF with one empty page (612 x 792 letter).
    const PDF: &[u8] = b"%PDF-1.4\n\
1 0 obj<< /Type /Catalog /Pages 2 0 R >>endobj\n\
2 0 obj<< /Type /Pages /Kids [3 0 R] /Count 1 >>endobj\n\
3 0 obj<< /Type /Page /Parent 2 0 R /MediaBox [0 0 612 792] /Contents 4 0 R /Resources << >> >>endobj\n\
4 0 obj<< /Length 44 >>stream\n\
BT /F1 24 Tf 100 700 Td (Hello, Assinafy!) Tj ET\n\
endstream\nendobj\n\
xref\n0 5\n0000000000 65535 f \n\
0000000010 00000 n \n\
0000000060 00000 n \n\
0000000111 00000 n \n\
0000000211 00000 n \n\
trailer<< /Size 5 /Root 1 0 R >>\nstartxref\n299\n%%EOF\n";
    PDF.to_vec()
}
