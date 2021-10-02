//! This module contains utilities related to building OAuth/OIDC urls
//! and templatized html for redirecting a browser to them

use std::collections::HashMap;
use std::net::ToSocketAddrs;
use std::time::Duration;

use anyhow::{anyhow, Result};
use base64;
use maplit::hashmap;
use rand::distributions::Alphanumeric;
use rand::{thread_rng, Rng};
use reqwest::Url;
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};

use crate::async_runtime::block_on;
use crate::config::AuthInfo;
use crate::types::{AuthorizationCode, RefreshToken, TokenResponse};

use super::ip_addr_ext::IpAddrExt;

pub const OIDC_SCOPES: [&str; 4] = ["openid", "offline_access", "profile", "email"];

/// OIDC Client id used to identify this client to the oidc server
pub const OIDC_CLIENT_ID: &str = "phylum_cli";

pub enum AuthAction {
    Login,
    Register,
}

/// Typed wrapper for PKCE challenge code string
pub struct ChallengeCode(String);

impl Into<String> for &ChallengeCode {
    fn into(self) -> String {
        self.0.to_owned()
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
        let code_verifier: String = thread_rng()
            .sample_iter(&Alphanumeric)
            .take(length as usize)
            .map(char::from)
            .collect();
        let mut hasher = Sha256::new();
        hasher.update(&code_verifier);
        let hash = hasher.finalize();
        let base_64_url_safe = base64::encode_config(&hash, base64::URL_SAFE_NO_PAD);
        Ok((CodeVerifier(code_verifier), ChallengeCode(base_64_url_safe)))
    }
}

impl Into<String> for &CodeVerifier {
    fn into(self) -> String {
        self.0.to_owned()
    }
}

/// The public urls and supported features for the given OIDC server.
/// We only deserialize the ones we care about.
#[derive(Debug, Serialize, Deserialize)]
pub struct OidcServerSettings {
    pub issuer: Url,
    pub authorization_endpoint: Url,
    pub token_endpoint: Url,
}

/// Using config information, build the url for the keycloak login page.
pub fn build_auth_url(
    action: &AuthAction,
    oidc_settings: &OidcServerSettings,
    callback_url: &Url,
    code_challenge: &ChallengeCode,
    state: impl AsRef<str>,
) -> Result<Url> {
    let mut auth_url = match *action {
        // Login uses the oidc defined /auth endpoint as is
        AuthAction::Login => oidc_settings.authorization_endpoint.to_owned(),
        // Register uses the non-standard /registrations endpoint
        AuthAction::Register => {
            let mut auth_url = oidc_settings.authorization_endpoint.to_owned();
            auth_url
                .path_segments_mut()
                .map_err(|_| anyhow!("Can not be base url"))?
                .pop()
                .push("registrations");
            auth_url
        }
    };

    auth_url
        .query_pairs_mut()
        .clear()
        .append_pair("client_id", OIDC_CLIENT_ID)
        .append_pair("code_challenge", &(Into::<String>::into(code_challenge)))
        .append_pair("code_challenge_method", "S256")
        .append_pair("redirect_uri", &callback_url.to_string())
        .append_pair("response_type", "code")
        .append_pair("response_mode", "fragment")
        .append_pair("scope", &OIDC_SCOPES.join(""))
        .append_pair("state", state.as_ref());

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

pub async fn fetch_oidc_server_settings(auth_info: &AuthInfo) -> Result<OidcServerSettings> {
    let client = reqwest::Client::new();
    let response = client
        .get(auth_info.oidc_discovery_url.clone())
        .header("Accept", "application/json")
        .timeout(Duration::from_secs(5))
        .send()
        .await?
        .json::<OidcServerSettings>()
        .await?;
    Ok(response)
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
        "scopes".to_owned() => OIDC_SCOPES.join(" ")
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
        "scopes".to_owned() => OIDC_SCOPES.join(" ")
    };
    Ok(body)
}

/// Acquire tokens with the auth code
pub async fn acquire_tokens(
    oidc_settings: &OidcServerSettings,
    redirect_url: &Url,
    authorization_code: &AuthorizationCode,
    code_verifier: &CodeVerifier,
) -> Result<TokenResponse> {
    let token_url = oidc_settings.token_endpoint.clone();

    let body =
        build_grant_type_auth_code_post_body(redirect_url, authorization_code, code_verifier)?;

    let client = reqwest::Client::new();
    let response = client
        .post(token_url)
        .header("Content-Type", "application/json")
        .header("Accept", "application/json")
        .timeout(Duration::from_secs(5))
        .form(&body)
        .send()
        .await?
        .json::<TokenResponse>()
        .await?;
    Ok(response)
}

pub async fn refresh_tokens(
    oidc_settings: &OidcServerSettings,
    refresh_token: &RefreshToken,
) -> Result<TokenResponse> {
    let token_url = oidc_settings.token_endpoint.clone();

    let body = build_grant_type_refresh_token_post_body(refresh_token)?;

    let client = reqwest::Client::new();
    let response = client
        .post(token_url)
        .header("Content-Type", "application/json")
        .header("Accept", "application/json")
        .timeout(Duration::from_secs(5))
        .form(&body)
        .send()
        .await?
        .json::<TokenResponse>()
        .await?;
    Ok(response)
}

pub fn handle_refresh_tokens(
    auth_info: &AuthInfo,
    refresh_token: &RefreshToken,
) -> Result<TokenResponse> {
    block_on(async {
        let oidc_settings = fetch_oidc_server_settings(auth_info).await?;
        let tokens = refresh_tokens(&oidc_settings, refresh_token).await?;
        Result::<TokenResponse, anyhow::Error>::Ok(tokens)
    })
}
