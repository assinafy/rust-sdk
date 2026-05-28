//! Shared helpers for sandbox integration tests.

use assinafy::Client;

/// Pull required env vars and build a sandbox client.
///
/// Returns `None` if either credential is missing so the test can `return`
/// gracefully (the wrapping macro converts that into a skipped test).
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

/// Skip the body of an `#[ignore]` test if the sandbox env vars are missing.
/// Prints a hint so the runner output is self-explanatory.
#[macro_export]
macro_rules! sandbox_or_skip {
    () => {{
        match $crate::common::sandbox_client() {
            Some(x) => x,
            None => {
                eprintln!("skipping: ASSINAFY_API_KEY / ASSINAFY_ACCOUNT_ID not set");
                return;
            }
        }
    }};
}
