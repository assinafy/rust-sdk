//! Shared helpers for live integration tests.

use assinafy::{BaseUrl, Client};

/// Pull required env vars and build a client for the target deployment.
///
/// Returns `None` if any of the three is missing. `ASSINAFY_BASE_URL` has no
/// default on purpose: these tests create accounts and can send real
/// notifications, so the target is always stated explicitly rather than
/// falling back to production.
pub fn live_client() -> Option<(Client, String)> {
    let _ = dotenvy::dotenv();
    let key = std::env::var("ASSINAFY_API_KEY").ok()?;
    let account = std::env::var("ASSINAFY_ACCOUNT_ID").ok()?;
    let base_url = std::env::var("ASSINAFY_BASE_URL").ok()?;
    let client = Client::builder()
        .api_key(key)
        .base_url(BaseUrl::custom(base_url).expect("ASSINAFY_BASE_URL must be a valid https URL"))
        .build()
        .expect("client builder");
    Some((client, account))
}

/// Require live credentials when an explicitly ignored live test is run.
#[macro_export]
macro_rules! live_or_skip {
    () => {{
        match $crate::common::live_client() {
            Some(x) => x,
            None => {
                panic!(
                    "ASSINAFY_API_KEY, ASSINAFY_ACCOUNT_ID and ASSINAFY_BASE_URL \
                     are required for live tests"
                );
            }
        }
    }};
}
