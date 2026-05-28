//! Create a signer in the configured sandbox account.

use assinafy::Client;
use assinafy::resources::CreateSignerBody;

#[tokio::main]
async fn main() -> assinafy::Result<()> {
    let api_key = std::env::var("ASSINAFY_API_KEY").expect("set ASSINAFY_API_KEY");
    let account_id = std::env::var("ASSINAFY_ACCOUNT_ID").expect("set ASSINAFY_ACCOUNT_ID");

    let client = Client::builder().api_key(api_key).sandbox().build()?;

    let body = CreateSignerBody::new("Jane Doe").email("jane.doe@example.com");

    let signer = client.signers(&account_id).create(&body).await?;
    println!("created signer: {} ({})", signer.full_name, signer.id);
    Ok(())
}
