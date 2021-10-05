/// enables logging statically for any test module this module it is imported into
pub mod logging {

    use static_init::dynamic;

    #[dynamic]
    static mut _LOGGER_INIT: bool = {
        env_logger::init();
        true
    };
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
    use crate::types::*;

    const DUMMY_REFRESH_TOKEN: &str = "DUMMY_REFRESH_TOKEN";
    const DUMMY_ACCESS_TOKEN: &str = "DUMMY_ACCESS_TOKEN";
    const DUMMY_ID_TOKEN: &str = "DUMMY_ID_TOKEN";
    const DUMMY_AUTH_CODE: &str = "DUMMY_AUTH_CODE";

    const OIDC_URI: &str = "oidc";
    const AUTH_URI: &str = "auth";
    const TOKEN_URI: &str = "token";

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

    fn build_oidc_server_settings_mock_response(base_uri: &str) -> OidcServerSettings {
        let base_url = Url::from_str(base_uri).expect("Failed to parse base url");
        OidcServerSettings {
            issuer: base_url.clone(),
            authorization_endpoint: base_url.join(AUTH_URI).unwrap(),
            token_endpoint: base_url.join(TOKEN_URI).unwrap(),
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

    fn build_authenticated_auth_info(mock_server: &MockServer) -> AuthInfo {
        AuthInfo {
            offline_access: Some(RefreshToken::new(DUMMY_REFRESH_TOKEN)),
            oidc_discovery_url: Url::from_str(&format!("{}/{}", mock_server.uri(), OIDC_URI))
                .expect("Failed to parse test url"),
        }
    }

    pub async fn build_phylum_api(mock_server: &MockServer) -> Result<PhylumApi, PhylumApiError> {
        let phylum = PhylumApi::new(
            &mut build_authenticated_auth_info(mock_server),
            mock_server.uri().as_str(),
            None,
        )
        .await?;
        Ok(phylum)
    }
}
