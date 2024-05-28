use std::sync::Arc;

use anyhow::{anyhow, Result};
use axum::body::Body;
use axum::extract::{Query, State};
use axum::http::response::Response;
use axum::http::StatusCode;
use axum::response::IntoResponse;
use axum::routing::get;
use axum::Router;
use chrono::{DateTime, Utc};
use log::{debug, error};
use phylum_types::types::auth::{AuthorizationCode, RefreshToken};
use rand::distributions::Alphanumeric;
use rand::{thread_rng, Rng};
use reqwest::Url;
use serde::Deserialize;
use tokio::net::TcpListener;
use tokio::sync::mpsc::{self, Sender};
use tokio::sync::Mutex;

use super::oidc::{
    acquire_tokens, build_auth_url, check_if_routable, fetch_locksmith_server_settings, AuthAction,
    ChallengeCode, CodeVerifier, LocksmithServerSettings,
};
#[cfg(test)]
use crate::test::open;

pub const AUTH_CALLBACK_TEMPLATE: &str = include_str!("./auth_callback_template.html");

/// Auth server state.
struct ServerState {
    /// Auth code return value.
    auth_code: Mutex<Option<String>>,
    /// OAuth 2 state parameter to check in the callback.
    oauth2_callback_state: String,
    /// Shutdown channel.
    shutdown: Sender<()>,
}

/// Auth callback query parameters.
#[derive(Deserialize)]
struct AuthQuery {
    state: Option<String>,
    code: Option<String>,
    error: Option<String>,
    error_description: Option<String>,
}

/// Handler to be used as the GET endpoint that keycloak redirects to.
///
/// If a code is present, it is stored in the server's state.
async fn keycloak_callback_handler(
    State(state): State<Arc<ServerState>>,
    Query(query): Query<AuthQuery>,
) -> Response<Body> {
    debug!("Callback handler triggered!");

    // TODO: Necessary?
    // log::debug!("Oauth server has called redirect uri: {}", request.uri());

    // Check that XSRF prevention state was properly returned.
    match query.state {
        Some(nonce) if nonce != state.oauth2_callback_state => {
            let msg = "OAuth server returned wrong XSRF prevention state nonce";
            error!("{msg}");
            return (StatusCode::INTERNAL_SERVER_ERROR, msg).into_response();
        },
        Some(_) => (),
        None => {
            let msg = "Oauth server did return XSRF prevention state nonce";
            error!("{msg}");
            return (StatusCode::INTERNAL_SERVER_ERROR, msg).into_response();
        },
    };

    // Construct response body.
    let response_body = match query.code {
        Some(code) => {
            debug!("Authoriztion successful, acquired authorization code");
            *state.auth_code.lock().await = Some(code);
            AUTH_CALLBACK_TEMPLATE.replace("{{}}", "Login / Registration succeeded")
        },
        None => {
            error!(
                "Encountered error during auth response\n  Error: {} :{}",
                query.error.unwrap_or_default(),
                query.error_description.unwrap_or_default(),
            );
            AUTH_CALLBACK_TEMPLATE
                .replace("{{}}", "Login / Registration failed, did not get authorization code")
        },
    };

    let response = Response::builder()
        .status(200)
        .header("Content-Type", "text/html")
        .header("Cache-Control", "no-cache")
        .body(response_body.into());
    let response = match response {
        Ok(response) => response,
        Err(err) => {
            let msg = format!("Could not build auth server response: {err}");
            error!("{msg}");
            return (StatusCode::INTERNAL_SERVER_ERROR, msg).into_response();
        },
    };

    // Shutdown the web server.
    if let Err(err) = state.shutdown.send(()).await {
        error!("Failed to shutdown auth server: {err}");
    }

    // Return the response.
    response
}

