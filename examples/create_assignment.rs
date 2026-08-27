//! Request virtual signatures for a document.
//!
//! ```bash
//! ASSINAFY_API_KEY=... ASSINAFY_DOCUMENT_ID=... ASSINAFY_SIGNER_IDS=id1,id2 \
//!   cargo run --example create_assignment
//! ```

use assinafy::Client;
use assinafy::models::AssignmentMethod;
use assinafy::resources::CreateAssignmentBody;

#[tokio::main]
async fn main() -> assinafy::Result<()> {
    let api_key = std::env::var("ASSINAFY_API_KEY").expect("set ASSINAFY_API_KEY");
    let document_id = std::env::var("ASSINAFY_DOCUMENT_ID").expect("set ASSINAFY_DOCUMENT_ID");
    let signer_ids: Vec<String> = std::env::var("ASSINAFY_SIGNER_IDS")
        .expect("set ASSINAFY_SIGNER_IDS (comma-separated)")
        .split(',')
        .map(|s| s.trim().to_owned())
        .filter(|s| !s.is_empty())
        .collect();

    let client = Client::builder().api_key(api_key).sandbox().build()?;
    let body = CreateAssignmentBody::new(AssignmentMethod::Virtual, signer_ids)
        .message("Please sign this document.");

    let assignment = client.assignments().create(&document_id, &body).await?;

    println!("assignment {} created", assignment.id);
    println!(
        "{} private signing links issued",
        assignment.signing_urls.len()
    );
    for signing_url in &assignment.signing_urls {
        println!("  signer {}", signing_url.signer_id);
    }
    Ok(())
}
