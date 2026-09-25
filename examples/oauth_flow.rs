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
    let query: HashMap<String, String> = url::Url::parse(callback.trim())?
        .query_pairs()
        .into_owned()
        .collect();
    let param = |name: &str| query.get(name).map(String::as_str);

    // Before anything else, `error=` returns included: the response must carry
    // this attempt's `state` and the Assinafy issuer, or it is not ours.
    if param("state") != Some(state.as_str()) || param("iss") != Some(ISSUER) {
        eprintln!("state or iss mismatch: ignoring this response");
        std::process::exit(1);
    }
    if let Some(error) = param("error") {
        let description = param("error_description").unwrap_or_default();
        eprintln!("authorization failed: {error} {description}");
        std::process::exit(1);
    }
    let code = param("code").expect("an approved response carries a code");

    // 4. Exchange the code at once (it expires 60 seconds after approval),
    //    repeating the `resource` sent in step 2.
    let token = client
        .oauth()
        .token(
            &TokenRequest::authorization_code(&client_id, code, &redirect_uri, &pkce)
                .resource(&resource.resource),
        )
        .await?;
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
