//! Demonstrates the full tag lifecycle: create, list, update, delete.
//!
//! ```bash
//! ASSINAFY_API_KEY=... ASSINAFY_ACCOUNT_ID=... cargo run --example tags_workflow
//! ```

use assinafy::Client;
use assinafy::resources::{CreateTagBody, UpdateTagBody};

#[tokio::main]
async fn main() -> assinafy::Result<()> {
    let api_key = std::env::var("ASSINAFY_API_KEY").expect("set ASSINAFY_API_KEY");
    let account_id = std::env::var("ASSINAFY_ACCOUNT_ID").expect("set ASSINAFY_ACCOUNT_ID");

    let client = Client::builder().api_key(api_key).sandbox().build()?;
    let tags = client.tags(&account_id);

    let unique = format!("rust-sdk-demo-{}", uuid::Uuid::new_v4().simple());
    let created = tags
        .create(&CreateTagBody::new(&unique).color("3399ff"))
        .await?;
    println!("created tag {} ({})", created.name, created.id);

    let workflow: assinafy::Result<_> = async {
        let updated = tags
            .update(&created.id, &UpdateTagBody::new().color("ff9900"))
            .await?;
        let page = tags.list().search(&unique).send().await?;
        Ok((updated, page))
    }
    .await;
    let cleanup = tags.delete(&created.id).await;
    let (updated, page) = workflow?;
    cleanup?;
    println!("updated color to {:?}", updated.color);
    println!("search returned {} tag(s)", page.data.len());
    println!("deleted tag {}", created.id);

    Ok(())
}
