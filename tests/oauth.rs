//! Interactive production OAuth connection test using an HTTPS callback tunnel.
//!
//! Set `ASSINAFY_OAUTH_CLIENT_ID`, `ASSINAFY_OAUTH_REDIRECT_URI` and
//! `ASSINAFY_OAUTH_CALLBACK_ADDR` (a loopback socket), then run:
//! `cargo test --test oauth -- --ignored --nocapture`.

use std::collections::HashMap;
use std::io::{Read, Write};
use std::net::{SocketAddr, TcpListener};
use std::time::{Duration, Instant};

use assinafy::resources::{
    AuthorizationRequest, PkceChallenge, RevokeRequest, TokenRequest, scope,
};
use assinafy::{Auth, Client, Error, Result};

#[tokio::test]
#[ignore = "requires a registered OAuth app, HTTPS tunnel and browser consent"]
async fn connection_exchanges_refreshes_and_revokes_tokens() -> Result<()> {
    let _ = dotenvy::dotenv();
    let env = |name| std::env::var(name).map_err(|_| Error::Config(format!("set {name}")));
    let client_id = env("ASSINAFY_OAUTH_CLIENT_ID")?;
    let redirect_uri = env("ASSINAFY_OAUTH_REDIRECT_URI")?;
    let address: SocketAddr = env("ASSINAFY_OAUTH_CALLBACK_ADDR")?
        .parse()
        .map_err(|_| Error::Config("callback address must be a loopback socket".into()))?;
    if !address.ip().is_loopback() {
        return Err(Error::Config("callback must bind to loopback".into()));
    }
    let listener = TcpListener::bind(address)?;
    listener.set_nonblocking(true)?;
    let client = Client::builder().build()?;
    let metadata = client.oauth().protected_resource_metadata().await?;
    let issuer = metadata
        .authorization_servers
        .first()
        .ok_or_else(|| Error::UnexpectedResponse("missing authorization server".into()))?;
    let server = client.oauth().authorization_server_metadata(issuer).await?;
    let pkce = PkceChallenge::generate()?;
    let state = PkceChallenge::generate()?.verifier().to_owned();
    let authorization = AuthorizationRequest::new(&client_id, &redirect_uri, &pkce)
        .scopes([
            scope::DOCUMENTS_READ,
            scope::ACCOUNT_READ,
            scope::OPENID,
            scope::OFFLINE_ACCESS,
        ])
        .state(&state)
        .resource(&metadata.resource)
        .url(&server.authorization_endpoint)?;
    println!("Open and approve this temporary test connection:\n{authorization}");

    let callback = tokio::task::spawn_blocking(move || -> std::io::Result<String> {
        let deadline = Instant::now() + Duration::from_secs(300);
        while Instant::now() < deadline {
            match listener.accept() {
                Ok((mut stream, _)) => {
                    stream.set_read_timeout(Some(Duration::from_secs(5)))?;
                    let mut bytes = [0; 8192];
                    let mut request = Vec::new();
                    while !request.windows(4).any(|v| v == b"\r\n\r\n") {
                        let read = stream.read(&mut bytes)?;
                        if read == 0 || request.len() + read > 8192 {
                            return Err(std::io::Error::other("invalid callback request"));
                        }
                        request.extend_from_slice(&bytes[..read]);
                    }
                    stream.write_all(b"HTTP/1.1 200 OK\r\nContent-Type: text/plain\r\nCache-Control: no-store\r\nContent-Length: 41\r\nConnection: close\r\n\r\nCallback received. Check the test output.")?;
                    let request = String::from_utf8_lossy(&request);
                    return request
                        .lines()
                        .next()
                        .and_then(|line| line.strip_prefix("GET "))
                        .and_then(|line| line.strip_suffix(" HTTP/1.1"))
                        .map(str::to_owned)
                        .ok_or_else(|| std::io::Error::other("invalid callback request line"));
                }
                Err(error) if error.kind() == std::io::ErrorKind::WouldBlock => {
                    std::thread::sleep(Duration::from_millis(100));
                }
                Err(error) => return Err(error),
            }
        }
        Err(std::io::Error::new(std::io::ErrorKind::TimedOut, "browser consent timed out"))
    })
    .await
    .map_err(|_| Error::UnexpectedResponse("callback listener failed".into()))??;
    let callback = url::Url::parse(&redirect_uri)?.join(&callback)?;
    let registered = url::Url::parse(&redirect_uri)?;
    if callback.origin() != registered.origin() || callback.path() != registered.path() {
        return Err(Error::Validation("callback URI mismatch".into()));
    }
    let mut params = HashMap::new();
    for (key, value) in callback.query_pairs().into_owned() {
        if params.insert(key, value).is_some() {
            return Err(Error::Validation("duplicate callback parameter".into()));
        }
    }
    if params.get("state") != Some(&state) || params.get("iss") != Some(issuer) {
        return Err(Error::Validation(
            "callback state or issuer mismatch".into(),
        ));
    }
    let code = params
        .get("code")
        .ok_or_else(|| Error::Validation("authorization was not approved".into()))?;
    let secret = std::env::var("ASSINAFY_OAUTH_CLIENT_SECRET").ok();
    let authenticate = |body: TokenRequest| match &secret {
        Some(secret) => body.client_secret(secret),
        None => body,
    };
    let mut token = client
        .oauth()
        .token(&authenticate(
            TokenRequest::authorization_code(&client_id, code, &redirect_uri, &pkce)
                .resource(&metadata.resource),
        ))
        .await?;
    let workflow: Result<_> = async {
        let as_user = client.with_auth(Auth::Bearer(token.access_token.clone()));
        let accounts = as_user.accounts_api().list().await?;
        let workspace = accounts
            .first()
            .ok_or_else(|| Error::UnexpectedResponse("missing authorized workspace".into()))?;
        as_user
            .documents()
            .list(&workspace.id, Default::default())
            .await?;
        let user = as_user.oauth().userinfo().await?;
        let refresh = token.refresh_token.as_deref().ok_or_else(|| {
            Error::UnexpectedResponse("offline access did not issue a refresh token".into())
        })?;
        let fresh = client
            .oauth()
            .token(&authenticate(
                TokenRequest::refresh_token(&client_id, refresh).resource(&metadata.resource),
            ))
            .await?;
        let rotated = fresh.refresh_token != token.refresh_token;
        token = fresh;
        let refreshed_accounts = client
            .with_auth(Auth::Bearer(token.access_token.clone()))
            .accounts_api()
            .list()
            .await?;
        Ok((accounts, user, refreshed_accounts, rotated))
    }
    .await;
    let mut revoke = match &token.refresh_token {
        Some(refresh) => RevokeRequest::refresh_token(&client_id, refresh),
        None => RevokeRequest::access_token(&client_id, &token.access_token),
    };
    if let Some(secret) = secret {
        revoke = revoke.client_secret(secret);
    }
    client.oauth().revoke(&revoke).await?;
    let (accounts, user, refreshed_accounts, rotated) = workflow?;
    assert_eq!(accounts.len(), 1);
    assert_eq!(refreshed_accounts.len(), 1);
    assert_eq!(accounts[0].id, refreshed_accounts[0].id);
    assert!(!user.sub.is_empty());
    assert!(rotated);
    assert!(!token.has_scope(scope::OFFLINE_ACCESS));
    let rejected = client
        .with_auth(Auth::Bearer(token.access_token))
        .accounts_api()
        .list()
        .await
        .expect_err("revoked access token must be refused");
    assert_eq!(rejected.status(), Some(401));
    Ok(())
}