async fn spawn_server_and_get_auth_code(
    locksmith_settings: &LocksmithServerSettings,
    redirect_type: AuthAction,
    code_challenge: &ChallengeCode,
    state: impl Into<String>,
    port: u16,
) -> Result<(AuthorizationCode, Url)> {
    let auth_address = format!("127.0.0.1:{port}");

    // Get OIDC auth url.
    let state = state.into();
    let callback_url = Url::parse(&format!("http://{}/", auth_address))?;
    let authorization_url =
        build_auth_url(redirect_type, locksmith_settings, &callback_url, code_challenge, &state)?;
    debug!("Authorization url is {}", authorization_url);

    // Ensure external auth urls use https, rather than http.
    let auth_host = authorization_url
        .host_str()
        .ok_or_else(|| anyhow!("Authorization server url must be absolute"))?;
    let auth_scheme = authorization_url.scheme();
    let fallback_port: u16 = if auth_scheme == "https" { 443 } else { 80 };
    let port = authorization_url.port().unwrap_or(fallback_port);
    let is_routable = check_if_routable(format!("{auth_host}:{port}"))?;
    if is_routable && auth_scheme == "http" {
        return Err(anyhow!(
            "Authorization host {auth_host} is publically routable, must use https to connect."
        ));
    }

    // Instruct user on how to complete login.
    eprintln!("Please use browser window to complete login process");
    eprintln!("If browser window doesn't open, you can use the link below:");
    eprintln!("    {authorization_url}");

    // Try automatically opening the browser at the login page.
    if let Err(err) = open::that(authorization_url.as_ref()) {
        debug!("Could not open browser: {err}");
    }

    // Configure server routes.
    let (send_shutdown, mut receive_shutdown) = mpsc::channel(4);
    let state = Arc::new(ServerState {
        oauth2_callback_state: state,
        auth_code: Mutex::new(None),
        shutdown: send_shutdown,
    });
    let router = Router::new().route("/", get(keycloak_callback_handler)).with_state(state.clone());

    // Start server.
    debug!("Starting local login server at {:?}", auth_address);
    let listener = TcpListener::bind(auth_address).await?;
    axum::serve(listener, router)
        .with_graceful_shutdown(async move {
            let _ = receive_shutdown.recv().await;
        })
        .await?;

    let auth_code = state.auth_code.lock().await.take();
    match auth_code {
        Some(auth_code) => Ok((AuthorizationCode::new(auth_code), callback_url)),
        None => Err(anyhow!("Failed to get auth code")),
    }
}

/// Handle the user login/registration flow.
pub async fn handle_auth_flow(
    auth_action: AuthAction,
    token_name: Option<String>,
    expiry: Option<DateTime<Utc>>,
    ignore_certs: bool,
    api_uri: &str,
    port: u16,
) -> Result<RefreshToken> {
    let locksmith_settings = fetch_locksmith_server_settings(ignore_certs, api_uri).await?;
    let (code_verifier, challenge_code) = CodeVerifier::generate(64)?;
    let state: String = thread_rng().sample_iter(&Alphanumeric).take(32).map(char::from).collect();
    let (auth_code, callback_url) = spawn_server_and_get_auth_code(
        &locksmith_settings,
        auth_action,
        &challenge_code,
        state,
        port,
    )
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
    use super::*;
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

        spawn_server_and_get_auth_code(
            &locksmith_settings,
            AuthAction::Login,
            &challenge,
            state,
            6662,
        )
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

        let result = handle_auth_flow(AuthAction::Login, None, None, false, &api_uri, 6663).await?;

        debug!("{:?}", result);

        Ok(())
    }

    #[tokio::test]
    async fn when_started_with_good_configuration_handle_auth_flow_for_register_is_successful(
    ) -> Result<()> {
        let mock_server = build_mock_server().await;
        let api_uri = mock_server.uri();

        let (_verifier, _challenge) =
            CodeVerifier::generate(64).expect("Failed to build PKCE verifier and challenge");

        let result =
            handle_auth_flow(AuthAction::Register, None, None, false, &api_uri, 6664).await?;

        debug!("{:?}", result);

        Ok(())
    }
}
