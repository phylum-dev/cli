use std::collections::HashMap;
use std::net::SocketAddr;
use std::sync::Arc;
use std::time::Duration;

use anyhow::anyhow;
use anyhow::Result;
use futures::TryFutureExt;
use hyper::{Body, Request, Response, Server};
use phylum_cli::async_runtime::ASYNC_RUNTIME;
use phylum_cli::config::Config;
use phylum_cli::types::{AuthorizationCode, TokenResponse};
use rand::distributions::Alphanumeric;
use rand::{thread_rng, Rng};
use reqwest::Url;
use routerify::ext::RequestExt;
use routerify::{Router, RouterService};
use tokio::sync::oneshot::{self, Sender};
use tokio::sync::Mutex;

use crate::commands::auth::oidc::acquire_tokens;

use super::oidc::{
    build_auth_url, check_if_routable, fetch_oidc_server_settings, CodeVerifier, OidcServerSettings,
};
use super::oidc::{AuthAction, ChallengeCode};

pub const AUTH_CALLBACK_TEMPLATE: &str = include_str!("./auth_callback_template.html");

// State to store the auth code
// Not high concurrency, so using a simple mutex
#[derive(Clone)]
struct AuthCodeState(Arc<Mutex<Option<String>>>);

// State to store the shutdown hook state
struct ShutdownHookState(Mutex<Option<Sender<()>>>);

/// Handler to be used as the GET endpoint that keycloak redirects to.
///
/// This handler tries to parse the request and extract the code.
///
/// If a code is present, it updates the internal state and stores the code in it
async fn keycloak_callback_handler(request: Request<Body>) -> Result<Response<Body>> {
    let shutdown_hook = request
        .data::<ShutdownHookState>()
        .expect("Shutdown hook not set as hyper state");

    let auth_code: &AuthCodeState = request
        .data::<AuthCodeState>()
        .expect("State for holding auth code not set");

    let query_parameters: HashMap<String, String> = request
        .uri()
        .query()
        .map(|v| {
            url::form_urlencoded::parse(v.as_bytes())
                .into_owned()
                .collect()
        })
        .unwrap_or_else(HashMap::new);

    let response_body = match query_parameters.get("code") {
        None => {
            log::error!(
                "Encountered error during auth response\n  Error: {} :{}",
                query_parameters.get("error").unwrap_or(&"".to_owned()),
                query_parameters
                    .get("error_description")
                    .unwrap_or(&"".to_owned())
            );
            AUTH_CALLBACK_TEMPLATE.replace(
                "{{}}",
                "Login / Registration failed, did not get authorization code",
            )
        }
        Some(code) => {
            log::debug!("Authoriztion successful, acquired authorization code");
            let mut lock = auth_code.0.lock().await;
            *lock = Some(code.to_owned());
            AUTH_CALLBACK_TEMPLATE.replace("{{}}", "Login / Registration succeeded")
        }
    };

    let response = Response::builder()
        .status(200)
        .header("Content-Type", "text/html")
        .header("Cache-Control", "no-cache")
        .body(response_body.into())?;

    // Schedule shutdown of the hyper server
    let mut shutdown_lock = shutdown_hook.0.lock().await;
    if let Some(sender) = (*shutdown_lock).take() {
        // Slight delay to ensure we send a browser response before the server shuts down...
        tokio::spawn(async move {
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
    oidc_settings: &OidcServerSettings,
    redirect_type: &AuthAction,
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
        // Todo callback handler needs to shutdown server
        .get("/", keycloak_callback_handler)
        .build()
        .expect("Failed to build router");
    let router_service = RouterService::new(router).expect("Failed to build router service");

    // Fire up on a random port
    // In keycloak, configure with a random callback pattern of http://localhost:*
    let addr = SocketAddr::from(([127, 0, 0, 1], 0));
    let server = Server::bind(&addr).serve(router_service);
    let server_address = server.local_addr();

    let callback_url = Url::parse(&format!("http://{}/", &server_address))?;

    // Set graceful shutdown hook
    let finished_serving = server.with_graceful_shutdown(async {
        receive_shutdown.await.ok();
    });

    let authorization_url = build_auth_url(
        redirect_type,
        oidc_settings,
        &callback_url,
        code_challenge,
        state,
    )?;

    // If this routable beyond the local segment / interface / host / loopback and protocol is still http
    // we are going to throw an error because something is misconfigured.
    let auth_host = authorization_url
        .host_str()
        .ok_or_else(|| anyhow!("Authorization server url must be absolute"))?;
    let auth_scheme = authorization_url.scheme();
    let is_routable = check_if_routable(auth_host)?;
    if is_routable && auth_scheme == "http" {
        return Err(anyhow!(
            "Authorization host {} is publically routable, must use https to connect.",
            auth_host
        ));
    }

    // Open browser pointing at this server's /redirect url
    // We don't want to join on this, might not even make sense.
    open::that_in_background(&authorization_url.to_string());

    let auth_code = finished_serving
        .map_err(anyhow::Error::from)
        .and_then(|_| async move {
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
pub fn handle_auth_flow(auth_action: &AuthAction, config: &Config) -> Result<TokenResponse> {
    let oidc_settings = fetch_oidc_server_settings(config).await?;
    let (code_verifier, challenge_code) = CodeVerifier::generate(64)?;
    let state: String = thread_rng()
        .sample_iter(&Alphanumeric)
        .take(32)
        .map(char::from)
        .collect();
    let tokens = ASYNC_RUNTIME.block_on(async move {
        let (auth_code, callback_url) =
            spawn_server_and_get_auth_code(&oidc_settings, auth_action, &challenge_code, state)
                .await?;
        acquire_tokens(&oidc_settings, &callback_url, &auth_code, &code_verifier).await?
    })?;
    Ok(tokens)
}
