//! Upload a PDF as a document.
//!
//! ```bash
//! ASSINAFY_API_KEY=... ASSINAFY_ACCOUNT_ID=... \
//!   cargo run --example upload_document -- ./contract.pdf
//! ```

use std::path::PathBuf;

use assinafy::Client;
use assinafy::resources::UploadDocumentRequest;

#[tokio::main]
async fn main() -> assinafy::Result<()> {
    let api_key = std::env::var("ASSINAFY_API_KEY").expect("set ASSINAFY_API_KEY");
    let account_id = std::env::var("ASSINAFY_ACCOUNT_ID").expect("set ASSINAFY_ACCOUNT_ID");
    let path: PathBuf = std::env::args()
        .nth(1)
        .expect("usage: upload_document <path-to-pdf>")
        .into();

    let client = Client::builder().api_key(api_key).sandbox().build()?;
    let upload = UploadDocumentRequest::from_path(&path).await?;
    let doc = client.documents().upload(&account_id, upload).await?;

    println!(
        "uploaded `{}` (id={} status={}, {} pages)",
        doc.name,
        doc.id,
        doc.status,
        doc.pages.len()
    );
    Ok(())
}
