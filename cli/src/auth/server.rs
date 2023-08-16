use std::collections::HashMap;
use std::net::SocketAddr;
use std::sync::Arc;
use std::time::Duration;

use anyhow::{anyhow, Result};
use chrono::{DateTime, Utc};
use futures::TryFutureExt;
use hyper::{Body, Request, Response, Server};
#[cfg(not(test))]
use open;
use phylum_types::types::auth::{AuthorizationCode, RefreshToken};
use rand::distributions::Alphanumeric;
use rand::{thread_rng, Rng};
use reqwest::Url;
use routerify::ext::RequestExt;
use routerify::{Router, RouterService};
use tokio::sync::oneshot::{self, Sender};
use tokio::sync::Mutex;

use super::oidc::{
    acquire_tokens, build_auth_url, check_if_routable, fetch_locksmith_server_settings, AuthAction,
    ChallengeCode, CodeVerifier, LocksmithServerSettings,
};
#[cfg(test)]
use crate::test::open;

pub const AUTH_CALLBACK_TEMPLATE: &str = include_str!("./auth_callback_template.html");

// State to store the auth code
// Not high concurrency, so using a simple mutex
#[derive(Clone)]
struct AuthCodeState(Arc<Mutex<Option<String>>>);

// State to store the oauth2 state parameter so it can be set and checked in the
// callback
#[derive(Clone)]
struct OAuth2CallbackState(Arc<String>);

// State to store the shutdown hook state
struct ShutdownHookState(Mutex<Option<Sender<()>>>);

/// Handler to be used as the GET endpoint that keycloak redirects to.
///
/// This handler tries to parse the request and extract the code.
///
/// If a code is present, it updates the internal state and stores the code in
/// it
async fn keycloak_callback_handler(request: Request<Body>) -> Result<Response<Body>> {
    log::debug!("Callback handler triggered!");

    let shutdown_hook =
        request.data::<ShutdownHookState>().expect("Shutdown hook not set as hyper state");

    let auth_code: &AuthCodeState =
        request.data::<AuthCodeState>().expect("State for holding auth code not set");

    let saved_state: &OAuth2CallbackState = request
        .data::<OAuth2CallbackState>()
        .expect("oauth2 XSRF prevention state parameter was not set");

    log::debug!("Oauth server has called redirect uri: {}", request.uri());

    let query_parameters: HashMap<String, String> = request
        .uri()
        .query()
        .map(|v| url::form_urlencoded::parse(v.as_bytes()).into_owned().collect())
        .unwrap_or_default();

    // Check that XSRF prevention state was properly returned.
    match query_parameters.get("state") {
        None => {
            let msg = "Oauth server did return XSRF prevention state nonce";
            log::error!("{}", msg);
            return Err(anyhow!(msg));
        },
        Some(state) => {
            if *state != *saved_state.0 {
                let msg = "OAuth server returned wrong XSRF prevention state nonce";
                log::error!("{}", msg);
                return Err(anyhow!(msg));
            }
        },
    };

    let response_body = match query_parameters.get("code") {
        None => {
            log::error!(
                "Encountered error during auth response\n  Error: {} :{}",
                query_parameters.get("error").unwrap_or(&"".to_owned()),
                query_parameters.get("error_description").unwrap_or(&"".to_owned())
            );
            AUTH_CALLBACK_TEMPLATE
                .replace("{{}}", "Login / Registration failed, did not get authorization code")
        },
        Some(code) => {
            log::debug!("Authoriztion successful, acquired authorization code");
            let mut lock = auth_code.0.lock().await;
            *lock = Some(code.to_owned());
            AUTH_CALLBACK_TEMPLATE.replace("{{}}", "Login / Registration succeeded")
        },
    };

    let response = Response::builder()
        .status(200)
        .header("Content-Type", "text/html")
        .header("Cache-Control", "no-cache")
        .body(response_body.into())?;

    // Schedule shutdown of the hyper server
    let mut shutdown_lock = shutdown_hook.0.lock().await;
    if let Some(sender) = (*shutdown_lock).take() {
        // Slight delay to ensure we send a browser response before the server shuts
        // down...
        tokio::spawn(async {
            tokio::time::sleep(Duration::from_millis(250)).await;
            match sender.send(()) {
                Err(error) => log::error!("Failed to send hyper shutdown signal: {:?}", error),
                _ => log::debug!("Sent hyper server shutdown signal"),
            }
        });
    } else {
        return Err(anyhow!("Missing shutdown hook, can't shut down."));
    }

    // Return the response
    Ok(response)
}

