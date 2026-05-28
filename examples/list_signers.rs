//! List signers in a sandbox account.
//!
//! Run with the helper script `./scripts/run_example.sh list_signers` or:
//!
//! ```bash
//! ASSINAFY_API_KEY=... ASSINAFY_ACCOUNT_ID=... \
//!   cargo run --example list_signers
//! ```

use assinafy::Client;

#[tokio::main]
async fn main() -> assinafy::Result<()> {
    tracing_subscriber::fmt()
        .with_env_filter(
            tracing_subscriber::EnvFilter::try_from_default_env().unwrap_or_else(|_| "info".into()),
        )
        .init();

    let api_key = std::env::var("ASSINAFY_API_KEY").expect("set ASSINAFY_API_KEY");
    let account_id = std::env::var("ASSINAFY_ACCOUNT_ID").expect("set ASSINAFY_ACCOUNT_ID");

    let client = Client::builder().api_key(api_key).sandbox().build()?;

    let page = client
        .signers(&account_id)
        .list()
        .per_page(50)
        .send()
        .await?;

    println!(
        "{} signers (page {:?} of {:?}):",
        page.meta.total_count.unwrap_or(0),
        page.meta.current_page,
        page.meta.page_count
    );
    for s in page {
        println!(
            "  {}  {}  email={:?}",
            s.id,
            s.full_name,
            s.email.as_deref().unwrap_or("-")
        );
    }
    Ok(())
}
