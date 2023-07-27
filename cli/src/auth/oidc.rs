//! This module contains utilities related to building OAuth/OIDC urls
//! and templatized html for redirecting a browser to them

use std::collections::HashMap;
use std::net::ToSocketAddrs;
use std::time::Duration;

use anyhow::{anyhow, Context, Result};
use base64::engine::general_purpose;
use base64::Engine as _;
use maplit::hashmap;
use phylum_types::types::auth::{AccessToken, AuthorizationCode, RefreshToken, TokenResponse};
use rand::distributions::Alphanumeric;
use rand::{thread_rng, Rng};
use reqwest::Url;
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};

use super::ip_addr_ext::IpAddrExt;
use super::is_locksmith_token;
use crate::api::endpoints;
use crate::app::USER_AGENT;

pub const OIDC_SCOPES: [&str; 2] = ["openid", "offline_access"];

/// OIDC Client id used to identify this client to the oidc server
pub const OIDC_CLIENT_ID: &str = "phylum_cli";

#[derive(Debug, PartialEq, Eq, Clone, Copy)]
pub enum AuthAction {
    Login,
    Reauth,
    Register,
}

/// Typed wrapper for PKCE challenge code string
pub struct ChallengeCode(String);

impl From<ChallengeCode> for String {
    fn from(challenge_code: ChallengeCode) -> Self {
        challenge_code.0
    }
}

impl From<&ChallengeCode> for String {
    fn from(challenge_code: &ChallengeCode) -> Self {
        challenge_code.0.clone()
    }
}

impl<'a> From<&'a ChallengeCode> for &'a str {
    fn from(challenge_code: &'a ChallengeCode) -> Self {
        &challenge_code.0
    }
}

/// Typed wrapper for PKCE code verifier string
pub struct CodeVerifier(String);

impl CodeVerifier {
    /// Generate a OIDC PKCE challenge code and verifier
    pub fn generate(length: u8) -> Result<(CodeVerifier, ChallengeCode)> {
        if !(43..=128).contains(&length) {
            return Err(anyhow!("length must be between 43 and 128 inclusive."));
        }
        let code_verifier: String =
            thread_rng().sample_iter(&Alphanumeric).take(length as usize).map(char::from).collect();
        let mut hasher = Sha256::new();
        hasher.update(&code_verifier);
        let hash = hasher.finalize();
        let base_64_url_safe = general_purpose::URL_SAFE_NO_PAD.encode(hash);
        Ok((CodeVerifier(code_verifier), ChallengeCode(base_64_url_safe)))
    }
}

impl From<CodeVerifier> for String {
    fn from(code_verfier: CodeVerifier) -> Self {
        code_verfier.0
    }
}

impl From<&CodeVerifier> for String {
    fn from(code_verfier: &CodeVerifier) -> Self {
        code_verfier.0.clone()
    }
}

impl<'a> From<&'a CodeVerifier> for &'a str {
    fn from(code_verfier: &'a CodeVerifier) -> Self {
        &code_verfier.0
    }
}

/// The public urls and supported features for the given OIDC server.
/// We only deserialize the ones we care about.
#[derive(Debug, Serialize, Deserialize)]
pub struct OidcServerSettings {
    pub issuer: Url,
    pub authorization_endpoint: Url,
    pub token_endpoint: Url,
    pub userinfo_endpoint: Url,
}

/// Locksmith URLs
#[derive(Debug, Serialize, Deserialize)]
pub struct LocksmithServerSettings {
    pub authorization_endpoint: Url,
    pub token_endpoint: Url,
    pub userinfo_endpoint: Url,
}

/// Using config information, build the url for the keycloak login page.
pub fn build_auth_url(
    action: AuthAction,
    oidc_settings: &OidcServerSettings,
    callback_url: &Url,
    code_challenge: &ChallengeCode,
    state: impl AsRef<str>,
) -> Result<Url> {
    let mut auth_url = match action {
        // Login uses the oidc defined /auth endpoint as is
        AuthAction::Login | AuthAction::Reauth => oidc_settings.authorization_endpoint.to_owned(),
        // Register uses the non-standard /registrations endpoint
        AuthAction::Register => {
            let mut auth_url = oidc_settings.authorization_endpoint.to_owned();
            auth_url
                .path_segments_mut()
                .map_err(|_| anyhow!("Can not be base url"))?
                .pop()
                .push("registrations");
            auth_url
        },
    };

    auth_url
        .query_pairs_mut()
        .clear()
        .append_pair("client_id", OIDC_CLIENT_ID)
        .append_pair("code_challenge", code_challenge.into())
        .append_pair("code_challenge_method", "S256")
        .append_pair("redirect_uri", callback_url.as_ref())
        .append_pair("response_type", "code")
        .append_pair("response_mode", "query")
        .append_pair("scope", &OIDC_SCOPES.join(" "))
        .append_pair("state", state.as_ref());

    if action == AuthAction::Reauth {
        auth_url.query_pairs_mut().append_pair("prompt", "login");
    }

    Ok(auth_url)
}

/// Check if an address is routable beyond the local network segment.
pub fn check_if_routable(hostname: impl AsRef<str>) -> Result<bool> {
    let is_routable = hostname
        .as_ref()
        .to_socket_addrs()?
        .map(|socket_addr| socket_addr.ip().is_routable())
        .reduce(|a, b| a | b)
        .unwrap_or(false);
    Ok(is_routable)
}