/// Spawn a server to redirect users to either login or register,
/// and return an authorization code and the callback uri of THIS client
/// which then need to passed on to the /token endpoint to obtain tokens
async fn spawn_server_and_get_auth_code(
    locksmith_settings: &LocksmithServerSettings,
    redirect_type: AuthAction,
    code_challenge: &ChallengeCode,
    state: impl AsRef<str> + 'static,
) -> Result<(AuthorizationCode, Url)> {
    // Oneshot channel to shutdown server
    let (send_shutdown, receive_shutdown) = oneshot::channel::<()>();

    let auth_code_state = AuthCodeState(Arc::new(Mutex::new(None::<String>)));

    // Router
    let router = Router::builder()
        // Place to store Auth Code once acquired
        .data(auth_code_state.clone())
        // Shutdown oneshot channel
        .data(ShutdownHookState(Mutex::new(Some(send_shutdown))))
        .data(OAuth2CallbackState(Arc::new(state.as_ref().to_owned())))
        .get("/", keycloak_callback_handler)
        .build()
        .expect("Failed to build router");
    let router_service = RouterService::new(router).expect("Failed to build router service");

    // Fire up on a random port
    // In keycloak, configure redirect_uri with a pattern of http://127.0.0.1:*
    let addr = SocketAddr::from(([127, 0, 0, 1], 0));
    let server = Server::bind(&addr).serve(router_service);
    let server_address = server.local_addr();

    let callback_url = Url::parse(&format!("http://{}/", &server_address))?;

    log::debug!("Started local login server: {:?}", server_address);

    // Set graceful shutdown hook
    let finished_serving = server.with_graceful_shutdown(async {
        receive_shutdown.await.ok();
    });

    let authorization_url =
        build_auth_url(redirect_type, locksmith_settings, &callback_url, code_challenge, state)?;

    log::debug!("Authorization url is {}", authorization_url);

    // If this routable beyond the local segment / interface / host / loopback
    // and protocol is still http we are going to throw an error because
    // something is misconfigured.
    let auth_host = authorization_url
        .host_str()
        .ok_or_else(|| anyhow!("Authorization server url must be absolute"))?;
    let auth_scheme = authorization_url.scheme();
    let fallback_port: u16 = if auth_scheme == "https" { 443 } else { 80 };
    let port = authorization_url.port().unwrap_or(fallback_port);
    let is_routable = check_if_routable(format!("{auth_host}:{port}"))?;
    if is_routable && auth_scheme == "http" {
        return Err(anyhow!(
            "Authorization host {} is publically routable, must use https to connect.",
            auth_host
        ));
    }

    println!("Please use browser window to complete login process");
    println!(
        "If browser window doesn't open, you can use the link below:\n    {authorization_url}"
    );

    // Open browser pointing at this server's /redirect url
    // We don't want to join on this, might not even make sense.
    if let Err(e) = open::that(authorization_url.as_ref()) {
        log::debug!("Could not open browser: {}", e);
    } else {
        log::debug!("Opened browser window");
    }

    let auth_code = finished_serving
        .map_err(anyhow::Error::from)
        .and_then(|_| async {
            let mut lock = auth_code_state.0.lock().await;
            match (*lock).take() {
                None => Err(anyhow!("Failed to get auth code")),
                Some(auth_code) => Ok(auth_code),
            }
        })
        .await?;

    Ok((AuthorizationCode::new(auth_code), callback_url))
}

/// Handle the user login/registration flow.
pub async fn handle_auth_flow(
    auth_action: AuthAction,
    token_name: Option<String>,
    expiry: Option<DateTime<Utc>>,
    ignore_certs: bool,
    api_uri: &str,
) -> Result<RefreshToken> {
    let locksmith_settings = fetch_locksmith_server_settings(ignore_certs, api_uri).await?;
    let (code_verifier, challenge_code) = CodeVerifier::generate(64)?;
    let state: String = thread_rng().sample_iter(&Alphanumeric).take(32).map(char::from).collect();
    let (auth_code, callback_url) =
        spawn_server_and_get_auth_code(&locksmith_settings, auth_action, &challenge_code, state)
            .await?;
    acquire_tokens(
        &locksmith_settings,
        &callback_url,
        &auth_code,
        &code_verifier,
        token_name,
        expiry,
        ignore_certs,
    )
    .await
    .map(|tokens| tokens.token)
}

#[cfg(test)]
mod test {
    use anyhow::Result;
    use rand::distributions::Alphanumeric;
    use rand::{thread_rng, Rng};

    use super::{handle_auth_flow, spawn_server_and_get_auth_code};
    use crate::auth::{AuthAction, CodeVerifier};
    use crate::test::mockito::*;

    #[tokio::test]
    async fn when_started_with_good_configuration_spawn_server_and_get_auth_code_is_successful(
    ) -> Result<()> {
        let locksmith_settings =
            build_locksmith_server_settings_mock_response("https://127.0.0.1/oauth");

        let (_verifier, challenge) =
            CodeVerifier::generate(64).expect("Failed to build PKCE verifier and challenge");

        let state: String =
            thread_rng().sample_iter(&Alphanumeric).take(32).map(char::from).collect();

        spawn_server_and_get_auth_code(&locksmith_settings, AuthAction::Login, &challenge, state)
            .await?;

        Ok(())
    }

    #[tokio::test]
    async fn when_started_with_good_configuration_handle_auth_flow_for_login_is_successful(
    ) -> Result<()> {
        let mock_server = build_mock_server().await;
        let api_uri = mock_server.uri();

        let (_verifier, _challenge) =
            CodeVerifier::generate(64).expect("Failed to build PKCE verifier and challenge");

        let result = handle_auth_flow(AuthAction::Login, None, None, false, &api_uri).await?;

        log::debug!("{:?}", result);

        Ok(())
    }

    #[tokio::test]
    async fn when_started_with_good_configuration_handle_auth_flow_for_register_is_successful(
    ) -> Result<()> {
        let mock_server = build_mock_server().await;
        let api_uri = mock_server.uri();

        let (_verifier, _challenge) =
            CodeVerifier::generate(64).expect("Failed to build PKCE verifier and challenge");

        let result = handle_auth_flow(AuthAction::Register, None, None, false, &api_uri).await?;

        log::debug!("{:?}", result);

        Ok(())
    }
}
