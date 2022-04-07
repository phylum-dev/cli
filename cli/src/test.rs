//! Module containing useful doc / unit test utilities.

/// enables logging statically for any test module this module it is imported into
pub mod logging {

    use lazy_static::lazy_static;

    lazy_static! {
        static ref _LOGGER_INIT: bool = {
            env_logger::init();
            true
        };
    }
}

pub mod mockito {

    use reqwest::Url;
    use std::borrow::Cow;
    use std::collections::HashMap;
    use std::str::FromStr;
    use wiremock::matchers::{method, path};
    use wiremock::{Mock, MockBuilder, MockServer, Request, Respond, ResponseTemplate};

    use crate::api::{PhylumApi, PhylumApiError};
    use crate::auth::OidcServerSettings;
    use crate::config::AuthInfo;
    use phylum_types::types::auth::*;

    pub const DUMMY_REFRESH_TOKEN: &str = "DUMMY_REFRESH_TOKEN";
    pub const DUMMY_ACCESS_TOKEN: &str = "DUMMY_ACCESS_TOKEN";
    pub const DUMMY_ID_TOKEN: &str = "DUMMY_ID_TOKEN";
    pub const DUMMY_AUTH_CODE: &str = "DUMMY_AUTH_CODE";

    pub const OIDC_URI: &str = "oidc";
    pub const AUTH_URI: &str = "auth";
    pub const USER_URI: &str = "user";
    pub const TOKEN_URI: &str = "token";

    pub struct ResponderFn<F>(F)
    where
        F: Fn(&Request) -> ResponseTemplate + Send + Sync;

    impl<F> Respond for ResponderFn<F>
    where
        F: Fn(&Request) -> ResponseTemplate + Send + Sync,
    {
        fn respond(&self, request: &Request) -> ResponseTemplate {
            self.0(request)
        }
    }

    pub trait MockResponderExt {
        fn respond_with_fn<F>(self, function: F) -> Mock
        where
            F: Fn(&Request) -> ResponseTemplate + Send + Sync + 'static;
    }

    impl MockResponderExt for MockBuilder {
        fn respond_with_fn<F>(self, function: F) -> Mock
        where
            F: Fn(&Request) -> ResponseTemplate + Send + Sync + 'static,
        {
            self.respond_with(ResponderFn(function))
        }
    }

    pub fn build_oidc_server_settings_mock_response(base_uri: &str) -> OidcServerSettings {
        let base_url = Url::from_str(base_uri).expect("Failed to parse base url");
        OidcServerSettings {
            issuer: base_url.clone(),
            authorization_endpoint: base_url.join(AUTH_URI).unwrap(),
            token_endpoint: base_url.join(TOKEN_URI).unwrap(),
            userinfo_endpoint: base_url.join(USER_URI).unwrap(),
        }
    }

    pub async fn build_mock_server() -> MockServer {
        let mock_server = MockServer::start().await;
        let base_url = mock_server.uri();

        // Set OIDC Server Settings Response
        Mock::given(method("GET"))
            .and(path(OIDC_URI))
            .respond_with_fn(move |_| {
                let oidc_response = build_oidc_server_settings_mock_response(&base_url);
                ResponseTemplate::new(200).set_body_json(oidc_response)
            })
            .mount(&mock_server)
            .await;

        // Set auth endpoint response
        Mock::given(method("POST"))
            .and(path(AUTH_URI))
            .respond_with_fn(|request| {
                let query_params = request
                    .url
                    .query_pairs()
                    .collect::<HashMap<Cow<str>, Cow<str>>>();
                let redirect_uri = query_params
                    .get("redirect_uri")
                    .expect("redirect_uri not set")
                    .to_string();
                ResponseTemplate::new(302).insert_header::<&str, &str>(
                    "Location",
                    &(redirect_uri + &format!("/?code={}", DUMMY_AUTH_CODE)),
                )
            })
            .mount(&mock_server)
            .await;

        // Set token endpoint response
        Mock::given(method("POST"))
            .and(path(TOKEN_URI))
            .respond_with_fn(|_| {
                ResponseTemplate::new(200).set_body_json(TokenResponse {
                    access_token: AccessToken::new(DUMMY_ACCESS_TOKEN),
                    refresh_token: RefreshToken::new(DUMMY_REFRESH_TOKEN),
                    id_token: IdToken::new(DUMMY_ID_TOKEN),
                    expires_in_seconds: 3600,
                })
            })
            .mount(&mock_server)
            .await;

        mock_server
    }

    pub fn build_authenticated_auth_info(mock_server: &MockServer) -> AuthInfo {
        AuthInfo {
            offline_access: Some(RefreshToken::new(DUMMY_REFRESH_TOKEN)),
            oidc_discovery_url: Url::from_str(&format!("{}/{}", mock_server.uri(), OIDC_URI))
                .expect("Failed to parse test url"),
        }
    }

    pub fn build_unauthenticated_auth_info(mock_server: &MockServer) -> AuthInfo {
        AuthInfo {
            offline_access: None,
            oidc_discovery_url: Url::from_str(&format!("{}/{}", mock_server.uri(), OIDC_URI))
                .expect("Failed to parse test url"),
        }
    }

    pub async fn build_phylum_api(mock_server: &MockServer) -> Result<PhylumApi, PhylumApiError> {
        let phylum = PhylumApi::new(
            &mut build_authenticated_auth_info(mock_server),
            mock_server.uri().as_str(),
            None,
            false,
        )
        .await?;
        Ok(phylum)
    }
}

pub mod open {
    use std::collections::HashMap;
    use std::io::Result;
    use std::str::FromStr;
    use std::time::Duration;

    use reqwest::Url;
    use tokio::runtime::Handle;

    /// Dummy impl of [open::that] which instead of opening a browser for
    /// the url, fetches it after sleeping 100ms
    pub fn that(authorization_url: &str) -> Result<()> {
        let authorization_url = authorization_url.to_owned();

        // Fire and forget
        Handle::current().spawn(async move {
            let url = Url::from_str(&authorization_url).expect("Failed to parse url.");

            let code = "FAKE_OAUTH_ATHORIZATION_CODE";

            let query_params = url
                .query_pairs()
                .into_owned()
                .collect::<HashMap<String, String>>();
            let state = query_params
                .get("state")
                .expect("Failed to find request state on auth url");
            let redirect_uri = query_params
                .get("redirect_uri")
                .expect("Failed to find redirect_uri on auth url");

            let mut callback_url =
                Url::from_str(redirect_uri).expect("Failed to parse redirect_uri");
            callback_url
                .query_pairs_mut()
                .append_pair("code", code)
                .append_pair("state", state);

            log::debug!("Calling callback url: {}", callback_url);

            // Wait for the server to be up
            tokio::time::sleep(Duration::from_millis(100)).await;

            reqwest::get(callback_url)
                .await
                .expect("Failed to get response")
                .text()
                .await
                .expect("Failed to get body");

            log::debug!("Callback complete");
        });
        Ok(())
    }
}
