//! Shared helpers for sandbox integration tests.

use assinafy::Client;

/// Pull required env vars and build a sandbox client.
///
/// Returns `None` if either credential is missing.
pub fn sandbox_client() -> Option<(Client, String)> {
    let _ = dotenvy::dotenv();
    let key = std::env::var("ASSINAFY_API_KEY").ok()?;
    let account = std::env::var("ASSINAFY_ACCOUNT_ID").ok()?;
    let client = Client::builder()
        .api_key(key)
        .sandbox()
        .build()
        .expect("client builder");
    Some((client, account))
}

/// Require sandbox credentials when an explicitly ignored live test is run.
#[macro_export]
macro_rules! sandbox_or_skip {
    () => {{
        match $crate::common::sandbox_client() {
            Some(x) => x,
            None => {
                panic!("ASSINAFY_API_KEY and ASSINAFY_ACCOUNT_ID are required for live tests");
            }
        }
    }};
}