pub async fn fetch_oidc_server_settings(
    ignore_certs: bool,
    api_uri: &str,
) -> Result<OidcServerSettings> {
    let client = reqwest::Client::builder()
        .user_agent(USER_AGENT.as_str())
        .danger_accept_invalid_certs(ignore_certs)
        .build()?;
    let response = client
        .get(endpoints::oidc_discovery(api_uri)?)
        .header("Accept", "application/json")
        .timeout(Duration::from_secs(5))
        .send()
        .await?;

    if let Err(error) = response.error_for_status_ref() {
        Err(anyhow!(response.text().await?)).context(error)
    } else {
        Ok(response.json::<OidcServerSettings>().await?)
    }
}

pub async fn fetch_locksmith_server_settings(
    ignore_certs: bool,
    api_uri: &str,
) -> Result<LocksmithServerSettings> {
    let client = reqwest::Client::builder()
        .user_agent(USER_AGENT.as_str())
        .danger_accept_invalid_certs(ignore_certs)
        .build()?;
    let response = client
        .get(endpoints::locksmith_discovery(api_uri)?)
        .header("Accept", "application/json")
        .timeout(Duration::from_secs(5))
        .send()
        .await?;

    if let Err(error) = response.error_for_status_ref() {
        Err(anyhow!(response.text().await?)).context(error)
    } else {
        Ok(response.json::<LocksmithServerSettings>().await?)
    }
}

fn build_grant_type_auth_code_post_body(
    redirect_url: &Url,
    authorization_code: &AuthorizationCode,
    code_verfier: &CodeVerifier,
) -> Result<HashMap<String, String>> {
    let body = hashmap! {
        "client_id".to_owned() => OIDC_CLIENT_ID.to_owned(),
        "code".to_owned() => authorization_code.into(),
        "code_verifier".to_owned() => code_verfier.into(),
        "grant_type".to_owned() => "authorization_code".to_owned(),
        // Must match previous request to /authorize but not redirected to by server
        "redirect_uri".to_owned() => redirect_url.to_string(),
    };
    Ok(body)
}

fn build_grant_type_refresh_token_post_body(
    refresh_token: &RefreshToken,
) -> Result<HashMap<String, String>> {
    let body = hashmap! {
        "client_id".to_owned() => OIDC_CLIENT_ID.to_owned(),
        "grant_type".to_owned() => "refresh_token".to_owned(),
        "refresh_token".to_owned() => refresh_token.into(),
    };
    Ok(body)
}

/// Acquire tokens with the auth code
pub async fn acquire_tokens(
    oidc_settings: &OidcServerSettings,
    redirect_url: &Url,
    authorization_code: &AuthorizationCode,
    code_verifier: &CodeVerifier,
    ignore_certs: bool,
) -> Result<TokenResponse> {
    let token_url = oidc_settings.token_endpoint.clone();

    let body =
        build_grant_type_auth_code_post_body(redirect_url, authorization_code, code_verifier)?;

    let client = reqwest::Client::builder()
        .user_agent(USER_AGENT.as_str())
        .danger_accept_invalid_certs(ignore_certs)
        .build()?;
    let response = client
        .post(token_url)
        .header("Accept", "application/json")
        .timeout(Duration::from_secs(5))
        .form(&body)
        .send()
        .await?;

    if let Err(error) = response.error_for_status_ref() {
        let body = response.text().await?;
        let mut err = Err(anyhow!(body.clone())).context(error);

        // Provide additional detail when user requires activation.
        if serde_json::from_str::<ResponseError>(&body)
            .map_or(false, |response| response.error == "not_allowed")
        {
            err = err.context(
                "Your account is not authorized to perform this action. Please contact Phylum \
                 support.",
            );
        }

        err
    } else {
        Ok(response.json::<TokenResponse>().await?)
    }
}

pub async fn refresh_tokens(
    oidc_settings: &OidcServerSettings,
    refresh_token: &RefreshToken,
    ignore_certs: bool,
) -> Result<TokenResponse> {
    let token_url = oidc_settings.token_endpoint.clone();

    let body = build_grant_type_refresh_token_post_body(refresh_token)?;

    let client = reqwest::Client::builder()
        .user_agent(USER_AGENT.as_str())
        .danger_accept_invalid_certs(ignore_certs)
        .build()?;
    let response = client
        .post(token_url)
        .header("Content-Type", "application/json")
        .header("Accept", "application/json")
        .timeout(Duration::from_secs(5))
        .form(&body)
        .send()
        .await?;

    if let Err(error) = response.error_for_status_ref() {
        // Print authentication error reason for the user.
        Err(anyhow!(response.text().await?)).context(error)
    } else {
        Ok(response.json::<TokenResponse>().await?)
    }
}

pub async fn handle_refresh_tokens(
    refresh_token: &RefreshToken,
    ignore_certs: bool,
    api_uri: &str,
) -> Result<AccessToken> {
    // Locksmith tokens are their own access tokens
    if is_locksmith_token(refresh_token) {
        return Ok(AccessToken::new(refresh_token));
    }

    let oidc_settings = fetch_oidc_server_settings(ignore_certs, api_uri).await?;
    refresh_tokens(&oidc_settings, refresh_token, ignore_certs)
        .await
        .map(|token| token.access_token)
}

/// Represents the userdata stored for an authentication token.
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct UserInfo {
    pub email: String,
    pub sub: Option<String>,
    pub name: Option<String>,
    pub given_name: Option<String>,
    pub family_name: Option<String>,
    pub preferred_username: Option<String>,
    pub email_verified: Option<bool>,
}

/// Keycloak error response.
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct ResponseError {
    error: String,
}
