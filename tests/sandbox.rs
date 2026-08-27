//! End-to-end integration tests against the Assinafy sandbox.
//!
//! These tests are `#[ignore]` by default. Run them with:
//!
//! ```bash
//! ASSINAFY_API_KEY=<sandbox key> ASSINAFY_ACCOUNT_ID=<sandbox account> \
//! ASSINAFY_TEST_EMAIL_PRIMARY=<test inbox> \
//! ASSINAFY_TEST_EMAIL_SECONDARY=<test inbox> \
//!   cargo test --test sandbox -- --ignored --test-threads=1
//! ```

mod common;

use assinafy::models::{AssignmentMethod, NotificationMethod, VerificationMethod};
use assinafy::resources::{
    CreateAssignmentSigner, CreateFieldBody, CreateSignerBody, CreateTagBody,
    CreateTemplateRequest, DocumentStatsQuery, EstimateAssignmentCostBody, LegacySendTokenBody,
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

fn unique_email(variable: &str) -> String {
    let address =
        std::env::var(variable).unwrap_or_else(|_| panic!("{variable} is required for live tests"));
    let (local, domain) = address
        .rsplit_once('@')
        .filter(|(local, domain)| !local.is_empty() && !domain.is_empty())
        .unwrap_or_else(|| panic!("{variable} must contain a valid test email address"));
    format!("{local}+{}@{domain}", unique("rust-sdk"))
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
    let email = unique_email("ASSINAFY_TEST_EMAIL_PRIMARY");
    let created = signers
        .create(&CreateSignerBody::new(&full_name).email(&email))
        .await
        .expect("create signer");

    let new_name = format!("{full_name} (updated)");
    let updated_email = unique_email("ASSINAFY_TEST_EMAIL_SECONDARY");
    let workflow: assinafy::Result<_> = async {
        let fetched = signers.get(&created.id).await?;
        let updated = signers
            .update(
                &created.id,
                &UpdateSignerBody::new()
                    .full_name(&new_name)
                    .email(&updated_email),
            )
            .await?;
        let page = signers
            .list()
            .per_page(100)
            .search(&new_name)
            .send()
            .await?;
        Ok((fetched, updated, page))
    }
    .await;

    signers.delete(&created.id).await.expect("delete signer");
    let (fetched, updated, page) = workflow.expect("signer workflow");

    assert_eq!(created.full_name, full_name);
    assert_eq!(created.email.as_deref(), Some(email.as_str()));
    assert_eq!(fetched.id, created.id);
    assert_eq!(updated.full_name, new_name);
    assert_eq!(updated.email.as_deref(), Some(updated_email.as_str()));
    assert!(
        page.data.iter().any(|s| s.id == created.id),
        "newly created signer should appear in search results"
    );
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
    let workflow: assinafy::Result<_> = async {
        let updated = tags
            .update(&created.id, &UpdateTagBody::new().color("ff6633"))
            .await?;
        let page = tags.list().search(&name).send().await?;
        Ok((updated, page))
    }
    .await;

    assert!(tags.delete(&created.id).await.expect("delete tag"));
    let (updated, page) = workflow.expect("tag workflow");
    assert_eq!(created.name, name);
    assert_eq!(updated.color.as_deref(), Some("ff6633"));
    assert!(page.data.iter().any(|tag| tag.id == created.id));
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
    let new_name = format!("{name} Updated");
    let workflow: assinafy::Result<_> = async {
        let fetched = fields.get(&created.id).await?;
        let valid = fields.validate(&created.id, "ABC").await?;
        let updated = fields
            .update(
                &created.id,
                &UpdateFieldBody::new()
                    .name(&new_name)
                    .clear_regex()
                    .required(false),
            )
            .await?;
        let all = fields.list().send().await?;
        Ok((fetched, valid, updated, all))
    }
    .await;

    fields.delete(&created.id).await.expect("delete field");
    let (fetched, valid, updated, all) = workflow.expect("field workflow");
    assert_eq!(created.name, name);
    assert_eq!(created.kind, "text");
    assert_eq!(fetched.id, created.id);
    assert!(valid.success, "expected valid field value: {valid:?}");
    assert_eq!(updated.name, new_name);
    assert!(all.iter().any(|field| field.id == created.id));
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
    let workflow = wait_until_deletable(&client, &doc.id, &doc.status).await;
    client
        .documents()
        .delete(&doc.id)
        .await
        .expect("delete uploaded document");
    workflow.expect("wait for uploaded document");
    assert!(!doc.id.is_empty(), "uploaded doc has id");
    assert!(
        doc.artifacts.contains_key("original"),
        "upload response should expose the original artifact url"
    );
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
    let (client, account_id) = sandbox_or_skip!();
    let docs = client.documents();

    let upload =
        UploadDocumentRequest::from_bytes(format!("{}.pdf", unique("rust-rename")), minimal_pdf());
    let doc = docs.upload(&account_id, upload).await.expect("upload");

    let new_name = format!("{}.pdf", unique("rust-renamed"));
    let workflow: assinafy::Result<_> = async {
        wait_until_deletable(&client, &doc.id, &doc.status).await?;
        let renamed = docs.rename(&doc.id, &new_name).await?;
        let page = docs
            .search(
                &account_id,
                SearchDocumentsRequest::new("rust-renamed").per_page(5),
            )
            .await?;
        Ok((renamed, page))
    }
    .await;
    docs.delete(&doc.id).await.expect("delete document");
    let (renamed, page) = workflow.expect("rename and search workflow");
    assert_eq!(renamed.name, new_name);
    assert!(page.meta.current_page.is_some(), "search is paginated");
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
    let workflow: assinafy::Result<_> = async {
        let mut fetched = templates.get(&created.id).await?;
        let mut tries = 0;
        while matches!(
            fetched.status,
            TemplateStatus::Processing | TemplateStatus::Uploading | TemplateStatus::Uploaded
        ) {
            if tries >= 30 {
                return Err(assinafy::Error::UnexpectedResponse(format!(
                    "template {} stuck in {}",
                    created.id, fetched.status
                )));
            }
            tokio::time::sleep(Duration::from_secs(1)).await;
            fetched = templates.get(&created.id).await?;
            tries += 1;
        }
        Ok(fetched)
    }
    .await;
    templates
        .delete(&created.id)
        .await
        .expect("delete template");
    let fetched = workflow.expect("template processing workflow");
    assert!(!created.id.is_empty(), "template has an id");
    assert_eq!(fetched.id, created.id);
}

#[tokio::test]
#[ignore = "hits live sandbox"]
async fn users_self_returns_profile() {
    let (client, _account_id) = sandbox_or_skip!();
    let me = client.users().me().await.expect("users/self");
    assert!(!me.email.is_empty(), "authenticated user has an email");
}

#[tokio::test]
#[ignore = "hits live sandbox"]
async fn production_only_stats_and_preferences_are_absent_from_sandbox() {
    let (client, account_id) = sandbox_or_skip!();
    let query = DocumentStatsQuery::monthly();
    let errors = [
        client
            .account(&account_id)
            .stats(&query)
            .await
            .expect_err("account stats should be absent from sandbox"),
        client
            .users()
            .stats(&query)
            .await
            .expect_err("user stats should be absent from sandbox"),
        client
            .users()
            .notification_preferences()
            .await
            .expect_err("notification preferences should be absent from sandbox"),
    ];
    assert!(errors.iter().all(|error| error.status() == Some(404)));
}

#[tokio::test]
#[ignore = "hits live sandbox"]
async fn document_tags_attach_by_name_and_detach_by_id() {
    let (client, account_id) = sandbox_or_skip!();
    let docs = client.documents();
    let tags = client.tags(&account_id);

    let upload =
        UploadDocumentRequest::from_bytes(format!("{}.pdf", unique("rust-doctag")), minimal_pdf());
    let doc = docs.upload(&account_id, upload).await.expect("upload");
    let name = unique("rust-doctag");
    let replacement = unique("rust-doctag-set");
    let workflow: assinafy::Result<_> = async {
        wait_until_deletable(&client, &doc.id, &doc.status).await?;
        let attached = tags.add_to_document(&doc.id, [name.as_str()]).await?;
        let created_id = attached
            .iter()
            .find(|tag| tag.name == name)
            .map(|tag| tag.id.clone())
            .ok_or_else(|| {
                assinafy::Error::UnexpectedResponse("attached tag was not returned".into())
            })?;
        let listed = tags.list_for_document(&doc.id).await?;
        let after_set = tags
            .set_on_document(&doc.id, [replacement.as_str()])
            .await?;
        let mut all_detached = true;
        for tag in tags.list_for_document(&doc.id).await? {
            all_detached &= tags.remove_from_document(&doc.id, &tag.id).await?;
        }
        let remaining = tags.list_for_document(&doc.id).await?;
        Ok((created_id, listed, after_set, all_detached, remaining))
    }
    .await;

    let mut cleanup_errors = Vec::new();
    if let Err(error) = docs.delete(&doc.id).await {
        cleanup_errors.push(format!("delete document: {error}"));
    }
    for cleanup_name in [&name, &replacement] {
        match tags.list().search(cleanup_name).send().await {
            Ok(page) => {
                for tag in page.data.iter().filter(|tag| &tag.name == cleanup_name) {
                    if let Err(error) = tags.delete_with_force(&tag.id, true).await {
                        cleanup_errors.push(format!("delete tag {}: {error}", tag.id));
                    }
                }
            }
            Err(error) => cleanup_errors.push(format!("find tag {cleanup_name}: {error}")),
        }
    }
    assert!(
        cleanup_errors.is_empty(),
        "cleanup failed: {cleanup_errors:?}"
    );

    let (created_id, listed, after_set, all_detached, remaining) =
        workflow.expect("document tag workflow");
    assert!(listed.iter().any(|tag| tag.id == created_id));
    assert!(after_set.iter().any(|tag| tag.name == replacement));
    assert!(all_detached);
    assert!(remaining.is_empty());
}

#[tokio::test]
#[ignore = "hits live sandbox"]
async fn download_artifact_returns_raw_bytes_and_redirects_thumbnail() {
    use assinafy::models::ArtifactName;

    let (client, account_id) = sandbox_or_skip!();
    let docs = client.documents();

    let upload =
        UploadDocumentRequest::from_bytes(format!("{}.pdf", unique("rust-dl")), minimal_pdf());
    let doc = docs.upload(&account_id, upload).await.expect("upload");
    let workflow: assinafy::Result<_> = async {
        wait_until_deletable(&client, &doc.id, &doc.status).await?;
        let original = docs
            .download_artifact(&doc.id, ArtifactName::Original)
            .await?;
        let via_artifact = docs
            .download_artifact(&doc.id, ArtifactName::Thumbnail)
            .await;
        let via_thumbnail = docs.download_thumbnail(&doc.id).await;
        Ok((original, via_artifact, via_thumbnail))
    }
    .await;
    docs.delete(&doc.id).await.expect("delete document");
    let ((bytes, content_type), via_artifact, via_thumbnail) = workflow.expect("download workflow");
    assert!(!bytes.is_empty(), "artifact bytes are non-empty");
    assert!(content_type.contains("pdf"));
    assert_eq!(via_artifact.is_ok(), via_thumbnail.is_ok());
}

#[tokio::test]
#[ignore = "hits live sandbox"]
async fn assignment_lifecycle_covers_estimate_get_resend_and_reset() {
    let (client, account_id) = sandbox_or_skip!();
    let docs = client.documents();
    let signers = client.signers(&account_id);
    let assignments = client.assignments();

    let email = unique_email("ASSINAFY_TEST_EMAIL_PRIMARY");

    let upload = UploadDocumentRequest::from_bytes(
        format!("{}.pdf", unique("rust-assignment")),
        minimal_pdf(),
    );
    let doc = docs.upload(&account_id, upload).await.expect("upload");
    if let Err(error) = wait_until_deletable(&client, &doc.id, &doc.status).await {
        let cleanup = docs.delete(&doc.id).await;
        panic!("wait for assignment document: {error}; cleanup: {cleanup:?}");
    }

    let signer = match signers
        .create(&CreateSignerBody::new(unique("Rust SDK Assignment Signer")).email(email.as_str()))
        .await
    {
        Ok(signer) => signer,
        Err(error) => {
            docs.delete(&doc.id)
                .await
                .expect("clean up document after signer creation failure");
            panic!("create assignment signer: {error}");
        }
    };

    let body = EstimateAssignmentCostBody::from_signers(
        AssignmentMethod::Virtual,
        [CreateAssignmentSigner::new(&signer.id)
            .verification_method(VerificationMethod::Email)
            .notification_methods(vec![NotificationMethod::Email])],
    )
    .message("Rust SDK sandbox contract test");

    const EXPIRES_AT: &str = "2099-12-31T23:59:59Z";
    let workflow: assinafy::Result<_> = async {
        let estimate = assignments.estimate_cost(&doc.id, &body).await?;
        let assignment = assignments.create(&doc.id, &body).await?;
        let fetched_document = docs.get(&doc.id).await?;
        let reset = assignments
            .reset_expiration_at(&doc.id, &assignment.id, EXPIRES_AT)
            .await?;
        let resend_estimate = assignments
            .estimate_resend_cost(&doc.id, &assignment.id, &signer.id)
            .await?;
        let resend = assignments
            .resend_to_signer(&doc.id, &assignment.id, &signer.id)
            .await?;
        let public_document = client.public().document(&doc.id).await?;
        #[allow(deprecated)]
        client
            .public()
            .send_token_legacy(&doc.id, &LegacySendTokenBody::email(&email))
            .await?;

        Ok((
            estimate,
            assignment,
            fetched_document,
            reset,
            resend_estimate,
            resend,
            public_document,
        ))
    }
    .await;

    // Both cleanup calls run even when an operation above fails.
    let document_cleanup = docs.delete(&doc.id).await;
    let signer_cleanup = signers.delete(&signer.id).await;
    document_cleanup.expect("delete assignment test document");
    signer_cleanup.expect("delete assignment test signer");

    let (estimate, assignment, fetched, reset, resend_estimate, resend, public) =
        workflow.expect("assignment workflow");
    assert_eq!(estimate.documents, 1.0, "one document is consumed");
    assert!(
        estimate.document_balance >= 0.0,
        "document balance is reported"
    );
    assert!(assignment.signers.iter().any(|item| item.id == signer.id));
    assert_eq!(
        fetched.assignment.as_ref().map(|item| item.id.as_str()),
        Some(assignment.id.as_str())
    );
    assert_eq!(reset.id, assignment.id);
    assert!(reset.expires_at.is_some(), "reset expiration is returned");
    assert!(
        resend_estimate.has_sufficient_resources || resend_estimate.has_sufficient_credits,
        "resend estimate reports sufficient resources"
    );
    assert!(resend.is_sent, "resend notification was sent");
    assert_eq!(resend.document_id.as_deref(), Some(doc.id.as_str()));
    assert_eq!(resend.signer_id.as_deref(), Some(signer.id.as_str()));
    assert_eq!(public.id, doc.id);
}

/// Poll a freshly uploaded document until it reaches a deletable status.
async fn wait_until_deletable(
    client: &assinafy::Client,
    document_id: &str,
    initial: &assinafy::models::DocumentStatus,
) -> assinafy::Result<()> {
    use assinafy::models::DocumentStatus;
    use std::time::Duration;

    let statuses = client.documents().statuses().await?;
    let is_deletable = |s: &DocumentStatus| {
        statuses
            .iter()
            .any(|info| info.code == *s && info.deletable)
    };

    let mut status = initial.clone();
    let mut tries = 0;
    while !is_deletable(&status) {
        if tries >= 30 {
            return Err(assinafy::Error::UnexpectedResponse(format!(
                "document {document_id} stuck in non-deletable status {status}"
            )));
        }
        tokio::time::sleep(Duration::from_secs(1)).await;
        status = client.documents().get(document_id).await?.status;
        tries += 1;
    }
    Ok(())
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
