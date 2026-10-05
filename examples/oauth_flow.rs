//! Walk the OAuth 2.1 authorization-code flow with PKCE.
//!
//! Steps 1 and 2 (discovery and building the authorization URL) need no
//! credentials, so running the example with only `ASSINAFY_OAUTH_CLIENT_ID`
//! set prints a URL you can open in a browser. Paste the full address the
//! browser comes back to and the example checks it and exchanges the code; an
//! empty line stops after the URL.
//!
//! ```bash
//! ASSINAFY_OAUTH_CLIENT_ID=my-client-id \
//! ASSINAFY_OAUTH_REDIRECT_URI=https://app.example.invalid/callback \
//!   cargo run --example oauth_flow
//! ```
//!
//! The OAuth endpoints are served by production, which is the default base URL.

use std::collections::HashMap;

use assinafy::resources::{AuthorizationRequest, PkceChallenge, TokenRequest, scope};
use assinafy::{Auth, Client};

/// The `iss` every Assinafy authorization response carries (RFC 9207).
const ISSUER: &str = "https://auth.assinafy.com.br";

#[tokio::main]
async fn main() -> assinafy::Result<()> {
    let _ = dotenvy::dotenv();
    let client_id = std::env::var("ASSINAFY_OAUTH_CLIENT_ID")
        .expect("set ASSINAFY_OAUTH_CLIENT_ID to your registered client id");
    let redirect_uri = std::env::var("ASSINAFY_OAUTH_REDIRECT_URI")
        .unwrap_or_else(|_| "https://app.example.invalid/callback".to_owned());

    let client = Client::builder().build()?;

    // 1. Discovery: the API names its authorization server, which in turn
    //    names the browser-facing authorization endpoint.
    let resource = client.oauth().protected_resource_metadata().await?;
    println!("resource            : {}", resource.resource);
    println!(
        "scopes supported    : {}",
        resource.scopes_supported.join(", ")
    );

    let issuer = resource
        .authorization_servers
        .first()
        .expect("the API must name at least one authorization server");
    let server = client.oauth().authorization_server_metadata(issuer).await?;
    println!("authorization url : {}", server.authorization_endpoint);
    println!("token endpoint      : {}", server.token_endpoint);

    // 2. Build the URL the user's browser opens. Keep `pkce` and `state` for
    //    the callback: the verifier proves this client started the flow, and
    //    the state binds the callback to this browser session.
    let pkce = PkceChallenge::generate()?;
    // `state` only has to be unguessable and single-use; a generated PKCE
    // verifier is 32 bytes of operating-system entropy, which serves.
    let state = PkceChallenge::generate()?.verifier().to_owned();
    let authorize_url = AuthorizationRequest::new(&client_id, &redirect_uri, &pkce)
        .scopes([
            scope::DOCUMENTS_READ,
            scope::DOCUMENTS_WRITE,
            scope::OPENID,
            scope::OFFLINE_ACCESS,
        ])
        .state(&state)
        .resource(&resource.resource)
        .url(&server.authorization_endpoint)?;

    println!("\n3. Open this URL, approve access, then paste the address you return to:");
    println!("{authorize_url}\n");

    let mut callback = String::new();
    std::io::stdin().read_line(&mut callback)?;
    if callback.trim().is_empty() {
        return Ok(());
    }
    let query = callback_parameters(callback.trim(), &redirect_uri, &state)?;
    let param = |name: &str| query.get(name).map(String::as_str);

    if let Some(error) = param("error") {
        let description = param("error_description").unwrap_or_default();
        eprintln!("authorization failed: {error} {description}");
        std::process::exit(1);
    }
    let code = param("code").expect("an approved response carries a code");

    // 4. Exchange the code at once (it expires 60 seconds after approval),
    //    repeating the `resource` sent in step 2.
    let mut request = TokenRequest::authorization_code(&client_id, code, &redirect_uri, &pkce)
        .resource(&resource.resource);
    if let Ok(secret) = std::env::var("ASSINAFY_OAUTH_CLIENT_SECRET") {
        request = request.client_secret(secret);
    }
    let token = client.oauth().token(&request).await?;
    println!(
        "granted scopes      : {}",
        token.scopes().collect::<Vec<_>>().join(", ")
    );
    println!("expires in          : {:?}s", token.expires_in);
    println!("refreshable         : {}", token.refresh_token.is_some());

    // 5. Act as the user. The token replaces the client's credential; it is
    //    never printed, only used. It reaches one workspace, the only one the
    //    account list returns: store its id with the tokens.
    let as_user = client.with_auth(Auth::Bearer(token.access_token.clone()));
    let workspace = &as_user.accounts_api().list().await?[0].id;
    println!("workspace           : {workspace}");
    if token.has_scope(scope::OPENID) {
        let who = as_user.oauth().userinfo().await?;
        println!("acting for          : {} ({:?})", who.sub, who.name);
    }

    Ok(())
}

fn callback_parameters(
    callback: &str,
    redirect_uri: &str,
    state: &str,
) -> assinafy::Result<HashMap<String, String>> {
    let callback = url::Url::parse(callback)?;
    let registered = url::Url::parse(redirect_uri)?;
    if callback.origin() != registered.origin() || callback.path() != registered.path() {
        return Err(assinafy::Error::Validation("callback URI mismatch".into()));
    }
    let mut query = HashMap::new();
    for (key, value) in callback.query_pairs().into_owned() {
        if query.insert(key, value).is_some() {
            return Err(assinafy::Error::Validation(
                "duplicate callback parameter".into(),
            ));
        }
    }
    if query.get("state").map(String::as_str) != Some(state)
        || query.get("iss").map(String::as_str) != Some(ISSUER)
    {
        return Err(assinafy::Error::Validation(
            "callback state or issuer mismatch".into(),
        ));
    }
    Ok(query)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn callback_rejects_wrong_uri_state_issuer_and_duplicate_parameters() {
        let base = "https://app.example.invalid/callback";
        let valid = format!("{base}?code=example-redacted-code&state=expected&iss={ISSUER}");
        assert_eq!(
            callback_parameters(&valid, base, "expected").unwrap()["code"],
            "example-redacted-code"
        );
        for invalid in [
            valid.replace("/callback?", "/different?"),
            valid.replace("app.example.invalid", "other.example.invalid"),
            valid.replace("state=expected", "state=other"),
            valid.replace(ISSUER, "https://other.example.invalid"),
            format!("{valid}&code=second"),
            format!("{base}?error=access_denied&state=other&iss={ISSUER}"),
        ] {
            assert!(matches!(
                callback_parameters(&invalid, base, "expected"),
                Err(assinafy::Error::Validation(_))
            ));
        }
        let denied = format!("{base}?error=access_denied&state=expected&iss={ISSUER}");
        assert_eq!(
            callback_parameters(&denied, base, "expected").unwrap()["error"],
            "access_denied"
        );
    }
}
